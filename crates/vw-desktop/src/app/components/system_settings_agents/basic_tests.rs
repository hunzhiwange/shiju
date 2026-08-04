use super::basic::{is_main_agent, shows_registration_controls, view};
use crate::app::state::DelegateAgentSettingsEntry;
use iced::advanced::widget::Tree;

fn view_tree(
    app: &crate::app::App,
    entry: &DelegateAgentSettingsEntry,
    provider_options: Vec<String>,
) -> Tree {
    let element = view(app, entry, provider_options);
    Tree::new(element.as_widget())
}

fn assert_common_field_structure(tree: &Tree) {
    assert_eq!(tree.children.len(), 11);
    for row_index in 3..=9 {
        assert_eq!(tree.children[row_index].children.len(), 2);
    }
    assert_eq!(tree.children[6].children[1].children.len(), 2);
    assert_eq!(tree.children[2].children.len(), 2);
    assert_eq!(tree.children[2].children[1].children.len(), 2);
}

#[test]
fn basic_view_main_hides_availability_and_shows_runtime_controls() {
    let (app, _) = crate::app::App::new();
    let main = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .expect("default settings should contain main agent");

    let tree = view_tree(&app, main, vec!["openai".to_string(), main.provider.clone()]);

    assert_common_field_structure(&tree);
    assert!(tree.children[1].children.is_empty());
    assert_eq!(tree.children[10].children.len(), 6);
}

#[test]
fn basic_agent_visibility_helpers_distinguish_main_from_workers() {
    let (app, _) = crate::app::App::new();
    let main = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .expect("main agent exists");
    let worker = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key != "main")
        .expect("worker agent exists");

    assert!(is_main_agent(main));
    assert!(!shows_registration_controls(main));
    assert!(!is_main_agent(worker));
    assert!(shows_registration_controls(worker));
}

#[test]
fn basic_view_worker_shows_availability_and_hides_main_runtime_controls() {
    let (app, _) = crate::app::App::new();
    let worker = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key != "main")
        .expect("default settings should contain a worker agent");

    let tree = view_tree(&app, worker, vec!["openai".to_string(), worker.provider.clone()]);

    assert_common_field_structure(&tree);
    assert_eq!(tree.children[1].children.len(), 2);
    assert_eq!(tree.children[1].children[1].children.len(), 1);
    assert_eq!(tree.children[1].children[1].children[0].children.len(), 2);
    assert!(tree.children[10].children.is_empty());
}

#[test]
fn basic_view_keeps_current_provider_and_model_even_when_missing_from_catalog() {
    let (app, _) = crate::app::App::new();
    let mut entry: DelegateAgentSettingsEntry = app.agents_settings.entries[0].clone();
    entry.provider = "custom-provider".to_string();
    entry.model = "custom-model".to_string();

    let tree = view_tree(&app, &entry, vec!["openai".to_string()]);

    assert_common_field_structure(&tree);
}

#[test]
fn basic_view_preserves_fields_for_blank_selections_and_numeric_boundaries() {
    let (app, _) = crate::app::App::new();
    let mut entry = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .expect("default settings should contain main agent")
        .clone();
    entry.provider = "   ".to_string();
    entry.model = "\t".to_string();

    for (temperature, max_depth, max_iterations, max_tool_iterations, max_history_messages) in
        [(0.0, 1, 1, 1, 1), (2.0, 32, 50_001, 50_002, 1000)]
    {
        entry.temperature = temperature;
        entry.max_depth = max_depth;
        entry.max_iterations = max_iterations;
        entry.max_tool_iterations = max_tool_iterations;
        entry.max_history_messages = max_history_messages;

        let tree = view_tree(&app, &entry, Vec::new());

        assert_common_field_structure(&tree);
        assert!(tree.children[1].children.is_empty());
        assert_eq!(tree.children[10].children.len(), 6);
    }
}

#[test]
fn basic_view_renders_main_runtime_boolean_and_text_variants() {
    let (app, _) = crate::app::App::new();
    let mut entry = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .expect("default settings should contain main agent")
        .clone();
    entry.api_key_input = "secret override".to_string();
    entry.compact_context = true;
    entry.parallel_tools = true;
    entry.agentic = true;
    entry.tool_dispatcher = "sequential".to_string();

    let tree = view_tree(&app, &entry, vec![entry.provider.clone()]);

    assert_common_field_structure(&tree);
    assert!(tree.children[1].children.is_empty());
    assert_eq!(tree.children[10].children.len(), 6);
}
