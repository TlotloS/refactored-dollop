// Renders decoded video frames via a CAMetalLayer.
//
// We deliberately bypass libwebrtc's RTCMTLVideoView so we have direct
// control over the present pipeline (and can switch to ProMotion 120 Hz,
// turn off the vsync stall, and tune for lowest latency). Frames arrive
// from WebRTCClient as CVPixelBuffer in NV12 (VideoToolbox output) and
// are bound as Metal textures via CVMetalTextureCache.
//
// Stub for the scaffold; the renderer's interesting bits land in Phase 1.

import SwiftUI
import MetalKit
import CoreVideo

struct MetalVideoView: UIViewRepresentable {
    func makeUIView(context: Context) -> MTKView {
        let view = MTKView()
        view.device = MTLCreateSystemDefaultDevice()
        view.framebufferOnly = true
        view.isPaused = true                  // we present on incoming frames
        view.enableSetNeedsDisplay = false
        view.colorPixelFormat = .bgra8Unorm
        view.delegate = context.coordinator
        // Request ProMotion 120 Hz when available.
        if #available(iOS 15.0, *) {
            view.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
        }
        return view
    }

    func updateUIView(_ uiView: MTKView, context: Context) {}

    func makeCoordinator() -> Renderer { Renderer() }

    final class Renderer: NSObject, MTKViewDelegate {
        func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}
        func draw(in view: MTKView) {
            // TODO Phase 1: bind current CVPixelBuffer as a Metal texture
            // via CVMetalTextureCacheCreateTextureFromImage, render a
            // textured quad, present via view.currentDrawable.
        }
    }
}
