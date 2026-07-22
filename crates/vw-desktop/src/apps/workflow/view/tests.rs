//! Workflow 顶层视图测试，覆盖应用切换选项、样式分支与主要视图状态。

use super::*;
use crate::apps::workflow::model::{WorkflowAppMeta, WorkflowDocument};
use crate::apps::workflow::state::{
    WorkflowAppEditorDraft, WorkflowAppEntry, WorkflowCanvasContextMenu, WorkflowHistorySnapshot,
    WorkflowNodeEditorDraft, WorkflowNodeEditorValidation, WorkflowNodeVisualDraft,
    WorkflowRunDetailDialog, WorkflowRunInputDraft, WorkflowRunPreview, WorkflowRunPromptDraft,
    WorkflowRunStatus, WorkflowSavedAppSummary, WorkflowVariableEditorDraft,
    WorkflowVariableEditorMode,
};
use iced::widget::{button, text_editor, text_input};
use iced::{Point, Size, Theme};
use serde_yaml::Value;

#[test]
fn module_test_anchor() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}

#[test]
fn workflow_app_display_name_uses_fallback_only_for_blank_names() {
    assert_eq!(workflow_app_display_name(""), "未命名应用");
    assert_eq!(workflow_app_display_name("   "), "未命名应用");
    assert_eq!(workflow_app_display_name(" 客服助手 "), " 客服助手 ");
}

#[test]
fn workflow_app_switch_option_display_returns_the_prepared_label() {
    let saved = WorkflowAppSwitchOption {
        id: "saved".to_string(),
        label: "🤖 已保存应用".to_string(),
        source: WorkflowAppSwitchSource::SavedUuid("saved".to_string()),
    };
    let open = WorkflowAppSwitchOption {
        id: "open".to_string(),
        label: "🧪 草稿应用 *".to_string(),
        source: WorkflowAppSwitchSource::OpenApp("open".to_string()),
    };

    assert_eq!(saved.to_string(), "🤖 已保存应用");
    assert_eq!(open.to_string(), "🧪 草稿应用 *");
}

#[test]
fn workflow_app_switch_options_include_saved_apps_with_dirty_active_marker() {
    let state = WorkflowState {
        local_uuid: Some("saved-a".to_string()),
        active_is_dirty: true,
        saved_apps: vec![
            WorkflowSavedAppSummary {
                uuid: "saved-a".to_string(),
                name: "  ".to_string(),
                description: String::new(),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
            WorkflowSavedAppSummary {
                uuid: "saved-b".to_string(),
                name: "知识助手".to_string(),
                description: String::new(),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
        ],
        ..WorkflowState::default()
    };

    let options = workflow_app_switch_options(&state);

    assert_eq!(options.len(), 2);
    assert_eq!(options[0].id, "saved-a");
    assert_eq!(options[0].label, "🤖 未命名应用 *");
    assert_eq!(options[0].source, WorkflowAppSwitchSource::SavedUuid("saved-a".to_string()));
    assert_eq!(options[1].label, "🤖 知识助手");
}

#[test]
fn workflow_app_switch_options_include_only_open_apps_missing_from_saved_list() {
    let saved_uuid = "saved-a".to_string();
    let state = WorkflowState {
        active_app_id: Some("open-active".to_string()),
        source_name: "运行中应用".to_string(),
        active_is_dirty: true,
        saved_apps: vec![WorkflowSavedAppSummary {
            uuid: saved_uuid.clone(),
            name: "已保存".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }],
        apps: vec![
            test_app_entry("open-active", None, "旧名称", "🧪", false),
            test_app_entry("open-dirty", None, "草稿应用", "🧠", true),
            test_app_entry("open-saved", Some(saved_uuid), "不应重复", "🤖", true),
        ],
        ..WorkflowState::default()
    };

    let options = workflow_app_switch_options(&state);

    assert_eq!(options.len(), 3);
    assert_eq!(options[0].label, "🤖 已保存");
    assert_eq!(options[1].label, "🧪 运行中应用 *");
    assert_eq!(options[1].source, WorkflowAppSwitchSource::OpenApp("open-active".to_string()));
    assert_eq!(options[2].label, "🧠 草稿应用 *");
}

#[test]
fn build_app_switcher_handles_empty_saved_and_open_apps() {
    let empty_state = WorkflowState::default();
    let _ = build_app_switcher(&empty_state);

    let saved_state = WorkflowState {
        local_uuid: Some("saved".to_string()),
        saved_apps: vec![WorkflowSavedAppSummary {
            uuid: "saved".to_string(),
            name: "已保存应用".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }],
        ..WorkflowState::default()
    };
    let _ = build_app_switcher(&saved_state);

    let open_state = WorkflowState {
        active_app_id: Some("open".to_string()),
        apps: vec![test_app_entry("open", None, "打开应用", "🤖", false)],
        ..WorkflowState::default()
    };
    let _ = build_app_switcher(&open_state);
}

#[test]
fn build_app_switcher_prefers_saved_selection_when_saved_uuid_is_active() {
    let state = WorkflowState {
        local_uuid: Some("saved".to_string()),
        active_app_id: Some("open".to_string()),
        saved_apps: vec![WorkflowSavedAppSummary {
            uuid: "saved".to_string(),
            name: "已保存应用".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }],
        apps: vec![test_app_entry("open", None, "打开应用", "🧪", false)],
        ..WorkflowState::default()
    };
    let options = workflow_app_switch_options(&state);

    assert!(matches!(options[0].source, WorkflowAppSwitchSource::SavedUuid(_)));
    assert!(matches!(options[1].source, WorkflowAppSwitchSource::OpenApp(_)));
    let _ = build_app_switcher(&state);
}

#[test]
fn style_helpers_cover_light_dark_and_input_statuses() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_eq!(is_dark_theme(&theme), matches!(theme, Theme::Dark));

        let _ = editor_style(&theme, text_editor::Status::Active);
        let _ = editor_style(&theme, text_editor::Status::Hovered);
        let description_focused = node_editor_description_style(
            &theme,
            text_editor::Status::Focused { is_hovered: true },
        );
        assert!(description_focused.border.width > 0.0);

        let title_active = node_editor_title_input_style(&theme, text_input::Status::Active);
        let title_hovered = node_editor_title_input_style(&theme, text_input::Status::Hovered);
        let title_focused = node_editor_title_input_style(
            &theme,
            text_input::Status::Focused { is_hovered: false },
        );
        assert_eq!(title_active.border.width, 1.0);
        assert_eq!(title_hovered.border.width, 1.0);
        assert_eq!(title_focused.border.width, 1.0);

        let jump_active = next_step_jump_card_style(&theme, button::Status::Active);
        let jump_hovered = next_step_jump_card_style(&theme, button::Status::Hovered);
        let jump_pressed = next_step_jump_card_style(&theme, button::Status::Pressed);
        assert!(jump_active.background.is_some());
        assert!(jump_hovered.background.is_some());
        assert!(jump_pressed.background.is_some());
    }
}

#[test]
fn view_builds_saved_app_layers_without_active_app() {
    let mut state = WorkflowState {
        saved_apps: vec![WorkflowSavedAppSummary {
            uuid: "saved".to_string(),
            name: "保存的应用".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }],
        error_message: Some("加载失败".to_string()),
        confirm_delete_saved_app_uuid: Some("saved".to_string()),
        ..WorkflowState::default()
    };

    let _ = view(&state);

    state.saved_app_run_preview = Some(WorkflowRunPreview {
        uuid: "saved".to_string(),
        run_id: Some("run-1".to_string()),
        status: WorkflowRunStatus::Running,
        nodes: Vec::new(),
        text_output: "运行输出".to_string(),
        error: None,
        is_finished: false,
    });

    let _ = view(&state);

    state.app_editor = Some(WorkflowAppEditorDraft {
        mode: WorkflowAppEditorMode::Create,
        name: "新应用".to_string(),
        description: String::new(),
        icon: "🤖".to_string(),
        use_icon_as_answer_icon: false,
        max_active_requests_input: "0".to_string(),
    });

    let _ = view(&state);
}

#[test]
fn view_builds_active_app_layers_for_empty_and_node_documents() {
    let app = test_app_entry("active", None, "打开应用", "🤖", false);
    let mut state = WorkflowState {
        apps: vec![app],
        active_app_id: Some("active".to_string()),
        source_name: "打开应用".to_string(),
        zoom: 1.0,
        status_message: Some("已加载".to_string()),
        error_message: Some("保存失败".to_string()),
        quick_insert_panel_open: true,
        action_menu_open: true,
        zoom_menu_open: true,
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Canvas,
            anchor: Point::new(12.0, 16.0),
            world: Point::new(40.0, 50.0),
        }),
        ..WorkflowState::default()
    };

    let _ = view(&state);

    state.document.nodes.push(test_node("start", "start"));
    state.selected_node_id = Some("start".to_string());
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Node("start".to_string()),
        anchor: Point::new(18.0, 24.0),
        world: Point::new(60.0, 72.0),
    });
    state.variable_panel = Some(WorkflowVariablePanelKind::System);

    let _ = view(&state);
}

#[test]
fn view_builds_saved_app_run_prompt_detail_and_preview_with_copied_error() {
    let mut state = WorkflowState {
        saved_apps: vec![WorkflowSavedAppSummary {
            uuid: "saved".to_string(),
            name: "保存的应用".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }],
        run_error_copied: true,
        saved_app_run_prompt: Some(WorkflowRunPromptDraft {
            uuid: "saved".to_string(),
            query: "运行查询".to_string(),
            loading: false,
            inputs: vec![WorkflowRunInputDraft {
                name: "input_a".to_string(),
                label: "输入A".to_string(),
                input_type: "text-input".to_string(),
                required: true,
                hidden: false,
                options: Vec::new(),
                value: "默认值".to_string(),
                placeholder: "请输入".to_string(),
                hint: "提示".to_string(),
                max_length_input: "48".to_string(),
            }],
            error: Some("参数缺失".to_string()),
        }),
        saved_app_run_detail: Some(WorkflowRunDetailDialog {
            title: "运行输出".to_string(),
            content: text_editor::Content::with_text("详细文本"),
        }),
        saved_app_run_preview: Some(WorkflowRunPreview {
            uuid: "saved".to_string(),
            run_id: Some("run-1".to_string()),
            status: WorkflowRunStatus::Failed,
            nodes: Vec::new(),
            text_output: "运行输出".to_string(),
            error: None,
            is_finished: true,
        }),
        app_editor: Some(WorkflowAppEditorDraft {
            mode: WorkflowAppEditorMode::Edit("saved".to_string()),
            name: "新应用".to_string(),
            description: String::new(),
            icon: "🤖".to_string(),
            use_icon_as_answer_icon: false,
            max_active_requests_input: "0".to_string(),
        }),
        ..WorkflowState::default()
    };

    // active_app_id 为空，命中已保存应用分支并叠加全部弹层。
    assert!(state.active_app_id.is_none());
    let _ = view(&state);

    // run_error_copied 为 false 时仍构建预览层，覆盖另一分支。
    state.run_error_copied = false;
    let _ = view(&state);
}

#[test]
fn view_builds_active_app_node_and_variable_editor_layers() {
    let app = test_app_entry("active", None, "打开应用", "🤖", false);
    let mut state = WorkflowState {
        apps: vec![app],
        active_app_id: Some("active".to_string()),
        source_name: "打开应用".to_string(),
        zoom: 1.0,
        node_editor: Some(test_node_editor("answer", None)),
        variable_panel: Some(WorkflowVariablePanelKind::Environment),
        variable_editor: Some(WorkflowVariableEditorDraft {
            mode: WorkflowVariableEditorMode::CreateEnvironment,
            name: "env".to_string(),
            description: String::new(),
            value_type: "string".to_string(),
            raw_value_editor: text_editor::Content::with_text("value"),
        }),
        ..WorkflowState::default()
    };
    state.document.nodes.push(test_node("answer", "answer"));

    let _ = view(&state);
}

#[test]
fn view_builds_start_variable_editor_layer() {
    let app = test_app_entry("active", None, "打开应用", "🤖", false);
    let mut state = WorkflowState {
        apps: vec![app],
        active_app_id: Some("active".to_string()),
        source_name: "打开应用".to_string(),
        zoom: 1.0,
        node_editor: Some(test_node_editor(
            "start",
            Some(WorkflowNodeVisualDraft::Start { variables: Vec::new() }),
        )),
        ..WorkflowState::default()
    };
    state.document.nodes.push(test_node("start", "start"));

    // 通过状态方法打开 start 变量编辑器，命中 view 中对应弹层分支。
    state.open_node_editor_start_variable_create();
    assert!(
        state
            .node_editor
            .as_ref()
            .and_then(|editor| editor.start_variable_editor.as_ref())
            .is_some()
    );

    let _ = view(&state);
}

fn test_node_editor(
    block_type: &str,
    visual_draft: Option<WorkflowNodeVisualDraft>,
) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Edit(block_type.to_string()),
        active_tab: WorkflowNodeEditorTab::Description,
        block_type: block_type.to_string(),
        title: block_type.to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft,
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text(""),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}

fn test_app_entry(
    id: &str,
    local_uuid: Option<String>,
    name: &str,
    icon: &str,
    is_dirty: bool,
) -> WorkflowAppEntry {
    let document = WorkflowDocument::default();
    let snapshot = WorkflowHistorySnapshot {
        meta: WorkflowAppMeta {
            name: name.to_string(),
            icon: icon.to_string(),
            ..WorkflowAppMeta::default()
        },
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    };

    WorkflowAppEntry {
        id: id.to_string(),
        local_uuid,
        meta: snapshot.meta.clone(),
        source_path: None,
        raw_root: Value::Null,
        document,
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
        connection_draft: None,
        is_dirty,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        saved_snapshot: snapshot,
    }
}

fn test_node(id: &str, block_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: id.to_string(),
        description: String::new(),
        position: Point::new(0.0, 0.0),
        size: Size::new(180.0, 120.0),
        parent_id: None,
        selected: false,
        source_side: super::super::model::WorkflowHandleSide::Right,
        target_side: super::super::model::WorkflowHandleSide::Left,
        source_handles: Vec::new(),
        target_handles: Vec::new(),
        z_index: 0.0,
        raw_node: Value::Null,
    }
}
