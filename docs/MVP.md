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
- An **IddCx 1.4 user-mode virtual display driver** (UMDF, hosted by
  `WUDFHost.exe` under `IddCxClass`) that appears as a real second
  monitor in Display Settings.
- Fixed mode: **1920×1200 @ 60 Hz**. No resolution picker, no mode
  negotiation.
- **Frame source = the IddCx swap-chain**, consumed in
  `EVT_IDD_CX_MONITOR_ASSIGN_SWAPCHAIN` via `AcquireBuffer`. No DXGI
  Desktop Duplication on the virtual monitor (it would be redundant and
  racy). DDA is used **only** during M2 to mirror the *primary* monitor
  before the driver lands, then deleted.
- Cross-process: driver IPCs **shared D3D11 texture handles**
  (`IDXGIResource1::CreateSharedHandle` + `DuplicateHandle`) to
  `winext-host.exe` over a named pipe (`\\.\pipe\winext-frames`).
  Matches DESIGN.md §4.1.
- **Media Foundation H.264** is the only mandatory encoder
  (`CLSID_MSH264EncoderMFT` with `CODECAPI_AVLowLatencyMode = TRUE`, CBR,
  no B-frames). An **optional NVENC fast path** is probed at startup; if
  present, used; if absent, MF. No MVP feature gates on the fast path.
- **Cross-platform `webrtc` Rust crate** for transport. Single H.264
  video track outbound (Constrained Baseline + Main, `packetization-mode=1`
  — see M1.5); single `RTCDataChannel` inbound named `input`.
- **Test-mode-signed driver only.** Users enable
  `bcdedit /set testsigning on` and install a self-signed CA into both
  `Trusted Root Certification Authorities` and `Trusted Publishers`. We
  do not buy the EV cert until v1.0. See §5 for the dev-box prep
  checklist this implies.
- **Network bind**: explicitly bind the WebRTC and signaling sockets to
  the LAN-facing NIC. Stock dev machines have Hyper-V / WSL2 virtual
  adapters that take precedence on default enumeration; the iPad cannot
  reach those.
- Run from the command line (`winext-host.exe --signaling ws://... --room CODE`).
  No installer, no service, no system-tray UI.
- **Tooling pinned**: WDK 10.0.22621+, MSVC 2022, Rust target
  `x86_64-pc-windows-msvc` (already pinned in `rust-toolchain.toml`).

### 2.2 iPad client

- Runs on **iPadOS 16+**.
- SwiftUI screen with **mDNS host browser** (`NetServiceBrowser` for
  `_winext._tcp.local`). Tap a discovered host, enter the room code, hit
  Connect. Manual URL entry is the fallback, not the default.
- WebRTC.xcframework, VideoToolbox H.264 decode, Metal renderer.
- **ProMotion 120 Hz** display path (capped to 60 Hz inbound for v0.1
  since the host sends 60 fps, but the present pipeline is ready).
- **Touch input**:
  - **Single-finger** → mouse left-button:
    - Touch down → `MOUSEEVENTF_LEFTDOWN` at the projected position.
    - Touch move → mouse move.
    - Touch up → `MOUSEEVENTF_LEFTUP`.
    - Tap = a down/up pair with no movement.
  - **Two-finger tap** → mouse right-button down+up
    (`MOUSEEVENTF_RIGHTDOWN/UP`).
  - **Two-finger pan** → mouse wheel
    (`UIPanGestureRecognizer` w/ `numberOfTouchesRequired = 2`, vertical
    delta normalized to lines, sent as `MOUSEEVENTF_WHEEL`; horizontal
    delta sent as `MOUSEEVENTF_HWHEEL`).
  - No pinch. No three-finger gestures. No trackpad pointer.
- Hardware-keyboard pass-through is **deferred to v0.2** alongside Pencil
  pressure and the rest of the input richness.
- **Apple Developer Program account ($99/yr) required** even for MVP:
  free-tier sideload profiles expire every 7 days, which would brick
  AC #4 / AC #5 on day 8 of testing.
- Distributed via **Xcode sideload** to one paired iPad. No TestFlight,
  no App Store.

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
3. **Latency**: end-to-end input-to-render latency satisfies
   **p50 ≤ 120 ms and p95 ≤ 180 ms** on a quiet 5 GHz Wi-Fi network at
   1920×1200 60 Hz. Measurement is **not** visual; the host emits a
   monotonic timestamp `t_input_at_host` on each pointer event it
   injects, and the iPad logs `t_render_on_ipad` when the resulting
   frame draws. The delta is the metric. Encoder choice is whatever the
   AC rig probes (MF or optional NVENC); the AC must pass on at least
   one configuration of the test rig.
4. **Input round-trip**: tapping a button rendered on the iPad clicks
   that button in the Windows app. Two-finger tap right-clicks. Two-finger
   pan scrolls a long document. (Hardware keyboard is **not** tested in
   MVP — deferred to v0.2.)
5. **Usable as a monitor**: a tester reads a 10-page PDF and watches a
   5-minute 1080p video on the iPad without reaching for the PC — no
   pause, no swap, no fall-back to the primary monitor.
6. **Stability**: a **10-minute** continuous session without
   disconnects, BSODs, or driver crashes.

If any of those six fails, MVP is not done. If all six pass plus extras
work, we still call it MVP and ship; extras become v0.2 features.

Auto-recovery from a Wi-Fi blip (ICE restart + SDP renegotiation) is
**not** an MVP acceptance criterion — deferred to v0.2. A manual
"Reconnect" button in the iPad UI is the MVP behaviour.

---

## 3. Out of scope for MVP (deferred)

Listed so we don't argue about them mid-build.

### 3.1 Deferred to v0.2 (next release after MVP)

| Item                                              | Why it's not in MVP                                            |
|---------------------------------------------------|----------------------------------------------------------------|
| **Apple Pencil pressure → Windows Ink**           | The wedge. v0.2 headline feature.                              |
| Pinch-to-zoom + three-finger gestures             | Needs gesture FSM; design before code.                         |
| Trackpad pointer (Magic Keyboard trackpad)        | Needs `UIPointerInteraction` plumbing.                         |
| Hardware-keyboard pass-through                    | USB HID scancode map + dead-key / AltGr handling; lands next to gestures. |
| Auto-recovery from Wi-Fi blip                     | ICE restart + SDP renegotiation; manual reconnect is fine for MVP. |
| Quick Sync / AMF probing                          | NVENC fast path is in MVP (optional); Intel/AMD parity is v0.2. |

### 3.2 Deferred to v0.3+

| Item                                              | Why                                                            |
|---------------------------------------------------|----------------------------------------------------------------|
| SPAKE2 PIN pairing                                | Room codes are fine for v0.1/v0.2 single-user.                 |
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
| **M1**    | Hard-coded test pattern (via a custom `VideoTrackSource` — you cannot push raw PNG into an `RTCVideoTrack`) streamed from Windows host to iPad over WebRTC and rendered to Metal. No real capture, no encode, no input. Proves WebRTC plumbing on both ends. | 1 wk |
| **M1.5** | **Interop smoke test.** `webrtc-rs` ↔ `WebRTC.xcframework` negotiate H.264 Constrained Baseline + Main with `packetization-mode=1` and transport-cc; iPad renders a real-encoded frame, not black. If this fails, transport plan changes before any further work. | 0.5 wk |
| **M2**    | DXGI Desktop Duplication captures the **primary** monitor; Media Foundation H.264 encodes; iPad displays the live primary desktop (mirror, not extend). NVENC fast path probed and used if present. | 1.5 wk |
| **M3**    | Touch + two-finger gestures move the Windows cursor, click, right-click, scroll — on the mirrored primary desktop. mDNS host discovery on the iPad works. (Demoable end-to-end *without* the driver.) | 1.5 wk |
| **M4**    | IddCx virtual monitor lands. UMDF driver in `WUDFHost` consumes its own swap-chain and IPCs textures to `winext-host.exe`. The system swaps from mirroring the primary to extending onto the virtual monitor. DDA deleted. | 2 wk |
| **M5**    | Polish: bind-to-LAN NIC, manual-reconnect button, six acceptance criteria pass. | 0.5 wk |

**Total MVP estimate: ~7 weeks realistic, ~9 weeks safe.** The previous
5.5 wk figure underbudgeted the IddCx UMDF + IPC work, the WebRTC iOS
interop smoke test, and the dev-box prep in §5.

The milestone order is deliberate: **M3 (input + gestures + discovery)
is before M4 (driver)** so an IddCx schedule slip cannot block a
demoable system. Mirror + input is a working demo. Extend without input
is not.

---

## 5. Test rig (what we develop and demo on)

### Hardware

- **Windows**: one x64 PC with an NVIDIA, Intel, or AMD GPU. NVIDIA
  recommended so the optional NVENC fast path covers AC #3.
- **iPad**: any model from iPad (10th gen) / iPad Air 4 / iPad Pro 11"
  (3rd gen) or newer running iPadOS 16+. ProMotion not required for MVP.
- **Network**: a single 5 GHz Wi-Fi AP, both devices on the same /24,
  no AP isolation. Avoid DFS channels for the AC #3 latency run —
  channel changes drop UDP for ~1 s.
- **Mac**: required for building the iPad client (Xcode). If you don't
  have one, MVP cannot ship — Apple's signing chain has no escape hatch.
  Borrow or rent a cloud Mac (MacStadium / MacInCloud).

### Windows dev-box prep (mandatory; doing this wrong = M2 never starts)

1. **Secure Boot OFF** in firmware. Otherwise `bcdedit /set testsigning on`
   fails with `0xC0000428` and the driver does not load.
2. **HVCI / Memory Integrity OFF** (Settings → Windows Security → Device
   security → Core isolation). Default ON on stock Win11 22H2+ OEM
   machines; blocks unsigned UMDF drivers even in test mode.
3. Generate a self-signed code-signing CA and install it into **both**
   `Trusted Root Certification Authorities` *and* `Trusted Publishers`
   stores (LocalMachine, not CurrentUser).
4. Build the driver package with `Inf2Cat` + `SignTool` to produce a
   signed `.cat`, alongside the signed `.sys` / `.dll`.
5. **BitLocker warning**: enabling `testsigning` on an OEM machine with
   BitLocker on triggers a recovery-key prompt at next boot. Suspend
   BitLocker before flipping the bit, or have the recovery key ready.
6. **Disable WSL2 / Hyper-V virtual switches**, or note their IPs and
   confirm the host binds to the real LAN NIC (see §2.1 "Network bind").

### Tooling

- **WDK 10.0.22621+** (IddCx 1.4 APIs).
- **Visual Studio 2022** with MSVC v143 and C++ Spectre-mitigated libs.
- **Rust** with `x86_64-pc-windows-msvc` target.
- **Xcode 15+** on macOS 13+.
- **Apple Developer Program account ($99/yr)** — see §2.2.

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
