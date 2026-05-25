import type { WebSocket } from "ws";

export interface Peer {
  ws: WebSocket;
  role: "host" | "viewer";
  clientId: string;
}

// A Room is a rendezvous bucket. v1 = exactly two peers max:
// one host (Windows) and one viewer (iPad).
export class Room {
  readonly code: string;
  readonly createdAt = Date.now();
  private peers = new Map<string, Peer>();

  constructor(code: string) {
    this.code = code;
  }

  size(): number {
    return this.peers.size;
  }

  has(role: "host" | "viewer"): boolean {
    for (const p of this.peers.values()) {
      if (p.role === role) return true;
    }
    return false;
  }

  add(peer: Peer): "ok" | "role_taken" | "full" {
    if (this.peers.size >= 2) return "full";
    if (this.has(peer.role)) return "role_taken";
    this.peers.set(peer.clientId, peer);
    return "ok";
  }

  remove(clientId: string): Peer | undefined {
    const p = this.peers.get(clientId);
    if (p) this.peers.delete(clientId);
    return p;
  }

  peer(clientId: string): Peer | undefined {
    return this.peers.get(clientId);
  }

  // Returns the *other* peer in the room, if any.
  other(clientId: string): Peer | undefined {
    for (const p of this.peers.values()) {
      if (p.clientId !== clientId) return p;
    }
    return undefined;
  }

  isEmpty(): boolean {
    return this.peers.size === 0;
  }
}

export class RoomRegistry {
  private rooms = new Map<string, Room>();

  getOrCreate(code: string): Room {
    let r = this.rooms.get(code);
    if (!r) {
      r = new Room(code);
      this.rooms.set(code, r);
    }
    return r;
  }

  get(code: string): Room | undefined {
    return this.rooms.get(code);
  }

  delete(code: string): void {
    this.rooms.delete(code);
  }

  count(): number {
    return this.rooms.size;
  }
}
