// Thin WebSocket client matching the server in ../../signaling/.
// Speaks the JSON shapes defined in protocol/messages.ts.

import Foundation

enum SignalingRole: String { case host, viewer }

protocol SignalingClientDelegate: AnyObject {
    func signalingDidJoin(_ client: SignalingClient, peerPresent: Bool)
    func signalingPeerJoined(_ client: SignalingClient)
    func signalingPeerLeft(_ client: SignalingClient)
    func signaling(_ client: SignalingClient, didReceiveOffer sdp: String)
    func signaling(_ client: SignalingClient, didReceiveAnswer sdp: String)
    func signaling(_ client: SignalingClient, didReceiveIce candidate: [String: Any])
    func signaling(_ client: SignalingClient, didFail reason: String)
}

final class SignalingClient: NSObject, URLSessionWebSocketDelegate {
    private let url: URL
    private let role: SignalingRole
    private let roomCode: String
    private let clientId = UUID().uuidString
    private var task: URLSessionWebSocketTask?
    weak var delegate: SignalingClientDelegate?

    init(url: URL, role: SignalingRole, roomCode: String) {
        self.url = url
        self.role = role
        self.roomCode = roomCode
    }

    func connect() {
        let session = URLSession(configuration: .default, delegate: self, delegateQueue: nil)
        let task = session.webSocketTask(with: url)
        self.task = task
        task.resume()
        sendHello()
        listen()
    }

    func disconnect() {
        task?.cancel(with: .normalClosure, reason: nil)
        task = nil
    }

    func send(_ payload: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: payload),
              let text = String(data: data, encoding: .utf8) else { return }
        task?.send(.string(text)) { _ in }
    }

    private func sendHello() {
        send([
            "kind": "hello",
            "role": role.rawValue,
            "roomCode": roomCode,
            "clientId": clientId,
        ])
    }

    private func listen() {
        task?.receive { [weak self] result in
            guard let self else { return }
            switch result {
            case .failure(let err):
                self.delegate?.signaling(self, didFail: err.localizedDescription)
            case .success(.string(let text)):
                self.handle(text: text)
                self.listen()
            case .success(.data(let data)):
                if let text = String(data: data, encoding: .utf8) { self.handle(text: text) }
                self.listen()
            @unknown default:
                self.listen()
            }
        }
    }

    private func handle(text: String) {
        guard let data = text.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let kind = obj["kind"] as? String else { return }
        switch kind {
        case "joined":
            let peerPresent = (obj["peerPresent"] as? Bool) ?? false
            delegate?.signalingDidJoin(self, peerPresent: peerPresent)
        case "peer-joined":
            delegate?.signalingPeerJoined(self)
        case "peer-left":
            delegate?.signalingPeerLeft(self)
        case "offer":
            if let sdp = obj["sdp"] as? String { delegate?.signaling(self, didReceiveOffer: sdp) }
        case "answer":
            if let sdp = obj["sdp"] as? String { delegate?.signaling(self, didReceiveAnswer: sdp) }
        case "ice":
            if let cand = obj["candidate"] as? [String: Any] { delegate?.signaling(self, didReceiveIce: cand) }
        case "error":
            let msg = (obj["message"] as? String) ?? "unknown"
            delegate?.signaling(self, didFail: msg)
        default:
            break
        }
    }
}
