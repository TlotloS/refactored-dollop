// WinExt host binary - M1.
//
// M1 scope: stream a pre-encoded VP8 IVF test pattern to one viewer.
// Real capture + encode (M2) replaces the IVF source; this entry point
// stays roughly the same shape.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;
use winext::{capture, encoder, transport};

#[derive(Parser, Debug)]
#[command(about = "WinExt host: stream this Windows desktop to a paired iPad over WebRTC")]
struct Args {
    /// Signaling server URL (ws:// or wss://).
    #[arg(long, default_value = "ws://127.0.0.1:8443/ws")]
    signaling: String,

    /// Room code shared with the viewer.
    #[arg(long)]
    room: String,

    /// H.264 Annex-B test pattern (M1 only - replaced by live capture in M2).
    #[arg(long, default_value = "assets/testpattern.h264")]
    media: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    info!(signaling = %args.signaling, room = %args.room, "winext-host starting");

    capture::probe_primary()?;
    encoder::probe_available()?;

    transport::run(transport::SessionConfig {
        signaling_url: args.signaling,
        room_code: args.room,
        role: transport::Role::Host { media_path: args.media },
    })
    .await?;
    Ok(())
}
