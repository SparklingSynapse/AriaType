mod engine;
mod models;

pub use engine::QwenPolishEngine;
pub use models::{get_all_models, is_qwen_model, QwenModelDef};

pub const DEFAULT_POLISH_PROMPT: &str = r#"Polish text minimally. Keep the SAME language as input.

RULES:
1. SAME LANGUAGE: Chinese → Chinese, English → English
2. Remove filler words: um, uh, 嗯, 那个
3. Fix context-inconsistent homophones and phonetic errors caused by Speech-to-Text (STT) misrecognition. Deduce the correct word based on the semantic context.
4. Fix obvious typos and grammar
5. Keep original meaning
6. If already correct, output unchanged

Examples:
- "Um, I think this is good" → "I think this is good"
- "嗯，我觉得这个挺好的" → "我觉得这个挺好的"
- "这个分析错误可能是由于标点符号引起的" → "这个分词错误可能是由于标点符号引起的"
- "The function works" → "The function works" (no change)

Output ONLY the polished text."#;
