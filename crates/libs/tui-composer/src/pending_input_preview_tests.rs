use super::*;
use insta::assert_snapshot;
use pretty_assertions::assert_eq;

// 验证 `desired_height_empty` 场景的触发条件与预期结果。
#[test]
fn desired_height_empty() {
    let queue = PendingInputPreview::new();
    assert_eq!(queue.desired_height(/*width*/ 40), 0);
}

// 验证 `desired_height_one_message` 场景的触发条件与预期结果。
#[test]
fn desired_height_one_message() {
    let mut queue = PendingInputPreview::new();
    queue.queued_messages.push("Hello, world!".to_string());
    assert_eq!(queue.desired_height(/*width*/ 40), 3);
}

// 验证 `render_one_message` 场景的触发条件与预期结果。
#[test]
fn render_one_message() {
    let mut queue = PendingInputPreview::new();
    queue.queued_messages.push("Hello, world!".to_string());
    let width = 40;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!("render_one_message", format!("{buf:?}"));
}

// 验证 `render_one_message_with_shift_left_binding` 场景的触发条件与预期结果。
#[test]
fn render_one_message_with_shift_left_binding() {
    let mut queue = PendingInputPreview::new();
    queue.queued_messages.push("Hello, world!".to_string());
    queue.set_edit_binding(Some(key_hint::shift(KeyCode::Left).into()));
    let width = 40;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!("render_one_message_with_shift_left_binding", format!("{buf:?}"));
}

// 验证 `render_two_messages` 场景的触发条件与预期结果。
#[test]
fn render_two_messages() {
    let mut queue = PendingInputPreview::new();
    queue.queued_messages.push("Hello, world!".to_string());
    queue.queued_messages.push("This is another message".to_string());
    let width = 40;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!("render_two_messages", format!("{buf:?}"));
}

// 验证 `render_more_than_three_messages` 场景的触发条件与预期结果。
#[test]
fn render_more_than_three_messages() {
    let mut queue = PendingInputPreview::new();
    queue.queued_messages.push("Hello, world!".to_string());
    queue.queued_messages.push("This is another message".to_string());
    queue.queued_messages.push("This is a third message".to_string());
    queue.queued_messages.push("This is a fourth message".to_string());
    let width = 40;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!("render_more_than_three_messages", format!("{buf:?}"));
}

// 验证 `render_wrapped_message` 场景的触发条件与预期结果。
#[test]
fn render_wrapped_message() {
    let mut queue = PendingInputPreview::new();
    queue.queued_messages.push("This is a longer message that should be wrapped".to_string());
    queue.queued_messages.push("This is another message".to_string());
    let width = 40;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!("render_wrapped_message", format!("{buf:?}"));
}

// 验证 `render_many_line_message` 场景的触发条件与预期结果。
#[test]
fn render_many_line_message() {
    let mut queue = PendingInputPreview::new();
    queue.queued_messages.push("This is\na message\nwith many\n\nlines".to_string());
    let width = 40;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!("render_many_line_message", format!("{buf:?}"));
}

// 验证 `long_url_like_message_does_not_expand_into_wrapped_ellipsis_rows` 场景的触发条件与预期结果。
#[test]
fn long_url_like_message_does_not_expand_into_wrapped_ellipsis_rows() {
    let mut queue = PendingInputPreview::new();
    queue.queued_messages.push(
        "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/reports/performance/summary/detail/session_id=abc123def456ghi789"
            .to_string(),
    );

    let width = 36;
    let height = queue.desired_height(width);
    assert_eq!(height, 3, "expected header, one message row, and hint row for URL-like token");

    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);

    let rendered_rows = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect::<String>()
        })
        .collect::<Vec<_>>();

    assert!(
        !rendered_rows.iter().any(|row| row.contains('…')),
        "expected no wrapped-ellipsis row for URL-like token, got rows: {rendered_rows:?}"
    );
}

// 验证 `render_one_pending_steer` 场景的触发条件与预期结果。
#[test]
fn render_one_pending_steer() {
    let mut queue = PendingInputPreview::new();
    queue.pending_steers.push("Please continue.".to_string());
    let width = 48;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!("render_one_pending_steer", format!("{buf:?}"));
}

// 验证 `render_one_pending_steer_with_remapped_interrupt_binding` 场景的触发条件与预期结果。
#[test]
fn render_one_pending_steer_with_remapped_interrupt_binding() {
    let mut queue = PendingInputPreview::new();
    queue.pending_steers.push("Please continue.".to_string());
    queue.set_interrupt_binding(Some(key_hint::plain(KeyCode::F(12)).into()));
    let width = 48;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!(
        "render_one_pending_steer_with_remapped_interrupt_binding",
        format!("{buf:?}")
    );
}

// 验证 `render_pending_steers_above_queued_messages` 场景的触发条件与预期结果。
#[test]
fn render_pending_steers_above_queued_messages() {
    let mut queue = PendingInputPreview::new();
    queue.pending_steers.push("Please continue.".to_string());
    queue.pending_steers.push("Check the last command output.".to_string());
    queue.rejected_steers.push("Rejected steer that will be retried.".to_string());
    queue.queued_messages.push("Queued follow-up question".to_string());
    let width = 52;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!("render_pending_steers_above_queued_messages", format!("{buf:?}"));
}

// 验证 `render_multiline_pending_steer_uses_single_prefix_and_truncates` 场景的触发条件与预期结果。
#[test]
fn render_multiline_pending_steer_uses_single_prefix_and_truncates() {
    let mut queue = PendingInputPreview::new();
    queue.pending_steers.push("First line\nSecond line\nThird line\n\nFourth line".to_string());
    let width = 48;
    let height = queue.desired_height(width);
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    queue.render(Rect::new(0, 0, width, height), &mut buf);
    assert_snapshot!(
        "render_multiline_pending_steer_uses_single_prefix_and_truncates",
        format!("{buf:?}")
    );
}
