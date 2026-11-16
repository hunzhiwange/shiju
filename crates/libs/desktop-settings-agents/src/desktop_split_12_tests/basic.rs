//! 验证代理基础设置的注册、委派控件可见性与视图状态。
//! 不同代理来源和能力组合用于确认仅在支持时暴露相应控制项。

use super::basic_view as view;
use crate::state::DelegateAgentSettingsEntry;
use iced::advanced::widget::Tree;

fn view_tree(
    app: &crate::agents_state::AgentsSettingsState,
    entry: &DelegateAgentSettingsEntry,
    provider_options: Vec<String>,
) -> Tree {
    let element = view(app, entry, provider_options);
    Tree::new(element.as_widget())
}

fn assert_common_field_structure(tree: &Tree, has_registration: bool) {
    let offset = usize::from(has_registration);
    assert_eq!(tree.children.len(), 4 + offset);
    // Profile fields and the single defaults/model group precede runtime controls.
    assert_eq!(tree.children[0].children.len(), 5);
    assert_eq!(tree.children[1].children.len(), 2);
    let runtime = &tree.children[3 + offset];
    assert_eq!(runtime.children.len(), 13); // Seven rows and six dividers.
}

#[test]
fn basic_view_main_hides_availability_and_shows_common_runtime_controls() {
    let app = crate::agents_state::AgentsSettingsState::default();
    let main = app
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .expect("default settings should contain main agent");

    let tree = view_tree(&app, main, vec!["openai".to_string(), main.provider.clone()]);

    assert_common_field_structure(&tree, false);
}

#[test]
fn basic_view_worker_shows_availability_and_common_runtime_controls() {
    let app = crate::agents_state::AgentsSettingsState::default();
    let worker = app
        .entries
        .iter()
        .find(|entry| entry.key != "main")
        .expect("default settings should contain a worker agent");

    let tree = view_tree(&app, worker, vec!["openai".to_string(), worker.provider.clone()]);

    assert_common_field_structure(&tree, true);
    assert_eq!(tree.children[2].children.len(), 2);
    assert_eq!(tree.children[2].children[1].children.len(), 2);
}

#[test]
fn basic_view_keeps_current_provider_and_model_even_when_missing_from_catalog() {
    let app = crate::agents_state::AgentsSettingsState::default();
    let mut entry: DelegateAgentSettingsEntry = app.entries[0].clone();
    entry.provider = "custom-provider".to_string();
    entry.model = "custom-model".to_string();

    let tree = view_tree(&app, &entry, vec!["openai".to_string()]);

    assert_common_field_structure(&tree, false);
}

#[test]
fn basic_view_preserves_fields_for_blank_selections_and_numeric_boundaries() {
    let app = crate::agents_state::AgentsSettingsState::default();
    let mut entry = app
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .expect("default settings should contain main agent")
        .clone();
    entry.provider = "   ".to_string();
    entry.model = "\t".to_string();

    for (temperature, max_depth, max_iterations, max_history_messages) in
        [(0.0, 1, 1, 1), (2.0, 32, 50_001, 1000)]
    {
        entry.temperature = temperature;
        entry.max_depth = max_depth;
        entry.max_iterations = max_iterations;
        entry.max_history_messages = max_history_messages;

        let tree = view_tree(&app, &entry, Vec::new());

        assert_common_field_structure(&tree, false);
    }
}

#[test]
fn basic_view_renders_runtime_boolean_and_text_variants() {
    let app = crate::agents_state::AgentsSettingsState::default();
    let mut entry = app
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .expect("default settings should contain main agent")
        .clone();
    entry.api_key_input = "secret override".to_string();
    entry.compact_context = true;
    entry.parallel_tools = true;
    entry.agentic = true;

    let tree = view_tree(&app, &entry, vec![entry.provider.clone()]);

    assert_common_field_structure(&tree, false);
}

#[test]
fn agent_runtime_visibility_acp_preserves_profile_defaults_and_delegation_eligibility() {
    use config_agent::DelegateAgentConfig;
    use config_agent_profile::AgentRuntime;
    let app = crate::agents_state::AgentsSettingsState::default();
    for backend in ["codex", "claude"] {
        let entry = DelegateAgentSettingsEntry::from_config(
            "custom-runtime",
            Some(DelegateAgentConfig {
                runtime: AgentRuntime::Acp { backend: backend.into() },
                ..Default::default()
            }),
        );
        let tree = view_tree(&app, &entry, Vec::new());
        assert_eq!(tree.children.len(), 4);
        assert_eq!(tree.children[0].children.len(), 5);
        assert_eq!(tree.children[1].children.len(), 2);
        assert_eq!(tree.children[3].children[1].children.len(), 1);
    }
}
