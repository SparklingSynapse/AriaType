use std::num::NonZeroU32;
use std::path::Path;
use tracing::{error, info, warn};

/// Configuration for engine-specific behavior
pub struct EngineConfig {
    pub log_prefix: &'static str,
    pub strip_think_tags: bool,
}

/// Shared language detection logic
pub fn detect_language(text: &str) -> &'static str {
    let total = text.chars().count();
    if total == 0 {
        return "English";
    }
    let cjk = text
        .chars()
        .filter(|c| {
            matches!(c,
                '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
                '\u{3040}'..='\u{30FF}' |  // Hiragana + Katakana
                '\u{AC00}'..='\u{D7AF}'    // Hangul
            )
        })
        .count();
    if cjk * 3 > total {
        let kana = text
            .chars()
            .filter(|c| matches!(c, '\u{3040}'..='\u{30FF}'))
            .count();
        let hangul = text
            .chars()
            .filter(|c| matches!(c, '\u{AC00}'..='\u{D7AF}'))
            .count();
        if kana > hangul && kana > 0 {
            "Japanese"
        } else if hangul > 0 {
            "Korean"
        } else {
            "Chinese"
        }
    } else {
        "English"
    }
}

/// Convert language code to full name
pub fn language_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "pt" => "Portuguese",
        "ru" => "Russian",
        "ar" => "Arabic",
        _ => "",
    }
}

/// Run text polishing using GGUF model via llama-cpp-2.
/// This is a blocking call — run it inside `spawn_blocking`.
pub fn polish_text_blocking(
    text: &str,
    system_prompt: &str,
    language: &str,
    model_path: &Path,
    default_prompt: &str,
    config: &EngineConfig,
) -> Result<String, String> {
    use llama_cpp_2::{
        context::params::LlamaContextParams,
        llama_backend::LlamaBackend,
        llama_batch::LlamaBatch,
        model::{params::LlamaModelParams, AddBos, LlamaModel},
        token::data_array::LlamaTokenDataArray,
    };

    let prefix = config.log_prefix;

    info!("[{}] ====== POLISH CONFIG ======", prefix);
    info!("[{}] language: {}", prefix, language);
    info!("[{}] system_prompt: {}", prefix, system_prompt);
    info!("[{}] =============================", prefix);

    info!("[{}] ========== INPUT START ==========", prefix);
    info!("[{}] {}", prefix, text);
    info!("[{}] ========== INPUT END ({} chars) ==========", prefix, text.len());

    if !model_path.exists() {
        error!("[{}] model not found at {:?}", prefix, model_path);
        return Err("Polish model not downloaded".to_string());
    }

    let t0 = std::time::Instant::now();

    info!("[{}] initializing llama backend", prefix);
    let backend = LlamaBackend::init().map_err(|e| {
        error!("[{}] backend init failed: {e}", prefix);
        format!("Backend init: {e}")
    })?;

    info!("[{}] loading model from {:?}", prefix, model_path);
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, &model_path, &model_params).map_err(|e| {
        error!("[{}] model load failed: {e}", prefix);
        format!("Model load: {e}")
    })?;
    info!("[{}] model loaded in {:.2}s", prefix, t0.elapsed().as_secs_f32());

    let ctx_params = LlamaContextParams::default().with_n_ctx(Some(NonZeroU32::new(2048).unwrap()));
    let mut ctx = model.new_context(&backend, ctx_params).map_err(|e| {
        error!("[{}] context creation failed: {e}", prefix);
        format!("Context: {e}")
    })?;

    let lang_hint = {
        let name = language_name(language);
        if name.is_empty() {
            detect_language(text)
        } else {
            name
        }
    };

    let extra_instruction = if system_prompt == default_prompt {
        format!("\nCRITICAL: Your output MUST be in {lang_hint}. ONLY fix grammar mistakes and punctuation errors. Do NOT add or remove words. Preserve the original text exactly as it is, including any mix of multiple languages.")
    } else {
        String::new()
    };

    // Both Qwen and LFM use ChatML format
    let prompt = format!(
        "<|im_start|>system\n{system_prompt}{extra_instruction}<|im_end|>\n<|im_start|>user\n{text}<|im_end|>\n<|im_start|>assistant\n"
    );
    info!("[{}] full input prompt: {}", prefix, prompt);

    let tokens = model.str_to_token(&prompt, AddBos::Always).map_err(|e| {
        error!("[{}] tokenization failed: {e}", prefix);
        format!("Tokenize: {e}")
    })?;
    info!("[{}] prompt tokenized: {} tokens", prefix, tokens.len());

    let mut batch = LlamaBatch::new(tokens.len(), 1);
    let last_idx = (tokens.len() - 1) as i32;
    for (i, &tok) in tokens.iter().enumerate() {
        batch
            .add(tok, i as i32, &[0], i as i32 == last_idx)
            .map_err(|e| format!("Batch add: {e}"))?;
    }

    let t_decode = std::time::Instant::now();
    ctx.decode(&mut batch).map_err(|e| {
        error!("[{}] prefill decode failed: {e}", prefix);
        format!("Decode: {e}")
    })?;
    info!("[{}] prefill done in {:.2}s", prefix, t_decode.elapsed().as_secs_f32());

    let n_prompt = tokens.len() as i32;
    let max_new_tokens = 512_i32;
    let mut n_cur = n_prompt;
    let mut output_bytes = Vec::new();
    let t_gen = std::time::Instant::now();

    loop {
        let mut candidates_p =
            LlamaTokenDataArray::from_iter(ctx.candidates_ith(batch.n_tokens() - 1), false);
        let new_token = candidates_p.sample_token_greedy();

        if new_token == model.token_eos() {
            info!("[{}] EOS reached after {} new tokens", prefix, n_cur - n_prompt);
            break;
        }
        if (n_cur - n_prompt) >= max_new_tokens {
            warn!("[{}] max_new_tokens ({}) reached, truncating", prefix, max_new_tokens);
            break;
        }

        let bytes = model
            .token_to_piece_bytes(new_token, 32, false, None)
            .map_err(|e| format!("Token to bytes: {e}"))?;
        output_bytes.extend_from_slice(&bytes);

        batch.clear();
        batch
            .add(new_token, n_cur, &[0], true)
            .map_err(|e| format!("Batch add: {e}"))?;
        ctx.decode(&mut batch).map_err(|e| {
            error!("[{}] generation decode failed at token {}: {e}", prefix, n_cur);
            format!("Decode: {e}")
        })?;
        n_cur += 1;
    }

    let output = String::from_utf8_lossy(&output_bytes).to_string();

    let n_new = n_cur - n_prompt;
    let gen_secs = t_gen.elapsed().as_secs_f32();
    let total_secs = t0.elapsed().as_secs_f32();
    info!(
        "[{}] done — {} new tokens in {:.2}s ({:.1} tok/s), total {:.2}s",
        prefix,
        n_new,
        gen_secs,
        n_new as f32 / gen_secs.max(0.001),
        total_secs
    );

    let result = output.trim().to_string();
    info!("[{}] raw full output: {}", prefix, result);

    // Strip <think>...</think> block if configured (Qwen3 chain-of-thought)
    let final_result = if config.strip_think_tags {
        if let Some(end_idx) = result.find("</think>") {
            // Complete think block found, extract content after it
            result[end_idx + "</think>".len()..].trim().to_string()
        } else if result.contains("<think>") {
            // Incomplete think block (truncated) - return empty or error
            warn!("[{}] incomplete <think> block detected (likely truncated), returning empty result", prefix);
            String::new()
        } else {
            // No think tags at all, return as-is
            result
        }
    } else {
        result
    };

    info!("[{}] ========== OUTPUT START ==========", prefix);
    info!("[{}] {}", prefix, final_result);
    info!("[{}] ========== OUTPUT END ({} chars) ==========", prefix, final_result.len());
    Ok(final_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_name_known_codes() {
        assert_eq!(language_name("en"), "English");
        assert_eq!(language_name("zh"), "Chinese");
        assert_eq!(language_name("ja"), "Japanese");
        assert_eq!(language_name("ko"), "Korean");
        assert_eq!(language_name("fr"), "French");
        assert_eq!(language_name("de"), "German");
        assert_eq!(language_name("es"), "Spanish");
        assert_eq!(language_name("pt"), "Portuguese");
        assert_eq!(language_name("ru"), "Russian");
        assert_eq!(language_name("ar"), "Arabic");
    }

    #[test]
    fn test_language_name_unknown_code() {
        assert_eq!(language_name("unknown"), "");
        assert_eq!(language_name("xyz"), "");
        assert_eq!(language_name(""), "");
    }

    #[test]
    fn test_detect_language_empty() {
        assert_eq!(detect_language(""), "English");
    }

    #[test]
    fn test_detect_language_english() {
        assert_eq!(detect_language("Hello world"), "English");
        assert_eq!(detect_language("The quick brown fox"), "English");
        assert_eq!(detect_language("Testing 123"), "English");
    }

    #[test]
    fn test_detect_language_chinese() {
        assert_eq!(detect_language("你好世界"), "Chinese");
        assert_eq!(detect_language("这是一个测试"), "Chinese");
        assert_eq!(detect_language("中文测试内容"), "Chinese");
    }

    #[test]
    fn test_detect_language_japanese() {
        assert_eq!(detect_language("こんにちは世界"), "Japanese");
        assert_eq!(detect_language("テストです"), "Japanese");
        assert_eq!(detect_language("ひらがなカタカナ"), "Japanese");
    }

    #[test]
    fn test_detect_language_korean() {
        assert_eq!(detect_language("안녕하세요"), "Korean");
        assert_eq!(detect_language("한글 테스트"), "Korean");
    }

    #[test]
    fn test_detect_language_mixed_mostly_english() {
        // Less than 1/3 CJK characters should be detected as English
        assert_eq!(detect_language("Hello 你好"), "English");
        assert_eq!(detect_language("Test 测试 more english words"), "English");
    }

    #[test]
    fn test_detect_language_mixed_mostly_cjk() {
        // More than 1/3 CJK characters should be detected as CJK
        assert_eq!(detect_language("你好世界 hello"), "Chinese");
    }

    #[test]
    fn test_engine_config_creation() {
        let config = EngineConfig {
            log_prefix: "test",
            strip_think_tags: true,
        };
        assert_eq!(config.log_prefix, "test");
        assert!(config.strip_think_tags);

        let config2 = EngineConfig {
            log_prefix: "another",
            strip_think_tags: false,
        };
        assert_eq!(config2.log_prefix, "another");
        assert!(!config2.strip_think_tags);
    }
}
