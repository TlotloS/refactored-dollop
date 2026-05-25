// Manual smoke test: two clients join a room, exchange an "offer"/"answer".
// Run against a running server on PORT (default 18443).
import WebSocket from "ws";

const PORT = process.env.PORT ?? 18443;
const URL = `ws://127.0.0.1:${PORT}/ws`;

const received = { host: [], viewer: [] };

function open(role) {
  const ws = new WebSocket(URL);
  return new Promise((resolve, reject) => {
    ws.on("open", () => {
      ws.send(JSON.stringify({ kind: "hello", role, roomCode: "TEST1234", clientId: role + "-id" }));
      resolve(ws);
    });
    ws.on("message", (raw) => {
      const msg = JSON.parse(raw.toString());
      received[role].push(msg);
    });
    ws.on("error", reject);
  });
}

const host = await open("host");
const viewer = await open("viewer");

// Let "joined" / "peer-joined" land.
await new Promise((r) => setTimeout(r, 100));

host.send(JSON.stringify({ kind: "offer", sdp: "v=0\nfake-offer" }));
await new Promise((r) => setTimeout(r, 50));
viewer.send(JSON.stringify({ kind: "answer", sdp: "v=0\nfake-answer" }));
await new Promise((r) => setTimeout(r, 50));

host.close();
viewer.close();
await new Promise((r) => setTimeout(r, 100));

const assert = (cond, label) => {
  if (!cond) {
    console.error("FAIL:", label, JSON.stringify(received, null, 2));
    process.exit(1);
  }
  console.log("ok  ", label);
};

assert(received.host.some((m) => m.kind === "joined"), "host got joined");
assert(received.viewer.some((m) => m.kind === "joined"), "viewer got joined");
assert(received.host.some((m) => m.kind === "peer-joined"), "host got peer-joined");
assert(received.viewer.some((m) => m.kind === "offer" && m.sdp.includes("fake-offer")), "viewer received forwarded offer");
assert(received.host.some((m) => m.kind === "answer" && m.sdp.includes("fake-answer")), "host received forwarded answer");

console.log("\nall good");
process.exit(0);
