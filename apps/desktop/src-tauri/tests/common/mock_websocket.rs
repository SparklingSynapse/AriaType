//! Mock WebSocket for testing cloud STT WebSocket connections

use futures_util::{Sink, Stream};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone)]
pub enum MockWebSocketError {
    Closed,
    Custom(String),
}

impl std::fmt::Display for MockWebSocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "WebSocket closed"),
            Self::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for MockWebSocketError {}

#[derive(Debug, Clone)]
pub struct MockWebSocket {
    responses: Arc<Mutex<VecDeque<Message>>>,
    errors: Arc<Mutex<VecDeque<MockWebSocketError>>>,
    response_delay_ms: u64,
    close_after_count: Option<usize>,
    messages_sent: Arc<Mutex<Vec<Message>>>,
    is_closed: Arc<Mutex<bool>>,
}

impl MockWebSocket {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::new())),
            errors: Arc::new(Mutex::new(VecDeque::new())),
            response_delay_ms: 0,
            close_after_count: None,
            messages_sent: Arc::new(Mutex::new(Vec::new())),
            is_closed: Arc::new(Mutex::new(false)),
        }
    }

    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .push_back(Message::Text(response.into().into()));
        self
    }

    pub fn with_responses(mut self, responses: Vec<String>) -> Self {
        let mut queue = self.responses.lock().unwrap();
        for r in responses {
            queue.push_back(Message::Text(r.into()));
        }
        drop(queue);
        self
    }

    pub fn with_json_response<T: serde::Serialize>(mut self, data: &T) -> Self {
        let json = serde_json::to_string(data).unwrap();
        self.responses
            .lock()
            .unwrap()
            .push_back(Message::Text(json.into()));
        self
    }

    pub fn with_error(mut self, error: MockWebSocketError) -> Self {
        self.errors.lock().unwrap().push_back(error);
        self
    }

    pub fn with_response_delay(mut self, delay_ms: u64) -> Self {
        self.response_delay_ms = delay_ms;
        self
    }

    pub fn with_auto_close(mut self, count: usize) -> Self {
        self.close_after_count = Some(count);
        self
    }

    pub fn get_sent_messages(&self) -> Vec<Message> {
        self.messages_sent.lock().unwrap().clone()
    }

    pub fn is_closed(&self) -> bool {
        *self.is_closed.lock().unwrap()
    }

    pub fn sent_count(&self) -> usize {
        self.messages_sent.lock().unwrap().len()
    }

    pub fn clear_sent(&self) {
        self.messages_sent.lock().unwrap().clear();
    }
}

impl Default for MockWebSocket {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MockWebSocketBuilder {
    responses: VecDeque<Message>,
    errors: VecDeque<MockWebSocketError>,
    response_delay_ms: u64,
    close_after_count: Option<usize>,
}

impl MockWebSocketBuilder {
    pub fn new() -> Self {
        Self {
            responses: VecDeque::new(),
            errors: VecDeque::new(),
            response_delay_ms: 0,
            close_after_count: None,
        }
    }

    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.responses
            .push_back(Message::Text(response.into().into()));
        self
    }

    pub fn with_responses(mut self, responses: Vec<String>) -> Self {
        for r in responses {
            self.responses.push_back(Message::Text(r.into()));
        }
        self
    }

    pub fn with_json_response<T: serde::Serialize>(mut self, data: &T) -> Self {
        let json = serde_json::to_string(data).unwrap();
        self.responses.push_back(Message::Text(json.into()));
        self
    }

    pub fn with_error(mut self, error: MockWebSocketError) -> Self {
        self.errors.push_back(error);
        self
    }

    pub fn with_response_delay(mut self, delay_ms: u64) -> Self {
        self.response_delay_ms = delay_ms;
        self
    }

    pub fn with_auto_close(mut self, count: usize) -> Self {
        self.close_after_count = Some(count);
        self
    }

    pub fn build(self) -> MockWebSocket {
        MockWebSocket {
            responses: Arc::new(Mutex::new(self.responses)),
            errors: Arc::new(Mutex::new(self.errors)),
            response_delay_ms: self.response_delay_ms,
            close_after_count: self.close_after_count,
            messages_sent: Arc::new(Mutex::new(Vec::new())),
            is_closed: Arc::new(Mutex::new(false)),
        }
    }
}

impl Default for MockWebSocketBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Sink<Message> for MockWebSocket {
    type Error = MockWebSocketError;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, msg: Message) -> Result<(), Self::Error> {
        if *self.is_closed.lock().unwrap() {
            return Err(MockWebSocketError::Closed);
        }
        self.messages_sent.lock().unwrap().push(msg);

        if let Some(count) = self.close_after_count {
            if self.messages_sent.lock().unwrap().len() >= count {
                *self.is_closed.lock().unwrap() = true;
            }
        }
        Ok(())
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        if *self.is_closed.lock().unwrap() {
            return Poll::Ready(Err(MockWebSocketError::Closed));
        }
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        *self.is_closed.lock().unwrap() = true;
        Poll::Ready(Ok(()))
    }
}

impl Stream for MockWebSocket {
    type Item = Result<Message, MockWebSocketError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if *self.is_closed.lock().unwrap() {
            return Poll::Ready(None);
        }

        if let Some(error) = self.errors.lock().unwrap().pop_front() {
            return Poll::Ready(Some(Err(error)));
        }

        if let Some(response) = self.responses.lock().unwrap().pop_front() {
            return Poll::Ready(Some(Ok(response)));
        }

        Poll::Ready(None)
    }
}

pub mod volcengine {
    use super::*;
    use serde_json::json;

    pub fn make_partial_response(text: &str, is_final: bool) -> String {
        json!({
            "code": 1000,
            "message": "success",
            "data": {
                "result": {
                    "text": text,
                    "is_final": is_final
                }
            }
        })
        .to_string()
    }

    pub fn make_error_response(code: i32, message: &str) -> String {
        json!({
            "code": code,
            "message": message
        })
        .to_string()
    }

    pub fn mock_for_volcengine(text: &str, is_final: bool) -> MockWebSocket {
        MockWebSocket::new().with_response(make_partial_response(text, is_final))
    }

    pub fn mock_with_partials(partials: Vec<(&str, bool)>) -> MockWebSocket {
        let mut mock = MockWebSocket::new();
        for (text, is_final) in partials {
            mock = mock.with_response(make_partial_response(text, is_final));
        }
        mock
    }
}

pub mod openai {
    use super::*;
    use serde_json::json;

    pub fn make_transcript_response(text: &str) -> String {
        json!({
            "type": "transcript",
            "transcript": text
        })
        .to_string()
    }

    pub fn make_session_update_response() -> String {
        json!({
            "type": "session.update",
            "session": {
                "input_audio_transcription": {
                    "model": "whisper-1"
                }
            }
        })
        .to_string()
    }

    pub fn mock_for_openai(text: &str) -> MockWebSocket {
        MockWebSocket::new()
            .with_response(make_session_update_response())
            .with_response(make_transcript_response(text))
    }
}

pub mod deepgram {
    use super::*;
    use serde_json::json;

    pub fn make_response(text: &str, is_final: bool) -> String {
        json!({
            "type": "Results",
            "channel_index": [0],
            "is_final": is_final,
            "channel": {
                "alternatives": [{
                    "transcript": text,
                    "confidence": 0.99
                }]
            }
        })
        .to_string()
    }

    pub fn mock_for_deepgram(text: &str, is_final: bool) -> MockWebSocket {
        MockWebSocket::new().with_response(make_response(text, is_final))
    }
}

#[derive(Debug, Clone)]
pub struct MockWebSocketServer {
    responses: Arc<Mutex<VecDeque<String>>>,
    received_messages: Arc<Mutex<Vec<String>>>,
    is_connected: Arc<Mutex<bool>>,
}

impl MockWebSocketServer {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::new())),
            received_messages: Arc::new(Mutex::new(Vec::new())),
            is_connected: Arc::new(Mutex::new(false)),
        }
    }

    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.responses.lock().unwrap().push_back(response.into());
        self
    }

    pub fn get_received(&self) -> Vec<String> {
        self.received_messages.lock().unwrap().clone()
    }

    pub fn is_connected(&self) -> bool {
        *self.is_connected.lock().unwrap()
    }

    pub fn receive(&self, msg: String) {
        self.received_messages.lock().unwrap().push(msg);
    }

    pub fn next_response(&self) -> Option<String> {
        self.responses.lock().unwrap().pop_front()
    }

    pub fn connect(&self) {
        *self.is_connected.lock().unwrap() = true;
    }

    pub fn disconnect(&self) {
        *self.is_connected.lock().unwrap() = false;
    }
}

impl Default for MockWebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}
