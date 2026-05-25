// WebRTC transport.
//
// M1 scope:
//   - Connect to the signaling server, exchange offer/answer/ICE.
//   - Host: add a VP8 video track sourced from a pre-encoded IVF file
//     (the M1 test pattern). M2 will replace this with live capture +
//     encoded frames from the Windows-specific capture/encoder pipeline.
//   - Viewer: accept the offer, decode codecs, count incoming frames.
//
// Why VP8 here when iPad needs H.264? VP8 is enough to prove the
// plumbing - no external encoder/decoder needed, the webrtc crate
// handles it end-to-end. M1.5 swaps to H.264 once a Mac is in play.

use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Notify};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_VP8};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::io::ivf_reader::IVFReader;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

#[derive(Debug, Clone)]
pub enum Role {
    Host { ivf_path: PathBuf },
    Viewer,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub signaling_url: String,
    pub room_code: String,
    pub role: Role,
}

pub async fn run(cfg: SessionConfig) -> Result<()> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(&cfg.signaling_url).await?;
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    let role_str = match &cfg.role {
        Role::Host { .. } => "host",
        Role::Viewer => "viewer",
    };
    let client_id = Uuid::new_v4().to_string();
    let hello = serde_json::json!({
        "kind": "hello",
        "role": role_str,
        "roomCode": cfg.room_code,
        "clientId": client_id,
    });
    ws_tx.send(WsMessage::Text(hello.to_string())).await?;
    info!(role = role_str, room = %cfg.room_code, "signaling hello sent");

    let pc = build_peer_connection().await?;
    let done = Arc::new(Notify::new());
    install_state_logging(&pc, done.clone());

    // ICE candidates this side discovers need to be forwarded to the peer.
    let (local_ice_tx, mut local_ice_rx) = mpsc::unbounded_channel::<RTCIceCandidate>();
    let local_ice_tx_clone = local_ice_tx.clone();
    pc.on_ice_candidate(Box::new(move |c| {
        let tx = local_ice_tx_clone.clone();
        Box::pin(async move {
            if let Some(c) = c {
                let _ = tx.send(c);
            }
        })
    }));

    // Role-specific wiring.
    match &cfg.role {
        Role::Host { ivf_path } => {
            let track = add_vp8_track(&pc).await?;
            let path = ivf_path.clone();
            tokio::spawn(pump_ivf_to_track(path, track));
        }
        Role::Viewer => {
            install_viewer_on_track(&pc);
        }
    }

    // Signaling I/O loop. We share the writer end via a channel so the
    // ICE-forwarder task and the SDP handler can both send.
    let (sig_tx, mut sig_rx) = mpsc::unbounded_channel::<serde_json::Value>();
    let sig_tx_for_ice = sig_tx.clone();
    tokio::spawn(async move {
        while let Some(c) = local_ice_rx.recv().await {
            if let Ok(init) = c.to_json() {
                let _ = sig_tx_for_ice.send(serde_json::json!({
                    "kind": "ice",
                    "candidate": {
                        "candidate": init.candidate,
                        "sdpMid": init.sdp_mid,
                        "sdpMLineIndex": init.sdp_mline_index,
                        "usernameFragment": init.username_fragment,
                    }
                }));
            }
        }
    });

    let pc_for_io = pc.clone();
    let role_for_io = cfg.role.clone();
    let done_for_io = done.clone();
    let signaling_task = tokio::spawn(async move {
        // Outbound: take everything from sig_rx and write to the websocket.
        let outbound = async {
            while let Some(msg) = sig_rx.recv().await {
                if ws_tx
                    .send(WsMessage::Text(msg.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        };
        // Inbound: parse signaling messages and dispatch.
        let inbound = async {
            while let Some(Ok(msg)) = ws_rx.next().await {
                let text = match msg {
                    WsMessage::Text(t) => t,
                    WsMessage::Close(_) => break,
                    _ => continue,
                };
                let v: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(?e, "bad signaling json");
                        continue;
                    }
                };
                if let Err(e) =
                    handle_signaling(&pc_for_io, &role_for_io, &v, &sig_tx).await
                {
                    error!(?e, "signaling handler failed");
                }
            }
        };
        tokio::select! {
            _ = outbound => {},
            _ = inbound => {},
            _ = done_for_io.notified() => {},
        }
    });

    // Wait until the PeerConnection closes or fails.
    done.notified().await;
    info!("session ending");
    let _ = pc.close().await;
    signaling_task.abort();
    Ok(())
}

async fn build_peer_connection() -> Result<Arc<RTCPeerConnection>> {
    let mut m = MediaEngine::default();
    // Register VP8 only - keeps the SDP small and the negotiation predictable.
    m.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 96,
            ..Default::default()
        },
        RTPCodecType::Video,
    )?;
    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)?;
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();
    // No STUN/TURN - LAN-only for v0.1 (DESIGN §4.3).
    let cfg = RTCConfiguration {
        ice_servers: vec![RTCIceServer::default()],
        ..Default::default()
    };
    Ok(Arc::new(api.new_peer_connection(cfg).await?))
}

fn install_state_logging(pc: &Arc<RTCPeerConnection>, done: Arc<Notify>) {
    let done_for_peer = done.clone();
    pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        info!(state = %s, "peer connection state");
        if matches!(
            s,
            RTCPeerConnectionState::Failed
                | RTCPeerConnectionState::Closed
                | RTCPeerConnectionState::Disconnected
        ) {
            done_for_peer.notify_waiters();
        }
        Box::pin(async {})
    }));
    pc.on_ice_connection_state_change(Box::new(|s: RTCIceConnectionState| {
        info!(state = %s, "ice connection state");
        Box::pin(async {})
    }));
}

async fn add_vp8_track(
    pc: &Arc<RTCPeerConnection>,
) -> Result<Arc<TrackLocalStaticSample>> {
    let track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_VP8.to_owned(),
            ..Default::default()
        },
        "video".to_owned(),
        "winext-host".to_owned(),
    ));
    let rtp_sender = pc
        .add_track(track.clone() as Arc<dyn TrackLocal + Send + Sync>)
        .await?;
    // Drain RTCP from the sender so the SRTP context advances - if we
    // don't drain, the sender stalls and the receiver eventually times out.
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        while rtp_sender.read(&mut buf).await.is_ok() {}
    });
    Ok(track)
}

async fn pump_ivf_to_track(path: PathBuf, track: Arc<TrackLocalStaticSample>) {
    if let Err(e) = pump_ivf_inner(path, track).await {
        error!(?e, "ivf pump terminated");
    }
}

async fn pump_ivf_inner(path: PathBuf, track: Arc<TrackLocalStaticSample>) -> Result<()> {
    info!(?path, "starting ivf playback loop");
    // Loop indefinitely. Each pass re-opens the file so we don't have to
    // implement seek; M1's test pattern is short on purpose.
    loop {
        let bytes = tokio::fs::read(&path).await?;
        let mut cursor = std::io::Cursor::new(bytes);
        let (mut reader, header) = IVFReader::new(&mut cursor)?;
        let denom = header.timebase_denominator as u64;
        let numer = header.timebase_numerator as u64;
        if denom == 0 {
            return Err(anyhow!("ivf timebase denominator is zero"));
        }
        // Frame period in nanoseconds (assumes constant frame rate).
        let frame_ns = 1_000_000_000u64 * numer / denom;
        let frame_period = Duration::from_nanos(frame_ns.max(1));
        let mut frames = 0u64;
        loop {
            let frame = match reader.parse_next_frame() {
                Ok((data, _hdr)) => data,
                Err(_) => break, // end of file
            };
            track
                .write_sample(&Sample {
                    data: Bytes::from(frame),
                    duration: frame_period,
                    ..Default::default()
                })
                .await?;
            frames += 1;
            tokio::time::sleep(frame_period).await;
        }
        debug!(frames, "ivf loop iteration complete, restarting");
    }
}

fn install_viewer_on_track(pc: &Arc<RTCPeerConnection>) {
    let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let counter_for_stats = counter.clone();
    tokio::spawn(async move {
        let mut last = 0u64;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let now = counter_for_stats.load(std::sync::atomic::Ordering::Relaxed);
            let delta = now - last;
            last = now;
            info!(rtp_packets_per_sec = delta, total = now, "viewer stats");
        }
    });
    pc.on_track(Box::new(move |track, _, _| {
        let codec = track.codec().capability.mime_type.clone();
        let kind = track.kind();
        info!(%codec, kind = %kind, "viewer received track");
        let counter = counter.clone();
        Box::pin(async move {
            // Read RTP packets in a loop and count them.
            tokio::spawn(async move {
                let mut buf = vec![0u8; 1500];
                loop {
                    match track.read(&mut buf).await {
                        Ok(_) => {
                            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(e) => {
                            warn!(?e, "viewer track read ended");
                            break;
                        }
                    }
                }
            });
        })
    }));
}

async fn handle_signaling(
    pc: &Arc<RTCPeerConnection>,
    role: &Role,
    msg: &serde_json::Value,
    out: &mpsc::UnboundedSender<serde_json::Value>,
) -> Result<()> {
    let kind = msg.get("kind").and_then(|k| k.as_str()).unwrap_or("");
    match kind {
        "joined" => {
            let peer_present = msg
                .get("peerPresent")
                .and_then(|p| p.as_bool())
                .unwrap_or(false);
            if peer_present && matches!(role, Role::Host { .. }) {
                send_offer(pc, out).await?;
            }
        }
        "peer-joined" => {
            if matches!(role, Role::Host { .. }) {
                send_offer(pc, out).await?;
            }
        }
        "offer" => {
            let sdp = msg
                .get("sdp")
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow!("offer missing sdp"))?;
            let offer = RTCSessionDescription::offer(sdp.to_string())?;
            pc.set_remote_description(offer).await?;
            let answer = pc.create_answer(None).await?;
            pc.set_local_description(answer.clone()).await?;
            out.send(serde_json::json!({"kind": "answer", "sdp": answer.sdp}))?;
        }
        "answer" => {
            let sdp = msg
                .get("sdp")
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow!("answer missing sdp"))?;
            let answer = RTCSessionDescription::answer(sdp.to_string())?;
            pc.set_remote_description(answer).await?;
        }
        "ice" => {
            let cand = msg
                .get("candidate")
                .ok_or_else(|| anyhow!("ice missing candidate object"))?;
            let init = webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
                candidate: cand
                    .get("candidate")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string(),
                sdp_mid: cand
                    .get("sdpMid")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
                sdp_mline_index: cand
                    .get("sdpMLineIndex")
                    .and_then(|n| n.as_u64())
                    .map(|n| n as u16),
                username_fragment: cand
                    .get("usernameFragment")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
            };
            pc.add_ice_candidate(init).await?;
        }
        "peer-left" => {
            info!("peer left, closing");
            let _ = pc.close().await;
        }
        "error" => {
            let m = msg.get("message").and_then(|s| s.as_str()).unwrap_or("?");
            error!(message = %m, "signaling error from server");
        }
        _ => debug!(kind, "ignoring signaling message"),
    }
    Ok(())
}

async fn send_offer(
    pc: &Arc<RTCPeerConnection>,
    out: &mpsc::UnboundedSender<serde_json::Value>,
) -> Result<()> {
    let offer = pc.create_offer(None).await?;
    pc.set_local_description(offer.clone()).await?;
    out.send(serde_json::json!({"kind": "offer", "sdp": offer.sdp}))?;
    info!("offer sent");
    Ok(())
}
