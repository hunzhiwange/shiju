use super::*;

fn key(composer: &mut ChatComposer, code: KeyCode) -> V3Action {
    composer.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE))
}

fn search(composer: &mut ChatComposer) {
    assert!(matches!(composer.handle_key_event(KeyEvent::new(
        KeyCode::Char('r'), KeyModifiers::CONTROL,
    )), V3Action::None));
}

fn history() -> ChatComposer {
    let mut composer = ChatComposer::new();
    for text in ["older 中文", "latest 中文", "other"] {
        composer.replace_text(text);
        composer.submit_or_queue(false);
    }
    composer
}

#[test]
fn terminal_req_08_ctrl_r_repeats_older_matches() {
    let mut composer = history();
    search(&mut composer);
    composer.handle_paste("中文".into());
    assert_eq!(composer.text(), "latest 中文");
    search(&mut composer);
    assert_eq!(composer.text(), "older 中文");
}

#[test]
fn terminal_req_08_enter_accepts_without_sending_or_queueing() {
    let mut composer = history();
    composer.set_task_running(true);
    search(&mut composer);
    assert!(matches!(key(&mut composer, KeyCode::Enter), V3Action::None));
    assert_eq!(composer.text(), "other");
    assert!(composer.history_search.is_none());
}

#[test]
fn terminal_req_08_escape_restores_rich_draft_and_cursor() {
    let mut composer = history();
    composer.replace_text("草稿😀suffix");
    composer.textarea.set_cursor("草稿".len());
    composer.handle_paste("长".repeat(1001));
    let attachment = V3Attachment {
        kind: tui_contracts::event::V3AttachmentKind::Image,
        label: "draft".into(), path: None, media_type: Some("image/png".into()),
        text: None, bytes: Some(vec![1]),
    };
    composer.add_attachment(attachment.clone());
    let draft = composer.textarea.draft();
    search(&mut composer);
    composer.set_task_running(true);
    assert!(matches!(key(&mut composer, KeyCode::Esc), V3Action::None));
    assert_eq!(composer.textarea.draft(), draft);
    assert_eq!(composer.attachments(), &[attachment]);
}

#[test]
fn terminal_req_08_unicode_query_backspace_restarts_search() {
    let mut composer = history();
    search(&mut composer);
    key(&mut composer, KeyCode::Char('中'));
    key(&mut composer, KeyCode::Char('😀'));
    assert!(!composer.history_search.as_ref().unwrap().matched);
    key(&mut composer, KeyCode::Backspace);
    assert_eq!(composer.history_search.as_ref().unwrap().query, "中");
    assert_eq!(composer.text(), "latest 中文");
}

#[test]
fn terminal_req_08_no_match_preserves_query_and_draft() {
    let mut composer = ChatComposer::new();
    composer.replace_text("unsent");
    search(&mut composer);
    composer.handle_paste("不存在😀".into());
    assert_eq!(composer.history_search.as_ref().unwrap().query, "不存在😀");
    assert_eq!(composer.text(), "unsent");
    key(&mut composer, KeyCode::Esc);
    assert_eq!(composer.text(), "unsent");
}

#[test]
fn terminal_req_08_boundary_keeps_preview_and_enter_never_sends() {
    let mut composer = history();
    search(&mut composer);
    for _ in 0..5 { search(&mut composer); }
    assert_eq!(composer.text(), "older 中文");
    assert!(matches!(key(&mut composer, KeyCode::Enter), V3Action::None));
    assert_eq!(composer.text(), "older 中文");
}

#[test]
fn terminal_req_08_search_does_not_start_file_completion() {
    let mut composer = history();
    composer.replace_text("@file");
    search(&mut composer);
    assert!(composer.active_file_search_query().is_none());
}
