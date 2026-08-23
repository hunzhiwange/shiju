//! 验证会话上下文来源的注入顺序、去重与可见性边界。

use serde_json::json;

use super::{ContextRole, assemble_agents_context};
use crate::session::instruction::{InstructionSource, Resolved};

fn resolved(filepath: &str, content: &str) -> Resolved {
    Resolved {
        filepath: filepath.to_string(),
        content: format!("Instructions from: {filepath}\n{content}"),
        sources: vec![InstructionSource {
            filepath: filepath.to_string(),
            parent: None,
            conditions: Vec::new(),
        }],
        diagnostics: Vec::new(),
    }
}

#[test]
fn agents_context_injection_prepends_system_reminder() {
    let history = vec![
        json!({ "role": "user", "content": "original user bytes" }),
        json!({ "role": "assistant", "content": "prior answer" }),
    ];
    let instructions = vec![
        resolved("/workspace/AGENTS.md", "first project rule"),
        resolved("/workspace/nested/AGENTS.local.md", "second project rule"),
    ];

    let injection = assemble_agents_context(&instructions, &history);

    assert_eq!(injection.transient_messages.len(), 1);
    let reminder = &injection.transient_messages[0];
    assert_eq!(reminder.role, ContextRole::System);
    assert!(!reminder.visible);
    assert!(reminder.content.starts_with("<system-reminder>\n"));
    assert!(reminder.content.ends_with("\n</system-reminder>"));
    assert!(reminder.content.contains("AGENTS.md loaded for this request"));
    assert!(reminder.content.contains("override default instructions"));
    let first_source = reminder.content.find("/workspace/AGENTS.md").expect("first source");
    let second_source =
        reminder.content.find("/workspace/nested/AGENTS.local.md").expect("second source");
    assert!(first_source < second_source);
    assert_eq!(injection.history, history);
    let messages = injection.provider_messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[1], history[0]);
    assert_eq!(messages[2], history[1]);
}

#[test]
fn agents_context_injection_omits_reminder_for_empty_instructions() {
    let history = vec![json!({ "role": "user", "content": "original user bytes" })];

    let injection = assemble_agents_context(&[], &history);

    assert!(injection.transient_messages.is_empty());
    assert_eq!(injection.history, history);
}
