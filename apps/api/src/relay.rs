//! Relay mode: app <-> this server <-> voice provider.
//!
//! The app speaks a small provider-neutral protocol (below); a [`RelayCodec`]
//! translates it to the provider's. Because every frame passes through here,
//! the server can:
//! - send the system prompt itself (the app never controls the setup),
//! - cut the session off at exactly its time limit, for any provider,
//! - count audio seconds in and out for the cost log,
//! - build the transcript server-side.
//!
//! App -> server:
//! `{"type":"audio","seq":42,"pcm16_b64":"..."}` (PCM16 mono 16 kHz),
//! `{"type":"end_turn"}`, `{"type":"stop"}`
//!
//! Server -> app:
//! `{"type":"ready"}`, `{"type":"audio","seq":17,"pcm16_b64":"...","sample_rate":24000}`,
//! `{"type":"caption","role":"tutor","text":"...","final":false}`,
//! `{"type":"interrupted"}`, `{"type":"turn_complete"}`,
//! `{"type":"minutes","remaining_secs":312}`, `{"type":"ended","reason":"minutes_exhausted"}`
//!
//! **Backpressure:** there are no internal queues. Each send is awaited inside
//! the loop, so while one side is slow we stop reading from the other; memory
//! per session stays bounded. A peer that blocks for longer than
//! `send_timeout` ends the session.

use std::time::Duration;

use axum::extract::ws::{Message as ClientMessage, WebSocket};
use futures_util::{SinkExt, StreamExt, stream::SplitStream};
use serde::{Deserialize, Serialize};
use tokio::time::{Instant, sleep, sleep_until, timeout};
use tokio_tungstenite::tungstenite::{
    Message as UpstreamMessage,
    client::IntoClientRequest,
    http::{HeaderName, HeaderValue},
};

use crate::providers::voice::{CaptionRole, RelayCodec, RelayEvent, RelayTarget, SessionConfig};

/// Largest accepted audio chunk from the app (base64). 128 KiB is ~3 s of
/// 16 kHz PCM16; the app sends 100 ms chunks (~4 KiB).
pub const MAX_AUDIO_B64: usize = 128 * 1024;
/// Largest message the app may send at all (enforced by the WebSocket upgrade).
pub const MAX_CLIENT_MESSAGE: usize = 256 * 1024;
const MAX_TURNS: usize = 2_000;
const MAX_TRANSCRIPT_CHARS: usize = 100_000;

#[derive(Debug, Clone)]
pub struct RelayLimits {
    pub connect_timeout: Duration,
    pub send_timeout: Duration,
    /// End the session if the app sends nothing for this long.
    pub idle: Duration,
    /// How often to tell the app how much time is left.
    pub minutes_every: Duration,
}

impl Default for RelayLimits {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            send_timeout: Duration::from_secs(10),
            idle: Duration::from_secs(60),
            minutes_every: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    ClientStopped,
    ClientDisconnected,
    MinutesExhausted,
    Idle,
    UpstreamClosed,
    ProviderUnavailable,
    ProviderError,
    ProtocolError,
    Backpressure,
}

impl EndReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClientStopped => "client_stopped",
            Self::ClientDisconnected => "client_disconnected",
            Self::MinutesExhausted => "minutes_exhausted",
            Self::Idle => "idle",
            Self::UpstreamClosed => "upstream_closed",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ProviderError => "provider_error",
            Self::ProtocolError => "protocol_error",
            Self::Backpressure => "backpressure",
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FromClient {
    Audio {
        #[allow(dead_code)]
        seq: Option<u64>,
        pcm16_b64: String,
    },
    EndTurn,
    Stop,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ToClient<'a> {
    Ready,
    Audio {
        seq: u64,
        pcm16_b64: &'a str,
        sample_rate: u32,
    },
    Caption {
        role: CaptionRole,
        text: &'a str,
        #[serde(rename = "final")]
        finished: bool,
    },
    Interrupted,
    TurnComplete,
    Minutes {
        remaining_secs: u64,
    },
    Ended {
        reason: EndReason,
    },
}

/// One transcript turn, the same shape the app uploads in direct mode.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Turn {
    pub role: CaptionRole,
    pub text: String,
    pub t_ms: u64,
}

/// Joins streamed caption fragments into turns (one per speaker change).
#[derive(Debug, Default)]
pub struct TranscriptBuilder {
    turns: Vec<Turn>,
    closed: bool,
    chars: usize,
}

impl TranscriptBuilder {
    pub fn add(&mut self, role: CaptionRole, text: &str, finished: bool, t_ms: u64) {
        let text = text.trim_end_matches('\n');
        if text.trim().is_empty() || self.chars >= MAX_TRANSCRIPT_CHARS {
            return;
        }
        self.chars += text.chars().count();
        // Decide first with a short shared borrow, then mutate. Matching on
        // `last_mut()` directly would keep `self.turns` mutably borrowed while
        // the other arm reads `self.turns.len()`.
        let continues_turn = !self.closed && self.turns.last().is_some_and(|t| t.role == role);
        if continues_turn {
            if let Some(last) = self.turns.last_mut() {
                let needs_space = !last.text.ends_with(char::is_whitespace)
                    && !text.starts_with(|c: char| c.is_whitespace() || ".,!?;:'".contains(c));
                if needs_space {
                    last.text.push(' ');
                }
                last.text.push_str(text);
            }
        } else if self.turns.len() < MAX_TURNS {
            self.turns.push(Turn {
                role,
                text: text.trim_start().to_owned(),
                t_ms,
            });
            self.closed = false;
        }
        if finished {
            self.closed = true;
        }
    }

    pub fn close_turn(&mut self) {
        self.closed = true;
    }

    pub fn into_turns(self) -> Vec<Turn> {
        self.turns
            .into_iter()
            .map(|mut t| {
                t.text = t.text.split_whitespace().collect::<Vec<_>>().join(" ");
                t
            })
            .filter(|t| !t.text.is_empty())
            .collect()
    }
}

pub struct RelayOutcome {
    pub reason: EndReason,
    pub transcript: Vec<Turn>,
    /// Learner audio forwarded upstream, in seconds.
    pub audio_in_secs: f64,
    /// Tutor audio sent to the app, in seconds.
    pub audio_out_secs: f64,
}

/// Seconds of PCM16 mono audio in a base64 string at `rate`.
fn pcm16_b64_secs(b64: &str, rate: u32) -> f64 {
    let bytes = b64.len() as f64 * 3.0 / 4.0;
    bytes / 2.0 / f64::from(rate.max(1))
}

async fn send_client(
    tx: &mut futures_util::stream::SplitSink<WebSocket, ClientMessage>,
    msg: &ToClient<'_>,
    limit: Duration,
) -> Result<(), EndReason> {
    let text = serde_json::to_string(msg).map_err(|_| EndReason::ProtocolError)?;
    match timeout(limit, tx.send(ClientMessage::Text(text.into()))).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(EndReason::ClientDisconnected),
        Err(_) => Err(EndReason::Backpressure),
    }
}

/// The app's side of a finished relay, after our `ended` message and Close
/// frame went out. Call [`Closing::finish`] once the session is stored.
pub struct Closing(SplitStream<WebSocket>);

impl Closing {
    /// Finish the WebSocket closing handshake: read and drop whatever the app
    /// still sends until it answers with Close (or `limit` passes). Dropping a
    /// socket with unread data makes some TCP stacks (Windows) reset the
    /// connection, and the app can then lose the `ended` message it has not read.
    pub async fn finish(mut self, limit: Duration) {
        let _ = timeout(limit, async {
            while let Some(Ok(msg)) = self.0.next().await {
                if matches!(msg, ClientMessage::Close(_)) {
                    break;
                }
            }
        })
        .await;
    }
}

/// Run one relay session until it ends; never panics, always sends `ended`
/// and Close to the app. The caller drains the app's side with [`Closing`].
pub async fn run(
    client: WebSocket,
    target: RelayTarget,
    mut codec: Box<dyn RelayCodec>,
    cfg: SessionConfig,
    deadline: Instant,
    limits: RelayLimits,
) -> (RelayOutcome, Closing) {
    let started = Instant::now();
    let (mut client_tx, mut client_rx) = client.split();
    let mut transcript = TranscriptBuilder::default();
    let (mut audio_in_secs, mut audio_out_secs) = (0.0, 0.0);

    // Connect upstream with the provider's auth headers (they hold our API key).
    let upstream = match connect_upstream(&target, limits.connect_timeout).await {
        Some(ws) => ws,
        None => {
            let _ = send_client(
                &mut client_tx,
                &ToClient::Ended {
                    reason: EndReason::ProviderUnavailable,
                },
                limits.send_timeout,
            )
            .await;
            let _ = timeout(limits.send_timeout, client_tx.close()).await;
            let outcome = RelayOutcome {
                reason: EndReason::ProviderUnavailable,
                transcript: vec![],
                audio_in_secs,
                audio_out_secs,
            };
            return (outcome, Closing(client_rx));
        }
    };
    let (mut up_tx, mut up_rx) = upstream.split();

    let reason = 'session: {
        // Our setup goes first; nothing from the app reaches the provider before it.
        for msg in codec.open(&cfg) {
            if !send_upstream(&mut up_tx, msg, limits.send_timeout).await {
                break 'session EndReason::ProviderUnavailable;
            }
        }

        let hard_stop = sleep_until(deadline);
        let idle = sleep(limits.idle);
        tokio::pin!(hard_stop, idle);
        let mut minutes = tokio::time::interval(limits.minutes_every);
        let mut ready = false;
        let mut seq: u64 = 0;

        loop {
            tokio::select! {
                // Time is up: the one rule the app cannot bypass in relay mode.
                () = &mut hard_stop => break EndReason::MinutesExhausted,
                () = &mut idle => break EndReason::Idle,
                _ = minutes.tick() => {
                    let remaining_secs = deadline.saturating_duration_since(Instant::now()).as_secs();
                    if let Err(r) = send_client(&mut client_tx, &ToClient::Minutes { remaining_secs }, limits.send_timeout).await {
                        break r;
                    }
                }
                msg = client_rx.next() => {
                    let text = match msg {
                        Some(Ok(ClientMessage::Text(t))) => t,
                        Some(Ok(ClientMessage::Close(_))) | None | Some(Err(_)) => break EndReason::ClientDisconnected,
                        Some(Ok(_)) => continue, // ping/pong/binary: ignored
                    };
                    idle.as_mut().reset(Instant::now() + limits.idle);
                    match serde_json::from_str::<FromClient>(&text) {
                        Ok(FromClient::Audio { pcm16_b64, .. }) => {
                            if pcm16_b64.len() > MAX_AUDIO_B64 {
                                break EndReason::ProtocolError;
                            }
                            if !ready {
                                continue; // the provider has not accepted the setup yet
                            }
                            let Ok(out) = codec.audio(&pcm16_b64) else { break EndReason::ProtocolError };
                            audio_in_secs += pcm16_b64_secs(&pcm16_b64, 16_000);
                            let mut ok = true;
                            for m in out {
                                if !send_upstream(&mut up_tx, m, limits.send_timeout).await { ok = false; break; }
                            }
                            if !ok { break EndReason::UpstreamClosed; }
                        }
                        Ok(FromClient::EndTurn) => {
                            let mut ok = true;
                            for m in codec.end_turn() {
                                if !send_upstream(&mut up_tx, m, limits.send_timeout).await { ok = false; break; }
                            }
                            if !ok { break EndReason::UpstreamClosed; }
                        }
                        Ok(FromClient::Stop) => break EndReason::ClientStopped,
                        Err(_) => break EndReason::ProtocolError,
                    }
                }
                msg = up_rx.next() => {
                    let text = match msg {
                        Some(Ok(UpstreamMessage::Text(t))) => t.to_string(),
                        // Gemini sends JSON in binary frames too.
                        Some(Ok(UpstreamMessage::Binary(b))) => match String::from_utf8(b.to_vec()) {
                            Ok(t) => t,
                            Err(_) => continue,
                        },
                        Some(Ok(UpstreamMessage::Close(_))) | None | Some(Err(_)) => break EndReason::UpstreamClosed,
                        Some(Ok(_)) => continue,
                    };
                    let t_ms = started.elapsed().as_millis() as u64;
                    let mut end = None;
                    for ev in codec.decode(&text) {
                        let sent = match &ev {
                            RelayEvent::Ready => {
                                ready = true;
                                send_client(&mut client_tx, &ToClient::Ready, limits.send_timeout).await
                            }
                            RelayEvent::Audio { pcm16_b64, sample_rate } => {
                                seq += 1;
                                audio_out_secs += pcm16_b64_secs(pcm16_b64, *sample_rate);
                                send_client(&mut client_tx, &ToClient::Audio { seq, pcm16_b64, sample_rate: *sample_rate }, limits.send_timeout).await
                            }
                            RelayEvent::Caption { role, text, finished } => {
                                transcript.add(*role, text, *finished, t_ms);
                                send_client(&mut client_tx, &ToClient::Caption { role: *role, text, finished: *finished }, limits.send_timeout).await
                            }
                            RelayEvent::Interrupted => send_client(&mut client_tx, &ToClient::Interrupted, limits.send_timeout).await,
                            RelayEvent::TurnComplete => {
                                transcript.close_turn();
                                send_client(&mut client_tx, &ToClient::TurnComplete, limits.send_timeout).await
                            }
                            RelayEvent::Error(e) => {
                                tracing::warn!(error = %e, "voice provider reported an error");
                                Err(EndReason::ProviderError)
                            }
                        };
                        if let Err(r) = sent { end = Some(r); break; }
                    }
                    if let Some(r) = end { break r; }
                }
            }
        }
    };

    // Close both sides; errors here just mean the peer is already gone.
    let _ = send_client(
        &mut client_tx,
        &ToClient::Ended { reason },
        limits.send_timeout,
    )
    .await;
    let _ = timeout(limits.send_timeout, client_tx.close()).await;
    let _ = timeout(limits.send_timeout, up_tx.close()).await;

    let outcome = RelayOutcome {
        reason,
        transcript: transcript.into_turns(),
        audio_in_secs,
        audio_out_secs,
    };
    (outcome, Closing(client_rx))
}

type Upstream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect_upstream(target: &RelayTarget, limit: Duration) -> Option<Upstream> {
    let mut request = match target.url.as_str().into_client_request() {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "bad relay upstream url");
            return None;
        }
    };
    for (name, value) in &target.headers {
        // Header values hold API keys: never log them.
        let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) else {
            tracing::error!("bad relay upstream header");
            return None;
        };
        request.headers_mut().insert(name, value);
    }
    match timeout(limit, tokio_tungstenite::connect_async(request)).await {
        Ok(Ok((ws, _response))) => Some(ws),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "relay upstream connect failed");
            None
        }
        Err(_) => {
            tracing::warn!("relay upstream connect timed out");
            None
        }
    }
}

async fn send_upstream(
    tx: &mut futures_util::stream::SplitSink<Upstream, UpstreamMessage>,
    msg: String,
    limit: Duration,
) -> bool {
    matches!(
        timeout(limit, tx.send(UpstreamMessage::Text(msg.into()))).await,
        Ok(Ok(()))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_joins_fragments_and_splits_turns() {
        let mut t = TranscriptBuilder::default();
        t.add(CaptionRole::Tutor, "Hi! Tell me", false, 0);
        t.add(CaptionRole::Tutor, "about your day.", false, 100);
        t.close_turn();
        t.add(CaptionRole::Learner, "I go to office", false, 3000);
        t.add(CaptionRole::Learner, ".", false, 3100);
        t.add(CaptionRole::Tutor, "You mean: I went.", false, 5000);
        let turns = t.into_turns();
        assert_eq!(
            turns,
            vec![
                Turn {
                    role: CaptionRole::Tutor,
                    text: "Hi! Tell me about your day.".into(),
                    t_ms: 0
                },
                Turn {
                    role: CaptionRole::Learner,
                    text: "I go to office.".into(),
                    t_ms: 3000
                },
                Turn {
                    role: CaptionRole::Tutor,
                    text: "You mean: I went.".into(),
                    t_ms: 5000
                },
            ]
        );
    }

    #[test]
    fn finished_captions_close_their_turn() {
        let mut t = TranscriptBuilder::default();
        t.add(CaptionRole::Learner, "First.", true, 0);
        t.add(CaptionRole::Learner, "Second.", true, 10);
        assert_eq!(t.into_turns().len(), 2);
    }

    #[test]
    fn transcript_is_capped() {
        let mut t = TranscriptBuilder::default();
        let big = "x".repeat(60_000);
        t.add(CaptionRole::Learner, &big, true, 0);
        t.add(CaptionRole::Learner, &big, true, 1);
        t.add(CaptionRole::Learner, "ignored", true, 2);
        assert_eq!(
            t.into_turns().len(),
            2,
            "third turn dropped after the char cap"
        );
    }

    #[test]
    fn audio_seconds_from_base64_length() {
        // 1 s of 16 kHz PCM16 = 32 000 bytes = 42 668 base64 chars (rounded up).
        let secs = pcm16_b64_secs(&"A".repeat(42_668), 16_000);
        assert!((secs - 1.0).abs() < 0.001, "{secs}");
    }

    #[test]
    fn client_messages_parse() {
        assert!(matches!(
            serde_json::from_str::<FromClient>(r#"{"type":"audio","seq":1,"pcm16_b64":"AA=="}"#),
            Ok(FromClient::Audio { .. })
        ));
        assert!(matches!(
            serde_json::from_str::<FromClient>(r#"{"type":"stop"}"#),
            Ok(FromClient::Stop)
        ));
        assert!(serde_json::from_str::<FromClient>(r#"{"type":"setup","prompt":"evil"}"#).is_err());
    }
}
