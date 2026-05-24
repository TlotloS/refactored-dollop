# Extend Windows Desktop to iPad — Technical Design

Status: Draft v0.1 — design only, no code yet.
Owner: tlotlo@b1sa.co.za
Target platforms: Windows 10 21H2+ / Windows 11; iPadOS 16+.

---

## 1. Goals and Non-Goals

### 1.1 Primary goal

Let a user attach their iPad as a real **extended display** of a Windows PC over the local network — so the OS treats it as a second monitor, windows can be dragged onto it, and Apple Pencil / touch input is sent back as pointer events. Comparable in feel to Spacedesk, Duet Display, or Apple Sidecar (which is Mac-only).

### 1.2 Must-have for v1.0

- Appears in Windows **Display Settings** as a second monitor with selectable resolution and refresh rate.
- End-to-end glass-to-glass latency under **80 ms** on a quiet 5 GHz Wi-Fi network at 1920×1200 @ 60 Hz.
- Touch on the iPad moves the Windows cursor; tap = left click; two-finger tap = right click.
- Survives sleep / wake / Wi-Fi flap without requiring a Windows reboot.
- Pairing model that doesn't expose the host to any device on the LAN.

### 1.3 Stretch goals (post-v1.0)

- Apple Pencil pressure + tilt forwarded as Windows Ink / Wintab events.
- ProMotion 120 Hz on iPad Pro.
- HDR / wide color (BT.2020 + PQ).
- Internet relay (TURN) for off-LAN use.
- Audio forwarding (Windows audio device routed to iPad).

### 1.4 Explicit non-goals

- macOS host. Mac users already have Sidecar.
- Android tablets. Out of scope.
- USB-tethered mode in v1.0 (Lightning / USB-C). Considered for v2.
- App Store distribution in v1.0 — iPad client distributed via TestFlight or sideload.
- Replacing Remote Desktop / RDP. We extend the desktop, not replace the session.

---

## 2. High-level Architecture

```
+---------------------------- Windows PC -----------------------------+
|                                                                    |
|  +------------------+    +---------------+    +-----------------+  |
|  | Indirect Display |    | Capture &     |    | Streaming /     |  |
|  | Driver (IddCx)   |--->| Encoder       |--->| Signaling host  |  |
|  | (user-mode UMDF) |    | (H.264/H.265) |    | (WebRTC + mDNS) |  |
|  +------------------+    +---------------+    +-----------------+  |
|         ^                                            |             |
|         | virtual HID input                          |             |
|  +------+-----------+                                |             |
|  | Input injector   |<-------------------------------+             |
|  | (SendInput +     |   touch / pencil / keyboard events           |
|  |  ViGEm-style)    |                                              |
|  +------------------+                                              |
+--------------------------------------------------------------------+
                              ^
                              | WebRTC (DTLS/SRTP) over UDP, plus DataChannel
                              v
+----------------------------- iPad ---------------------------------+
|                                                                    |
|  +-----------------+    +-----------------+    +----------------+  |
|  | WebRTC client   |--->| VideoToolbox    |--->| Metal renderer |  |
|  | (libwebrtc /    |    | H.264/H.265 dec |    | CAMetalLayer   |  |
|  |  WebRTC.framework)|  +-----------------+    +----------------+  |
|  +-----------------+                                               |
|         |                                                          |
|         | DataChannel: input events out                            |
|  +------+-----------+                                               |
|  | UIKit / Pencil-  |                                               |
|  | Kit input layer  |                                               |
|  +------------------+                                               |
+--------------------------------------------------------------------+
```

Two TCP/UDP channels exist between host and iPad over a single WebRTC peer connection:

1. A video track (one-way, host → iPad).
2. A `RTCDataChannel` named `input` (one-way, iPad → host, ordered + reliable).

A separate **signaling** path (HTTPS over the LAN with a self-signed but pinned cert) brokers the SDP offer/answer + ICE candidates. Discovery uses **mDNS / Bonjour** (`_winext._tcp.local`) so the iPad app can list available hosts.

---

## 3. The Hard Problem: a Virtual Monitor on Windows

To **extend** the desktop (not just mirror), the OS must believe a second physical monitor exists. There are three viable approaches; option A is what we will pursue.

### 3.1 Option A — Indirect Display Driver (IddCx) — **chosen**

Microsoft's **Indirect Display Driver Class Extension** (`IddCx`) is purpose-built for exactly this use case (USB display adapters, virtual KVMs). It's a **UMDF user-mode driver** that lets a non-DDA software stack present itself as a display adapter.

Pros
- User-mode. No KMDF / kernel pointer arithmetic. Crashes don't BSOD.
- Designed for **soft-add / soft-remove** — iPad connect/disconnect maps onto monitor hot-plug.
- Microsoft ships a working sample: `IddSampleDriver` in the Windows-driver-samples GitHub repo, MIT-licensed. We can fork or rewrite.
- Receives finished frames as DXGI textures — we don't have to mediate D3D commands.

Cons
- Must be **WHQL-signed** for distribution. Without WHQL, we either ship as a "test mode" driver (poor UX) or pay for **attestation signing** via the Microsoft Hardware Dev Center (one-time ~$99, EV code-signing cert required, ~$300/yr).
- IddCx tops out at the refresh rates the OS allows for indirect displays — historically 60 Hz max on older Windows builds; Windows 11 22H2 lifted this and supports higher rates if the driver advertises them.
- Latency floor: IddCx delivers frames roughly each VSync of the *virtual* monitor; we can't deliver intra-frame.

Driver responsibilities
- Advertise a list of supported modes (e.g. 1920×1200, 2160×1620, 2732×2048 @ 60/120 Hz).
- Implement `EVT_IDD_CX_MONITOR_ASSIGN_SWAPCHAIN` — we receive an `IDARG_OUT_GETCONSUMER`, then in a worker thread:
  1. Acquire each frame from the swap-chain (`AcquireBuffer`).
  2. Hand the `IDXGIResource` to the **encoder process** via a shared D3D11 texture (cross-process via `IDXGIResource1::CreateSharedHandle`).
  3. Release the buffer.
- Hot-plug via `IDD_CX_MONITOR_ARRIVAL` / `IDD_CX_MONITOR_DEPARTURE` triggered by an IPC signal from the streaming host when a paired iPad connects or disconnects.

### 3.2 Option B — A real WDDM display miniport

A full WDDM kernel miniport gives us total control (custom refresh, hardware cursor, etc.) but is months of additional work, BSODs are real, and signing is even harder. **Rejected.**

### 3.3 Option C — A "virtual monitor" via EDID-injecting dongle

Some projects ship a USB-C dongle that pretends to be a real HDMI display and capture its output. Hardware. Not a software-only solution. **Rejected.**

### 3.4 Driver-signing path

For development:
- Toggle `bcdedit /set testsigning on`. Sign the driver with a self-issued cert. Reboot. The driver loads.
- This is what every contributor uses locally.

For distribution:
1. Buy an **EV code-signing certificate** from DigiCert / Sectigo (~$300/yr; physical USB token; ID verification).
2. Register on the **Microsoft Hardware Dev Center** (one-time $99).
3. Submit the driver package (`.inf` + signed `.sys`/`.dll` + symbols) for **attestation signing**. Microsoft countersigns; the driver now installs on any Windows 10/11 system without test mode.
4. WHQL submission is optional, more rigorous, and required only for Windows Update distribution — which we don't need; the user installs an MSIX/installer that bundles the signed driver.

Budget for signing: **~$400 in year one, ~$300/yr after**. Time from cert purchase to first attestation-signed build: ~2 weeks (cert delivery dominates).

---

## 4. Windows Host: Capture, Encode, Transport

The Windows host is a single user-mode process — call it `winext-host.exe` — running as a Windows Service (`LocalService` + adjusted token for desktop access via session 1 input desktop). The IddCx driver runs in its own UMDF host process and shares textures with `winext-host.exe` via IPC.

### 4.1 Capture pipeline

Frames arrive on the driver side as DXGI swap-chain textures. The driver does **not** copy or encode — it shares the `ID3D11Texture2D` handle with `winext-host.exe` over a named pipe (`\\.\pipe\winext-frames`) plus `DuplicateHandle`. The host opens the shared texture and uses it as encoder input.

Why not Desktop Duplication API (DDA)? DDA captures a *real* monitor and would force us to invent a virtual one another way. With IddCx we already get the framebuffer the OS just rendered — no separate capture step.

### 4.2 Encoder

We need a hardware encoder; software x264 cannot hit our latency budget.

| Encoder            | When used                                    | Latency           |
|--------------------|----------------------------------------------|-------------------|
| **NVIDIA NVENC**   | If a GeForce / RTX GPU is present (most users) | best (~3–6 ms)    |
| **Intel Quick Sync (D3D11VA / oneVPL)** | iGPU on Intel laptops             | ~6–10 ms          |
| **AMD AMF**        | AMD GPUs                                     | ~5–10 ms          |
| **Media Foundation H.264** | Fallback. Universal but slowest.     | ~12–20 ms         |

Selection at runtime by probing in the above order. Codec choice: **H.264 High Profile baseline-friendly, no B-frames**, IDR every 2 s, target bitrate adaptive (4–25 Mbps depending on resolution and bandwidth estimator from WebRTC). H.265 considered for v1.1 (iPad supports it, but signaling negotiation is fiddly with libwebrtc).

Critical encoder settings for low latency
- `rcMode = CBR_LOWDELAY_HQ` (NVENC) / equivalent on others.
- No B-frames (`B_FRAMES = 0`).
- `gopLength = refresh_rate × 2` for IDR every 2 s.
- Slice intra-refresh enabled, or per-frame IDR on packet loss (driven by RTCP PLI/FIR).
- `repeatSPSPPS = true` to survive packet loss.

### 4.3 Transport

**WebRTC** for the video track. Reasons:
- Battle-tested NACK + PLI + FIR loss recovery.
- DTLS-SRTP handles auth + encryption.
- libwebrtc has high-quality iOS bindings (Google's `WebRTC.xcframework`).
- Built-in congestion controller (GCC / transport-cc) — adaptive bitrate is free.

Drawbacks: libwebrtc is huge (~50 MB iOS framework) and its public C++ API is unstable. For v1 we accept that. For v2 we might replace it with a lean **QUIC-based** transport (`quinn` / `msquic`) once we no longer need the GCC.

Topology: peer-to-peer over LAN. No SFU. ICE candidates are restricted to host candidates from the same /24 (configurable). STUN/TURN only enabled when the user opts into "off-LAN" mode (post-v1.0).

### 4.4 Discovery & pairing

- Host publishes `_winext._tcp.local` via Bonjour (use Apple's Bonjour SDK for Windows — actively maintained as part of iTunes; or `mdnsresponder` directly).
- TXT record: `version=1`, `host=<machine-name>`, `fp=<sha256-of-host-cert>`.
- iPad app scans for `_winext._tcp.local` and shows a picker.
- First-time pairing: 6-digit PIN displayed on the Windows host, entered on the iPad. SPAKE2 PAKE establishes a shared secret; both sides persist the peer's public key. Subsequent connects are mutual-cert-pinned.

### 4.5 Input back-channel

iPad sends events over the `input` RTCDataChannel as length-prefixed CBOR messages:

```
{ "t": "pointer", "id": 0, "phase": "down|move|up",
  "x": 0.0..1.0, "y": 0.0..1.0,
  "pressure": 0..1, "tiltX": deg, "tiltY": deg, "tool": "touch|pencil" }
{ "t": "key", "code": "...", "phase": "down|up", "mods": ["shift","ctrl"] }
{ "t": "scroll", "dx": ..., "dy": ..., "phase": "begin|update|end" }
```

Coordinates are normalized to the *target monitor*, not the iPad screen, so resolution changes don't matter. The host maps `(x,y)` to the virtual monitor's pixel rect and injects:

- Touch / mouse → `SendInput` with `MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK`, using the virtual monitor's coordinates.
- Pencil pressure → Windows Pointer Input Messages (`InjectSyntheticPointerInput` from `User32`), so apps that respect Windows Ink (OneNote, Photoshop, Krita, Affinity) get real pressure. Requires the host to register a synthetic pointer device per pencil tool.
- Keyboard → `SendInput` with `KEYEVENTF_SCANCODE`.

We deliberately **do not** install a virtual HID driver in v1.0 — `SendInput` + `InjectSyntheticPointerInput` cover what we need without another signing burden. ViGEm-style HID is a v2 option if we discover edge cases (per-app raw input filtering, anti-cheat games).

---

## 5. iPad Client

### 5.1 Stack

- **SwiftUI** for the discovery / pairing / settings UI.
- **UIKit** for the active session view — we need raw `UIView` to host a `CAMetalLayer` and to capture `UITouch` / `UIPencilInteraction` with full fidelity.
- **WebRTC.framework** (the official Google build for iOS).
- **VideoToolbox** for hardware H.264/H.265 decode.
- **Metal** for rendering — a single textured quad, optional sharpening shader, gamma + color-space conversion.
- **CoreHaptics** for subtle pairing feedback.

Minimum iPadOS: **16.0** — needed for `UIScene`-based multi-window and Stage Manager interactions.

### 5.2 Decode + render path

1. WebRTC delivers `RTCVideoFrame` (`CVPixelBufferRef` in NV12 from VideoToolbox).
2. Bypass libwebrtc's renderer; pull the `CVPixelBuffer` directly.
3. Bind as a Metal texture via `CVMetalTextureCacheCreateTextureFromImage`.
4. Render in a `CAMetalLayer` with `presentsWithTransaction = false` and `maximumDrawableCount = 2`, driven by `CADisplayLink`.
5. On ProMotion-capable iPads, request 120 Hz via `preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)`.

### 5.3 Input capture

- `UIView` overrides `touchesBegan/Moved/Ended/Cancelled`.
- Distinguish pencil vs. touch via `UITouch.type == .pencil`.
- Read `force`, `altitudeAngle`, `azimuthAngle` from pencil touches; quantize to 1 kHz and dedupe.
- Hardware keyboards: `UIKeyCommand` + `pressesBegan/Ended` (full key set including modifiers).
- Trackpad on Magic Keyboard: `UIPointerInteraction` + `UIHoverGestureRecognizer` for hover events.
- All events go out the `input` DataChannel with a millisecond timestamp from `mach_absolute_time`.

### 5.4 Distribution

- v1.0: **TestFlight** (90-day cycles, up to 10k testers). Apple Developer account ($99/yr) covers it.
- v1.1: full App Store submission if we decide to ship publicly. Apple review will scrutinize the "Windows extension" use case but Duet / Astropad / Spacedesk all pass review, so the category is accepted.

---

## 6. Latency Budget

Target glass-to-glass: **≤80 ms** at 1080p60 on quiet 5 GHz Wi-Fi.

| Stage                                | Budget       | Notes |
|--------------------------------------|--------------|-------|
| Frame produced by Windows app        | 0 ms         | t=0 reference |
| DWM compositor → IddCx swap-chain    | 8–16 ms      | one VSync of the virtual monitor |
| IPC handoff to encoder process       | < 1 ms       | shared texture, no copy |
| Hardware encode (NVENC low-latency)  | 4–6 ms       | first slice out |
| Packetize + DTLS/SRTP                | 1–2 ms       |  |
| Wi-Fi RTT (5 GHz, ~20 MHz, 1 hop AP) | 6–15 ms      | dominated by AP scheduling |
| VideoToolbox decode                  | 5–8 ms       |  |
| Metal present + display scanout      | 8–16 ms      | one ProMotion frame |
| **Total**                            | **33–64 ms** | meets target with margin |

Anything over ~120 ms feels broken for cursor work. Anything under ~50 ms feels native. The above gives us headroom.

---

## 7. Security Model

Threats considered:
1. Untrusted device on the LAN attempts to connect.
2. Man-in-the-middle on the LAN.
3. A malicious paired iPad later sends crafted input or video junk.
4. Local privilege escalation via the IddCx driver or the host service.

Mitigations:
- **Pairing**: SPAKE2 PIN-based exchange on first connect. PIN displayed on the host, entered on the iPad. PIN rotates per pair attempt.
- **Auth**: after pairing, both sides hold each other's Ed25519 public key. WebRTC's DTLS handshake fingerprints are pinned to those keys (we override the default cert verifier).
- **Transport**: DTLS-SRTP, mandatory. No fallback to plaintext.
- **Host service hardening**:
  - Runs as `LocalService` with explicit privileges, not `LocalSystem`. Uses `WTSQueryUserToken` to get the interactive user's token for `SendInput`.
  - No COM / RPC surfaces exposed to non-admin local users.
  - All input from the iPad is validated (normalized coords, no out-of-range scancodes).
- **Driver hardening**:
  - User-mode (UMDF); a crash takes down the driver host, not the kernel.
  - No data parsed from the wire inside the driver. The driver only receives shared D3D textures from `winext-host.exe`.
- **Auto-disconnect** after 5 minutes of no input + no decode acks from the iPad, to limit window of exposure on lock-screen scenarios.

Out of scope for v1.0: defense against a fully-compromised paired iPad. If your iPad is rooted, it's your trusted second display.

---

## 8. Multi-Monitor, DPI, and Color

- **DPI**: the IddCx driver advertises native DPI based on the iPad's `nativeBounds` and physical size, but Windows ignores per-monitor DPI from indirect displays in some builds. We expose a "Scale" setting (100/125/150/200%) in the host UI that adjusts the advertised mode list — e.g. 200% means we advertise 1366×1024 instead of 2732×2048 even though the iPad's panel is 2732×2048.
- **Refresh rate negotiation**: the host probes the network during pairing (5 s RTT + jitter measurement) and only advertises 120 Hz modes if jitter < 4 ms. Otherwise we cap at 60 Hz.
- **Color**: sRGB only in v1.0. The encoder is fed BT.709 limited-range. HDR / P3 / BT.2020 is a v1.1 stretch.
- **Hot-plug**: when the iPad disconnects, the driver triggers `IDD_CX_MONITOR_DEPARTURE` and Windows reflows any windows back to the primary. Windows remembers positions per monitor identity, so reconnect restores layout.

---

## 9. Phased Delivery Plan

The point of breaking this into phases is to have something demonstrable at the end of each phase rather than a 6-month invisible march.

### Phase 0 — Spike (1 week)
- Get Microsoft's `IddSampleDriver` building and installed in test mode.
- Verify a fake monitor appears in Display Settings and we can drag a window onto it.
- Capture one frame and dump it to disk.

### Phase 1 — Mirror over the wire (3 weeks)
- Skip the driver entirely. Use Desktop Duplication to capture the primary monitor.
- NVENC encode + libwebrtc transport + iPad client that decodes and displays.
- No input. No pairing. Hard-coded IP. Goal: prove the streaming stack on its own.

### Phase 2 — Replace capture with virtual monitor (2 weeks)
- Swap DDA capture for the IddCx driver feeding the same encoder.
- Now we're extending instead of mirroring.

### Phase 3 — Input back-channel (2 weeks)
- DataChannel + CBOR protocol.
- `SendInput` mouse + keyboard.
- `InjectSyntheticPointerInput` for Pencil pressure.

### Phase 4 — Pairing, discovery, polish (2 weeks)
- mDNS discovery.
- SPAKE2 PIN pairing.
- Cert pinning. Hot-plug. Sleep/wake survival.

### Phase 5 — Signing & installer (2 weeks, mostly waiting)
- Buy EV cert.
- Sign driver + host.
- Submit to Hardware Dev Center for attestation.
- Build MSIX installer.

### Phase 6 — TestFlight beta (ongoing)
- Submit iPad app.
- Internal beta, then external.

**Total to v1.0 beta: ~12 calendar weeks for one engineer**, assuming no signing delays.

---

## 10. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| IddCx max refresh rate caps us at 60 Hz on some Windows versions | Medium | Medium | Advertise 60 Hz fallback; document ProMotion as best-effort. |
| EV cert / attestation signing takes weeks | High | Medium | Start Phase 5 in parallel with Phase 3. |
| WebRTC GCC underestimates Wi-Fi bandwidth and we look blurry | Medium | High | Override min/max bitrate; expose manual bitrate slider. |
| libwebrtc upgrade breaks our patches | High | Low | Pin to a specific tag; cherry-pick security fixes only. |
| Apple rejects iPad app citing "remote desktop policy" | Low | High | Distribute via TestFlight indefinitely as fallback. Many competitors are on the store, precedent is favorable. |
| Pencil pressure feels laggy due to event coalescing | Medium | Medium | Use `UIEvent.coalescedTouches` to capture sub-frame events; send at 240 Hz. |
| Windows IPSec / firewall blocks UDP between processes | Low | Medium | Installer adds firewall rule for `winext-host.exe`. |
| User on corporate-managed Windows can't install signed drivers | Medium | Low | Document as a known limitation; offer mirror-only fallback. |

---

## 11. Open Questions

Things I'd want decided before writing code:

1. **Single-user or multi-user?** Should one Windows host accept multiple iPads (e.g. extending to two iPads as monitors 2 and 3 simultaneously)? Easy to support architecturally but doubles the testing matrix.
2. **Pencil hover** — Apple Pencil Gen 2 + M2 iPads support hover. Worth forwarding to Windows? Most Windows apps don't consume hover from synthetic pointers; benefit may be small.
3. **Audio** — do you ever need iPad to act as a second speaker for the PC? If yes, we need a Windows virtual audio device driver too (similar signing burden), in which case it should land alongside the display driver, not as a v2 item.
4. **Open source or closed?** Open-sourcing the iPad client is easy; open-sourcing the Windows driver requires us to either (a) eat the signing cost ourselves and ship binaries from a build server, or (b) require users to enable test mode. Pick before naming the GitHub repo.
5. **Branding / project name?** `refactored-dollop` is a placeholder; the iPad app needs a real name for App Store Connect.

---

## 12. Appendix: Reference Material

- Microsoft IddCx documentation: <https://learn.microsoft.com/en-us/windows-hardware/drivers/display/iddcx-overview>
- Microsoft IddSampleDriver source: <https://github.com/microsoft/Windows-driver-samples/tree/main/general/IndirectDisplay>
- Windows Hardware Dev Center (attestation signing): <https://partner.microsoft.com/en-us/dashboard/hardware>
- libwebrtc iOS releases: <https://webrtc.googlesource.com/src/+/refs/heads/main/sdk/objc>
- VideoToolbox decompression session reference: <https://developer.apple.com/documentation/videotoolbox>
- `InjectSyntheticPointerInput` (Windows Pointer Input Injection): <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-injectsyntheticpointerinput>
- SPAKE2 (RFC 9382): <https://datatracker.ietf.org/doc/rfc9382/>

---

## 13. What this document does NOT contain

- Any production source code.
- Wire-format byte layouts beyond the CBOR sketch in §4.5 — those firm up in Phase 1.
- UI mockups — design comes after Phase 1 is up on a real iPad.
- CI/CD plumbing — covered in a separate doc once Phase 1 lands.
