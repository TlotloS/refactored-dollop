// WinExt — iPad client for the Windows desktop extender.
//
// Entry point. The app is a single-window iPad app for v1; multi-window
// (Stage Manager) is post-v1.0. Open `WinExt.xcodeproj` in Xcode 15+,
// add the WebRTC.xcframework dependency (see ipad-client/Dependencies.md
// when written), and build for an iPad with iPadOS 16+.

import SwiftUI

@main
struct WinExtApp: App {
    @StateObject private var session = SessionController()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(session)
                .statusBarHidden()
                .persistentSystemOverlays(.hidden)
                .ignoresSafeArea()
        }
    }
}

struct RootView: View {
    @EnvironmentObject var session: SessionController

    var body: some View {
        switch session.state {
        case .idle, .discovering, .pairing:
            DiscoveryView()
        case .connecting, .connected:
            SessionView()
        case .failed(let reason):
            FailureView(reason: reason)
        }
    }
}

struct FailureView: View {
    let reason: String
    @EnvironmentObject var session: SessionController

    var body: some View {
        VStack(spacing: 16) {
            Text("Disconnected").font(.title)
            Text(reason).foregroundStyle(.secondary)
            Button("Try again") { session.reset() }
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
