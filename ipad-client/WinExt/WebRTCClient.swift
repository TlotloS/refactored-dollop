// WebRTC peer-connection wrapper. Bridges the SignalingClient to a
// real RTCPeerConnection from WebRTC.xcframework, owns the incoming
// video track (decoded into CVPixelBuffer by VideoToolbox), and exposes
// the "input" DataChannel for outbound input events.
//
// Stub for the scaffold. WebRTC.xcframework is added in Phase 1 via
// CocoaPods (`pod 'GoogleWebRTC'`) or SwiftPM mirror. The protocol
// here is the shape SessionController needs; the body fills in.

import Foundation
import CoreVideo

protocol WebRTCClientDelegate: AnyObject {
    func webrtcClientDidConnect(_ client: WebRTCClient)
    func webrtcClient(_ client: WebRTCClient, didFail reason: String)
    func webrtcClient(_ client: WebRTCClient, didReceiveFrameAt tUs: Int64)
}

final class WebRTCClient: NSObject, SignalingClientDelegate {
    weak var delegate: WebRTCClientDelegate?
    private(set) var latestPixelBuffer: CVPixelBuffer?

    func close() {
        // TODO Phase 1: tear down RTCPeerConnection.
    }

    func sendInputCBOR(_ bytes: Data) {
        // TODO Phase 3: send over the "input" RTCDataChannel.
    }

    // MARK: SignalingClientDelegate

    func signalingDidJoin(_ client: SignalingClient, peerPresent: Bool) {
        // Viewer waits for the host to send an offer; nothing to do
        // until peerPresent goes true.
    }

    func signalingPeerJoined(_ client: SignalingClient) {
        // TODO Phase 1: create RTCPeerConnection, set up data channel,
        // wait for offer.
    }

    func signalingPeerLeft(_ client: SignalingClient) {
        delegate?.webrtcClient(self, didFail: "peer disconnected")
    }

    func signaling(_ client: SignalingClient, didReceiveOffer sdp: String) {
        // TODO Phase 1: setRemoteDescription, createAnswer, send back.
    }

    func signaling(_ client: SignalingClient, didReceiveAnswer sdp: String) {
        // Host-side only; viewer never receives an answer.
    }

    func signaling(_ client: SignalingClient, didReceiveIce candidate: [String: Any]) {
        // TODO Phase 1: addIceCandidate on the RTCPeerConnection.
    }

    func signaling(_ client: SignalingClient, didFail reason: String) {
        delegate?.webrtcClient(self, didFail: reason)
    }
}
