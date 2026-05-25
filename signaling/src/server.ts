import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";
import { extname, join, normalize, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { WebSocketServer, type WebSocket } from "ws";
import type { SignalingMessage, ErrorCode } from "../../protocol/messages.js";
import { RoomRegistry, type Peer } from "./room.js";
import { log } from "./log.js";

const PORT = Number(process.env.PORT ?? 8443);
const HOST = process.env.HOST ?? "0.0.0.0";
const PUBLIC_DIR = join(fileURLToPath(new URL(".", import.meta.url)), "..", "public");

const rooms = new RoomRegistry();

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
};

const http = createServer(async (req: IncomingMessage, res: ServerResponse) => {
  if (req.url === "/healthz") {
    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify({ ok: true, rooms: rooms.count() }));
    return;
  }
  // Static file serving from signaling/public/.
  const rawPath = req.url === "/" || !req.url ? "/viewer.html" : req.url.split("?")[0]!;
  const normalized = normalize(rawPath).replace(/^(\.\.[/\\])+/, "");
  if (normalized.startsWith("..") || normalized.includes(`..${sep}`)) {
    res.writeHead(403);
    res.end();
    return;
  }
  const filePath = join(PUBLIC_DIR, normalized);
  try {
    const data = await readFile(filePath);
    res.writeHead(200, { "content-type": MIME[extname(filePath)] ?? "application/octet-stream" });
    res.end(data);
  } catch {
    res.writeHead(404);
    res.end();
  }
});

const wss = new WebSocketServer({ server: http, path: "/ws" });

wss.on("connection", (ws: WebSocket, req: IncomingMessage) => {
  const ip = req.socket.remoteAddress ?? "?";
  let registered: { room: string; clientId: string } | null = null;

  log("info", "ws_connect", { ip });

  ws.on("message", (raw) => {
    let msg: SignalingMessage;
    try {
      msg = JSON.parse(raw.toString()) as SignalingMessage;
    } catch {
      sendError(ws, "bad_message", "json parse failed");
      return;
    }

    if (!registered) {
      // Only "hello" is accepted before registration.
      if (msg.kind !== "hello") {
        sendError(ws, "bad_message", "expected hello");
        return;
      }
      handleHello(ws, msg);
      if (ws.readyState === ws.OPEN && (ws as WsWithMeta).__room) {
        const meta = ws as WsWithMeta;
        registered = { room: meta.__room!, clientId: meta.__clientId! };
      }
      return;
    }

    // Forward signaling-relay messages to the other peer.
    if (msg.kind === "offer" || msg.kind === "answer" || msg.kind === "ice") {
      const room = rooms.get(registered.room);
      const other = room?.other(registered.clientId);
      if (!other) {
        sendError(ws, "peer_gone", "no peer in room");
        return;
      }
      send(other.ws, msg);
      return;
    }

    sendError(ws, "bad_message", `unexpected kind ${msg.kind}`);
  });

  ws.on("close", () => {
    if (!registered) return;
    const room = rooms.get(registered.room);
    if (!room) return;
    const removed = room.remove(registered.clientId);
    if (removed) {
      const other = room.other(registered.clientId);
      if (other) send(other.ws, { kind: "peer-left" });
    }
    if (room.isEmpty()) rooms.delete(room.code);
    log("info", "ws_close", { room: registered.room, clientId: registered.clientId });
  });
});

interface WsWithMeta extends WebSocket {
  __room?: string;
  __clientId?: string;
}

function handleHello(ws: WebSocket, msg: Extract<SignalingMessage, { kind: "hello" }>) {
  if (!msg.roomCode || msg.roomCode.length < 4 || msg.roomCode.length > 32) {
    sendError(ws, "bad_message", "roomCode length must be 4..32");
    ws.close();
    return;
  }
  if (msg.role !== "host" && msg.role !== "viewer") {
    sendError(ws, "bad_message", "role must be host or viewer");
    ws.close();
    return;
  }

  const room = rooms.getOrCreate(msg.roomCode);
  const clientId = msg.clientId || randomUUID();
  const peer: Peer = { ws, role: msg.role, clientId };
  const result = room.add(peer);

  if (result === "full" || result === "role_taken") {
    sendError(ws, "room_full", result);
    ws.close();
    return;
  }

  (ws as WsWithMeta).__room = room.code;
  (ws as WsWithMeta).__clientId = clientId;

  const peerPresent = room.size() === 2;
  send(ws, { kind: "joined", roomCode: room.code, peerPresent });

  const other = room.other(clientId);
  if (other) send(other.ws, { kind: "peer-joined" });

  log("info", "ws_joined", { room: room.code, role: msg.role, clientId });
}

function send(ws: WebSocket, msg: SignalingMessage): void {
  if (ws.readyState !== ws.OPEN) return;
  ws.send(JSON.stringify(msg));
}

function sendError(ws: WebSocket, code: ErrorCode, message: string): void {
  send(ws, { kind: "error", code, message });
}

http.listen(PORT, HOST, () => {
  log("info", "listening", { host: HOST, port: PORT, path: "/ws" });
});

process.on("SIGINT", () => {
  log("info", "shutdown");
  wss.close(() => http.close(() => process.exit(0)));
});
