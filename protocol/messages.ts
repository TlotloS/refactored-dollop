// Wire protocol shared by the signaling server, Windows host, and iPad client.
// Signaling messages travel as JSON over WebSocket.
// Input messages (iPad -> Windows) travel as CBOR over the WebRTC DataChannel
// named "input"; see InputEvent.

// ---------- Signaling (WebSocket) ----------

export type SignalingMessage =
  | { kind: "hello"; role: "host" | "viewer"; roomCode: string; clientId: string }
  | { kind: "joined"; roomCode: string; peerPresent: boolean }
  | { kind: "peer-joined" }
  | { kind: "peer-left" }
  | { kind: "offer"; sdp: string }
  | { kind: "answer"; sdp: string }
  | { kind: "ice"; candidate: RTCIceCandidateInitLike }
  | { kind: "error"; code: ErrorCode; message: string };

export interface RTCIceCandidateInitLike {
  candidate: string;
  sdpMid?: string | null;
  sdpMLineIndex?: number | null;
  usernameFragment?: string | null;
}

export type ErrorCode =
  | "room_full"
  | "bad_message"
  | "peer_gone"
  | "rate_limited"
  | "internal";

// ---------- Input back-channel (CBOR over DataChannel) ----------

export type InputEvent = PointerEvent | KeyEvent | ScrollEvent;

export interface PointerEvent {
  t: "pointer";
  // Stable per pointer (finger / pencil). Phase "down" introduces an id;
  // "up" or "cancel" retires it.
  id: number;
  phase: "down" | "move" | "up" | "cancel";
  // Normalized to the target monitor rect, top-left origin, [0,1].
  x: number;
  y: number;
  // Pencil-only. Touch sets pressure=0, tilt=0.
  pressure: number; // 0..1
  tiltX: number;    // degrees, -90..90
  tiltY: number;    // degrees, -90..90
  tool: "touch" | "pencil" | "mouse";
  // Client monotonic timestamp in microseconds.
  tUs: number;
}

export interface KeyEvent {
  t: "key";
  // USB HID usage code from page 0x07 (Keyboard / Keypad), read directly
  // off UIKey on iOS. The host maps usage -> VK/scancode via the active
  // KLID; PS/2 set 1 is NOT used on the wire because it loses AltGr,
  // dead keys, and non-US layouts.
  hidUsage: number;
  // `UIKey.charactersIgnoringModifiers`. Lets the host fall back to
  // `KEYEVENTF_UNICODE` for IME / dead keys / AltGr-only glyphs.
  chars: string;
  phase: "down" | "up";
  // Optional repeat marker for autorepeat from the iPad keyboard.
  repeat?: boolean;
  tUs: number;
}

export interface ScrollEvent {
  t: "scroll";
  // Lines, not pixels. Positive dy = scroll down.
  dx: number;
  dy: number;
  phase: "begin" | "update" | "end";
  tUs: number;
}
