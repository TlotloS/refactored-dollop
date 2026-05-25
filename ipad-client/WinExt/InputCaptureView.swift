// Transparent UIView overlaid on the video. Captures every touch,
// pencil sample, hardware key press, and trackpad pointer event and
// forwards them as InputEvent CBOR messages over the WebRTC data
// channel (see protocol/messages.ts).
//
// Stub for the scaffold; the event-serialization plumbing lands in Phase 3.

import SwiftUI
import UIKit

struct InputCaptureView: UIViewRepresentable {
    func makeUIView(context: Context) -> UIView {
        let v = RawInputView(frame: .zero)
        v.backgroundColor = .clear
        v.isMultipleTouchEnabled = true
        return v
    }
    func updateUIView(_ uiView: UIView, context: Context) {}
}

final class RawInputView: UIView {
    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        forwardPointer(touches: touches, event: event, phase: "down")
    }
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        forwardPointer(touches: touches, event: event, phase: "move")
    }
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        forwardPointer(touches: touches, event: event, phase: "up")
    }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        forwardPointer(touches: touches, event: event, phase: "cancel")
    }

    private func forwardPointer(touches: Set<UITouch>, event: UIEvent?, phase: String) {
        // TODO Phase 3: for each touch, build a PointerEvent (normalized
        // coords, pressure, tilt, tool) and hand it to WebRTCClient for
        // CBOR-encoding + DataChannel send. Use event.coalescedTouches(for:)
        // to capture sub-frame samples at up to 240 Hz on Pencil.
    }

    override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        // TODO Phase 3: map UIPress.key.keyCode -> PS/2 scancode, send KeyEvent.
    }

    override func pressesEnded(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        // TODO Phase 3.
    }
}
