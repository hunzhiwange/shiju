use super::ChatMessage;
use crate::app::TodoPanelPlacement;
use crate::app::components::chat_panel::message_measure::ChatMessageMeasurement;
use crate::app::components::chat_panel::virtualizer::{ChatScrollMode, ChatVirtualizer};
use crate::app::models::{ChatMessage as ChatEntry, ChatRole};

fn chat_entry(role: ChatRole, content: &str) -> ChatEntry {
    ChatEntry { role, content: content.to_string(), think_timing: Vec::new() }
}

#[test]
fn input_changed_updates_legacy_input_text() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::InputChanged("hello".to_string()));

    assert_eq!(app.input_text, "hello");
}

#[test]
fn scroll_changed_clamps_offsets_and_updates_autoscroll() {
    let (mut app, _) = crate::app::App::new();
    app.chat_auto_scroll = false;

    let _ =
        super::update(&mut app, ChatMessage::ScrollChanged { offset_y: 1.5, viewport_h: -10.0 });

    assert_eq!(app.chat_scroll_offset_y, 0.0);
    assert_eq!(app.chat_scroll_viewport_h, 0.0);
    assert!(app.chat_auto_scroll);
}

#[test]
fn chat_scroll_semantics_toggle_think_preserves_manual_anchor() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![
        chat_entry(ChatRole::User, "question"),
        chat_entry(ChatRole::Assistant, "<think>hidden reasoning</think>\nanswer"),
        chat_entry(ChatRole::User, "follow-up"),
        chat_entry(ChatRole::Assistant, "later answer"),
    ];
    app.chat_auto_scroll = false;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[90.0, 130.0, 90.0, 130.0]);
    app.chat_virtualizer.set_viewport_height(180.0);
    app.chat_virtualizer.set_scroll_top_px(105.0);
    app.sync_chat_virtualizer_compat_fields();
    let before_scroll_top = app.chat_virtualizer.scroll_top_px();

    let _task = super::update(&mut app, ChatMessage::ToggleThink(1, 0, false));

    assert!(!app.chat_auto_scroll);
    assert_eq!(app.chat_virtualizer.scroll_mode(), ChatScrollMode::Anchored);
    assert_eq!(app.chat_virtualizer.scroll_top_px(), before_scroll_top);
    assert!(app.chat_think_expanded.contains(&((1_u64 << 32) | 0)));
}

#[test]
fn fullscreen_toggles_clear_competing_git_fullscreen_modes() {
    let (mut app, _) = crate::app::App::new();
    app.git_diff_fullscreen = true;
    app.git_diff_half_fullscreen = true;

    let _ = super::update(&mut app, ChatMessage::ToggleFullscreen);

    assert!(app.chat_panel_fullscreen);
    assert!(!app.chat_panel_half_fullscreen);
    assert!(!app.git_diff_fullscreen);
    assert!(!app.git_diff_half_fullscreen);
    assert!(app.fullscreen_layout_settling);
}

#[test]
fn half_fullscreen_toggles_clear_fullscreen_mode() {
    let (mut app, _) = crate::app::App::new();
    app.chat_panel_fullscreen = true;

    let _ = super::update(&mut app, ChatMessage::ToggleHalfFullscreen);

    assert!(app.chat_panel_half_fullscreen);
    assert!(!app.chat_panel_fullscreen);
    assert!(app.fullscreen_layout_settling);
}

#[test]
fn hover_messages_set_and_clear_state() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::ThinkHover(2, 3));
    assert_eq!(app.chat_think_hovered_idx, Some((2_u64 << 32) | 3));
    let _ = super::update(&mut app, ChatMessage::ThinkHoverLeave);
    assert_eq!(app.chat_think_hovered_idx, None);

    let _ = super::update(&mut app, ChatMessage::ToolHover(4, 5));
    assert_eq!(app.chat_tool_hovered_idx, Some((4_u64 << 32) | 5));
    app.chat_tool_file_hovered = Some("file".to_string());
    let _ = super::update(&mut app, ChatMessage::ToolHoverLeave);
    assert_eq!(app.chat_tool_hovered_idx, None);
    assert_eq!(app.chat_tool_file_hovered, None);
}

#[test]
fn toggle_sets_add_and_remove_keys() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::ToggleToolFile(1, 2, "src/main.rs".to_string()));
    assert!(app.chat_tool_file_expanded.contains("1:2:src/main.rs"));
    let _ = super::update(&mut app, ChatMessage::ToggleToolFile(1, 2, "src/main.rs".to_string()));
    assert!(!app.chat_tool_file_expanded.contains("1:2:src/main.rs"));

    let _ = super::update(&mut app, ChatMessage::ToggleTool(3, 4));
    assert!(app.chat_tool_expanded.contains(&((3_u64 << 32) | 4)));
    let _ = super::update(&mut app, ChatMessage::ToggleTool(3, 4));
    assert!(!app.chat_tool_expanded.contains(&((3_u64 << 32) | 4)));

    let _ = super::update(&mut app, ChatMessage::ToggleExploreSummary(5, 6));
    assert!(app.chat_explore_expanded.contains(&((5_u64 << 32) | 6)));
    let _ = super::update(&mut app, ChatMessage::ToggleExploreSummary(5, 6));
    assert!(!app.chat_explore_expanded.contains(&((5_u64 << 32) | 6)));
}

#[test]
fn todo_panel_messages_update_state_and_animation() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(
        &mut app,
        ChatMessage::SetTodoPanelPlacement(TodoPanelPlacement::ChatTopRight),
    );
    assert_eq!(app.chat_todo_placement, TodoPanelPlacement::ChatTopRight);
    assert!(app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 1.0);

    let _ = super::update(&mut app, ChatMessage::ToggleTodoPanel);
    assert!(!app.chat_todo_expanded);
    let _ = super::update(&mut app, ChatMessage::TodoAnimTick);
    assert!(app.chat_todo_anim < 1.0);
}

#[test]
fn simple_input_flags_update_directly() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::InputAreaDragHoverChanged(true));
    assert!(app.input_drop_hovered);
    let _ = super::update(&mut app, ChatMessage::FileReferenceHoverChanged(Some(2)));
    assert_eq!(app.file_ref_hovered_index, Some(2));
    let _ = super::update(&mut app, ChatMessage::ToolFilesFilterChanged("rs".to_string()));
    assert_eq!(app.tool_files_filter, "rs");
    let _ = super::update(&mut app, ChatMessage::FullscreenOverlayEntered);
    assert!(app.show_chat_fullscreen_overlay);
    let _ = super::update(&mut app, ChatMessage::FullscreenOverlayExited);
    assert!(!app.show_chat_fullscreen_overlay);
}

#[test]
fn toggle_think_resets_existing_override_to_default_state() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "reasoning")];
    app.chat_auto_scroll = false;
    let key = 11;
    app.chat_think_collapsed.insert(key);

    let _ = super::update(&mut app, ChatMessage::ToggleThink(0, 11, true));

    assert!(!app.chat_think_collapsed.contains(&key));
    assert!(!app.chat_think_expanded.contains(&key));
}

#[test]
fn tool_file_hover_messages_set_and_clear_path() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::ToolFileHover("src/lib.rs".to_string()));
    assert_eq!(app.chat_tool_file_hovered.as_deref(), Some("src/lib.rs"));

    let _ = super::update(&mut app, ChatMessage::ToolFileHoverLeave);
    assert_eq!(app.chat_tool_file_hovered, None);
}

#[test]
fn todo_animation_tick_snaps_both_directions_at_completion_threshold() {
    let (mut app, _) = crate::app::App::new();
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.001;

    let _ = super::update(&mut app, ChatMessage::TodoAnimTick);
    assert_eq!(app.chat_todo_anim, 0.0);

    app.chat_todo_expanded = true;
    app.chat_todo_anim = 0.999;
    let _ = super::update(&mut app, ChatMessage::TodoAnimTick);
    assert_eq!(app.chat_todo_anim, 1.0);
}

#[test]
fn unchanged_scroll_geometry_reports_no_viewport_or_offset_change() {
    let (mut app, _) = crate::app::App::new();
    app.chat_auto_scroll = false;

    let outcome = super::input::apply_chat_scroll_changed(&mut app, 0.0, 0.0);

    assert!(!outcome.viewport_or_offset_changed);
    assert!(outcome.pending_chunk_starts.is_empty());
    assert!(outcome.measure_visible_messages);
}

#[test]
fn scroll_autoscroll_threshold_is_relaxed_during_hold_window() {
    let (mut held_app, _) = crate::app::App::new();
    held_app.chat_auto_scroll = true;
    held_app.chat_autoscroll_hold_until_ms = u64::MAX;

    let _ = super::input::apply_chat_scroll_changed(&mut held_app, 0.85, 100.0);
    assert!(held_app.chat_auto_scroll);

    let (mut regular_app, _) = crate::app::App::new();
    regular_app.chat_auto_scroll = true;
    regular_app.chat_autoscroll_hold_until_ms = 0;

    let _ = super::input::apply_chat_scroll_changed(&mut regular_app, 0.85, 100.0);
    assert!(!regular_app.chat_auto_scroll);
}

#[test]
fn manual_scroll_requires_tail_threshold_to_resume_autoscroll() {
    let (mut app, _) = crate::app::App::new();
    app.chat_auto_scroll = false;

    let _ = super::input::apply_chat_scroll_changed(&mut app, 0.974, 100.0);
    assert!(!app.chat_auto_scroll);

    let _ = super::input::apply_chat_scroll_changed(&mut app, 0.975, 100.0);
    assert!(app.chat_auto_scroll);
}

#[test]
fn message_measurements_ignore_empty_invalid_and_out_of_range_values() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0]);

    assert!(!super::input::apply_chat_message_measurements(&mut app, Vec::new()));
    assert!(!super::input::apply_chat_message_measurements(
        &mut app,
        vec![
            ChatMessageMeasurement { message_idx: 0, y: 0.0, height: f32::NAN },
            ChatMessageMeasurement { message_idx: 1, y: 0.0, height: 120.0 },
        ],
    ));
    assert!(app.chat_message_measured_heights.is_empty());
}

#[test]
fn message_measurements_clamp_negative_height_and_sync_virtualizer_length() {
    let (mut app, _) = crate::app::App::new();
    app.chat =
        vec![chat_entry(ChatRole::User, "question"), chat_entry(ChatRole::Assistant, "answer")];
    assert_eq!(app.chat_virtualizer.len(), 0);

    let changed = super::input::apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 1, y: 0.0, height: -12.0 }],
    );

    assert!(changed);
    assert_eq!(app.chat_virtualizer.len(), app.chat.len());
    assert_eq!(app.chat_message_measured_heights.get(&1), Some(&0.0));
}

#[test]
fn message_measurements_ignore_epsilon_changes_and_apply_larger_updates() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0]);

    assert!(!super::input::apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 0.0, height: 80.5 }],
    ));
    assert!(app.chat_message_measured_heights.is_empty());

    assert!(super::input::apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 0.0, height: 82.0 }],
    ));
    assert_eq!(app.chat_message_measured_heights.get(&0), Some(&82.0));

    assert!(!super::input::apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 0.0, height: 82.0 }],
    ));
}

#[test]
fn visible_message_measurements_message_updates_virtualizer_when_height_changes() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0]);

    let _ = super::update(&mut app, ChatMessage::VisibleMessageMeasurements(Vec::new()));
    assert!(app.chat_message_measured_heights.is_empty());

    let _ = super::update(
        &mut app,
        ChatMessage::VisibleMessageMeasurements(vec![ChatMessageMeasurement {
            message_idx: 0,
            y: 0.0,
            height: 104.0,
        }]),
    );

    assert_eq!(app.chat_message_measured_heights.get(&0), Some(&104.0));
}

#[test]
fn wasm_focus_message_is_noop_on_native_targets() {
    let (mut app, _) = crate::app::App::new();
    app.input_text = "unchanged".to_string();

    let _ = super::update(&mut app, ChatMessage::WasmImeFocus);

    assert_eq!(app.input_text, "unchanged");
}
