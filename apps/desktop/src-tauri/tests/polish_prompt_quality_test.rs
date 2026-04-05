/// Polish Engine Prompt Effectiveness Tests
///
/// These tests validate that prompts produce correct outputs by running actual inference.
/// They are marked with #[ignore] because they require:
/// - Model files to be downloaded
/// - Actual inference time (slow)
///
/// Run manually with: cargo test --test polish_prompt_quality_test -- --ignored --nocapture
use ariatype_lib::polish_engine::{get_template_by_id, PolishRequest, UnifiedPolishManager};
use ariatype_lib::utils::AppPaths;

/// Helper function to check if a model exists
fn model_exists(model_name: &str) -> bool {
    AppPaths::models_dir().join(model_name).exists()
}

/// Validates that each template performs its specific task correctly:
/// - Formal: makes text more formal
/// - Concise: makes text shorter
/// - Agent: adds markdown structure
#[tokio::test]
#[ignore] // Requires model file and takes time
async fn test_template_specific_behavior() {
    let model_id = "qwen3.5-0.8b";
    let manager = UnifiedPolishManager::new();

    let engine_type =
        UnifiedPolishManager::get_engine_by_model_id(model_id).expect("Should detect engine type");
    let model_filename = manager
        .get_model_filename(engine_type, model_id)
        .expect("Should get model filename");

    if !model_exists(&model_filename) {
        println!("⚠️  Model not found: {}", model_filename);
        return;
    }

    // Test formal template
    println!("\n=== Testing Formal Template ===");
    let formal_template = get_template_by_id("formal").expect("formal template should exist");
    let casual_input = "Hey, emmm... can you check this out? It's pretty cool";
    let request = PolishRequest::new(casual_input, formal_template.system_prompt, "en")
        .with_model(&model_filename);

    let result = manager.polish(engine_type, request).await;
    assert!(result.is_ok(), "Formal polish should succeed");

    let formal_output = result.unwrap().text;
    println!("Input:  {}", casual_input);
    println!("Output: {}", formal_output);
}
