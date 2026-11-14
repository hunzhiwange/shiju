use super::*;
use pretty_assertions::assert_eq;

/// 行为：对 ASCII 输入，短暂地“保持”第一个快字符。如果之后没有爆发，该被持有的字符最终会像普通文本输入一样正常 flush（而非粘贴）。
/// Behavior: for ASCII input we "hold" the first fast char briefly. If no burst follows,
/// that held char should eventually flush as normal typed input (not as a paste).
// 验证 `ascii_first_char_is_held_then_flushes_as_typed` 场景的触发条件与预期结果。
#[test]
fn ascii_first_char_is_held_then_flushes_as_typed() {
    let mut burst = PasteBurst::default();
    let t0 = Instant::now();
    assert!(matches!(burst.on_plain_char('a', t0), CharDecision::RetainFirstChar));

    let t1 = t0 + PasteBurst::recommended_flush_delay() + Duration::from_millis(1);
    assert!(matches!(burst.flush_if_due(t1), FlushResult::Typed('a')));
    assert!(!burst.is_active());
}

/// 行为：若两个 ASCII 字符快速到达，则不应立即渲染第一个字符，而是将缓冲后的完整 Payload 一次性发送。
/// Behavior: if two ASCII chars arrive quickly, we should start buffering without ever
/// rendering the first one, then flush the whole buffered payload as a paste.
// 验证 `ascii_two_fast_chars_start_buffer_from_pending_and_flush_as_paste` 场景的触发条件与预期结果。
#[test]
fn ascii_two_fast_chars_start_buffer_from_pending_and_flush_as_paste() {
    let mut burst = PasteBurst::default();
    let t0 = Instant::now();
    assert!(matches!(burst.on_plain_char('a', t0), CharDecision::RetainFirstChar));

    let t1 = t0 + Duration::from_millis(1);
    assert!(matches!(burst.on_plain_char('b', t1), CharDecision::BeginBufferFromPending));
    burst.append_char_to_buffer('b', t1);

    let t2 = t1 + PasteBurst::recommended_active_flush_delay() + Duration::from_millis(1);
    assert!(matches!(
        burst.flush_if_due(t2),
        FlushResult::Paste(ref s) if s == "ab"
    ));
}

/// 当非字符输入即将被应用时，立即 flush 任何瞬态 burst 状态（包括一个待处理的单个 ASCII 字符），以确保状态不会跨输入泄漏。
/// Behavior: when non-char input is about to be applied, we flush any transient burst state
/// immediately (including a single pending ASCII char) so state doesn't leak across inputs.
// 验证 `flush_before_modified_input_includes_pending_first_char` 场景的触发条件与预期结果。
#[test]
fn flush_before_modified_input_includes_pending_first_char() {
    let mut burst = PasteBurst::default();
    let t0 = Instant::now();
    assert!(matches!(burst.on_plain_char('a', t0), CharDecision::RetainFirstChar));

    assert_eq!(burst.flush_before_modified_input(), Some("a".to_string()));
    assert!(!burst.is_active());
}

/// 行为：仅当已插入的前缀看起来像粘贴时（包含空白字符或足够长度）启用回滚抓取缓冲。
/// Behavior: retro-grab buffering is only enabled when the already-inserted prefix looks
/// paste-like (whitespace or "long enough") so short IME bursts don't get misclassified.
// 验证 `decide_begin_buffer_only_triggers_for_pastey_prefixes` 场景的触发条件与预期结果。
#[test]
fn decide_begin_buffer_only_triggers_for_pastey_prefixes() {
    let mut burst = PasteBurst::default();
    let now = Instant::now();

    assert!(burst.decide_begin_buffer(now, "ab", /*retro_chars*/ 2).is_none());
    assert!(!burst.is_active());

    let grab = burst
        .decide_begin_buffer(now, "a b", /*retro_chars*/ 2)
        .expect("whitespace should be considered paste-like");
    assert_eq!(grab.start_byte, 1);
    assert_eq!(grab.grabbed, " b");
    assert!(burst.is_active());
}

/// 行为：在粘贴式爆发后，短暂保持一个“输入抑制窗口”，以便稍晚的 Enter 键仍能插入换行符而非提交。
/// Behavior: after a paste-like burst, we keep an "enter suppression window" alive briefly so
/// a slightly-late Enter still inserts a newline instead of submitting.
// 验证 `newline_suppression_window_outlives_buffer_flush` 场景的触发条件与预期结果。
#[test]
fn newline_suppression_window_outlives_buffer_flush() {
    let mut burst = PasteBurst::default();
    let t0 = Instant::now();
    assert!(matches!(burst.on_plain_char('a', t0), CharDecision::RetainFirstChar));

    let t1 = t0 + Duration::from_millis(1);
    assert!(matches!(burst.on_plain_char('b', t1), CharDecision::BeginBufferFromPending));
    burst.append_char_to_buffer('b', t1);

    let t2 = t1 + PasteBurst::recommended_active_flush_delay() + Duration::from_millis(1);
    assert!(matches!(burst.flush_if_due(t2), FlushResult::Paste(ref s) if s == "ab"));
    assert!(!burst.is_active());

    assert!(burst.newline_should_insert_instead_of_submit(t2));
    let t3 = t1 + PASTE_ENTER_SUPPRESS_WINDOW + Duration::from_millis(1);
    assert!(!burst.newline_should_insert_instead_of_submit(t3));
}
