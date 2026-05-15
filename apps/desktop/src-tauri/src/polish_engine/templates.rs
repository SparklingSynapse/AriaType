pub struct PolishTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub system_prompt: &'static str,
}

pub const POLISH_TEMPLATES: &[PolishTemplate] = &[
    PolishTemplate {
        id: "filler",
        name: "Clean Dictation",
        description: "Clean raw speech into natural writing without changing meaning",
        system_prompt: r#"Clean up raw dictation. Keep the same language as input.

Transform spoken text into natural written text.
Remove filler words, verbal hesitations, accidental repetition, and abandoned self-corrections.
Fix obvious punctuation, grammar, typos, and Speech-to-Text homophone mistakes.
Keep the speaker's intended meaning, facts, order, and tone exactly the same.
Do not answer questions, summarize, expand, or add new information.

Examples:
Input: "Um, I think we should go"
Output: "I think we should go"

Input: "嗯，我觉得这个，那个，挺好的"
Output: "我觉得这个挺好的"

Input: "Do you want coffee, actually boba?"
Output: "Do you want boba?"

Input: "这个分析错误可能是由于标点引起的"
Output: "这个分词错误可能是由于标点引起的"

Output only the result."#,
    },
    PolishTemplate {
        id: "chat",
        name: "Chat Reply",
        description: "Turn speech into a concise natural chat message",
        system_prompt: "Rewrite as a natural chat message. Keep the same language as input.

Make the text clear, direct, and easy to send in chat apps.
Remove filler words, accidental repetition, and rough spoken phrasing.
Keep the speaker's intent, facts, tone, and level of warmth.
Prefer short paragraphs or a compact list only when the input clearly contains multiple points.
Do not make the text overly formal.
Do not answer questions, summarize, invent context, or add new information.

Examples:
Input: \"嗯，这个我看了一下，感觉可以，明天我们再对一下细节吧\"
Output: \"我看了一下，感觉可以。明天我们再对一下细节吧。\"

Input: \"Hey uh can you check this when you have time no rush\"
Output: \"Hey, can you check this when you have time? No rush.\"

Output only the result.",
    },
    PolishTemplate {
        id: "formal",
        name: "Professional Message",
        description: "Polish speech into professional email or workplace writing",
        system_prompt: "Rewrite as polished professional writing for email or workplace communication. Keep the same language as input.

Use formal language with clear, courteous, complete sentences.
Remove filler words, slang, rough phrasing, and unnecessary repetition.
Fix obvious punctuation, grammar, typos, and Speech-to-Text homophone mistakes.
Preserve the original facts, intent, level of detail, and order.
Do not make the text longer unless needed for grammar or clarity.
Do not answer questions, summarize, invent context, or add new information.

Examples:
Input: \"Hey, check this out\"
Output: \"Could you please review this?\"

Input: \"嘿，帮我看看这个呗\"
Output: \"请帮我审阅一下这个\"

Input: \"这个方案大概可以，下周我们再碰一下\"
Output: \"这个方案整体可行，我们下周再进一步讨论。\"

Output only the result.",
    },
    PolishTemplate {
        id: "concise",
        name: "Make Concise",
        description: "Shorten and simplify while keeping key information",
        system_prompt: "Make the text shorter and clearer. Keep the same language as input.

Remove filler words, repetition, hedging, and low-value phrasing.
Merge duplicate points and simplify long sentences.
Fix obvious punctuation, grammar, typos, and Speech-to-Text homophone mistakes.
Keep all key facts, decisions, constraints, names, dates, numbers, and requests.
Do not change intent, add new information, or over-compress important details.

Examples:
Input: \"I think we should probably go there\"
Output: \"We should go there\"

Input: \"我觉得我们应该可能需要去那里\"
Output: \"我们应该去那里\"

Input: \"这个事情我们可能最好还是今天先简单看一下，然后明天再正式讨论\"
Output: \"我们今天先简单看一下，明天再正式讨论。\"

Output only the result.",
    },
    PolishTemplate {
        id: "document",
        name: "Structured Notes",
        description: "Organize long dictation into readable notes or document prose",
        system_prompt: "Organize spoken content into readable notes or document prose. Keep the same language as input.

Use the transcript's own logic to add paragraphs, short section labels, or lists.
Remove filler words, accidental repetition, and abandoned self-corrections.
Fix obvious punctuation, grammar, typos, and Speech-to-Text homophone mistakes.
Preserve all explicit information, order, nuance, constraints, names, dates, numbers, and examples.
Do not summarize away details, invent headings, add conclusions, or add new information.
Do not use code blocks or quote blocks.

Examples:
Input: \"这个文档开头先介绍产品是什么 然后讲用户怎么操作 再讲背后的渲染流程 最后进入性能问题分析\"
Output:
文档开头先介绍产品是什么，以及用户如何完成基本操作。

接着，基于用户视角说明背后的渲染流程：数据如何被消费、修改时如何更新，以及最终如何渲染到 Canvas 上。

最后，再进入性能问题分析。

Input: \"first talk about the goal then list the risks one is latency two is privacy three is fallback behavior\"
Output:
First, explain the goal.

Then list the risks:
1. Latency
2. Privacy
3. Fallback behavior

Output only the result.",
    },
    PolishTemplate {
        id: "agent",
        name: "Agent Prompt",
        description: "Format as structured markdown for AI agents",
        system_prompt: "Format the dictation as a clear markdown prompt for an AI agent. Keep the same language as input.

Turn rough spoken requirements into actionable instructions.
Remove filler words, accidental repetition, and abandoned self-corrections.
Fix obvious punctuation, grammar, typos, and Speech-to-Text homophone mistakes.
Use markdown headings and bullets only when they make the task easier to follow.
Preserve all explicit requirements, constraints, file names, commands, acceptance criteria, and caveats.
Do not answer, implement, solve, summarize away details, or add new requirements.

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

Input: \"检查最新日志，看一下有没有走 STT 和 polish，链路是不是完整\"
Output:
## 任务
检查最新日志，确认：
- 是否走了 STT
- 是否走了 polish
- 调用链路是否完整

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
        assert!(POLISH_TEMPLATES.len() >= 6);
    }

    #[test]
    fn test_get_template_by_id_filler() {
        let template = get_template_by_id("filler");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "filler");
        assert_eq!(template.name, "Clean Dictation");
        assert!(template.system_prompt.contains("Clean up raw dictation"));
    }

    #[test]
    fn test_get_template_by_id_chat() {
        let template = get_template_by_id("chat");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "chat");
        assert_eq!(template.name, "Chat Reply");
        assert!(template.system_prompt.contains("chat message"));
    }

    #[test]
    fn test_get_template_by_id_formal() {
        let template = get_template_by_id("formal");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "formal");
        assert_eq!(template.name, "Professional Message");
        assert!(template.system_prompt.contains("professional"));
    }

    #[test]
    fn test_get_template_by_id_concise() {
        let template = get_template_by_id("concise");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "concise");
        assert_eq!(template.name, "Make Concise");
        assert!(
            template.system_prompt.contains("shorter")
                || template.system_prompt.contains("concise")
        );
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
    fn test_get_template_by_id_document() {
        let template = get_template_by_id("document");
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.id, "document");
        assert_eq!(template.name, "Structured Notes");
        assert!(template.system_prompt.contains("document prose"));
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
        assert!(ids.contains(&"chat"));
        assert!(ids.contains(&"formal"));
        assert!(ids.contains(&"concise"));
        assert!(ids.contains(&"document"));
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
                template.system_prompt.contains("Keep language unchanged")
                    || template.system_prompt.contains("SAME LANGUAGE")
                    || template.system_prompt.contains("same language"),
                "Template '{}' missing language preservation instruction",
                template.id
            );
        }
    }

    #[test]
    fn test_all_templates_keep_transform_boundaries() {
        for template in POLISH_TEMPLATES {
            assert!(
                template.system_prompt.contains("Do not")
                    && template.system_prompt.contains("add new"),
                "Template '{}' must forbid adding new information",
                template.id
            );
            assert!(
                template.system_prompt.contains("Output only the result"),
                "Template '{}' must output only the result",
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
