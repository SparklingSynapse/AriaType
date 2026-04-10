use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderFieldSchema {
    pub name: &'static str,
    pub key: &'static str,
    pub required: bool,
    pub default_value: &'static str,
    pub example: &'static str,
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSchema {
    pub id: &'static str,
    pub name: &'static str,
    pub fields: &'static [ProviderFieldSchema],
}

#[derive(Debug, Clone, Serialize)]
pub struct CloudProviderSchemas {
    pub stt: &'static [ProviderSchema],
    pub polish: &'static [ProviderSchema],
}

pub static STT_SCHEMAS: &[ProviderSchema] = &[
    ProviderSchema {
        id: "volcengine-streaming",
        name: "Volcengine Streaming",
        fields: &[
            ProviderFieldSchema {
                name: "App ID",
                key: "app_id",
                required: true,
                default_value: "",
                example: "1234567890",
                secret: false,
            },
            ProviderFieldSchema {
                name: "Access Token",
                key: "api_key",
                required: true,
                default_value: "",
                example: "xxxx.xxxx.xxxx",
                secret: true,
            },
            ProviderFieldSchema {
                name: "Base URL",
                key: "base_url",
                required: false,
                default_value: "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream",
                example: "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream",
                secret: false,
            },
        ],
    },
    ProviderSchema {
        id: "aliyun-stream",
        name: "Aliyun Realtime",
        fields: &[
            ProviderFieldSchema {
                name: "API Key",
                key: "api_key",
                required: true,
                default_value: "",
                example: "sk-xxxx.xxxx.xxxx",
                secret: true,
            },
            ProviderFieldSchema {
                name: "Base URL",
                key: "base_url",
                required: false,
                default_value: "wss://dashscope.aliyuncs.com/api-ws/v1/realtime",
                example: "wss://dashscope.aliyuncs.com/api-ws/v1/realtime",
                secret: false,
            },
            ProviderFieldSchema {
                name: "Model",
                key: "model",
                required: true,
                default_value: "qwen3-asr-flash-realtime",
                example: "qwen3-asr-flash-realtime",
                secret: false,
            },
        ],
    },
    ProviderSchema {
        id: "elevenlabs",
        name: "ElevenLabs",
        fields: &[
            ProviderFieldSchema {
                name: "API Key",
                key: "api_key",
                required: true,
                default_value: "",
                example: "sk_xxxx.xxxx.xxxx",
                secret: true,
            },
            ProviderFieldSchema {
                name: "Base URL",
                key: "base_url",
                required: false,
                default_value: "wss://api.elevenlabs.io/v1/speech-to-text/realtime",
                example: "wss://api.elevenlabs.io/v1/speech-to-text/realtime",
                secret: false,
            },
        ],
    },
];

pub static POLISH_SCHEMAS: &[ProviderSchema] = &[
    ProviderSchema {
        id: "anthropic",
        name: "Anthropic",
        fields: &[
            ProviderFieldSchema {
                name: "API Key",
                key: "api_key",
                required: true,
                default_value: "",
                example: "sk-ant-xxxx.xxxx.xxxx",
                secret: true,
            },
            ProviderFieldSchema {
                name: "Base URL",
                key: "base_url",
                required: false,
                default_value: "https://api.anthropic.com/v1/messages",
                example: "https://api.anthropic.com/v1/messages",
                secret: false,
            },
            ProviderFieldSchema {
                name: "Model",
                key: "model",
                required: true,
                default_value: "",
                example: "claude-sonnet-4-20250514",
                secret: false,
            },
        ],
    },
    ProviderSchema {
        id: "openai",
        name: "OpenAI",
        fields: &[
            ProviderFieldSchema {
                name: "API Key",
                key: "api_key",
                required: true,
                default_value: "",
                example: "sk-xxxx.xxxx.xxxx",
                secret: true,
            },
            ProviderFieldSchema {
                name: "Base URL",
                key: "base_url",
                required: false,
                default_value: "https://api.openai.com/v1/chat/completions",
                example: "https://api.openai.com/v1/chat/completions",
                secret: false,
            },
            ProviderFieldSchema {
                name: "Model",
                key: "model",
                required: true,
                default_value: "",
                example: "gpt-4.1",
                secret: false,
            },
        ],
    },
];

pub fn get_schemas() -> CloudProviderSchemas {
    CloudProviderSchemas {
        stt: STT_SCHEMAS,
        polish: POLISH_SCHEMAS,
    }
}
