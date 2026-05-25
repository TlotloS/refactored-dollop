// WebRTC transport.
//
// Connects to the signaling server, joins as role="host", waits for the
// viewer (iPad) to join. Once both peers are present we create an
// RTCPeerConnection with one outgoing video track (encoder output) and
// one ordered+reliable DataChannel named "input" for iPad -> host events.

use anyhow::Result;
use futures_util::SinkExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::info;
use uuid::Uuid;

pub async fn connect_signaling(url: &str, room_code: &str) -> Result<()> {
    let (mut ws, _) = connect_async(url).await?;
    let hello = serde_json::json!({
        "kind": "hello",
        "role": "host",
        "roomCode": room_code,
        "clientId": Uuid::new_v4().to_string(),
    });
    ws.send(Message::Text(hello.to_string())).await?;
    info!(%room_code, "transport::connect_signaling sent hello");
    // TODO Phase 1: drive offer/answer/ICE here; create RTCPeerConnection
    // via webrtc crate; bind video sender to encoder output.
    Ok(())
}
