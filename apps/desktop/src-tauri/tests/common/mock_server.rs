use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct VolcengineFlashMockServer {
    pub server: MockServer,
}

impl VolcengineFlashMockServer {
    pub async fn start() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }

    pub fn flash_url(&self) -> String {
        format!("{}/api/v3/auc/bigmodel/recognize/flash", self.server.uri())
    }

    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    pub async fn mock_success(&self, text: &str) {
        let response_body = serde_json::json!({
            "audio_info": {
                "duration": 2500
            },
            "result": {
                "text": text,
                "utterances": []
            }
        });

        Mock::given(method("POST"))
            .and(path("/api/v3/auc/bigmodel/recognize/flash"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_success_with_utterances(&self, text: &str, utterance_count: usize) {
        let utterances: Vec<_> = (0..utterance_count)
            .map(|i| {
                serde_json::json!({
                    "text": format!("句子{}", i + 1),
                    "start_time": i * 1000,
                    "end_time": (i + 1) * 1000,
                    "words": []
                })
            })
            .collect();

        let response_body = serde_json::json!({
            "audio_info": {
                "duration": utterance_count as u64 * 1000
            },
            "result": {
                "text": text,
                "utterances": utterances
            }
        });

        Mock::given(method("POST"))
            .and(path("/api/v3/auc/bigmodel/recognize/flash"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_auth_error(&self) {
        Mock::given(method("POST"))
            .and(path("/api/v3/auc/bigmodel/recognize/flash"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_server_error(&self) {
        Mock::given(method("POST"))
            .and(path("/api/v3/auc/bigmodel/recognize/flash"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_empty(&self) {
        let response_body = serde_json::json!({
            "audio_info": {
                "duration": 0
            },
            "result": {
                "text": "",
                "utterances": []
            }
        });

        Mock::given(method("POST"))
            .and(path("/api/v3/auc/bigmodel/recognize/flash"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_rate_limit(&self) {
        Mock::given(method("POST"))
            .and(path("/api/v3/auc/bigmodel/recognize/flash"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&self.server)
            .await;
    }

    pub async fn reset(&self) {
        self.server.reset().await;
    }
}
