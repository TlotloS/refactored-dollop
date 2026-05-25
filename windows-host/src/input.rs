// Input injection. Phase 3 wiring.
//
// Receives CBOR events from the iPad over the WebRTC DataChannel and
// translates them to Windows input:
//   - Pointer (touch / pencil) -> InjectSyntheticPointerInput (Windows Ink).
//   - Pointer (touch / mouse)  -> SendInput with MOUSEEVENTF_ABSOLUTE
//                                 | MOUSEEVENTF_VIRTUALDESK.
//   - Key                      -> SendInput with KEYEVENTF_SCANCODE.
//   - Scroll                   -> SendInput WHEEL / HWHEEL.
//
// Coordinates from the iPad are normalized to the *target monitor*; we map
// them to virtual-desktop pixel coordinates using the target monitor's
// position + size in the WDDM monitor list (Phase 2: the IddCx virtual
// monitor; Phase 1: the primary monitor we mirror).

#![allow(dead_code)]

// No public API yet - Phase 3.
