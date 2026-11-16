use crate::pet_interaction::{newest_open_thinking_text, preview_position};
use crate::pet_state::{TaskPetItem, TaskPetStatus};

#[test]
fn newest_open_thinking_text_ignores_visible_answer() {
    let messages = vec![(true, "<think>分析实现路径".to_string())];

    let detail =
        newest_open_thinking_text(messages.iter().map(|message| (message.0, message.1.as_str())));

    assert_eq!(detail, Some("分析实现路径".to_string()));
}

#[test]
fn newest_open_thinking_text_returns_none_without_open_think() {
    let messages = vec![(true, "<think>分析完成</think>\n最终回答".to_string())];

    let detail =
        newest_open_thinking_text(messages.iter().map(|message| (message.0, message.1.as_str())));

    assert_eq!(detail, None);
}

#[test]
fn newest_open_thinking_text_skips_user_and_uses_newest_open_assistant() {
    let messages = vec![
        (true, "<think>older analysis".to_string()),
        (false, "<think>user text".to_string()),
        (true, "<think>newest analysis".to_string()),
    ];

    assert_eq!(
        newest_open_thinking_text(messages.iter().map(|message| (message.0, message.1.as_str())))
            .as_deref(),
        Some("newest analysis")
    );
}

#[test]
fn mini_companion_scale_rejects_non_finite_and_bounds_values() {
    let mut state = crate::pet_state::State::new();
    assert_eq!(state.task_pet_scale, 1.0);
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(crate::pet_state::set_task_pet_scale(&mut state.task_pet_scale, invalid).is_err());
        assert_eq!(state.task_pet_scale, 1.0);
    }
    crate::pet_state::set_task_pet_scale(&mut state.task_pet_scale, 0.1).unwrap();
    assert_eq!(state.task_pet_scale, 0.5);
    crate::pet_state::set_task_pet_scale(&mut state.task_pet_scale, 8.0).unwrap();
    assert_eq!(state.task_pet_scale, 2.0);
}

fn mini_task(id: u64, status: TaskPetStatus) -> TaskPetItem {
    TaskPetItem {
        request_id: id,
        session_id: format!("session-{id}"),
        title: format!("任务 {id}"),
        detail: "正在整理上下文".into(),
        detail_feedback: None,
        project: None,
        status,
        last_click_at: None,
    }
}

#[test]
fn mini_companion_scaled_bounds_include_controls() {
    let mut state = crate::pet_state::State::new();
    for scale in [0.5, 1.0, 2.0] {
        state.task_pet_scale = scale;
        let bounds = state.interaction(&mut false, (960.0, 800.0), 0).task_pet_window_size();
        assert!(bounds.width >= 160.0);
        assert!(bounds.height >= 104.0 * scale.max(1.0) + 6.0 + 36.0 + 16.0);
    }
}

#[test]
fn mini_companion_size_preview_preserves_bottom_edge_and_card_dimensions() {
    let mut state = crate::pet_state::State::new();
    state.task_pet_position = iced::Point::new(500.0, 500.0);
    state.task_pet_items = vec![mini_task(1, TaskPetStatus::Running)];
    for progress in [0.0, 0.5, 1.0] {
        state.task_pet_expand_progress = progress;
        state.task_pet_scale = 1.0;
        let original = state.interaction(&mut false, (960.0, 800.0), 0).task_pet_window_size();
        let bottom = state.task_pet_position.y + original.height;
        state.task_pet_size_draft = Some(2.0);
        let enlarged = state.interaction(&mut false, (960.0, 800.0), 0).task_pet_window_size();
        state.task_pet_position = preview_position(state.task_pet_position, original, enlarged);
        let enlarged = state.interaction(&mut false, (960.0, 800.0), 0).task_pet_window_size();
        assert_eq!(enlarged.height - original.height, 104.0);
        assert_eq!(enlarged.width, original.width);
        assert_eq!(state.task_pet_position.y + enlarged.height, bottom);
        state.task_pet_size_draft = None;
    }
}

#[test]
fn session_title_preserves_first_match_and_empty_title_fallback() {
    use crate::pet_interaction::session_title;
    assert_eq!(
        session_title("s", "fallback", [("s", "  Session Title  ")], [("s", "project")]),
        "Session Title"
    );
    assert_eq!(session_title("s", "fallback", [("s", " ")], [("s", "project")]), "fallback");
    assert_eq!(session_title("s", "fallback", [], [("s", "project")]), "project");
}

#[test]
fn mini_companion_stack_prioritizes_running_tasks() {
    let mut state = crate::pet_state::State::new();
    state.task_pet_items = vec![mini_task(99, TaskPetStatus::Completed)];
    let snapshot = |id| {
        (
            id,
            format!("session-{id}"),
            None,
            crate::pet_interaction::TaskPetSnapshot { title: "任务".into(), detail: None },
        )
    };
    crate::pet_interaction::sync_items(
        &mut state.task_pet_items,
        [1, 3, 2].into_iter().map(snapshot).collect(),
    );
    assert_eq!(
        state.task_pet_items.iter().map(|i| i.request_id).collect::<Vec<_>>(),
        [3, 2, 1, 99]
    );
    crate::pet_interaction::sync_items(
        &mut state.task_pet_items,
        [1, 2].into_iter().map(snapshot).collect(),
    );
    assert_eq!(state.task_pet_items[0].request_id, 2);
    assert_eq!(state.task_pet_items[2].status, TaskPetStatus::Completed);
}

#[test]
fn mini_companion_dismiss_updates_top_card() {
    let mut state = crate::pet_state::State::new();
    state.task_pet_items =
        vec![mini_task(3, TaskPetStatus::Running), mini_task(2, TaskPetStatus::Completed)];
    state.interaction(&mut false, (960.0, 800.0), 0).open_task_pet_reply(3);
    state.interaction(&mut false, (960.0, 800.0), 0).update_task_pet_reply_input("草稿".into());
    state.interaction(&mut false, (960.0, 800.0), 0).dismiss_task_pet_item(3);
    assert_eq!(state.task_pet_items[0].request_id, 2);
    assert!(state.task_pet_reply_request_id.is_none());
    assert!(state.task_pet_reply_input.is_empty());
    crate::pet_interaction::sync_items(&mut state.task_pet_items, Vec::new());
    assert_eq!(state.task_pet_items[0].request_id, 2);
}

#[test]
fn mini_companion_none_keeps_controls() {
    let mut state = crate::pet_state::State::new();
    state.task_pet_avatar_kind = crate::pet_state::TaskPetAvatarKind::None;
    assert_eq!(state.interaction(&mut false, (960.0, 800.0), 0).task_pet_window_size(), iced::Size::new(160.0, 168.0));
    state.interaction(&mut false, (960.0, 800.0), 0).toggle_task_pet_collapsed();
    assert!(!state.task_pet_collapsed);
    assert_eq!(state.task_pet_avatar_kind, crate::pet_state::TaskPetAvatarKind::None);
}
