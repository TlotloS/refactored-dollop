// WinExt Windows host — Phase 1 entry point.
//
// Pipeline (per docs/DESIGN.md §4):
//   capture::DesktopDuplication
//     -> encoder::HardwareEncoder (NVENC | QuickSync | AMF | Media Foundation)
//     -> transport::WebRtcSender
// Input back-channel (input::Injector) plugs in during Phase 3.
// IddCx virtual-display driver replaces DesktopDuplication in Phase 2.

use anyhow::Result;
use clap::Parser;
use tracing::info;

mod capture;
mod encoder;
mod input;
mod transport;

#[derive(Parser, Debug)]
#[command(about = "WinExt host: stream this Windows desktop to a paired iPad over WebRTC")]
struct Args {
    /// Signaling server URL (ws:// or wss://).
    #[arg(long, default_value = "ws://127.0.0.1:8443/ws")]
    signaling: String,

    /// Room code shared with the iPad client.
    #[arg(long)]
    room: String,

    /// Target resolution (defaults to primary monitor).
    #[arg(long)]
    width: Option<u32>,

    /// Target resolution height.
    #[arg(long)]
    height: Option<u32>,

    /// Target frame rate.
    #[arg(long, default_value_t = 60)]
    fps: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    let args = Args::parse();
    info!(
        signaling = %args.signaling,
        room = %args.room,
        fps = args.fps,
        "winext-host starting"
    );

    // Wiring is filled in piece-by-piece across Phase 1.
    // The TODOs below are the exact integration points the design names.
    capture::probe_primary()?;
    encoder::probe_available()?;
    transport::connect_signaling(&args.signaling, &args.room).await?;

    Ok(())
}
