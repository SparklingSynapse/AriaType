pub mod aliyun_stream;
pub mod elevenlabs;
pub mod engine;
pub mod volcengine_streaming;

use crate::commands::settings::CloudSttConfig;
use crate::stt_engine::traits::{PartialResultCallback, RecordingConsumer, SttContext};
use aliyun_stream::AliyunStreamClient;
use async_trait::async_trait;
use elevenlabs::ElevenLabsStreamingClient;
use volcengine_streaming::VolcengineStreamingClient;

pub use crate::stt_engine::traits::{
    PartialResult as CloudPartialResult, PartialResultCallback as CloudPartialResultCallback,
};
pub use engine::CloudSttEngine;
pub use volcengine_streaming::{
    RECOMMENDED_CHUNK_SAMPLES, URL_BIGMODEL, URL_BIGMODEL_ASYNC, URL_BIGMODEL_NOSTREAM,
};

pub enum StreamingSttClient {
    Volcengine(VolcengineStreamingClient),
    Aliyun(AliyunStreamClient),
    ElevenLabs(ElevenLabsStreamingClient),
}

impl StreamingSttClient {
    pub fn new(
        config: CloudSttConfig,
        language: Option<&str>,
        context: SttContext,
    ) -> Result<Self, String> {
        match config.provider_type.as_str() {
            "volcengine-streaming" => Ok(Self::Volcengine(VolcengineStreamingClient::new(
                config, language, context,
            ))),
            "aliyun-stream" => Ok(Self::Aliyun(AliyunStreamClient::new(
                config, language, context,
            ))),
            "elevenlabs" => Ok(Self::ElevenLabs(ElevenLabsStreamingClient::new(
                config, language, context,
            ))),
            _ => Err(format!(
                "Unsupported streaming STT provider: {}",
                config.provider_type
            )),
        }
    }

    pub async fn connect(&mut self) -> Result<(), String> {
        match self {
            Self::Volcengine(c) => c.connect().await,
            Self::Aliyun(c) => c.connect().await,
            Self::ElevenLabs(c) => c.connect().await,
        }
    }

    pub async fn get_audio_sender(&self) -> Option<tokio::sync::mpsc::Sender<Vec<i16>>> {
        match self {
            Self::Volcengine(c) => c.get_audio_sender().await,
            Self::Aliyun(c) => c.get_audio_sender().await,
            Self::ElevenLabs(c) => c.get_audio_sender().await,
        }
    }

    pub async fn finish(&self) -> Result<String, String> {
        match self {
            Self::Volcengine(c) => c.finish().await,
            Self::Aliyun(c) => c.finish().await,
            Self::ElevenLabs(c) => c.finish().await,
        }
    }

    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Volcengine(_) => "Volcengine",
            Self::Aliyun(_) => "Aliyun",
            Self::ElevenLabs(_) => "Eleven Labs",
        }
    }

    pub fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        match self {
            Self::Volcengine(c) => c.set_partial_callback(callback),
            Self::Aliyun(c) => c.set_partial_callback(callback),
            Self::ElevenLabs(c) => c.set_partial_callback(callback),
        }
    }
}

/// Recording consumer backed by a cloud streaming STT provider.
///
/// The cloud client is connected and fully configured (partial callback,
/// audio sender obtained) before this struct is created. `send_chunk`
/// forwards to the WebSocket audio channel; `finish` drops the sender
/// and awaits the final result.
use tokio::sync::Mutex;

pub struct StreamingConsumer {
    client: StreamingSttClient,
    audio_tx: Mutex<Option<tokio::sync::mpsc::Sender<Vec<i16>>>>,
}

impl StreamingConsumer {
    pub async fn new(
        mut client: StreamingSttClient,
        partial_callback: PartialResultCallback,
    ) -> Result<Self, String> {
        client.set_partial_callback(partial_callback);
        client.connect().await?;

        let audio_tx = client
            .get_audio_sender()
            .await
            .ok_or("audio_sender_unavailable-streaming_client")?;

        Ok(Self {
            client,
            audio_tx: Mutex::new(Some(audio_tx)),
        })
    }

    pub fn provider_name(&self) -> &'static str {
        self.client.provider_name()
    }
}

#[async_trait]
impl RecordingConsumer for StreamingConsumer {
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        let tx = self
            .audio_tx
            .lock()
            .await
            .clone()
            .ok_or("audio_sender_already_dropped-streaming")?;
        tx.send(pcm_data)
            .await
            .map_err(|e| format!("audio_chunk_send_failed-streaming: {}", e))
    }

    async fn finish(&self) -> Result<String, String> {
        drop(self.audio_tx.lock().await.take());
        self.client.finish().await
    }

    fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        self.client.set_partial_callback(callback);
    }
}
