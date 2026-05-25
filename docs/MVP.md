# MVP 0.1 — Scope Lock

Status: Approved 2026-05-25.
Supersedes §1.2 of DESIGN.md for what ships first.
This document is the contract. Anything not listed here is out of scope
for MVP and must be pushed to v0.2+.

---

## 1. The one-line pitch

> A Windows PC extends to an iPad as a real second monitor over Wi-Fi,
> with touch input working.

That is all v0.1 has to do. It is **not** "better than Spacedesk yet" —
the differentiator (Apple Pencil pressure → Windows Ink) lands in v0.2.

---

## 2. In scope (must ship)

### 2.1 Windows host

- Runs on **Windows 10 21H2+ or Windows 11**, x64 only.
- An **IddCx user-mode virtual display driver** that appears as a real
  second monitor in Display Settings.
- Fixed mode: **1920×1200 @ 60 Hz**. No resolution picker, no mode
  negotiation.
- **Media Foundation H.264** encode only. No NVENC / Quick Sync / AMF
  probing — universal fallback is the only encoder.
- **DXGI Desktop Duplication** captures the virtual monitor's framebuffer
  and feeds the encoder. (In v0.1 the driver and the capture both run in
  the same `winext-host.exe`; we cross the process boundary in v0.2.)
- **Cross-platform `webrtc` Rust crate** for transport. Single video
  track outbound; single `RTCDataChannel` inbound named `input`.
- **Test-mode-signed driver only.** Users enable `bcdedit /set testsigning on`
  to install. We do not buy the EV cert until v0.2.
- Run from the command line (`winext-host.exe --signaling ws://... --room CODE`).
  No installer, no service, no system-tray UI.

### 2.2 iPad client

- Runs on **iPadOS 16+**.
- SwiftUI room-code-entry screen. Type signaling URL + room code, hit Connect.
- WebRTC.xcframework, VideoToolbox H.264 decode, Metal renderer.
- **ProMotion 120 Hz** display path (capped to 60 Hz inbound for v0.1
  since the host sends 60 fps, but the present pipeline is ready).
- **Touch = single-finger mouse**:
  - Touch down → mouse left-button down at the projected position.
  - Touch move → mouse move.
  - Touch up → mouse left-button up.
  - Tap = a down/up pair with no movement.
  - No multi-touch. No pinch. No two-finger scroll. No right-click gesture in v0.1.
- **Hardware keyboard pass-through** via `UIKeyCommand` / `pressesBegan`/`pressesEnded`.
  Magic Keyboard or any Bluetooth keyboard. Maps to PS/2 scancodes on the wire.
- Distributed via **Xcode sideload** to one paired iPad. No TestFlight, no App Store.

### 2.3 Signaling server

- The Node 20 + TypeScript WebSocket server already scaffolded under
  `signaling/`. Hardcoded room codes (no SPAKE pairing). Two peers per room.
- Runs on the same machine as the Windows host (or any LAN host the
  user wants). No TLS in v0.1 — `ws://`, not `wss://`. LAN-only is the
  threat model.

### 2.4 Acceptance criteria — Definition of Done

The MVP is **done** when, on a single test rig, all six of these are true:

1. **Detection**: Windows Display Settings shows a second monitor named
   "WinExt iPad" while the iPad is connected.
2. **Drag**: a Windows app window can be dragged onto the second monitor
   and stays positioned there across reconnects within a single session.
3. **Latency**: glass-to-glass cursor motion latency, measured by filming
   the iPad and the source PC at 240 fps, is **under 100 ms median** on
   a quiet 5 GHz Wi-Fi network at 1920×1200 60 Hz.
4. **Input round-trip**: tapping a button rendered on the iPad clicks
   that button in the Windows app; typing on the Magic Keyboard appears
   in the focused window.
5. **Stability**: a **30-minute continuous session** without
   disconnects, BSODs, or driver crashes.
6. **Recovery**: pulling Wi-Fi for 10 s and restoring it reconnects
   automatically without restarting either side.

If any of those six fails, MVP is not done. If all six pass plus extras
work, we still call it MVP and ship; extras become v0.2 features.

---

## 3. Out of scope for MVP (deferred)

Listed so we don't argue about them mid-build.

### 3.1 Deferred to v0.2 (next release after MVP)

| Item                                              | Why it's not in MVP                                            |
|---------------------------------------------------|----------------------------------------------------------------|
| **Apple Pencil pressure → Windows Ink**           | The wedge. v0.2 headline feature.                              |
| Multi-touch (pinch, two-finger scroll, secondary tap) | Needs gesture FSM; design before code.                      |
| Trackpad pointer (Magic Keyboard trackpad)        | Needs `UIPointerInteraction` plumbing.                         |
| NVENC / Quick Sync / AMF                          | Performance, not capability. MF works everywhere.              |

### 3.2 Deferred to v0.3+

| Item                                              | Why                                                            |
|---------------------------------------------------|----------------------------------------------------------------|
| mDNS / Bonjour discovery                          | Room codes are fine for v0.1/v0.2 single-user.                 |
| SPAKE2 PIN pairing                                | Same.                                                          |
| TURN / internet relay                             | LAN-only is the v0.1 threat model.                             |
| Multi-monitor (more than one iPad simultaneously) | Doubles testing matrix.                                        |
| DPI scaling settings                              | Single fixed mode in MVP.                                      |
| Refresh-rate negotiation (120 Hz host->iPad)      | Pin to 60 Hz in MVP.                                           |
| Audio forwarding                                  | Separate virtual audio driver, signed separately.              |
| Stage Manager / multi-window on iPad              | UI complexity.                                                 |
| HDR / P3 / BT.2020                                | sRGB only in MVP.                                              |

### 3.3 Deferred to v1.0 (first public release)

| Item                                              | Why                                                            |
|---------------------------------------------------|----------------------------------------------------------------|
| EV code-signing cert + attestation signing        | $300+/yr, ~2 weeks lead time. Worth it only once we ship.      |
| MSIX installer for Windows                        | Pointless until the driver is attestation-signed.              |
| TestFlight distribution                           | Requires Apple Developer account ($99/yr).                     |
| App Store submission                              | Review cycle; do after TestFlight beta.                        |
| Telemetry, crash reporting                        | Defer until we have real users.                                |
| Public docs / website                             | Same.                                                          |

---

## 4. Build order (sequencing inside the MVP)

We don't build the modules in module order — we build in the order that
gives us a working demo earliest, then improves it. Each milestone is a
demoable artifact, not a code-org checkpoint.

| Milestone | What works at the end                                                            | Est.        |
|-----------|----------------------------------------------------------------------------------|-------------|
| **M0**    | Signaling server live (already done). iPad client builds in Xcode. Rust host `cargo check` clean. | done       |
| **M1**    | Hard-coded PNG streamed from Windows host to iPad via WebRTC and rendered to Metal. No real capture, no encode, no input. Proves the WebRTC plumbing. | 1 wk |
| **M2**    | DXGI Desktop Duplication captures the **primary** monitor; Media Foundation H.264 encodes; the iPad displays the live primary desktop (mirror, not extend). | 1.5 wk |
| **M3**    | IddCx virtual monitor lands. Windows now sees a second monitor; DDA captures **that** virtual monitor instead of the primary. We are now extending. | 1.5 wk |
| **M4**    | Touch on iPad moves the Windows cursor and clicks. Hardware keyboard types. | 1 wk |
| **M5**    | Polish: reconnect logic, the six acceptance criteria pass on the test rig. | 0.5 wk |

**Total MVP estimate: ~5.5 weeks of focused engineering.** No buffer for
Windows driver weirdness — add 50% if you've never written an IddCx driver
before, which is normal.

---

## 5. Test rig (what we develop and demo on)

- Windows: one x64 PC with an NVIDIA, Intel, or AMD GPU. Test mode enabled.
- iPad: any model from iPad (10th gen) / iPad Air 4 / iPad Pro 11" (3rd gen)
  or newer running iPadOS 16+. ProMotion not required for MVP.
- Network: a single 5 GHz Wi-Fi AP, both devices on the same /24, no isolation.
- Mac: required for building the iPad client (Xcode).

If you don't have a Mac, MVP cannot ship — Apple's signing chain has no
escape hatch. Borrow one or rent a cloud Mac (MacStadium / MacInCloud)
during M0/M1 to set up the project, then sporadically for builds.

---

## 6. Explicit non-goals (things we will say no to during MVP)

If any of the following are requested during MVP build, the answer is
**"yes, v0.2"**. Documented here so we don't waver.

- "Can we add Pencil pressure?" → v0.2.
- "Can we add pinch-to-zoom?" → v0.2.
- "Can we support two iPads at once?" → v0.3.
- "Can we add a system-tray UI on Windows?" → v0.3.
- "Can we ship over the internet (not just LAN)?" → v0.3.
- "Can we add an installer?" → v1.0 (post-signing).
- "Can we support Linux as a host?" → not on the roadmap; raise as a
  separate proposal after v1.0.

---

## 7. What "ship" means for MVP

We do **not** ship the MVP to the public. MVP = a working build on a
single test rig that proves the architecture and exits the "could this
even work" risk. The first build we share with anyone outside this
project is v0.2 (Pencil pressure) at the earliest, and even then only
as a private link to friendly testers — not on TestFlight, not on the
App Store, not on a website.

The public release is **v1.0**, gated on:
- EV cert + attestation-signed driver.
- TestFlight or App Store distribution for iPad.
- Pencil pressure (v0.2).
- One basic mouse gesture set beyond single-finger (v0.2).
- Bonfire test: 5 strangers install it without us in the room.

That is months out. MVP is weeks out. Stay focused on weeks.
