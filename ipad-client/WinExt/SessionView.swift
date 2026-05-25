// The active streaming view. Hosts a MetalVideoView for the decoded
// frames and an InputCaptureView overlaid on top to forward touches,
// pencil events, hardware keyboard input, and trackpad pointer events
// back to the Windows host over the input DataChannel.

import SwiftUI

struct SessionView: View {
    @EnvironmentObject var session: SessionController

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            MetalVideoView()
                .ignoresSafeArea()
            InputCaptureView()
                .ignoresSafeArea()
        }
    }
}
