// Owns the high-level connection state machine and bridges UI <-> WebRTC.
// Phase 1 wiring: signaling client + WebRTC peer connection + decoded
// CVPixelBuffer pipeline to MetalVideoView. Pairing UI is a stub that
// just asks for a room code; SPAKE2 pairing arrives in Phase 4.

import Foundation
import Combine

enum ConnectionState: Equatable {
    case idle
    case discovering
    case pairing
    case connecting
    case connected
    case failed(String)
}

@MainActor
final class SessionController: ObservableObject {
    @Published private(set) var state: ConnectionState = .idle
    @Published var discoveredHosts: [DiscoveredHost] = []
    @Published var lastFrameTimestampUs: Int64 = 0

    private var signaling: SignalingClient?
    private var webrtc: WebRTCClient?

    func startDiscovery() {
        state = .discovering
        // TODO Phase 4: NetServiceBrowser for _winext._tcp.local
        // For Phase 1 the user types a room code + server URL manually.
    }

    func connect(signalingURL: URL, roomCode: String) {
        state = .connecting
        let s = SignalingClient(url: signalingURL, role: .viewer, roomCode: roomCode)
        let rtc = WebRTCClient()
        s.delegate = rtc
        rtc.delegate = self
        signaling = s
        webrtc = rtc
        s.connect()
    }

    func reset() {
        signaling?.disconnect()
        webrtc?.close()
        signaling = nil
        webrtc = nil
        state = .idle
    }
}

extension SessionController: WebRTCClientDelegate {
    func webrtcClientDidConnect(_ client: WebRTCClient) {
        state = .connected
    }

    func webrtcClient(_ client: WebRTCClient, didFail reason: String) {
        state = .failed(reason)
    }

    func webrtcClient(_ client: WebRTCClient, didReceiveFrameAt tUs: Int64) {
        lastFrameTimestampUs = tUs
    }
}

struct DiscoveredHost: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let host: String
    let port: Int
    let fingerprint: String
}
