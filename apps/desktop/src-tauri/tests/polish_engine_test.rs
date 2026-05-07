use ariatype_lib::polish_engine::{
    get_all_polish_models, get_all_templates, get_template_by_id, PolishEngineType, PolishRequest,
    PolishResult, UnifiedPolishManager, POLISH_TEMPLATES,
};

/// Integration tests for polish_engine module
/// These tests verify the public API and integration between components

#[test]
fn test_polish_engine_type_integration() {
    // Test that all engine types can be created and converted
    let qwen = PolishEngineType::Qwen;
    let lfm = PolishEngineType::Lfm;

    assert_eq!(qwen.as_str(), "qwen");
    assert_eq!(lfm.as_str(), "lfm");

    // Test parsing
    assert_eq!("qwen".parse::<PolishEngineType>().unwrap(), qwen);
    assert_eq!("lfm".parse::<PolishEngineType>().unwrap(), lfm);
}

#[test]
fn test_unified_manager_initialization() {
    let manager = UnifiedPolishManager::new();
    let engines = manager.available_engines();

    // Should have three local engines registered
    assert_eq!(engines.len(), 3);
    assert!(engines.contains(&PolishEngineType::Qwen));
    assert!(engines.contains(&PolishEngineType::Lfm));
    assert!(engines.contains(&PolishEngineType::Gemma));
}

#[test]
fn test_model_auto_detection() {
    // Test that model IDs are correctly mapped to engines
    assert_eq!(
        UnifiedPolishManager::get_engine_by_model_id("qwen3.5-0.8b"),
        Some(PolishEngineType::Qwen)
    );
    assert_eq!(
        UnifiedPolishManager::get_engine_by_model_id("lfm2.5-1.2b"),
        Some(PolishEngineType::Lfm)
    );
    assert_eq!(
        UnifiedPolishManager::get_engine_by_model_id("unknown-model"),
        None
    );
}

#[test]
fn test_all_models_available() {
    let models = get_all_polish_models();

    // Should have models from both engines
    assert!(models.len() >= 5);

    // Verify model structure
    for model in &models {
        assert!(!model.id.is_empty());
        assert!(!model.display_name.is_empty());
        assert!(!model.size_display.is_empty());
    }

    // Check specific models exist
    let qwen_model = models.iter().find(|m| m.id == "qwen3.5-0.8b");
    assert!(qwen_model.is_some());
    assert_eq!(qwen_model.unwrap().engine_type, PolishEngineType::Qwen);

    let lfm_model = models.iter().find(|m| m.id == "lfm2.5-1.2b");
    assert!(lfm_model.is_some());
    assert_eq!(lfm_model.unwrap().engine_type, PolishEngineType::Lfm);
}

#[test]
fn test_templates_system() {
    // Test that all templates are accessible
    let templates = get_all_templates();
    assert!(templates.len() >= 4);

    // Test specific templates
    let filler = get_template_by_id("filler");
    assert!(filler.is_some());
    assert_eq!(filler.unwrap().name, "Remove Fillers");

    let formal = get_template_by_id("formal");
    assert!(formal.is_some());
    assert_eq!(formal.unwrap().name, "Formal Style");

    let concise = get_template_by_id("concise");
    assert!(concise.is_some());
    assert_eq!(concise.unwrap().name, "Make Concise");

    let agent = get_template_by_id("agent");
    assert!(agent.is_some());
    assert_eq!(agent.unwrap().name, "Agent Prompt");
}

#[test]
fn test_polish_request_builder() {
    let request =
        PolishRequest::new("Test text to polish", "System prompt", "en").with_model("model.gguf");

    assert_eq!(request.text, "Test text to polish");
    assert_eq!(request.system_context.system_prompt, "System prompt");
    assert_eq!(request.language, "en");
    assert_eq!(request.model_name, Some("model.gguf".to_string()));
}

#[test]
fn test_polish_result_creation() {
    let result = PolishResult::new("Polished text".to_string(), PolishEngineType::Qwen, 1500);

    assert_eq!(result.text, "Polished text");
    assert_eq!(result.engine, PolishEngineType::Qwen);
    assert_eq!(result.total_ms, 1500);

    let result_with_metrics = PolishResult::with_metrics(
        "Polished".to_string(),
        PolishEngineType::Lfm,
        2000,
        Some(500),
        Some(1500),
    );

    assert_eq!(result_with_metrics.model_load_ms, Some(500));
    assert_eq!(result_with_metrics.inference_ms, Some(1500));
}

#[test]
fn test_manager_model_filename_lookup() {
    let manager = UnifiedPolishManager::new();

    // Test Qwen model filename lookup
    let qwen_filename = manager.get_model_filename(PolishEngineType::Qwen, "qwen3.5-0.8b");
    assert!(qwen_filename.is_some());
    assert!(qwen_filename.unwrap().ends_with(".gguf"));

    // Test LFM model filename lookup
    let lfm_filename = manager.get_model_filename(PolishEngineType::Lfm, "lfm2.5-1.2b");
    assert!(lfm_filename.is_some());
    assert!(lfm_filename.unwrap().ends_with(".gguf"));

    // Test non-existent model
    let invalid = manager.get_model_filename(PolishEngineType::Qwen, "nonexistent");
    assert!(invalid.is_none());
}

#[test]
fn test_cache_operations() {
    let manager = UnifiedPolishManager::new();

    // Test cache clearing operations (should not panic)
    manager.clear_cache();
    manager.clear_engine_cache(PolishEngineType::Qwen, None);
    manager.clear_engine_cache(PolishEngineType::Lfm, Some("model.gguf"));
}

#[test]
#[ignore]
fn test_template_language_preservation() {
    // All templates should have language preservation instructions
    for template in POLISH_TEMPLATES {
        let prompt = template.system_prompt.to_lowercase();
        assert!(
            prompt.contains("same language") || prompt.contains("exact same language"),
            "Template '{}' missing language preservation instruction",
            template.id
        );
    }
}

#[test]
fn test_model_info_completeness() {
    let models = get_all_polish_models();

    for model in models {
        // Each model should have complete information
        assert!(!model.id.is_empty(), "Model ID should not be empty");
        assert!(
            !model.display_name.is_empty(),
            "Display name should not be empty"
        );
        assert!(
            !model.size_display.is_empty(),
            "Size display should not be empty"
        );

        // Size display should contain size information
        assert!(
            model.size_display.contains("MB") || model.size_display.contains("GB"),
            "Size display should contain MB or GB: {}",
            model.size_display
        );
    }
}

#[test]
fn test_engine_type_serialization() {
    // Test JSON serialization/deserialization
    let qwen = PolishEngineType::Qwen;
    let json = serde_json::to_string(&qwen).unwrap();
    assert_eq!(json, "\"qwen\"");

    let deserialized: PolishEngineType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, qwen);

    let lfm = PolishEngineType::Lfm;
    let json = serde_json::to_string(&lfm).unwrap();
    assert_eq!(json, "\"lfm\"");

    let deserialized: PolishEngineType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, lfm);
}
