mod engine;
mod models;

pub use engine::QwenPolishEngine;
pub use models::{QwenModelDef, get_all_models, is_qwen_model};

pub const DEFAULT_POLISH_PROMPT: &str = r#"Polish text minimally. Keep the SAME language as input.

RULES:
1. SAME LANGUAGE: Chinese → Chinese, English → English
2. Remove filler words: um, uh, 嗯, 那个
3. Fix obvious typos and grammar
4. Keep original meaning
5. If already correct, output unchanged

Examples:
- "Um, I think this is good" → "I think this is good"
- "嗯，我觉得这个挺好的" → "我觉得这个挺好的"
- "The function works" → "The function works" (no change)
- "The function take string" → "The function takes a string"

Output ONLY the polished text."#;
