use super::*;
use insta::assert_snapshot;
use tui_contracts::action::V3Action;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render(composer: &ChatComposer, width: u16) -> String {
    let height = composer.desired_height(width).max(1);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    composer.render(Rect::new(0, 0, width, height), &mut buf);
    format!("{buf:?}")
}

// 验证 `v3_composer_emits_actions_without_owning_runtime` 场景的触发条件与预期结果。
#[test]
fn v3_composer_emits_actions_without_owning_runtime() {
    let mut composer = ChatComposer::new();
    let now = std::time::Instant::now();
    composer.handle_key_event_at(key(KeyCode::Char('h')), now);
    composer.handle_key_event_at(key(KeyCode::Char('i')), now);
    assert_eq!(
        composer.handle_key_event_at(key(KeyCode::Enter), now + std::time::Duration::from_secs(1)),
        V3Action::Submit { text: "hi".into(), attachments: Vec::new() }
    );

    composer.set_task_running(true);
    assert_eq!(composer.handle_key_event(key(KeyCode::Char('c'))), V3Action::None);
    assert_eq!(
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        V3Action::Interrupt
    );
}

// 验证 `composer_footer_snapshots` 场景的触发条件与预期结果。
#[test]
fn composer_footer_snapshots() {
    let empty = ChatComposer::new();
    assert_snapshot!("empty_40", render(&empty, 40));

    let mut multiline = ChatComposer::new();
    multiline.handle_paste("one\ntwo\nthree".into());
    assert_snapshot!("multiline_80", render(&multiline, 80));

    let mut working = ChatComposer::new();
    working.set_task_running(true);
    assert_snapshot!("working_120", render(&working, 120));

    let mut queued = ChatComposer::new();
    queued.set_task_running(true);
    queued.handle_paste("queued".into());
    assert_snapshot!("queued_80", render(&queued, 80));

    assert_snapshot!("narrow_40", render(&empty, 12));
}
