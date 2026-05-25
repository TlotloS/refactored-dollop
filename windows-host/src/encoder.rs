// Hardware H.264 encoder.
//
// Selection order (docs/DESIGN.md §4.2):
//   1. NVIDIA NVENC          (best latency)
//   2. Intel Quick Sync      (iGPU)
//   3. AMD AMF               (AMD GPU)
//   4. Media Foundation H264 (universal fallback)
//
// Low-latency settings per backend:
//   - CBR rate control with low-delay tuning.
//   - No B-frames.
//   - IDR every 2 s, plus PLI/FIR-triggered IDR on packet loss.
//   - repeatSPSPPS=true so a viewer joining mid-stream can decode.

use anyhow::Result;
use tracing::info;

pub enum Backend {
    Nvenc,
    QuickSync,
    Amf,
    MediaFoundation,
}

pub fn probe_available() -> Result<()> {
    info!("encoder::probe_available stub - NVENC/QuickSync/AMF/MF probing lands here");
    Ok(())
}
