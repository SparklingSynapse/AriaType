use std::num::NonZeroU32;
use std::path::Path;
use tracing::{debug, error, info, instrument, warn};

fn build_polish_context_params() -> llama_cpp_2::context::params::LlamaContextParams {
    use llama_cpp_2::context::params::LlamaContextParams;

    // Flash attention auto-selection crashes on some Metal-backed local polish runs
    // before generation begins, so force it off for the shared local polish runtime.
    LlamaContextParams::default()
        .with_n_ctx(Some(NonZeroU32::new(2048).unwrap()))
        .with_flash_attention_policy(llama_cpp_sys_2::LLAMA_FLASH_ATTN_TYPE_DISABLED)
}

/// Prompt template format used by different model families
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptFormat {
    /// ChatML format: `<|im_start|>role\n...<|im_end|>` (Qwen, LFM)
    ChatMl,
    /// Gemma format: `<start_of_turn>role\n...<end_of_turn>` (Gemma)
    Gemma,
}

/// Configuration for engine-specific behavior
pub struct EngineConfig {
    pub log_prefix: &'static str,
    pub strip_think_tags: bool,
    pub prompt_format: PromptFormat,
    /// Minimum acceptable model file size in MB (catches incomplete downloads)
    pub min_model_size_mb: u64,
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
#[instrument(
    skip(text, model_path, config),
    fields(
        language = %language,
        engine = config.log_prefix,
    ),
    ret,
    err
)]
pub fn polish_text_blocking(
    text: &str,
    system_prompt: &str,
    language: &str,
    model_path: &Path,
    default_prompt: &str,
    config: &EngineConfig,
) -> Result<String, String> {
    use llama_cpp_2::{
        llama_backend::LlamaBackend,
        llama_batch::LlamaBatch,
        model::{params::LlamaModelParams, AddBos, LlamaModel},
        token::data_array::LlamaTokenDataArray,
    };

    let engine = config.log_prefix;

    info!(engine = %engine, language = %language, "polish_config");
    debug!(engine = %engine, system_prompt = %system_prompt, "polish_system_prompt_configured");

    info!(engine = %engine, text_len = text.len(), "polish_input_start");
    debug!(engine = %engine, input = %text, "polish_input_text");

    if !model_path.exists() {
        error!(engine = %engine, path = ?model_path, "polish_model_not_found");
        return Err("Polish model not downloaded".to_string());
    }

    // Check model file size to detect incomplete downloads
    let model_metadata = std::fs::metadata(model_path).map_err(|e| {
        error!(engine = %engine, path = ?model_path, error = %e, "polish_model_metadata_failed");
        format!("Failed to read model metadata: {e}")
    })?;
    let model_size_mb = model_metadata.len() / (1024 * 1024);
    info!(engine = %engine, path = ?model_path, size_mb = model_size_mb, "polish_model_file_checked");

    if model_size_mb < config.min_model_size_mb {
        error!(engine = %engine, size_mb = model_size_mb, min_mb = config.min_model_size_mb, "polish_model_file_too_small");
        return Err(format!(
            "Model file appears incomplete: {}MB (expected at least {}MB)",
            model_size_mb, config.min_model_size_mb
        ));
    }

    let t0 = std::time::Instant::now();

    info!(engine = %engine, "polish_backend_init_start");
    let backend = LlamaBackend::init().map_err(|e| {
        error!(engine = %engine, error = %e, "polish_backend_init_failed");
        format!("Backend init: {e}")
    })?;

    info!(engine = %engine, path = ?model_path, "polish_model_load_start");
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, model_path, &model_params).map_err(|e| {
        error!(engine = %engine, error = %e, "polish_model_load_failed");
        format!("Model load: {e}")
    })?;
    let model_load_ms = t0.elapsed().as_millis() as u64;
    info!(engine = %engine, duration_ms = model_load_ms, "polish_model_loaded");

    let ctx_params = build_polish_context_params();
    let mut ctx = model.new_context(&backend, ctx_params).map_err(|e| {
        error!(engine = %engine, error = %e, "polish_context_creation_failed");
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

    let prompt = match config.prompt_format {
        PromptFormat::ChatMl => format!(
            "<|im_start|>system\n{system_prompt}{extra_instruction}<|im_end|>\n<|im_start|>user\n{text}<|im_end|>\n<|im_start|>assistant\n"
        ),
        PromptFormat::Gemma => format!(
            "<start_of_turn>user\n{system_prompt}{extra_instruction}\n\n{text}<end_of_turn>\n<start_of_turn>model\n"
        ),
    };
    debug!(engine = %engine, prompt_len = prompt.len(), "polish_full_prompt_constructed");

    let tokens = model.str_to_token(&prompt, AddBos::Always).map_err(|e| {
        error!(engine = %engine, error = %e, "polish_tokenization_failed");
        format!("Tokenize: {e}")
    })?;
    info!(engine = %engine, token_count = tokens.len(), "polish_tokenization_complete");

    let mut batch = LlamaBatch::new(tokens.len(), 1);
    let last_idx = (tokens.len() - 1) as i32;
    for (i, &tok) in tokens.iter().enumerate() {
        batch
            .add(tok, i as i32, &[0], i as i32 == last_idx)
            .map_err(|e| format!("Batch add: {e}"))?;
    }

    let t_decode = std::time::Instant::now();
    ctx.decode(&mut batch).map_err(|e| {
        error!(engine = %engine, error = %e, "polish_prefill_decode_failed");
        format!("Decode: {e}")
    })?;
    let prefill_ms = t_decode.elapsed().as_millis() as u64;
    info!(engine = %engine, duration_ms = prefill_ms, "polish_prefill_complete");

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
            info!(
                engine = %engine,
                new_tokens = n_cur - n_prompt,
                "polish_generation_eos"
            );
            break;
        }
        if (n_cur - n_prompt) >= max_new_tokens {
            warn!(
                engine = %engine,
                max_new_tokens = max_new_tokens,
                "polish_max_tokens_reached"
            );
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
            error!(
                engine = %engine,
                token_index = n_cur,
                error = %e,
                "polish_generation_decode_failed"
            );
            format!("Decode: {e}")
        })?;
        n_cur += 1;
    }

    let output = String::from_utf8_lossy(&output_bytes).to_string();

    let n_new = n_cur - n_prompt;
    let gen_ms = t_gen.elapsed().as_millis() as u64;
    let total_ms = t0.elapsed().as_millis() as u64;
    let tok_per_sec = if gen_ms > 0 {
        (n_new as f64 * 1000.0 / gen_ms as f64) as f64
    } else {
        0.0
    };
    info!(
        engine = %engine,
        new_tokens = n_new,
        generation_ms = gen_ms,
        tok_per_sec = tok_per_sec,
        total_ms = total_ms,
        "polish_generation_complete"
    );

    let result = output.trim().to_string();
    debug!(engine = %engine, output_len = result.len(), "polish_raw_output");

    // Strip ... block if configured (Qwen3 chain-of-thought)
    let final_result = if config.strip_think_tags {
        if let Some(end_idx) = result.find("") {
            // Complete think block found, extract content after it
            result[end_idx + "".len()..].trim().to_string()
        } else if result.contains("") {
            // Incomplete think block (truncated) - return empty or error
            warn!(engine = %engine, "polish_incomplete_think_block");
            String::new()
        } else {
            // No think tags at all, return as-is
            result
        }
    } else {
        result
    };

    info!(engine = %engine, output_len = final_result.len(), "polish_output_end");
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
            prompt_format: PromptFormat::ChatMl,
            min_model_size_mb: 100,
        };
        assert_eq!(config.log_prefix, "test");
        assert!(config.strip_think_tags);
        assert_eq!(config.prompt_format, PromptFormat::ChatMl);
        assert_eq!(config.min_model_size_mb, 100);

        let config2 = EngineConfig {
            log_prefix: "another",
            strip_think_tags: false,
            prompt_format: PromptFormat::Gemma,
            min_model_size_mb: 200,
        };
        assert_eq!(config2.log_prefix, "another");
        assert!(!config2.strip_think_tags);
        assert_eq!(config2.prompt_format, PromptFormat::Gemma);
        assert_eq!(config2.min_model_size_mb, 200);
    }

    #[test]
    fn test_polish_context_params_disable_flash_attention() {
        let params = build_polish_context_params();
        assert_eq!(params.n_ctx(), NonZeroU32::new(2048));
        assert_eq!(
            params.flash_attention_policy(),
            llama_cpp_sys_2::LLAMA_FLASH_ATTN_TYPE_DISABLED
        );
    }
}
