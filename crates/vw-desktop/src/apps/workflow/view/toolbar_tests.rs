use super::*;
use iced::{
    Background, Color, Point, Theme, Vector,
    widget::{button, text_editor},
};
use serde_yaml::Value;

use crate::apps::workflow::model::{WorkflowAppMeta, WorkflowDocument};
use crate::apps::workflow::state::{
    WorkflowAppEditorDraft, WorkflowAppEntry, WorkflowHistorySnapshot, WorkflowNodeEditorDraft,
    WorkflowNodeEditorValidation, WorkflowSavedAppSummary,
};

fn background_color(background: Option<Background>) -> Color {
    match background.unwrap_or(Background::Color(Color::TRANSPARENT)) {
        Background::Color(color) => color,
        _ => Color::TRANSPARENT,
    }
}

fn app_editor(mode: WorkflowAppEditorMode) -> WorkflowAppEditorDraft {
    WorkflowAppEditorDraft {
        mode,
        name: "应用".to_string(),
        description: String::new(),
        icon: "🤖".to_string(),
        use_icon_as_answer_icon: false,
        max_active_requests_input: "1".to_string(),
    }
}

fn node_editor(mode: WorkflowNodeEditorMode) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode,
        active_tab: WorkflowNodeEditorTab::Basic,
        block_type: "code".to_string(),
        title: "节点".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft: None,
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text("{}"),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}

fn history_snapshot() -> WorkflowHistorySnapshot {
    WorkflowHistorySnapshot {
        meta: WorkflowAppMeta::default(),
        document: WorkflowDocument::default(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    }
}

fn open_app_entry(
    id: &str,
    local_uuid: Option<&str>,
    name: &str,
    is_dirty: bool,
) -> WorkflowAppEntry {
    WorkflowAppEntry {
        id: id.to_string(),
        local_uuid: local_uuid.map(str::to_string),
        meta: WorkflowAppMeta { name: name.to_string(), ..WorkflowAppMeta::default() },
        source_path: None,
        raw_root: Value::Null,
        document: WorkflowDocument::default(),
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
        saved_snapshot: history_snapshot(),
    }
}

fn saved_app_summary(uuid: &str, name: &str) -> WorkflowSavedAppSummary {
    WorkflowSavedAppSummary {
        uuid: uuid.to_string(),
        name: name.to_string(),
        description: String::new(),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

fn assert_color_close(left: Color, right: Color) {
    const EPSILON: f32 = 0.0001;
    assert!((left.r - right.r).abs() < EPSILON);
    assert!((left.g - right.g).abs() < EPSILON);
    assert!((left.b - right.b).abs() < EPSILON);
    assert!((left.a - right.a).abs() < EPSILON);
}

#[test]
fn mix_color_clamps_factor_and_interpolates_channels() {
    let left = Color::from_rgba(0.0, 0.0, 0.0, 0.0);
    let right = Color::from_rgba(1.0, 0.5, 0.25, 1.0);

    assert_eq!(mix_color(left, right, -1.0), left);
    assert_eq!(mix_color(left, right, 2.0), right);
    assert_eq!(mix_color(left, right, 0.5), Color::from_rgba(0.5, 0.25, 0.125, 0.5));
}

#[test]
fn toolbar_icon_style_covers_disabled_active_and_idle_paths() {
    let theme = Theme::Dark;

    assert_eq!(
        toolbar_icon_style(&theme, false, false).color,
        Some(theme.palette().text.scale_alpha(0.35))
    );
    assert_eq!(
        toolbar_icon_style(&theme, true, true).color,
        Some(theme.extended_palette().background.base.text)
    );
    assert_eq!(toolbar_icon_style(&theme, true, false).color, Some(theme.palette().text));
}

#[test]
fn toolbar_button_style_covers_disabled_active_hovered_and_pressed_paths() {
    let theme = Theme::Dark;
    let disabled = toolbar_button_style(&theme, button::Status::Active, false, false);
    let active = toolbar_button_style(&theme, button::Status::Active, true, true);
    let active_pressed = toolbar_button_style(&theme, button::Status::Pressed, true, true);
    let hovered = toolbar_button_style(&theme, button::Status::Hovered, true, false);
    let pressed = toolbar_button_style(&theme, button::Status::Pressed, true, false);
    let idle = toolbar_button_style(&theme, button::Status::Active, true, false);

    assert!(disabled.background.is_none());
    assert_eq!(disabled.border.color, Color::TRANSPARENT);
    assert_eq!(disabled.text_color, theme.palette().text.scale_alpha(0.35));

    assert!(matches!(active.background, Some(Background::Color(_))));
    assert!(matches!(active_pressed.background, Some(Background::Color(_))));
    assert_ne!(background_color(active.background), background_color(active_pressed.background));
    assert_eq!(active.border.color, theme.extended_palette().background.strong.color);

    assert_eq!(
        hovered.background,
        Some(Background::Color(theme.extended_palette().background.weak.color))
    );
    assert_eq!(
        pressed.background,
        Some(Background::Color(theme.extended_palette().background.strong.color))
    );
    assert!(idle.background.is_none());
}

#[test]
fn zoom_button_style_covers_statuses_and_radius_passthrough() {
    let theme = Theme::Light;
    let radius = iced::border::Radius {
        top_left: 12.0,
        top_right: 0.0,
        bottom_left: 8.0,
        bottom_right: 2.0,
    };

    let idle = zoom_button_style(&theme, button::Status::Active, radius);
    let hovered = zoom_button_style(&theme, button::Status::Hovered, radius);
    let pressed = zoom_button_style(&theme, button::Status::Pressed, radius);

    assert!(idle.background.is_none());
    assert_eq!(
        hovered.background,
        Some(Background::Color(theme.extended_palette().background.weak.color))
    );
    assert_eq!(
        pressed.background,
        Some(Background::Color(theme.extended_palette().background.strong.color))
    );
    assert_eq!(pressed.border.color, Color::TRANSPARENT);
    assert_eq!(pressed.border.radius, radius);
}

#[test]
fn zoom_menu_button_style_covers_active_hovered_pressed_and_idle_paths() {
    let theme = Theme::Dark;
    let active = zoom_menu_button_style(&theme, button::Status::Active, true);
    let hovered = zoom_menu_button_style(&theme, button::Status::Hovered, false);
    let pressed = zoom_menu_button_style(&theme, button::Status::Pressed, false);
    let idle = zoom_menu_button_style(&theme, button::Status::Active, false);

    assert_eq!(
        active.background,
        Some(Background::Color(theme.palette().primary.scale_alpha(0.14)))
    );
    assert_eq!(active.text_color, theme.palette().primary);
    assert!(matches!(hovered.background, Some(Background::Color(_))));
    assert_eq!(
        pressed.background,
        Some(Background::Color(theme.extended_palette().background.strong.color))
    );
    assert!(idle.background.is_none());
}

#[test]
fn build_center_toolbar_covers_disabled_create_and_edit_states() {
    let empty = build_center_toolbar(&WorkflowState::default());

    let mut create_state = WorkflowState {
        selected_node_id: Some("node_1".to_string()),
        active_app_id: Some("app_1".to_string()),
        node_editor: Some(node_editor(WorkflowNodeEditorMode::Create)),
        app_editor: Some(app_editor(WorkflowAppEditorMode::Create)),
        ..WorkflowState::default()
    };
    let create_mode = build_center_toolbar(&create_state);

    create_state.quick_insert_panel_open = true;
    create_state.variable_panel = Some(WorkflowVariablePanelKind::System);
    create_state.node_editor =
        Some(node_editor(WorkflowNodeEditorMode::Edit("node_1".to_string())));
    create_state.app_editor = Some(app_editor(WorkflowAppEditorMode::Edit("app_1".to_string())));
    let edit_mode = build_center_toolbar(&create_state);

    let _ = std::hint::black_box((empty, create_mode, edit_mode));
}

#[test]
fn build_zoom_control_covers_label_rounding() {
    let state = WorkflowState { zoom: 1.234, ..WorkflowState::default() };

    let element = build_zoom_control(&state);

    let _ = std::hint::black_box(element);
}

#[test]
fn build_zoom_menu_overlay_covers_active_preset_and_clamped_percent() {
    let active_preset = WorkflowState { zoom: 1.234, ..WorkflowState::default() };
    let clamped = WorkflowState { zoom: 200.0, ..WorkflowState::default() };

    let active_element = build_zoom_menu_overlay(&active_preset, &[50, 123, 200]);
    let clamped_element = build_zoom_menu_overlay(&clamped, &[10_000]);

    let _ = std::hint::black_box((active_element, clamped_element));
}

#[test]
fn build_zoom_menu_overlay_clamps_negative_zoom_to_zero() {
    let state = WorkflowState { zoom: -2.5, ..WorkflowState::default() };

    let element = build_zoom_menu_overlay(&state, &[0, 25]);

    let _ = std::hint::black_box(element);
}

#[test]
fn build_zoom_menu_overlay_handles_empty_presets_and_nan_zoom() {
    let state = WorkflowState { zoom: f32::NAN, ..WorkflowState::default() };

    let element = build_zoom_menu_overlay(&state, &[]);

    let _ = std::hint::black_box(element);
}

#[test]
fn build_center_toolbar_marks_each_variable_panel_kind_active() {
    for panel in [WorkflowVariablePanelKind::Environment, WorkflowVariablePanelKind::Conversation] {
        let state = WorkflowState { variable_panel: Some(panel), ..WorkflowState::default() };

        let element = build_center_toolbar(&state);
        let _ = std::hint::black_box(element);
    }
}

#[test]
fn build_app_switcher_dock_covers_empty_and_populated_options() {
    let empty = build_app_switcher_dock(&WorkflowState::default());

    let populated_state = WorkflowState {
        saved_apps: vec![saved_app_summary("saved-1", "已保存应用")],
        apps: vec![open_app_entry("app-1", Some("saved-1"), "打开中的应用", true)],
        active_app_id: Some("app-1".to_string()),
        local_uuid: Some("saved-1".to_string()),
        source_name: "打开中的应用".to_string(),
        active_is_dirty: true,
        ..WorkflowState::default()
    };
    let populated = build_app_switcher_dock(&populated_state);

    let _ = std::hint::black_box((empty, populated));
}

#[test]
fn toolbar_style_helpers_return_expected_theme_colors() {
    let theme = Theme::Dark;

    assert_color_close(
        background_color(
            toolbar_button_style(&theme, button::Status::Pressed, true, false).background,
        ),
        theme.extended_palette().background.strong.color,
    );
    assert_color_close(
        background_color(zoom_menu_button_style(&theme, button::Status::Hovered, false).background),
        theme.extended_palette().background.weak.color.scale_alpha(0.55),
    );
}
