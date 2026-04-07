//! Qwen Omni Realtime STT Client (阿里云通义千问)
//!
//! Implements WebSocket-based streaming speech-to-text using Qwen Omni Realtime API
//! via Alibaba Cloud DashScope.
//!
//! # Protocol
//! - WebSocket URL: `wss://dashscope.aliyuncs.com/api-ws/v1/inference/`
//! - JSON-based OpenAI-compatible session protocol
//!
//! # Timeout
//! - **Server-side idle timeout: Unknown** — not documented in official API reference.
//! - Qwen-Omni-Realtime (full multimodal): 120-minute maximum session duration
//! - SDK client-side heartbeat: 6 seconds
//! - Client-side session ready timeout: 5 seconds
//! - Client-side final result timeout: 30 seconds
//!
//! # Reference
//! - <https://www.alibabacloud.com/help/en/model-studio/qwen-real-time-speech-recognition>
//! - <https://www.alibabacloud.com/help/en/model-studio/realtime> (Qwen-Omni-Realtime)

use async_trait::async_trait;
use futures_util::{SinkExt, Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::protocol::Message};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::commands::settings::CloudSttConfig;
use crate::stt_engine::traits::{
    EngineType, PartialResult, PartialResultCallback, StreamingSttEngine, SttContext,
    TranscriptionResult,
};

const QWEN_OMNI_REALTIME_ENDPOINT: &str = "wss://dashscope.aliyuncs.com/api-ws/v1/realtime";
const QWEN_OMNI_REALTIME_MODEL: &str = "qwen3-asr-flash-realtime";
const LEGACY_QWEN_OMNI_REALTIME_MODEL: &str = "qwen3.5-omni-plus-realtime";
const SESSION_READY_TIMEOUT: Duration = Duration::from_secs(2);
const FINAL_RESULT_TIMEOUT: Duration = Duration::from_secs(10);

pub const RECOMMENDED_CHUNK_SAMPLES: usize = 1600;

type BoxStream = Pin<Box<dyn Stream<Item = Result<Message, WsError>> + Send>>;
type BoxSink = Pin<Box<dyn futures_util::Sink<Message, Error = WsError> + Send>>;
type SessionResultRx = Arc<Mutex<Option<oneshot::Receiver<Result<(), String>>>>>;

pub struct QwenOmniRealtimeClient {
    tx: Arc<Mutex<Option<BoxSink>>>,
    rx: Arc<Mutex<Option<BoxStream>>>,
    config: CloudSttConfig,
    language: String,
    audio_tx: Arc<Mutex<Option<mpsc::Sender<Vec<i16>>>>>,
    on_partial: Option<PartialResultCallback>,
    audio_sender_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    last_error: Arc<Mutex<Option<String>>>,
    session_finished_rx: SessionResultRx,
    final_transcript: Arc<Mutex<String>>,
}

unsafe impl Send for QwenOmniRealtimeClient {}
unsafe impl Sync for QwenOmniRealtimeClient {}

impl QwenOmniRealtimeClient {
    pub fn new(config: CloudSttConfig, language: Option<&str>, _context: SttContext) -> Self {
        let lang = normalize_realtime_language(language).unwrap_or_default();

        Self {
            tx: Arc::new(Mutex::new(None)),
            rx: Arc::new(Mutex::new(None)),
            config,
            language: lang.to_string(),
            audio_tx: Arc::new(Mutex::new(None)),
            on_partial: None,
            audio_sender_task: Arc::new(Mutex::new(None)),
            last_error: Arc::new(Mutex::new(None)),
            session_finished_rx: Arc::new(Mutex::new(None)),
            final_transcript: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        self.on_partial = Some(callback);
    }

    pub async fn get_audio_sender(&self) -> Option<mpsc::Sender<Vec<i16>>> {
        self.audio_tx.lock().await.clone()
    }

    #[instrument(skip(self), fields(provider = "qwen-omni-realtime"), ret, err)]
    pub async fn connect(&mut self) -> Result<(), String> {
        if self.config.api_key.is_empty() {
            return Err("AliYun DashScope API key is empty. Please configure your AliYun DashScope API key in Settings > Cloud STT.".to_string());
        }

        let endpoint = if self.config.base_url.is_empty() {
            QWEN_OMNI_REALTIME_ENDPOINT
        } else {
            &self.config.base_url
        };

        let model = resolve_realtime_model(&self.config.model);

        let url = format!("{}?model={}", endpoint, model);

        info!(provider = "qwen-omni-realtime", url = %url, "websocket_connecting");

        let mut request =
            tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(&url)
                .map_err(|e| format!("Invalid URL: {}", e))?;

        let headers = request.headers_mut();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.config.api_key).parse().unwrap(),
        );
        headers.insert("OpenAI-Beta", "realtime=v1".parse().unwrap());

        let result = connect_async_tls_with_config(request, None, false, None).await;

        let (ws_stream, _response) = match result {
            Ok(stream) => stream,
            Err(e) => {
                let error_str = e.to_string();

                if error_str.contains("401")
                    || error_str.contains("Unauthorized")
                    || error_str.contains("invalid_api_key")
                {
                    error!(provider = "qwen-omni-realtime", http.status_code = 401, error = %error_str, "websocket_connect_failed");
                    return Err(format!(
                        "Qwen Omni Realtime API authentication failed (401 Unauthorized).\n\
                        \n\
                        Please verify your API key in Settings > Cloud STT.\n\
                        Get your API key from: https://bailian.console.aliyun.com/\n\
                        \n\
                        Technical details: {}",
                        error_str
                    ));
                }

                if error_str.contains("403") || error_str.contains("Forbidden") {
                    error!(provider = "qwen-omni-realtime", http.status_code = 403, error = %error_str, "websocket_connect_failed");
                    return Err(format!(
                        "Qwen Omni Realtime API access forbidden (403).\n\
                        \n\
                        Possible causes:\n\
                        1. Your API key doesn't have access to the Realtime API\n\
                        2. Realtime API is not enabled for your account\n\
                        3. You're using an organization API key without proper permissions\n\
                        \n\
                        Contact AliYun support or check your API key permissions.\n\
                        \n\
                        Technical details: {}",
                        error_str
                    ));
                }

                error!(provider = "qwen-omni-realtime", error = %error_str, "websocket_connect_failed");
                return Err(format!(
                    "Failed to connect to Qwen Omni Realtime API: {}",
                    error_str
                ));
            }
        };

        info!(
            provider = "qwen-omni-realtime",
            http.status_code = 101,
            "websocket_connected"
        );

        let (sink, stream) = ws_stream.split();

        *self.tx.lock().await = Some(Box::pin(sink));
        *self.rx.lock().await = Some(Box::pin(stream));

        *self.last_error.lock().await = None;
        self.final_transcript.lock().await.clear();

        let (session_ready_tx, session_ready_rx) = oneshot::channel();
        let (session_finished_tx, session_finished_rx) = oneshot::channel();
        *self.session_finished_rx.lock().await = Some(session_finished_rx);

        self.start_result_receiver(session_ready_tx, session_finished_tx)
            .await;

        self.send_session_update().await?;

        match tokio::time::timeout(SESSION_READY_TIMEOUT, session_ready_rx).await {
            Ok(Ok(Ok(()))) => {
                info!(
                    provider = "qwen-omni-realtime",
                    "session_update_acknowledged"
                );
            }
            Ok(Ok(Err(err))) => {
                self.close().await;
                return Err(err);
            }
            Ok(Err(_)) => {
                self.close().await;
                return Err(
                    "Qwen Omni Realtime connection closed before session was ready".to_string(),
                );
            }
            Err(_) => {
                self.close().await;
                return Err(
                    "Timed out waiting for Qwen Omni Realtime session acknowledgement".to_string(),
                );
            }
        }

        let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(50);
        *self.audio_tx.lock().await = Some(audio_tx.clone());

        self.start_audio_sender(audio_rx).await;

        Ok(())
    }

    async fn send_session_update(&self) -> Result<(), String> {
        let message = self.build_session_update_message();

        let message_str = message.to_string();
        debug!(provider = "qwen-omni-realtime", payload = %message_str, "session_update_payload");

        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or("WebSocket not connected")?;
        tx.send(Message::Text(message_str.into()))
            .await
            .map_err(|e| format!("Failed to send session update: {}", e))?;

        info!(provider = "qwen-omni-realtime", "session_update_sent");
        Ok(())
    }

    fn build_session_update_message(&self) -> serde_json::Value {
        let mut transcription = serde_json::json!({});

        if !self.language.is_empty() {
            transcription["language"] = serde_json::json!(self.language);
        }

        serde_json::json!({
            "type": "session.update",
            "event_id": Uuid::new_v4().to_string(),
            "session": {
                "modalities": ["text"],
                "input_audio_format": "pcm",
                "sample_rate": 16000,
                "input_audio_transcription": transcription,
                "turn_detection": serde_json::Value::Null
            }
        })
    }

    async fn start_result_receiver(
        &self,
        session_ready_tx: oneshot::Sender<Result<(), String>>,
        session_finished_tx: oneshot::Sender<Result<(), String>>,
    ) {
        let rx = self.rx.clone();
        let tx = self.tx.clone();
        let on_partial = self.on_partial.clone();
        let last_error = self.last_error.clone();
        let final_transcript = self.final_transcript.clone();

        tokio::spawn(async move {
            let mut session_ready_tx = Some(session_ready_tx);
            let mut session_finished_tx = Some(session_finished_tx);
            let mut rx_guard = rx.lock().await;
            let rx_stream = match rx_guard.take() {
                Some(s) => s,
                None => {
                    warn!(provider = "qwen-omni-realtime", "no_receiver_available");
                    return;
                }
            };
            drop(rx_guard);

            let mut stream = rx_stream;
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!(provider = "qwen-omni-realtime", message_preview = %format!("{:.200}...", text), "websocket_message_received");

                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
                                match msg_type {
                                    "session.created" => {
                                        debug!(provider = "qwen-omni-realtime", "session_created");
                                    }
                                    "session.updated" => {
                                        info!(provider = "qwen-omni-realtime", "session_updated");
                                        if let Some(tx) = session_ready_tx.take() {
                                            let _ = tx.send(Ok(()));
                                        }
                                    }
                                    "error" => {
                                        if let Some(error_obj) = parsed.get("error") {
                                            if let Ok(error_msg) =
                                                serde_json::from_value::<RealtimeError>(
                                                    error_obj.clone(),
                                                )
                                            {
                                                let user_error =
                                                    error_msg.to_user_friendly_message();
                                                error!(provider = "qwen-omni-realtime", error = %user_error, "server_error");

                                                *last_error.lock().await = Some(user_error);
                                                *tx.lock().await = None;

                                                if let Some(err) = last_error.lock().await.clone() {
                                                    if let Some(tx) = session_ready_tx.take() {
                                                        let _ = tx.send(Err(err.clone()));
                                                    }
                                                    if let Some(tx) = session_finished_tx.take() {
                                                        let _ = tx.send(Err(err));
                                                    }
                                                }

                                                if let Some(ref callback) = on_partial {
                                                    callback(PartialResult {
                                                        text: String::new(),
                                                        is_definite: false,
                                                        is_final: true,
                                                    });
                                                }
                                                break;
                                            } else {
                                                let user_error = format!(
                                                    "Qwen Omni Realtime API error: {}",
                                                    error_obj
                                                );
                                                error!(provider = "qwen-omni-realtime", error = %user_error, "server_error");
                                                *last_error.lock().await = Some(user_error.clone());
                                                *tx.lock().await = None;

                                                if let Some(tx) = session_ready_tx.take() {
                                                    let _ = tx.send(Err(user_error.clone()));
                                                }
                                                if let Some(tx) = session_finished_tx.take() {
                                                    let _ = tx.send(Err(user_error));
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    "conversation.item.input_audio_transcription.text" => {
                                        let confirmed = parsed
                                            .get("text")
                                            .and_then(|t| t.as_str())
                                            .unwrap_or("");
                                        let stash = parsed
                                            .get("stash")
                                            .and_then(|t| t.as_str())
                                            .unwrap_or("");
                                        let preview = format!("{}{}", confirmed, stash);

                                        if !preview.is_empty() {
                                            debug!(provider = "qwen-omni-realtime", text = %preview, "partial_transcription");

                                            if let Some(ref callback) = on_partial {
                                                callback(PartialResult {
                                                    text: preview,
                                                    is_definite: false,
                                                    is_final: false,
                                                });
                                            }
                                        }
                                    }
                                    "conversation.item.input_audio_transcription.completed" => {
                                        if let Some(transcript) =
                                            parsed.get("transcript").and_then(|t| t.as_str())
                                        {
                                            debug!(provider = "qwen-omni-realtime", text = %transcript, "transcription_completed");
                                            *final_transcript.lock().await = transcript.to_string();

                                            if let Some(ref callback) = on_partial {
                                                callback(PartialResult {
                                                    text: transcript.to_string(),
                                                    is_definite: true,
                                                    is_final: false,
                                                });
                                            }
                                        }
                                    }
                                    "conversation.item.input_audio_transcription.failed" => {
                                        let user_error = parsed
                                            .get("error")
                                            .and_then(|error| error.get("message"))
                                            .and_then(|message| message.as_str())
                                            .map_or_else(
                                                || {
                                                    "Qwen Omni Realtime failed to transcribe the input audio"
                                                        .to_string()
                                                },
                                                |message| {
                                                    format!(
                                                        "Qwen Omni Realtime failed to transcribe the input audio: {}",
                                                        message
                                                    )
                                                },
                                            );

                                        error!(provider = "qwen-omni-realtime", error = %user_error, "transcription_failed");
                                        *last_error.lock().await = Some(user_error.clone());
                                        *tx.lock().await = None;

                                        if let Some(tx) = session_ready_tx.take() {
                                            let _ = tx.send(Err(user_error.clone()));
                                        }
                                        if let Some(tx) = session_finished_tx.take() {
                                            let _ = tx.send(Err(user_error));
                                        }
                                        break;
                                    }
                                    "session.finished" => {
                                        info!(provider = "qwen-omni-realtime", "session_finished");
                                        if let Some(tx) = session_ready_tx.take() {
                                            let _ = tx.send(Ok(()));
                                        }
                                        if let Some(ref callback) = on_partial {
                                            callback(PartialResult {
                                                text: String::new(),
                                                is_definite: false,
                                                is_final: true,
                                            });
                                        }
                                        if let Some(tx) = session_finished_tx.take() {
                                            let _ = tx.send(Ok(()));
                                        }
                                        break;
                                    }
                                    _ => {
                                        debug!(provider = "qwen-omni-realtime", message_type = %msg_type, "unhandled_message_type");
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        if let Some(frame) = frame {
                            warn!(
                                provider = "qwen-omni-realtime",
                                code = ?frame.code,
                                reason = %frame.reason,
                                "connection_closed_by_server"
                            );
                            *tx.lock().await = None;
                            let mut last_error_guard = last_error.lock().await;
                            if last_error_guard.is_none() {
                                *last_error_guard = Some(format!(
                                    "Qwen Omni Realtime connection closed by server (code={:?}, reason={})",
                                    frame.code, frame.reason
                                ));
                            }
                            if let Some(err) = last_error_guard.clone() {
                                if let Some(tx) = session_ready_tx.take() {
                                    let _ = tx.send(Err(err.clone()));
                                }
                                if let Some(tx) = session_finished_tx.take() {
                                    let _ = tx.send(Err(err));
                                }
                            }
                        } else {
                            warn!(
                                provider = "qwen-omni-realtime",
                                "connection_closed_no_frame"
                            );
                            *tx.lock().await = None;
                            let mut last_error_guard = last_error.lock().await;
                            if last_error_guard.is_none() {
                                *last_error_guard = Some(
                                    "Qwen Omni Realtime connection closed by server without close frame"
                                        .to_string(),
                                );
                            }
                            if let Some(err) = last_error_guard.clone() {
                                if let Some(tx) = session_ready_tx.take() {
                                    let _ = tx.send(Err(err.clone()));
                                }
                                if let Some(tx) = session_finished_tx.take() {
                                    let _ = tx.send(Err(err));
                                }
                            }
                        }
                        break;
                    }
                    Err(e) => {
                        error!(provider = "qwen-omni-realtime", error = %e, "websocket_error");
                        *tx.lock().await = None;
                        let err = format!("Qwen Omni Realtime WebSocket error: {}", e);
                        *last_error.lock().await = Some(err.clone());
                        if let Some(tx) = session_ready_tx.take() {
                            let _ = tx.send(Err(err.clone()));
                        }
                        if let Some(tx) = session_finished_tx.take() {
                            let _ = tx.send(Err(err));
                        }
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    async fn start_audio_sender(&self, mut audio_rx: mpsc::Receiver<Vec<i16>>) {
        let tx = self.tx.clone();
        let task_handle = tokio::spawn(async move {
            let mut chunk_count = 0u32;
            while let Some(pcm_data) = audio_rx.recv().await {
                let mut guard = tx.lock().await;
                if let Some(sender) = guard.as_mut() {
                    if let Err(e) = send_audio_chunk(sender, &pcm_data).await {
                        error!(provider = "qwen-omni-realtime", error = %e, "audio_chunk_send_failed");
                        break;
                    }
                    chunk_count += 1;
                } else {
                    debug!(provider = "qwen-omni-realtime", "audio_sender_stopped");
                    break;
                }
            }
            info!(
                provider = "qwen-omni-realtime",
                chunks = chunk_count,
                "audio_sender_finished"
            );
        });

        *self.audio_sender_task.lock().await = Some(task_handle);
    }

    pub async fn send_audio(&self, pcm_data: &[i16]) -> Result<(), String> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or("WebSocket not connected")?;
        send_audio_chunk(tx, pcm_data).await
    }

    #[instrument(skip(self), fields(provider = "qwen-omni-realtime"), err)]
    pub async fn send_audio_async(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        let guard = self.audio_tx.lock().await;
        let tx = guard
            .as_ref()
            .ok_or("Audio channel not initialized - call connect() first")?;
        tx.send(pcm_data)
            .await
            .map_err(|e| format!("Failed to queue audio: {}", e))
    }

    #[instrument(skip(self), fields(provider = "qwen-omni-realtime"), ret, err)]
    pub async fn finish(&self) -> Result<String, String> {
        let start = Instant::now();

        drop(self.audio_tx.lock().await.take());

        let task_handle = self.audio_sender_task.lock().await.take();
        if let Some(handle) = task_handle {
            match handle.await {
                Ok(()) => debug!(
                    provider = "qwen-omni-realtime",
                    "audio_sender_task_completed"
                ),
                Err(e) => {
                    warn!(provider = "qwen-omni-realtime", error = %e, "audio_sender_task_error")
                }
            }
        }

        if let Some(err) = self.last_error.lock().await.clone() {
            self.close().await;
            return Err(err);
        }

        let mut guard = self.tx.lock().await;
        if let Some(tx) = guard.as_mut() {
            let commit_message = serde_json::json!({
                "type": "input_audio_buffer.commit",
                "event_id": Uuid::new_v4().to_string()
            });

            tx.send(Message::Text(commit_message.to_string().into()))
                .await
                .map_err(|e| format!("Failed to send commit packet: {}", e))?;

            debug!(provider = "qwen-omni-realtime", "commit_packet_sent");
            let finish_message = serde_json::json!({
                "type": "session.finish",
                "event_id": Uuid::new_v4().to_string()
            });

            tx.send(Message::Text(finish_message.to_string().into()))
                .await
                .map_err(|e| format!("Failed to send session finish packet: {}", e))?;

            debug!(
                provider = "qwen-omni-realtime",
                "session_finish_packet_sent"
            );
        } else if let Some(err) = self.last_error.lock().await.clone() {
            return Err(err);
        } else {
            return Err("WebSocket not connected".to_string());
        }
        drop(guard);

        let session_finished_rx = self.session_finished_rx.lock().await.take();
        if let Some(rx) = session_finished_rx {
            match tokio::time::timeout(FINAL_RESULT_TIMEOUT, rx).await {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(err))) => {
                    self.close().await;
                    return Err(err);
                }
                Ok(Err(_)) => {
                    self.close().await;
                    return Err("Qwen Omni Realtime result channel closed".to_string());
                }
                Err(_) => {
                    self.close().await;
                    return Err("Timeout waiting for Qwen Omni Realtime final result".to_string());
                }
            }
        } else {
            self.close().await;
            return Err("No Qwen Omni Realtime result receiver available".to_string());
        }

        if let Some(err) = self.last_error.lock().await.clone() {
            self.close().await;
            return Err(err);
        }

        let final_text = self.final_transcript.lock().await.clone();
        self.close().await;

        let total_ms = start.elapsed().as_millis() as u64;
        info!(
            provider = "qwen-omni-realtime",
            duration_ms = total_ms,
            "transcription_finished"
        );

        Ok(final_text)
    }

    #[instrument(skip(self), fields(provider = "qwen-omni-realtime"))]
    pub async fn close(&self) {
        drop(self.audio_tx.lock().await.take());
        *self.session_finished_rx.lock().await = None;
        let mut guard = self.tx.lock().await;
        if let Some(mut tx) = guard.take() {
            let _ = tx.close().await;
            info!(provider = "qwen-omni-realtime", "connection_closed");
        }
        *self.rx.lock().await = None;
    }
}

#[async_trait]
impl StreamingSttEngine for QwenOmniRealtimeClient {
    async fn start(&mut self) -> Result<(), String> {
        self.connect().await
    }

    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        self.send_audio_async(pcm_data).await
    }

    async fn finish(&self) -> Result<String, String> {
        QwenOmniRealtimeClient::finish(self).await
    }

    fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        QwenOmniRealtimeClient::set_partial_callback(self, callback);
    }

    async fn get_audio_sender(&self) -> Option<mpsc::Sender<Vec<i16>>> {
        QwenOmniRealtimeClient::get_audio_sender(self).await
    }
}

fn normalize_realtime_language(language: Option<&str>) -> Option<String> {
    match language {
        Some("auto") | None => None,
        Some(language) => Some(language.split('-').next().unwrap_or(language).to_string()),
    }
}

async fn send_audio_chunk(sender: &mut BoxSink, pcm_data: &[i16]) -> Result<(), String> {
    let bytes: Vec<u8> = pcm_data.iter().flat_map(|&s| s.to_le_bytes()).collect();

    let base64_data = base64::encode(&bytes);

    let message = serde_json::json!({
        "type": "input_audio_buffer.append",
        "event_id": Uuid::new_v4().to_string(),
        "audio": base64_data
    });

    sender
        .send(Message::Text(message.to_string().into()))
        .await
        .map_err(|e| format!("Failed to send audio: {}", e))?;

    debug!(
        provider = "qwen-omni-realtime",
        samples = pcm_data.len(),
        "audio_samples_sent"
    );
    Ok(())
}

#[derive(serde::Deserialize, Debug)]
pub struct RealtimeError {
    pub code: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

impl RealtimeError {
    pub fn to_user_friendly_message(&self) -> String {
        match self.code.as_str() {
            "invalid_api_key" => format!(
                "AliYun DashScope API key is invalid or missing.\n\
                \n\
                Please verify your API key in Settings > Cloud STT.\n\
                Get your API key from: https://bailian.console.aliyun.com/\n\
                \n\
                Technical details: {}",
                self.message
            ),
            "insufficient_quota" => format!(
                "AliYun DashScope API quota exceeded.\n\
                \n\
                Your account has insufficient balance or quota for the Realtime API.\n\
                Check your usage and billing at: https://bailian.console.aliyun.com/\n\
                \n\
                Technical details: {}",
                self.message
            ),
            "model_not_found" => format!(
                "Qwen Omni Realtime model not found.\n\
                \n\
                The specified model '{}' may not be available or you may not have access.\n\
                Contact OpenAI support if you believe this is an error.\n\
                \n\
                Technical details: {}",
                QWEN_OMNI_REALTIME_MODEL, self.message
            ),
            _ => format!(
                "Qwen Omni Realtime API error: {}\n\
                Error code: {}\n\
                Error type: {}\n\
                \n\
                Check your API key, model availability, and network connection.",
                self.message, self.code, self.error_type
            ),
        }
    }
}

fn resolve_realtime_model(model: &str) -> &str {
    if model.is_empty() || model == LEGACY_QWEN_OMNI_REALTIME_MODEL {
        QWEN_OMNI_REALTIME_MODEL
    } else {
        model
    }
}

pub async fn transcribe_qwen_omni_realtime(
    config: &CloudSttConfig,
    audio_path: &std::path::Path,
    language: Option<&str>,
) -> Result<TranscriptionResult, String> {
    let start = Instant::now();

    if !config.enabled {
        return Err("Qwen Omni Realtime STT is not enabled".to_string());
    }

    if config.api_key.is_empty() {
        return Err("AliYun DashScope API key is empty".to_string());
    }

    let temp_wav_path = audio_path.with_extension("temp_16k.wav");

    let ffmpeg_result = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            audio_path.to_str().ok_or("Invalid audio path")?,
            "-ar",
            "16000",
            "-ac",
            "1",
            "-f",
            "wav",
            "-acodec",
            "pcm_s16le",
            temp_wav_path.to_str().ok_or("Invalid temp path")?,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    if !ffmpeg_result.status.success() {
        return Err(format!(
            "FFmpeg conversion failed: {}",
            String::from_utf8_lossy(&ffmpeg_result.stderr)
        ));
    }

    let converted_bytes = tokio::fs::read(&temp_wav_path)
        .await
        .map_err(|e| format!("Failed to read converted audio: {}", e))?;

    let _ = tokio::fs::remove_file(&temp_wav_path).await;

    if converted_bytes.len() < 44 {
        return Err("Converted audio too short".to_string());
    }

    let pcm_bytes = &converted_bytes[44..];

    let samples_16khz_mono: Vec<i16> = pcm_bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    let mut client = QwenOmniRealtimeClient::new(config.clone(), language, SttContext::default());

    client.connect().await?;

    let audio_tx = client
        .get_audio_sender()
        .await
        .ok_or("Failed to get audio sender")?;

    for chunk in samples_16khz_mono.chunks(RECOMMENDED_CHUNK_SAMPLES) {
        audio_tx
            .send(chunk.to_vec())
            .await
            .map_err(|e| format!("Failed to send audio chunk: {}", e))?;

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    drop(audio_tx);

    let final_text = client.finish().await?;

    let total_ms = start.elapsed().as_millis() as u64;

    info!(
        provider = "qwen-omni-realtime",
        text_len = final_text.len(),
        duration_ms = total_ms,
        "transcription_complete"
    );

    Ok(TranscriptionResult::with_metrics(
        final_text,
        EngineType::Cloud,
        total_ms,
        Some(0),
        Some(0),
        Some(total_ms),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qwen_omni_realtime_endpoint_constant() {
        assert_eq!(
            QWEN_OMNI_REALTIME_ENDPOINT,
            "wss://dashscope.aliyuncs.com/api-ws/v1/realtime"
        );
    }

    #[test]
    fn test_qwen_omni_realtime_model_constant() {
        assert_eq!(QWEN_OMNI_REALTIME_MODEL, "qwen3-asr-flash-realtime");
    }

    #[test]
    fn test_recommended_chunk_samples() {
        assert_eq!(RECOMMENDED_CHUNK_SAMPLES, 1600);
        let chunk_duration_ms = (RECOMMENDED_CHUNK_SAMPLES as f64 / 16000.0) * 1000.0;
        assert!((chunk_duration_ms - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_partial_result_fields() {
        let result = PartialResult {
            text: "hello".to_string(),
            is_definite: true,
            is_final: false,
        };
        assert_eq!(result.text, "hello");
        assert!(result.is_definite);
        assert!(!result.is_final);
    }

    #[test]
    fn test_partial_result_serialization() {
        let result = PartialResult {
            text: "test transcript".to_string(),
            is_definite: false,
            is_final: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test transcript"));
        assert!(json.contains("is_definite"));
        assert!(json.contains("is_final"));
    }

    #[test]
    fn test_realtime_error_deserialization() {
        let json = r#"{
            "code": "invalid_api_key",
            "type": "invalid_request_error",
            "message": "Invalid API key provided"
        }"#;
        let error: RealtimeError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, "invalid_api_key");
        assert_eq!(error.error_type, "invalid_request_error");
        assert_eq!(error.message, "Invalid API key provided");
    }

    #[test]
    fn test_realtime_error_user_friendly_invalid_api_key() {
        let error = RealtimeError {
            code: "invalid_api_key".to_string(),
            error_type: "invalid_request_error".to_string(),
            message: "Bad key".to_string(),
        };
        let msg = error.to_user_friendly_message();
        assert!(msg.contains("API key is invalid"));
        assert!(msg.contains("Bad key"));
    }

    #[test]
    fn test_realtime_error_user_friendly_insufficient_quota() {
        let error = RealtimeError {
            code: "insufficient_quota".to_string(),
            error_type: "insufficient_quota".to_string(),
            message: "Quota exceeded".to_string(),
        };
        let msg = error.to_user_friendly_message();
        assert!(msg.contains("quota exceeded"));
        assert!(msg.contains("Quota exceeded"));
    }

    #[test]
    fn test_realtime_error_user_friendly_model_not_found() {
        let error = RealtimeError {
            code: "model_not_found".to_string(),
            error_type: "not_found".to_string(),
            message: "Model not found".to_string(),
        };
        let msg = error.to_user_friendly_message();
        assert!(msg.contains("model not found"));
        assert!(msg.contains(QWEN_OMNI_REALTIME_MODEL));
    }

    #[test]
    fn test_realtime_error_user_friendly_unknown_code() {
        let error = RealtimeError {
            code: "unknown_error".to_string(),
            error_type: "some_type".to_string(),
            message: "Something went wrong".to_string(),
        };
        let msg = error.to_user_friendly_message();
        assert!(msg.contains("unknown_error"));
        assert!(msg.contains("some_type"));
        assert!(msg.contains("Something went wrong"));
    }

    #[test]
    fn test_openai_realtime_client_new_with_language() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "qwen-omni-realtime".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };
        let client = QwenOmniRealtimeClient::new(config, Some("en"), SttContext::default());
        assert_eq!(client.language, "en");
    }

    #[test]
    fn test_openai_realtime_client_normalizes_bcp47_language() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "qwen-omni-realtime".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };
        let client = QwenOmniRealtimeClient::new(config, Some("zh-CN"), SttContext::default());
        assert_eq!(client.language, "zh");
    }

    #[test]
    fn test_session_update_message_omits_output_audio_format_for_text_only() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "qwen-omni-realtime".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };
        let client = QwenOmniRealtimeClient::new(config, Some("zh-CN"), SttContext::default());
        let message = client.build_session_update_message();
        let session = message.get("session").unwrap();

        assert_eq!(
            session.get("modalities").unwrap(),
            &serde_json::json!(["text"])
        );
        assert_eq!(session.get("input_audio_format").unwrap(), "pcm");
        assert_eq!(session.get("sample_rate").unwrap(), 16000);
        assert!(session.get("output_audio_format").is_none());
        assert_eq!(
            session.get("turn_detection").unwrap(),
            &serde_json::Value::Null
        );
        assert_eq!(
            session
                .get("input_audio_transcription")
                .and_then(|value| value.get("language"))
                .unwrap(),
            "zh"
        );
        assert!(session
            .get("input_audio_transcription")
            .and_then(|value| value.get("model"))
            .is_none());
    }

    #[test]
    fn test_resolve_realtime_model_rewrites_legacy_default() {
        assert_eq!(
            resolve_realtime_model(LEGACY_QWEN_OMNI_REALTIME_MODEL),
            QWEN_OMNI_REALTIME_MODEL
        );
        assert_eq!(resolve_realtime_model(""), QWEN_OMNI_REALTIME_MODEL);
        assert_eq!(
            resolve_realtime_model("custom-qwen-model"),
            "custom-qwen-model"
        );
    }

    #[tokio::test]
    async fn test_connect_waits_for_session_updated_before_returning() {
        use futures_util::{SinkExt, StreamExt};
        use tokio::net::TcpListener;
        use tokio_tungstenite::accept_async;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let mock_url = format!("ws://127.0.0.1:{}/", port);

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws_stream = accept_async(stream).await.unwrap();

            ws_stream
                .send(Message::Text(
                    serde_json::json!({
                        "event_id": "event_created",
                        "type": "session.created",
                        "session": {
                            "id": "sess_001",
                            "object": "realtime.session",
                            "model": QWEN_OMNI_REALTIME_MODEL,
                            "modalities": ["text"],
                            "input_audio_format": "pcm",
                            "input_audio_transcription": null,
                            "turn_detection": {"type": "server_vad", "threshold": 0.2, "silence_duration_ms": 800}
                        }
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

            let message = ws_stream.next().await.unwrap().unwrap();
            let text = message.into_text().unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

            assert_eq!(
                parsed.get("type").and_then(|v| v.as_str()),
                Some("session.update")
            );
            assert_eq!(
                parsed
                    .pointer("/session/input_audio_format")
                    .and_then(|v| v.as_str()),
                Some("pcm")
            );
            assert_eq!(
                parsed
                    .pointer("/session/sample_rate")
                    .and_then(|v| v.as_i64()),
                Some(16000)
            );
            assert_eq!(
                parsed.pointer("/session/turn_detection"),
                Some(&serde_json::Value::Null)
            );
            assert_eq!(
                parsed
                    .pointer("/session/input_audio_transcription/language")
                    .and_then(|v| v.as_str()),
                Some("zh")
            );
            assert!(parsed
                .pointer("/session/input_audio_transcription/model")
                .is_none());

            ws_stream
                .send(Message::Text(
                    serde_json::json!({
                        "event_id": "event_updated",
                        "type": "session.updated",
                        "session": {
                            "id": "sess_001",
                            "object": "realtime.session",
                            "model": QWEN_OMNI_REALTIME_MODEL,
                            "modalities": ["text"],
                            "input_audio_format": "pcm",
                            "input_audio_transcription": {"language": "zh"},
                            "turn_detection": null
                        }
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

            let append = ws_stream.next().await.unwrap().unwrap();
            let append_text = append.into_text().unwrap();
            let append_json: serde_json::Value = serde_json::from_str(&append_text).unwrap();
            assert_eq!(
                append_json.get("type").and_then(|v| v.as_str()),
                Some("input_audio_buffer.append")
            );
            assert!(append_json.get("audio").and_then(|v| v.as_str()).is_some());
            assert!(append_json.get("data").is_none());

            ws_stream
                .send(Message::Text(
                    serde_json::json!({
                        "event_id": "event_partial",
                        "type": "conversation.item.input_audio_transcription.text",
                        "item_id": "item_001",
                        "content_index": 0,
                        "language": "zh",
                        "emotion": "neutral",
                        "text": "hello",
                        "stash": " world"
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

            let commit = ws_stream.next().await.unwrap().unwrap();
            let commit_text = commit.into_text().unwrap();
            let commit_json: serde_json::Value = serde_json::from_str(&commit_text).unwrap();
            assert_eq!(
                commit_json.get("type").and_then(|v| v.as_str()),
                Some("input_audio_buffer.commit")
            );

            let finish = ws_stream.next().await.unwrap().unwrap();
            let finish_text = finish.into_text().unwrap();
            let finish_json: serde_json::Value = serde_json::from_str(&finish_text).unwrap();
            assert_eq!(
                finish_json.get("type").and_then(|v| v.as_str()),
                Some("session.finish")
            );

            ws_stream
                .send(Message::Text(
                    serde_json::json!({
                        "event_id": "event_completed",
                        "type": "conversation.item.input_audio_transcription.completed",
                        "item_id": "item_001",
                        "content_index": 0,
                        "language": "zh",
                        "emotion": "neutral",
                        "transcript": "hello world"
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

            ws_stream
                .send(Message::Text(
                    serde_json::json!({
                        "event_id": "event_finished",
                        "type": "session.finished"
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
        });

        let config = CloudSttConfig {
            enabled: true,
            provider_type: "qwen-omni-realtime".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: mock_url,
            model: "".to_string(),
            language: "zh".to_string(),
        };

        let mut client = QwenOmniRealtimeClient::new(config, Some("zh-CN"), SttContext::default());
        client.connect().await.unwrap();

        let audio_tx = client.get_audio_sender().await.unwrap();
        audio_tx
            .send(vec![0; RECOMMENDED_CHUNK_SAMPLES])
            .await
            .unwrap();
        drop(audio_tx);

        let final_text = client.finish().await.unwrap();
        assert_eq!(final_text, "hello world");

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_connect_returns_close_reason_when_server_rejects_session_update() {
        use futures_util::{SinkExt, StreamExt};
        use tokio::net::TcpListener;
        use tokio_tungstenite::accept_async;
        use tokio_tungstenite::tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let mock_url = format!("ws://127.0.0.1:{}/", port);

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws_stream = accept_async(stream).await.unwrap();

            ws_stream
                .send(Message::Text(
                    serde_json::json!({
                        "event_id": "event_created",
                        "type": "session.created",
                        "session": {"id": "sess_001", "object": "realtime.session", "model": QWEN_OMNI_REALTIME_MODEL, "modalities": ["text"], "input_audio_format": "pcm", "input_audio_transcription": null, "turn_detection": null}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

            let _ = ws_stream.next().await.unwrap().unwrap();
            ws_stream
                .close(Some(CloseFrame {
                    code: CloseCode::Policy,
                    reason: "Access denied.".into(),
                }))
                .await
                .unwrap();
        });

        let config = CloudSttConfig {
            enabled: true,
            provider_type: "qwen-omni-realtime".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: mock_url,
            model: "".to_string(),
            language: "zh".to_string(),
        };

        let mut client = QwenOmniRealtimeClient::new(config, Some("zh"), SttContext::default());
        let err = client.connect().await.unwrap_err();
        assert!(err.contains("Access denied."), "unexpected error: {}", err);
        assert!(client.get_audio_sender().await.is_none());

        server.await.unwrap();
    }

    #[test]
    fn test_openai_realtime_client_new_auto_language() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "qwen-omni-realtime".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };
        let client = QwenOmniRealtimeClient::new(config, Some("auto"), SttContext::default());
        assert_eq!(client.language, "");
    }

    #[test]
    fn test_openai_realtime_client_new_none_language() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "qwen-omni-realtime".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };
        let client = QwenOmniRealtimeClient::new(config, None, SttContext::default());
        assert_eq!(client.language, "");
    }
}
