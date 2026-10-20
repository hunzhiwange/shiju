//! Dify 全节点编辑、导出、重载与实际暗色渲染回归。

use super::{model, state::WorkflowState, view};
use crate::app::Message;
use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
use iced::{Color, Element, Font, Pixels, Size, Theme, mouse};
use serde_yaml::{Mapping, Value};

fn key(value: &str) -> Value {
    Value::String(value.to_string())
}

fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Mapping(entries.into_iter().map(|(name, value)| (key(name), value)).collect())
}

fn valid_node_data(block_type: &str) -> Value {
    let yaml = match block_type {
        "llm" => Some(
            "model: {provider: openai, name: gpt-4.1, mode: chat}\n\
             prompt_template: []\ncontext: {enabled: false, variable_selector: []}\n\
             vision: {enabled: false}",
        ),
        "end" => Some("outputs: [{variable: result, value_selector: [answer, answer]}]"),
        "template-transform" => Some("variables: []\ntemplate: '结果：{{ value }}'"),
        "document-extractor" => Some("variable_selector: [start, file]"),
        "code" => Some(
            "variables: []\ncode_language: python3\ncode: 'def main(): return {\"result\": \"ok\"}'\n\
             outputs: {result: {type: string}}\nretry_config: {max_retries: 1, retry_interval: 100}\n\
             error_strategy: fail-branch",
        ),
        "tool" => Some(
            "provider_id: builtin\nprovider_type: builtin\nprovider_name: builtin\n\
             tool_name: search\ntool_parameters: {}\ntool_configurations: {}",
        ),
        "agent" => Some(
            "agent_strategy_provider_name: builtin\nagent_strategy_name: function_calling\n\
             agent_strategy_label: Function Calling\nagent_parameters: {}\noutput_schema: {}\n\
             memory: {enabled: false}",
        ),
        "http-request" => Some(
            "method: get\nurl: https://example.com\nauthorization: {type: no-auth}\n\
             headers: ''\nparams: ''\nbody: {type: none, data: []}\n\
             timeout: {connect: 10, read: 60, write: 20}\nssl_verify: true\n\
             retry_config: {max_retries: 1, retry_interval: 100}\nerror_strategy: fail-branch",
        ),
        "if-else" => Some(
            r#"cases:
  - case_id: "true"
    logical_operator: and
    conditions:
      - variable_selector: [start, query]
        comparison_operator: contains
        value: ready
        varType: string
  - case_id: "false"
    logical_operator: and
    conditions:
      - variable_selector: [start, query]
        comparison_operator: not contains
        value: ready
        varType: string
"#,
        ),
        "question-classifier" => Some(
            "query_variable_selector: [start, query]\n\
             model: {provider: openai, name: gpt-4.1, mode: chat}\n\
             classes: [{id: class-1, name: 默认分类}]\ninstruction: ''",
        ),
        "parameter-extractor" => Some(
            "query: [start, query]\nmodel: {provider: openai, name: gpt-4.1, mode: chat}\n\
             parameters: []\nreasoning_mode: prompt\ninstruction: ''",
        ),
        "knowledge-retrieval" => Some(
            "query_variable_selector: [start, query]\ndataset_ids: [dataset-1]\n\
             retrieval_mode: multiple\nmultiple_retrieval_config: {top_k: 3, score_threshold: 0.5}",
        ),
        "variable-aggregator" => Some("variables: [[start, query]]\noutput_type: string"),
        "list-operator" => Some(
            "variable: [start, items]\nfilter_by: {enabled: false, conditions: []}\n\
             order_by: {enabled: false, key: '', value: asc}\nlimit: {enabled: false, size: 10}",
        ),
        "iteration" => Some(
            "iterator_selector: [start, items]\noutput_selector: [iteration, item]\n\
             output_type: array[string]\nis_parallel: false\nparallel_nums: 10\n\
             error_handle_mode: terminated",
        ),
        _ => None,
    };
    let yaml = yaml.map(str::to_string).unwrap_or_else(|| {
        // The remaining registered defaults are already valid editor submissions.
        model::default_node_data_yaml(block_type).expect("registered node should have default YAML")
    });
    serde_yaml::from_str(&yaml).expect("representative node data should parse")
}

fn representative_dify_fixture() -> Value {
    let descriptors = model::supported_node_types();
    let nodes = descriptors
        .iter()
        .enumerate()
        .map(|(index, descriptor)| {
            let mut data = valid_node_data(descriptor.block_type);
            let data = data.as_mapping_mut().expect("node data should be a mapping");
            data.insert(key("type"), key(descriptor.block_type));
            data.insert(key("title"), key(&format!("节点 {index}")));
            data.insert(key("x-regression-unknown"), map([("keep", Value::Bool(true))]));

            let id = format!("node-{index}");
            let mut node = Mapping::new();
            node.insert(key("id"), key(&id));
            node.insert(key("type"), key("custom"));
            node.insert(
                key("position"),
                map([
                    ("x", Value::from((index % 5 * 260) as i64)),
                    ("y", Value::from((index / 5 * 190) as i64)),
                ]),
            );
            node.insert(key("positionAbsolute"), node[&key("position")].clone());
            node.insert(key("data"), Value::Mapping(data.clone()));
            if descriptor.block_type == "code" {
                let loop_index = descriptors
                    .iter()
                    .position(|candidate| candidate.block_type == "loop")
                    .expect("loop node should be registered");
                node.insert(key("parentId"), key(&format!("node-{loop_index}")));
            }
            Value::Mapping(node)
        })
        .collect::<Vec<_>>();

    let node_id = |block_type: &str| {
        let index = descriptors
            .iter()
            .position(|descriptor| descriptor.block_type == block_type)
            .unwrap_or_else(|| panic!("{block_type} should be registered"));
        format!("node-{index}")
    };
    let edge = |id: &str, source: &str, target: &str, source_handle: &str| {
        map([
            ("id", key(id)),
            ("source", key(&node_id(source))),
            ("target", key(&node_id(target))),
            ("sourceHandle", key(source_handle)),
            ("targetHandle", key("target")),
            (
                "data",
                map([
                    ("sourceType", key(source)),
                    ("targetType", key(target)),
                    ("x-regression-edge", key("keep")),
                ]),
            ),
        ])
    };

    map([
        (
            "app",
            map([
                ("name", key("全节点中文回归")),
                ("mode", key("advanced-chat")),
                ("x-regression-app", key("keep")),
            ]),
        ),
        (
            "workflow",
            map([
                (
                    "graph",
                    map([
                        ("nodes", Value::Sequence(nodes)),
                        (
                            "edges",
                            Value::Sequence(vec![
                                edge("start-answer", "start", "answer", "source"),
                                edge("if-true", "if-else", "end", "true"),
                                edge("if-false", "if-else", "end", "false"),
                                edge("loop-child", "loop", "code", "source"),
                            ]),
                        ),
                        ("viewport", map([("x", 0.into()), ("y", 0.into()), ("zoom", 1.into())])),
                    ]),
                ),
                ("environment_variables", Value::Sequence(Vec::new())),
                ("conversation_variables", Value::Sequence(Vec::new())),
                ("x-regression-workflow", key("keep")),
            ]),
        ),
    ])
}

fn data_unknown_is_preserved(state: &WorkflowState) {
    for node in &state.document.nodes {
        let data = node.raw_node["data"].as_mapping().expect("node data should remain a mapping");
        assert_eq!(
            data[&key("x-regression-unknown")]["keep"].as_bool(),
            Some(true),
            "{} should preserve unknown data",
            node.block_type
        );
    }
}

async fn assert_dark_element_draws(mut element: Element<'_, Message>) {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut tree = Tree::new(element.as_widget());
    let viewport = iced::Rectangle::with_size(Size::new(1440.0, 960.0));
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &iced::advanced::renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    let pixels = renderer.screenshot(Size::new(1440, 960), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn dify_node_editor_full_round_trip_and_render() {
    let fixture = representative_dify_fixture();
    let loaded = model::load_document_from_value(None, fixture).expect("fixture should import");
    let expected_types = model::supported_node_types()
        .iter()
        .map(|descriptor| descriptor.block_type)
        .collect::<Vec<_>>();
    assert_eq!(
        loaded.document.nodes.iter().map(|node| node.block_type.as_str()).collect::<Vec<_>>(),
        expected_types
    );

    let mut state = WorkflowState::default();
    state.apply_loaded(loaded, (1440.0, 960.0));
    for (index, block_type) in expected_types.iter().enumerate() {
        let node_id = format!("node-{index}");
        state.open_edit_node_editor(Some(&node_id)).expect("registered node editor should open");
        state.set_node_editor_title(format!("已编辑 {block_type} 中文标题"));
        assert!(state.node_editor.as_ref().is_some_and(|editor| editor.block_type == *block_type));
        if let Err(error) = state.submit_node_editor() {
            panic!(
                "{block_type} edit should submit: {error}; validation={:?}",
                state.node_editor.as_ref().map(|editor| &editor.validation)
            );
        }
    }
    data_unknown_is_preserved(&state);
    assert_eq!(state.document.edges.len(), 4, "valid branch and container edges should remain");

    let active = state.active_app().expect("loaded app should be active");
    let saved = model::serialize_workflow_yaml(
        &active.meta,
        &state.document,
        &state.environment_variables,
        &state.conversation_variables,
        &active.raw_root,
        state.document.viewport,
    )
    .expect("edited workflow should export");
    let reloaded =
        model::load_document_from_text(None, saved.clone()).expect("export should reload");
    assert_eq!(reloaded.document.nodes.len(), expected_types.len());
    assert_eq!(reloaded.document.edges.len(), 4);
    assert_eq!(reloaded.raw_root["app"]["x-regression-app"].as_str(), Some("keep"));
    assert_eq!(reloaded.raw_root["workflow"]["x-regression-workflow"].as_str(), Some("keep"));
    let resaved = model::serialize_workflow_yaml(
        &reloaded.app_meta,
        &reloaded.document,
        &reloaded.environment_variables,
        &reloaded.conversation_variables,
        &reloaded.raw_root,
        reloaded.document.viewport,
    )
    .expect("reloaded workflow should export");
    assert_eq!(
        serde_yaml::from_str::<Value>(&saved).expect("first export should parse"),
        serde_yaml::from_str::<Value>(&resaved).expect("second export should parse"),
        "export-reload-export should be structurally stable"
    );

    state.quick_insert_panel_open = true;
    state.status_message = Some("动态中文状态：全节点菜单已加载".to_string());
    assert_dark_element_draws(view::view(&state)).await;
    state.quick_insert_panel_open = false;
    state
        .open_edit_node_editor(Some("node-0"))
        .expect("Chinese start-node dialog should open for rendering");
    state.status_message = Some("动态中文状态：全节点已加载".to_string());
    assert_dark_element_draws(view::view(&state)).await;
}
