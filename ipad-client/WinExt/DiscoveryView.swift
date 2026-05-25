// Pairing / room-code entry. Phase 1 is intentionally crude:
// the user types the signaling-server URL and a room code shared with
// the Windows host. Phase 4 replaces this with Bonjour browsing + a
// 6-digit SPAKE2 PIN flow.

import SwiftUI

struct DiscoveryView: View {
    @EnvironmentObject var session: SessionController
    @State private var signalingURL: String = "ws://192.168.1.10:8443/ws"
    @State private var roomCode: String = ""

    var body: some View {
        VStack(spacing: 24) {
            Text("WinExt").font(.system(size: 48, weight: .semibold))
            Text("Extend your Windows desktop to this iPad")
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 12) {
                Text("Signaling server").font(.caption).foregroundStyle(.secondary)
                TextField("ws://host:port/ws", text: $signalingURL)
                    .textFieldStyle(.roundedBorder)
                    .keyboardType(.URL)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()

                Text("Room code").font(.caption).foregroundStyle(.secondary)
                TextField("e.g. SOFA-9421", text: $roomCode)
                    .textFieldStyle(.roundedBorder)
                    .textInputAutocapitalization(.characters)
                    .autocorrectionDisabled()
            }
            .frame(maxWidth: 480)

            Button("Connect") { connect() }
                .buttonStyle(.borderedProminent)
                .disabled(roomCode.count < 4 || URL(string: signalingURL) == nil)
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func connect() {
        guard let url = URL(string: signalingURL) else { return }
        session.connect(signalingURL: url, roomCode: roomCode)
    }
}
