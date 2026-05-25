// WinExt viewer binary - Rust test harness that pretends to be the iPad.
//
// Connects to the signaling server as role="viewer", accepts the offer,
// and reports incoming RTP packet rates. Exists purely to verify the
// host's WebRTC pipeline end-to-end without needing a browser or Mac.
// The real iPad client is in /ipad-client/.

use anyhow::Result;
use clap::Parser;
use tracing::info;
use winext::transport;

#[derive(Parser, Debug)]
#[command(about = "WinExt viewer: Rust stand-in for the iPad client (M1 test harness)")]
struct Args {
    #[arg(long, default_value = "ws://127.0.0.1:8443/ws")]
    signaling: String,

    #[arg(long)]
    room: String,
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
    info!(signaling = %args.signaling, room = %args.room, "winext-viewer starting");
    transport::run(transport::SessionConfig {
        signaling_url: args.signaling,
        room_code: args.room,
        role: transport::Role::Viewer,
    })
    .await?;
    Ok(())
}
