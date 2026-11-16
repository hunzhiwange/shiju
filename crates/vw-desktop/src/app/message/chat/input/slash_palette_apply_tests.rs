//! Slash palette 状态机与选中应用测试。

use super::file_search::handle_input_editor_action;
use super::slash_palette::{handle_slash_palette_select, handle_slash_palette_select_current};
use crate::app::state::{
    ModelSummary, ProviderModelsSummary, SkillsCatalogItem, SkillsCatalogKind, SlashPaletteKind,
};
use crate::app::App;
use iced::widget::text_editor;
use serde_json::json;
use std::sync::Arc;

fn paste(app: &mut App, value: String) -> iced::Task<crate::app::Message> {
    handle_input_editor_action(
        app,
        text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(value))),
    )
}

fn test_app() -> App {
    App::new().0
}

fn catalog_skill(id: &str, title: &str) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: title.to_string(),
        description: String::new(),
        kind: SkillsCatalogKind::Personal,
        resource_count: 0,
        installed: true,
        enabled: true,
        source: "test".to_string(),
        source_path: None,
    }
}



#[test]
fn slash_palette_select_model_applies_and_clears_token() {
    let mut app = test_app();
    app.model_settings.providers = vec![ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![ModelSummary {
            id: "gpt-4.1".to_string(),
            name: "gpt-4.1".to_string(),
            enabled: true,
            toolcall: true,
            attachment: true,
            context_limit: 128_000,
            detail: json!({}),
        }],
    }];

    let _ = paste(&mut app, "hello /gpt".to_string());
    assert!(app.input_suggestions.show_slash_palette);

    let _ = handle_slash_palette_select(
        &mut app,
        SlashPaletteKind::Model,
        "openai/gpt-4.1".to_string(),
    );

    assert!(!app.input_suggestions.show_slash_palette);
    assert_eq!(app.current_session_runtime().model, "openai/gpt-4.1");
    assert_eq!(app.current_session_runtime().input_editor.text().trim(), "hello");
}

#[test]
fn slash_palette_select_skill_toggles_manual_skill_and_clears_token() {
    let mut app = test_app();
    app.skills_settings.catalog = vec![catalog_skill("review", "代码审查")];

    let _ = paste(&mut app, "/review".to_string());
    assert!(app.input_suggestions.show_slash_palette);

    let _ = handle_slash_palette_select_current(&mut app);

    assert!(!app.input_suggestions.show_slash_palette);
    assert_eq!(
        app.current_session_runtime().tool_selector.selected_manual_skills(),
        vec!["review".to_string()]
    );
    assert!(app.current_session_runtime().input_editor.text().trim().is_empty());
}
