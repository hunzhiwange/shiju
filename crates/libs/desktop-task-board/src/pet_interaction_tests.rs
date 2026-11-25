use super::*;

#[test]
fn task_pet_title_uses_first_non_empty_line() {
    let title = task_pet_title("\n\n  修复网关重启配置丢失\n后续内容");

    assert_eq!(title, "修复网关重启配置丢失");
}

#[test]
fn truncate_for_pet_marks_long_text() {
    let value = truncate_for_pet("abcdefghijklmnopqrstuvwxyz", 5);

    assert_eq!(value, "abcde...");
}

#[test]
fn truncate_for_pet_preserves_unicode_at_character_boundary() {
    assert_eq!(truncate_for_pet("你好世界", 3), "你好世...");
    assert_eq!(truncate_for_pet("你好世界", 4), "你好世界");
}

#[test]
fn task_pet_detail_compacts_followup_lines_and_defaults_blank_input() {
    assert_eq!(
        task_pet_compact_detail(" heading \n detail one \n\n detail two "),
        "detail one detail two"
    );
    assert_eq!(task_pet_detail(" heading \n detail one \n detail two "), "detail one detail two");
    assert_eq!(task_pet_detail(" \n\t "), "思考中...");
}

#[test]
fn task_pet_project_label_rejects_blank_and_truncates_leaf_name() {
    assert_eq!(task_pet_project_label(None), None);
    assert_eq!(task_pet_project_label(Some(" \n ")), None);
    assert_eq!(task_pet_project_label(Some(" project-name ")).as_deref(), Some("project-name"));
    assert_eq!(
        task_pet_project_label(Some("abcdefghijklmnopqrs")).as_deref(),
        Some("abcdefghijklmnopqr...")
    );
}

#[test]
fn find_tool_start_requires_tool_at_line_start() {
    assert_eq!(find_tool_start("tool search"), Some(0));
    assert_eq!(find_tool_start("analysis\ntool search"), Some(9));
    assert_eq!(find_tool_start("analysis tool search"), None);
}

#[test]
fn think_tag_suffix_accepts_only_empty_or_whitespace_prefix() {
    assert!(valid_think_tag_suffix(""));
    assert!(valid_think_tag_suffix(" reason=\"work\""));
    assert!(!valid_think_tag_suffix("ing"));
}

#[test]
fn current_open_think_body_uses_new_empty_think() {
    let body = current_open_think_body("<think>旧思考</think>\n普通回复\n<think>");

    assert_eq!(body, Some(String::new()));
}

#[test]
fn current_open_think_body_stops_before_tool_output() {
    let body = current_open_think_body("<think>inspect state\ntool search\nresult");

    assert_eq!(body.as_deref(), Some("inspect state"));
}

#[test]
fn task_pet_dock_keeps_native_frame_until_collapse_settles() {
    use crate::pet_view::{DockMode, PetControls};
    use iced::time::{Duration, Instant};
    let mut controls = PetControls::default();
    let now = Instant::now();
    let settled = Duration::from_millis(220);
    controls.composer = true;
    controls.update(None, 0.0, now);
    let wide = controls.window_size(104.0, 0.0, false, now);
    controls.native_size = Some(wide);
    let midway = controls.view(None, now + settled / 2);
    assert_eq!(midway.mode, DockMode::Composer);
    assert!(midway.width > DockMode::Idle.size().width);
    assert!(midway.width < DockMode::Composer.size().width);

    let close = now + settled;
    controls.composer = false;
    controls.update(None, 0.0, close);
    assert_eq!(controls.window_size(104.0, 0.0, false, close), wide);
    assert!(controls.window_size(104.0, 0.0, false, close + settled).width < wide.width);
}

#[test]
fn task_pet_avatar_next_cycles_all_variants() {
    assert_eq!(TaskPetAvatarKind::Robot.next(), TaskPetAvatarKind::Beauty);
    assert_eq!(TaskPetAvatarKind::Beauty.next(), TaskPetAvatarKind::Handsome);
    assert_eq!(TaskPetAvatarKind::Handsome.next(), TaskPetAvatarKind::Robot);
}
