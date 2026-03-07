pub struct PolishTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub system_prompt: &'static str,
}

pub const POLISH_TEMPLATES: &[PolishTemplate] = &[
    PolishTemplate {
        id: "filler",
        name: "Remove Fillers",
        description: "Remove filler words and fix grammar while preserving meaning",
        system_prompt:
            r#"Remove filler words. Keep the same language as input.

Remove: um, uh, like, you know, 嗯, 那个, 就是说
Fix typos only if obvious.
Keep meaning exactly the same.

Examples:
Input: "Um, I think we should go"
Output: "I think we should go"

Input: "嗯，我觉得这个，那个，挺好的"
Output: "我觉得这个挺好的"

Input: "It works fine"
Output: "It works fine"

Output only the result."#,
    },
    PolishTemplate {
        id: "formal",
        name: "Formal Style",
        description: "Convert to professional, formal written style",
        system_prompt: "Make text formal. Keep the same language as input.

Use formal grammar. Remove slang.
Keep meaning the same.

Examples:
Input: \"Hey, check this out\"
Output: \"Could you please review this?\"

Input: \"嘿，帮我看看这个呗\"
Output: \"请帮我审阅一下这个\"

Input: \"I wanna eat\"
Output: \"I would like to eat\"

Output only the result.",
    },
    PolishTemplate {
        id: "concise",
        name: "Make Concise",
        description: "Shorten and simplify while keeping key information",
        system_prompt: "Make text shorter. Keep the same language as input.

Remove unnecessary words.
Keep all key information and meaning.

Examples:
Input: \"I think we should probably go there\"
Output: \"We should go there\"

Input: \"我觉得我们应该可能需要去那里\"
Output: \"我们应该去那里\"

Input: \"This is very important\"
Output: \"This is important\"

Output only the result.",
    },
    PolishTemplate {
        id: "agent",
        name: "Agent Prompt",
        description: "Format as structured markdown for AI agents",
        system_prompt: "Format as markdown. Keep the same language as input.

Remove fillers (um, uh, 嗯, 那个).
Add structure: ## headers, - lists.
Keep meaning the same.

Examples:
Input: \"I need a button that um shows loading\"
Output:
## Task
Create a button that shows loading

Input: \"帮我写函数，嗯，计算字符串长度\"
Output:
## 任务
写一个计算字符串长度的函数

Input: \"Fix login bug and add error handling\"
Output:
## Task
- Fix login bug
- Add error handling

Output only the result.",
    },
];

pub fn get_template_by_id(id: &str) -> Option<&'static PolishTemplate> {
    POLISH_TEMPLATES.iter().find(|t| t.id == id)
}

pub fn get_all_templates() -> Vec<(&'static str, &'static str, &'static str)> {
    POLISH_TEMPLATES
        .iter()
        .map(|t| (t.id, t.name, t.description))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polish_templates_not_empty() {
        assert!(!POLISH_TEMPLATES.is_empty());
        assert!(POLISH_TEMPLATES.len() >= 4);
    }

    #[test]
    fn test_get_template_by_id_filler() {
        let template = get_template_by_id("filler");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "filler");
        assert_eq!(template.name, "Remove Fillers");
        assert!(template.system_prompt.contains("Remove filler words"));
    }

    #[test]
    fn test_get_template_by_id_formal() {
        let template = get_template_by_id("formal");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "formal");
        assert_eq!(template.name, "Formal Style");
        assert!(template.system_prompt.contains("formal"));
    }

    #[test]
    fn test_get_template_by_id_concise() {
        let template = get_template_by_id("concise");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "concise");
        assert_eq!(template.name, "Make Concise");
        assert!(template.system_prompt.contains("shorter") || template.system_prompt.contains("concise"));
    }

    #[test]
    fn test_get_template_by_id_agent() {
        let template = get_template_by_id("agent");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "agent");
        assert_eq!(template.name, "Agent Prompt");
        assert!(template.system_prompt.contains("markdown"));
    }

    #[test]
    fn test_get_template_by_id_not_found() {
        let template = get_template_by_id("nonexistent");
        assert!(template.is_none());
    }

    #[test]
    fn test_get_all_templates() {
        let templates = get_all_templates();
        assert_eq!(templates.len(), POLISH_TEMPLATES.len());

        // Check that all expected templates are present
        let ids: Vec<&str> = templates.iter().map(|(id, _, _)| *id).collect();
        assert!(ids.contains(&"filler"));
        assert!(ids.contains(&"formal"));
        assert!(ids.contains(&"concise"));
        assert!(ids.contains(&"agent"));
    }

    #[test]
    fn test_all_templates_have_valid_fields() {
        for template in POLISH_TEMPLATES {
            // ID should not be empty
            assert!(!template.id.is_empty());

            // Name should not be empty
            assert!(!template.name.is_empty());

            // Description should not be empty
            assert!(!template.description.is_empty());

            // System prompt should not be empty
            assert!(!template.system_prompt.is_empty());

            // System prompt should contain language preservation instruction
            assert!(
                template.system_prompt.contains("Keep language unchanged") ||
                template.system_prompt.contains("SAME LANGUAGE") ||
                template.system_prompt.contains("same language"),
                "Template '{}' missing language preservation instruction",
                template.id
            );
        }
    }

    #[test]
    fn test_template_ids_are_unique() {
        let mut ids = std::collections::HashSet::new();
        for template in POLISH_TEMPLATES {
            assert!(
                ids.insert(template.id),
                "Duplicate template ID found: {}",
                template.id
            );
        }
    }
}
