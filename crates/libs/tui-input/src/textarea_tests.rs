use super::*;
use crate::key_hint;
// 此处故意未导入 crossterm 类型，以避免不必要的警告
// crossterm types are intentionally not imported here to avoid unused warnings
use pretty_assertions::assert_eq;
#[cfg(any())]
use rand::prelude::*;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[cfg(any())]
fn rand_grapheme(rng: &mut rand::rngs::StdRng) -> String {
    let r: u8 = rng.random_range(0..100);
    match r {
        0..=4 => "\n".to_string(),
        5..=12 => " ".to_string(),
        13..=35 => (rng.random_range(b'a'..=b'z') as char).to_string(),
        36..=45 => (rng.random_range(b'A'..=b'Z') as char).to_string(),
        46..=52 => (rng.random_range(b'0'..=b'9') as char).to_string(),
        53..=65 => {
            // 一些宽字符 Emoji
            // Some emoji (wide graphemes)
            let choices = ["👍", "😊", "🐍", "🚀", "🧪", "🌟"];
            choices[rng.random_range(0..choices.len())].to_string()
        }
        66..=75 => {
            // CJK 宽字符
            // CJK wide characters
            let choices = ["漢", "字", "測", "試", "你", "好", "界", "编", "码"];
            choices[rng.random_range(0..choices.len())].to_string()
        }
        76..=85 => {
            // 合并标记序列
            // Combining mark sequences
            let base = ["e", "a", "o", "n", "u"][rng.random_range(0..5)];
            let marks = ["\u{0301}", "\u{0308}", "\u{0302}", "\u{0303}"];
            format!("{base}{}", marks[rng.random_range(0..marks.len())])
        }
        86..=92 => {
            // 一些非拉丁字符的单字代码（希腊字母、西里尔字母、希伯来字母）
            // Some non-latin single codepoints (Greek, Cyrillic, Hebrew)
            let choices = ["Ω", "β", "Ж", "ю", "ש", "م", "ह"];
            choices[rng.random_range(0..choices.len())].to_string()
        }
        _ => {
            // ZWJ 序列（单字符但多码点）
            // ZWJ sequences (single graphemes but multi-codepoint)
            let choices = [
                "👩\u{200D}💻", // woman technologist
                "👨\u{200D}💻", // man technologist
                "🏳️\u{200D}🌈", // rainbow flag
            ];
            choices[rng.random_range(0..choices.len())].to_string()
        }
    }
}

fn ta_with(text: &str) -> TextArea {
    let mut t = TextArea::new();
    t.insert_str(text);
    t
}

// 验证 `insert_and_replace_update_cursor_and_text` 场景的触发条件与预期结果。
#[test]
fn insert_and_replace_update_cursor_and_text() {
    // 插入辅助函数
    // insert helpers
    let mut t = ta_with("hello");
    t.set_cursor(/*pos*/ 5);
    t.insert_str("!");
    assert_eq!(t.text(), "hello!");
    assert_eq!(t.cursor(), 6);

    t.insert_str_at(/*pos*/ 0, "X");
    assert_eq!(t.text(), "Xhello!");
    assert_eq!(t.cursor(), 7);

    // 插入后光标不应移动
    // Insert after the cursor should not move it
    t.set_cursor(/*pos*/ 1);
    let end = t.text().len();
    t.insert_str_at(end, "Y");
    assert_eq!(t.text(), "Xhello!Y");
    assert_eq!(t.cursor(), 1);

    // 替换范围情况
    // replace_range cases
    // 1) cursor before range
    let mut t = ta_with("abcd");
    t.set_cursor(/*pos*/ 1);
    t.replace_range(2..3, "Z");
    assert_eq!(t.text(), "abZd");
    assert_eq!(t.cursor(), 1);

    // 在范围内内显示光标
    // 2) cursor inside range
    let mut t = ta_with("abcd");
    t.set_cursor(/*pos*/ 2);
    t.replace_range(1..3, "Q");
    assert_eq!(t.text(), "aQd");
    assert_eq!(t.cursor(), 2);

    // 光标在偏移量范围内
    // 3) cursor after range with shifted by diff
    let mut t = ta_with("abcd");
    t.set_cursor(/*pos*/ 4);
    t.replace_range(0..1, "AA");
    assert_eq!(t.text(), "AAbcd");
    assert_eq!(t.cursor(), 5);
}

// 验证 `v3_textarea_preserves_unicode_cursor_and_paste_behavior` 场景的触发条件与预期结果。
#[test]
fn v3_textarea_preserves_unicode_cursor_and_paste_behavior() {
    use crate::paste_burst::{CharDecision, FlushResult, PasteBurst};
    use std::time::Instant;

    let mut textarea = TextArea::new();
    for ch in "你🙂e\u{301}".chars() {
        textarea.input(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    assert_eq!(textarea.text(), "你🙂e\u{301}");
    assert_eq!(textarea.cursor(), textarea.text().len());

    // 光标跟随扩展字符而非 UTF-8 字节或标量值。
    // Cursor movement follows extended graphemes, not UTF-8 bytes or scalar values.
    textarea.move_cursor_left();
    assert_eq!(textarea.cursor(), "你🙂".len());
    textarea.move_cursor_left();
    assert_eq!(textarea.cursor(), "你".len());
    textarea.input(KeyEvent::new(KeyCode::Char('好'), KeyModifiers::NONE));
    assert_eq!(textarea.text(), "你好🙂e\u{301}");

    let mut burst = PasteBurst::default();
    let start = Instant::now();
    assert!(matches!(burst.on_plain_char('多', start), CharDecision::RetainFirstChar));
    assert!(matches!(
        burst.on_plain_char('行', start + std::time::Duration::from_millis(1)),
        CharDecision::BeginBufferFromPending
    ));
    burst.append_char_to_buffer('粘', start);
    burst.append_newline_if_active(start);
    burst.append_char_to_buffer('贴', start);
    assert!(burst.newline_should_insert_instead_of_submit(start));
    assert!(matches!(
        burst.flush_before_modified_input(),
        Some(text) if text.contains('粘') && text.contains('贴')
    ));
    assert!(matches!(
        burst.flush_if_due(start + PasteBurst::recommended_active_flush_delay()),
        FlushResult::None
    ));
}

// 验证 `insert_str_at_clamps_to_char_boundary` 场景的触发条件与预期结果。
#[test]
fn insert_str_at_clamps_to_char_boundary() {
    let mut t = TextArea::new();
    t.insert_str("你");
    t.set_cursor(/*pos*/ 0);
    t.insert_str_at(/*pos*/ 1, "A");
    assert_eq!(t.text(), "A你");
    assert_eq!(t.cursor(), 1);
}

// 验证 `set_text_clamps_cursor_to_char_boundary` 场景的触发条件与预期结果。
#[test]
fn set_text_clamps_cursor_to_char_boundary() {
    let mut t = TextArea::new();
    t.insert_str("abcd");
    t.set_cursor(/*pos*/ 1);
    t.set_text_clearing_elements("你");
    assert_eq!(t.cursor(), 0);
    t.insert_str("a");
    assert_eq!(t.text(), "a你");
}

// 验证 `delete_backward_and_forward_edges` 场景的触发条件与预期结果。
#[test]
fn delete_backward_and_forward_edges() {
    let mut t = ta_with("abc");
    t.set_cursor(/*pos*/ 1);
    t.delete_backward(/*n*/ 1);
    assert_eq!(t.text(), "bc");
    assert_eq!(t.cursor(), 0);

    // 删除操作在开始处是无效的
    // deleting backward at start is a no-op
    t.set_cursor(/*pos*/ 0);
    t.delete_backward(/*n*/ 1);
    assert_eq!(t.text(), "bc");
    assert_eq!(t.cursor(), 0);

    // 删除下一个图符
    // forward delete removes next grapheme
    t.set_cursor(/*pos*/ 1);
    t.delete_forward(/*n*/ 1);
    assert_eq!(t.text(), "b");
    assert_eq!(t.cursor(), 1);

    // 删除操作在末尾是空操作
    // forward delete at end is a no-op
    t.set_cursor(t.text().len());
    t.delete_forward(/*n*/ 1);
    assert_eq!(t.text(), "b");
}

// 验证 `delete_backward_removes_thai_marks_one_at_a_time` 场景的触发条件与预期结果。
#[test]
fn delete_backward_removes_thai_marks_one_at_a_time() {
    let mut t = ta_with("ที่");

    t.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    let area = Rect::new(0, 0, /*width*/ 8, /*height*/ 1);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();
    insta::assert_snapshot!(
        "textarea_thai_backspace_preserves_remaining_marks",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );
    assert_eq!(t.text(), "ที");

    t.delete_backward(/*n*/ 1);
    assert_eq!(t.text(), "ท");

    t.delete_backward(/*n*/ 1);
    assert_eq!(t.text(), "");
}

// 验证 `delete_backward_preserves_other_grapheme_behavior` 场景的触发条件与预期结果。
#[test]
fn delete_backward_preserves_other_grapheme_behavior() {
    let mut thai_spacing_vowel = ta_with("ซ้ำ");
    thai_spacing_vowel.delete_backward(/*n*/ 1);
    assert_eq!(thai_spacing_vowel.text(), "ซ้");

    let mut decomposed_latin = ta_with("e\u{301}");
    decomposed_latin.delete_backward(/*n*/ 1);
    assert_eq!(decomposed_latin.text(), "");

    let mut family_emoji = ta_with("👨\u{200d}👩\u{200d}👧\u{200d}👦");
    family_emoji.delete_backward(/*n*/ 1);
    assert_eq!(family_emoji.text(), "");
}

// 验证 `delete_backward_keeps_thai_elements_atomic` 场景的触发条件与预期结果。
#[test]
fn delete_backward_keeps_thai_elements_atomic() {
    let mut t = TextArea::new();
    t.insert_str("before ");
    t.insert_element("ที่");
    t.insert_str(" after");

    t.set_cursor(t.elements[0].range.end);
    t.delete_backward(/*n*/ 1);

    assert_eq!(t.text(), "before  after");
    assert_eq!(t.cursor(), "before ".len());
    assert!(t.elements.is_empty());
}

// 验证 `delete_forward_deletes_element_at_left_edge` 场景的触发条件与预期结果。
#[test]
fn delete_forward_deletes_element_at_left_edge() {
    let mut t = TextArea::new();
    t.insert_str("a");
    t.insert_element("<element>");
    t.insert_str("b");

    let elem_start = t.elements[0].range.start;
    t.set_cursor(elem_start);
    t.delete_forward(/*n*/ 1);

    assert_eq!(t.text(), "ab");
    assert_eq!(t.cursor(), elem_start);
}

// 验证 `vim_insert_and_escape` 场景的触发条件与预期结果。
#[test]
fn vim_insert_and_escape() {
    let mut t = TextArea::new();
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(t.text(), "h");
    assert_eq!(t.vim_mode_label(), Some("Normal"));
    assert_eq!(t.cursor(), 0);
}

// 验证 `vim_insert_key_enters_insert_mode` 场景的触发条件与预期结果。
#[test]
fn vim_insert_key_enters_insert_mode() {
    let mut t = TextArea::new();
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert_eq!(t.text(), "h");
    assert_eq!(t.vim_mode_label(), Some("Insert"));
}

// 验证 `vim_normal_arrow_keys_move_cursor` 场景的触发条件与预期结果。
#[test]
fn vim_normal_arrow_keys_move_cursor() {
    let mut t = ta_with("ab\ncd");
    t.set_cursor(/*pos*/ 1);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(t.cursor(), 2);

    t.input(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(t.cursor(), 5);

    t.input(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    assert_eq!(t.cursor(), 4);

    t.input(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(t.cursor(), 1);
}

// 验证 `vim_escape_from_insert_at_start_does_not_underflow` 场景的触发条件与预期结果。
#[test]
fn vim_escape_from_insert_at_start_does_not_underflow() {
    let mut t = TextArea::new();
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(t.vim_mode_label(), Some("Normal"));
    assert_eq!(t.cursor(), 0);
}

// 验证 `vim_escape_from_insert_at_line_start_stays_on_line` 场景的触发条件与预期结果。
#[test]
fn vim_escape_from_insert_at_line_start_stays_on_line() {
    let mut t = ta_with("one\ntwo");
    t.set_cursor(/*pos*/ "one\n".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(t.vim_mode_label(), Some("Normal"));
    assert_eq!(t.cursor(), "one\n".len());
}

// 验证 `vim_escape_moves_by_grapheme_boundary` 场景的触发条件与预期结果。
#[test]
fn vim_escape_moves_by_grapheme_boundary() {
    let mut t = ta_with("👍👍");
    t.set_cursor(t.text().len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(t.vim_mode_label(), Some("Normal"));
    assert_eq!(t.cursor(), "👍".len());
}

// 验证 `vim_escape_respects_atomic_element_boundary` 场景的触发条件与预期结果。
#[test]
fn vim_escape_respects_atomic_element_boundary() {
    let mut t = TextArea::new();
    t.insert_str("a");
    t.insert_element("<element>");
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(t.vim_mode_label(), Some("Normal"));
    assert_eq!(t.cursor(), 1);
}

// 验证 `vim_shift_i_enters_insert_at_first_non_blank_with_shift_only_binding` 场景的触发条件与预期结果。
#[test]
fn vim_shift_i_enters_insert_at_first_non_blank_with_shift_only_binding() {
    let mut t = ta_with("hello\n  world");
    t.vim_normal_keymap.insert_line_start = vec![key_hint::shift(KeyCode::Char('i'))];
    t.set_cursor(/*pos*/ "hello\n  wor".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::NONE));

    assert_eq!(t.vim_mode_label(), Some("Insert"));
    assert_eq!(t.cursor(), "hello\n  ".len());
}

// 验证 `vim_shift_a_enters_insert_at_line_end_with_shift_only_binding` 场景的触发条件与预期结果。
#[test]
fn vim_shift_a_enters_insert_at_line_end_with_shift_only_binding() {
    let mut t = ta_with("hello\nworld");
    t.vim_normal_keymap.append_line_end = vec![key_hint::shift(KeyCode::Char('a'))];
    t.set_cursor(/*pos*/ 8);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));

    assert_eq!(t.vim_mode_label(), Some("Insert"));
    assert_eq!(t.cursor(), 11);
}

// 验证 `vim_shift_c_changes_to_line_end_and_enters_insert_mode` 场景的触发条件与预期结果。
#[test]
fn vim_shift_c_changes_to_line_end_and_enters_insert_mode() {
    let mut t = ta_with("hello world\nnext line");
    t.set_cursor(/*pos*/ 6);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::SHIFT));

    assert_eq!(t.text(), "hello \nnext line");
    assert_eq!(t.vim_mode_label(), Some("Insert"));
    assert_eq!(t.cursor(), 6);
    assert_eq!(t.kill_buffer, "world");
}

// 验证 `vim_uppercase_c_changes_to_line_end` 场景的触发条件与预期结果。
#[test]
fn vim_uppercase_c_changes_to_line_end() {
    let mut t = ta_with("hello world\nnext line");
    t.set_cursor(/*pos*/ 6);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::NONE));

    assert_eq!(t.text(), "hello \nnext line");
    assert_eq!(t.vim_mode_label(), Some("Insert"));
    assert_eq!(t.cursor(), 6);
}

// 验证 `vim_s_substitutes_current_character_and_enters_insert_mode` 场景的触发条件与预期结果。
#[test]
fn vim_s_substitutes_current_character_and_enters_insert_mode() {
    let mut t = ta_with("abc");
    t.set_cursor(/*pos*/ 1);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

    assert_eq!(t.text(), "ac");
    assert_eq!(t.cursor(), 1);
    assert_eq!(t.vim_mode_label(), Some("Insert"));

    t.input(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));

    assert_eq!(t.text(), "aXc");
    assert_eq!(t.cursor(), 2);
    assert_eq!(t.vim_mode_label(), Some("Insert"));
}

// 验证 `vim_s_on_empty_line_enters_insert_without_deleting_newline` 场景的触发条件与预期结果。
#[test]
fn vim_s_on_empty_line_enters_insert_without_deleting_newline() {
    let mut t = ta_with("before\n\nnext");
    t.set_cursor(/*pos*/ "before\n".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

    assert_eq!(t.text(), "before\n\nnext");
    assert_eq!(t.cursor(), "before\n".len());
    assert_eq!(t.vim_mode_label(), Some("Insert"));
}

// 验证 `vim_d_at_line_end_does_not_remove_newline` 场景的触发条件与预期结果。
#[test]
fn vim_d_at_line_end_does_not_remove_newline() {
    let mut t = ta_with("hello\nworld");
    t.set_cursor(/*pos*/ "hello".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE));

    assert_eq!(t.text(), "hello\nworld");
    assert_eq!(t.vim_mode_label(), Some("Normal"));
    assert_eq!(t.kill_buffer, "");
}

// 验证 `vim_c_at_line_end_enters_insert_without_removing_newline` 场景的触发条件与预期结果。
#[test]
fn vim_c_at_line_end_enters_insert_without_removing_newline() {
    let mut t = ta_with("hello\nworld");
    t.set_cursor(/*pos*/ "hello".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::NONE));

    assert_eq!(t.text(), "hello\nworld");
    assert_eq!(t.vim_mode_label(), Some("Insert"));
    assert_eq!(t.cursor(), "hello".len());
    assert_eq!(t.kill_buffer, "");
}

// 验证 `vim_shift_o_opens_line_above_with_shift_only_binding` 场景的触发条件与预期结果。
#[test]
fn vim_shift_o_opens_line_above_with_shift_only_binding() {
    let mut t = ta_with("hello\nworld");
    t.vim_normal_keymap.open_line_above = vec![key_hint::shift(KeyCode::Char('o'))];
    t.set_cursor(/*pos*/ 8);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::NONE));

    assert_eq!(t.text(), "hello\n\nworld");
    assert_eq!(t.vim_mode_label(), Some("Insert"));
    assert_eq!(t.cursor(), 6);
}

// 验证 `vim_o_opens_line_below_on_inserted_line` 场景的触发条件与预期结果。
#[test]
fn vim_o_opens_line_below_on_inserted_line() {
    let mut t = ta_with("one\ntwo");
    t.set_cursor(/*pos*/ 1);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

    assert_eq!(t.text(), "one\n\ntwo");
    assert_eq!(t.vim_mode_label(), Some("Insert"));
    assert_eq!(t.cursor(), "one\n".len());
}

// 验证 `vim_o_opens_line_below_final_line_and_moves_to_new_line` 场景的触发条件与预期结果。
#[test]
fn vim_o_opens_line_below_final_line_and_moves_to_new_line() {
    let mut t = ta_with("one");
    t.set_cursor(/*pos*/ 1);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

    assert_eq!(t.text(), "one\n");
    assert_eq!(t.vim_mode_label(), Some("Insert"));
    assert_eq!(t.cursor(), "one\n".len());
}

// 验证 `vim_delete_word` 场景的触发条件与预期结果。
#[test]
fn vim_delete_word() {
    let mut t = ta_with("hello world");
    t.set_cursor(/*pos*/ 0);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

    assert_eq!(t.text(), "world");
    assert_eq!(t.kill_buffer, "hello ");
}

// 验证 `vim_change_inner_word_deletes_word_and_enters_insert` 场景的触发条件与预期结果。
#[test]
fn vim_change_inner_word_deletes_word_and_enters_insert() {
    let mut t = ta_with("hello world");
    t.set_cursor(/*pos*/ "hello ".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

    assert_eq!(t.text(), "hello ");
    assert_eq!(t.kill_buffer, "world");
    assert_eq!(t.cursor(), "hello ".len());
    assert_eq!(t.vim_mode_label(), Some("Insert"));
}

// 验证 `vim_word_text_objects_cover_delete_yank_and_big_word` 场景的触发条件与预期结果。
#[test]
fn vim_word_text_objects_cover_delete_yank_and_big_word() {
    let mut t = ta_with("hello world");
    t.set_cursor(/*pos*/ 1);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

    assert_eq!(t.text(), "hello world");
    assert_eq!(t.kill_buffer, "hello ");
    assert_eq!(t.vim_mode_label(), Some("Normal"));

    let mut t = ta_with("foo.bar/baz qux");
    t.set_cursor(/*pos*/ "foo.".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('W'), KeyModifiers::NONE));

    assert_eq!(t.text(), " qux");
    assert_eq!(t.kill_buffer, "foo.bar/baz");
}

// 验证 `vim_word_text_objects_accept_cursor_at_word_end` 场景的触发条件与预期结果。
#[test]
fn vim_word_text_objects_accept_cursor_at_word_end() {
    let mut t = ta_with("hello world");
    t.set_cursor(/*pos*/ "hello".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

    assert_eq!(t.text(), "world");
    assert_eq!(t.kill_buffer, "hello ");

    let mut t = ta_with("foo bar");
    t.set_cursor(t.text().len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('W'), KeyModifiers::NONE));

    assert_eq!(t.text(), "foo ");
    assert_eq!(t.kill_buffer, "bar");
    assert_eq!(t.cursor(), "foo ".len());
    assert_eq!(t.vim_mode_label(), Some("Insert"));
}

// 验证 `vim_delimiter_text_objects_select_innermost_pair_and_aliases` 场景的触发条件与预期结果。
#[test]
fn vim_delimiter_text_objects_select_innermost_pair_and_aliases() {
    let mut t = ta_with("a(b(c)d)e");
    t.set_cursor(/*pos*/ "a(b(".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));

    assert_eq!(t.text(), "a(b()d)e");
    assert_eq!(t.kill_buffer, "c");
    assert_eq!(t.vim_mode_label(), Some("Insert"));

    let mut t = ta_with("a [b] c");
    t.set_cursor(/*pos*/ "a [".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));

    assert_eq!(t.text(), "a  c");
    assert_eq!(t.kill_buffer, "[b]");
}

// 验证 `vim_empty_inner_text_objects_are_valid_targets` 场景的触发条件与预期结果。
#[test]
fn vim_empty_inner_text_objects_are_valid_targets() {
    let mut t = ta_with("call()");
    t.set_cursor(/*pos*/ "call(".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));

    assert_eq!(t.text(), "call()");
    assert_eq!(t.kill_buffer, "");
    assert_eq!(t.cursor(), "call(".len());
    assert_eq!(t.vim_mode_label(), Some("Insert"));

    let mut t = ta_with(r#"say "" now"#);
    t.set_cursor(/*pos*/ r#"say ""#.len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE));

    assert_eq!(t.text(), r#"say "" now"#);
    assert_eq!(t.kill_buffer, "");
    assert_eq!(t.cursor(), r#"say ""#.len());
    assert_eq!(t.vim_mode_label(), Some("Insert"));
}

// 验证 `vim_quote_text_objects_are_line_local_and_handle_escapes` 场景的触发条件与预期结果。
#[test]
fn vim_quote_text_objects_are_line_local_and_handle_escapes() {
    let mut t = ta_with(r#"say "a \"b\" c" now"#);
    t.set_cursor(/*pos*/ r#"say "a \"#.len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('"'), KeyModifiers::SHIFT));

    assert_eq!(t.text(), r#"say "" now"#);
    assert_eq!(t.kill_buffer, r#"a \"b\" c"#);
    assert_eq!(t.vim_mode_label(), Some("Insert"));

    let mut t = ta_with("one \"two\nthree\" four");
    t.set_cursor(/*pos*/ "one \"two\n".len());
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE));

    assert_eq!(t.text(), "one \"two\nthree\" four");
    assert_eq!(t.kill_buffer, "");
}

// 验证 `vim_text_object_cancellation_does_not_edit` 场景的触发条件与预期结果。
#[test]
fn vim_text_object_cancellation_does_not_edit() {
    let mut t = ta_with("hello world");
    t.set_cursor(/*pos*/ 1);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert!(t.is_vim_operator_pending());
    t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(t.text(), "hello world");
    assert_eq!(t.kill_buffer, "");
    assert!(!t.is_vim_operator_pending());
}

// 验证 `vim_operator_invalid_motion_is_consumed` 场景的触发条件与预期结果。
#[test]
fn vim_operator_invalid_motion_is_consumed() {
    let mut t = ta_with("hello");
    t.set_cursor(/*pos*/ 0);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert!(t.is_vim_operator_pending());

    t.input(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));

    assert_eq!(t.text(), "hello");
    assert_eq!(t.vim_mode_label(), Some("Normal"));
    assert_eq!(t.cursor(), 0);
    assert!(!t.is_vim_operator_pending());
}

// 验证 `vim_e_lands_on_word_end_character` 场景的触发条件与预期结果。
#[test]
fn vim_e_lands_on_word_end_character() {
    let mut t = ta_with("abc");
    t.set_cursor(/*pos*/ 0);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

    assert_eq!(t.cursor(), 2);

    t.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

    assert_eq!(t.text(), "ab");
    assert_eq!(t.kill_buffer, "c");
}

// 验证 `vim_e_advances_from_each_word_end` 场景的触发条件与预期结果。
#[test]
fn vim_e_advances_from_each_word_end() {
    let mut t = ta_with("alpha beta gamma");
    t.set_cursor("alph".len()); // codespell:ignore alph
    t.set_vim_enabled(/*enabled*/ true);
    let mut states = Vec::new();

    for _ in 0..3 {
        t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        states.push(format!("{}\n{}^", t.text(), " ".repeat(t.cursor())));
    }

    insta::assert_snapshot!("vim_e_advances_from_each_word_end", states.join("\n\n"));
}

// 验证 `vim_delete_to_word_end_advances_from_existing_word_end` 场景的触发条件与预期结果。
#[test]
fn vim_delete_to_word_end_advances_from_existing_word_end() {
    let mut t = ta_with("alpha beta gamma");
    t.set_cursor("alph".len()); // codespell:ignore alph
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

    assert_eq!(t.text(), "alph gamma"); // codespell:ignore alph
    assert_eq!(t.kill_buffer, "a beta");
}

// 验证 `vim_e_from_word_end_can_land_on_trailing_space` 场景的触发条件与预期结果。
#[test]
fn vim_e_from_word_end_can_land_on_trailing_space() {
    let mut t = ta_with("alpha   ");
    t.set_cursor("alph".len()); // codespell:ignore alph
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

    assert_eq!(t.cursor(), "alpha  ".len());
}

// 验证 `vim_e_advances_across_atomic_element_word_ends` 场景的触发条件与预期结果。
#[test]
fn vim_e_advances_across_atomic_element_word_ends() {
    let mut t = TextArea::new();
    t.insert_str("alpha ");
    t.insert_element("<element>");
    t.insert_str(" gamma");
    let element_start = t.elements[0].range.start;
    t.set_cursor("alph".len()); // codespell:ignore alph
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    assert_eq!(t.cursor(), element_start);

    t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    assert_eq!(t.cursor(), "alpha <element> gamm".len());
}

// 验证 `vim_dollar_lands_on_line_end_character` 场景的触发条件与预期结果。
#[test]
fn vim_dollar_lands_on_line_end_character() {
    let mut t = ta_with("abc\n123");
    t.set_cursor(/*pos*/ 1);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('$'), KeyModifiers::NONE));

    assert_eq!(t.cursor(), 2);

    t.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

    assert_eq!(t.text(), "ab\n123");
    assert_eq!(t.kill_buffer, "c");
}

// 验证 `vim_linewise_yank_pastes_below_current_line` 场景的触发条件与预期结果。
#[test]
fn vim_linewise_yank_pastes_below_current_line() {
    let mut t = ta_with("abc\n123\nxyz");
    t.set_cursor(/*pos*/ 1);
    t.set_vim_enabled(/*enabled*/ true);

    t.input(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    t.input(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert_eq!(t.text(), "abc\nabc\n123\nxyz");
    assert_eq!(t.cursor(), "abc\n".len());
    assert_eq!(t.kill_buffer, "abc\n");
    assert_eq!(t.kill_buffer_kind, KillBufferKind::Linewise);
}

// 验证 `delete_backward_word_and_kill_line_variants` 场景的触发条件与预期结果。
#[test]
fn delete_backward_word_and_kill_line_variants() {
    // 删除末尾单词会移除前一个单词
    // delete backward word at end removes the whole previous word
    let mut t = ta_with("hello   world  ");
    t.set_cursor(t.text().len());
    t.delete_backward_word();
    assert_eq!(t.text(), "hello   ");
    assert_eq!(t.cursor(), 8);

    // 从单词内部删除到光标位置
    // From inside a word, delete from word start to cursor
    let mut t = ta_with("foo bar");
    t.set_cursor(/*pos*/ 6); // inside "bar" (after 'a')
    t.delete_backward_word();
    assert_eq!(t.text(), "foo r");
    assert_eq!(t.cursor(), 4);

    // 从末尾删除最后一个单词
    // From end, delete the last word only
    let mut t = ta_with("foo bar");
    t.set_cursor(t.text().len());
    t.delete_backward_word();
    assert_eq!(t.text(), "foo ");
    assert_eq!(t.cursor(), 4);

    // 当不在行尾时，执行终止换行
    // kill_to_end_of_line when not at EOL
    let mut t = ta_with("abc\ndef");
    t.set_cursor(/*pos*/ 1); // on first line, middle
    t.kill_to_end_of_line();
    assert_eq!(t.text(), "a\ndef");
    assert_eq!(t.cursor(), 1);

    // 当到达行尾时，执行删除换行符
    // kill_to_end_of_line when at EOL deletes newline
    let mut t = ta_with("abc\ndef");
    t.set_cursor(/*pos*/ 3); // EOL of first line
    t.kill_to_end_of_line();
    assert_eq!(t.text(), "abcdef");
    assert_eq!(t.cursor(), 3);

    // 从中间开始结束整行
    // kill_to_beginning_of_line from middle of line
    let mut t = ta_with("abc\ndef");
    t.set_cursor(/*pos*/ 5); // on second line, after 'e'
    t.kill_to_beginning_of_line();
    assert_eq!(t.text(), "abc\nef");

    // 在行首非第一行处移除前一个换行符
    // kill_to_beginning_of_line at beginning of non-first line removes the previous newline
    let mut t = ta_with("abc\ndef");
    t.set_cursor(/*pos*/ 4); // beginning of second line
    t.kill_to_beginning_of_line();
    assert_eq!(t.text(), "abcdef");
    assert_eq!(t.cursor(), 3);
}

// 验证 `kill_current_line_removes_current_line_linewise` 场景的触发条件与预期结果。
#[test]
fn kill_current_line_removes_current_line_linewise() {
    let mut t = ta_with("abc\ndef\nghi");
    t.set_cursor(/*pos*/ 5);

    t.kill_current_line();

    assert_eq!(t.text(), "abc\nghi");
    assert_eq!(t.cursor(), 4);
    assert_eq!(t.kill_buffer, "def\n");
    assert_eq!(t.kill_buffer_kind, KillBufferKind::Linewise);
}

// 验证 `kill_current_line_keeps_previous_newline_for_final_line` 场景的触发条件与预期结果。
#[test]
fn kill_current_line_keeps_previous_newline_for_final_line() {
    let mut t = ta_with("abc\ndef");
    t.set_cursor(/*pos*/ 5);

    t.kill_current_line();

    assert_eq!(t.text(), "abc\n");
    assert_eq!(t.cursor(), 4);
    assert_eq!(t.kill_buffer, "def");
    assert_eq!(t.kill_buffer_kind, KillBufferKind::Linewise);
}

// 验证 `kill_whole_line_keymap_dispatch_uses_linewise_kill` 场景的触发条件与预期结果。
#[test]
fn kill_whole_line_keymap_dispatch_uses_linewise_kill() {
    let mut t = ta_with("abc\ndef\nghi");
    t.set_cursor(/*pos*/ 5);
    let mut keymap = RuntimeKeymap::defaults().editor;
    Arc::make_mut(&mut keymap).kill_line_start.clear();
    Arc::make_mut(&mut keymap).kill_whole_line = vec![key_hint::ctrl(KeyCode::Char('u'))];

    t.input_with_keymap(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL), &keymap);

    assert_eq!(t.text(), "abc\nghi");
    assert_eq!(t.cursor(), 4);
    assert_eq!(t.kill_buffer, "def\n");
    assert_eq!(t.kill_buffer_kind, KillBufferKind::Linewise);
}

// 验证 `delete_forward_word_variants` 场景的触发条件与预期结果。
#[test]
fn delete_forward_word_variants() {
    let mut t = ta_with("hello   world ");
    t.set_cursor(/*pos*/ 0);
    t.delete_forward_word();
    assert_eq!(t.text(), "   world ");
    assert_eq!(t.cursor(), 0);

    let mut t = ta_with("hello   world ");
    t.set_cursor(/*pos*/ 1);
    t.delete_forward_word();
    assert_eq!(t.text(), "h   world ");
    assert_eq!(t.cursor(), 1);

    let mut t = ta_with("hello   world");
    t.set_cursor(t.text().len());
    t.delete_forward_word();
    assert_eq!(t.text(), "hello   world");
    assert_eq!(t.cursor(), t.text().len());

    let mut t = ta_with("foo   \nbar");
    t.set_cursor(/*pos*/ 3);
    t.delete_forward_word();
    assert_eq!(t.text(), "foo");
    assert_eq!(t.cursor(), 3);

    let mut t = ta_with("foo\nbar");
    t.set_cursor(/*pos*/ 3);
    t.delete_forward_word();
    assert_eq!(t.text(), "foo");
    assert_eq!(t.cursor(), 3);

    let mut t = ta_with("hello   world ");
    t.set_cursor(t.text().len() + 10);
    t.delete_forward_word();
    assert_eq!(t.text(), "hello   world ");
    assert_eq!(t.cursor(), t.text().len());
}

// 验证 `delete_forward_word_handles_atomic_elements` 场景的触发条件与预期结果。
#[test]
fn delete_forward_word_handles_atomic_elements() {
    let mut t = TextArea::new();
    t.insert_element("<element>");
    t.insert_str(" tail");

    t.set_cursor(/*pos*/ 0);
    t.delete_forward_word();
    assert_eq!(t.text(), " tail");
    assert_eq!(t.cursor(), 0);

    let mut t = TextArea::new();
    t.insert_str("   ");
    t.insert_element("<element>");
    t.insert_str(" tail");

    t.set_cursor(/*pos*/ 0);
    t.delete_forward_word();
    assert_eq!(t.text(), " tail");
    assert_eq!(t.cursor(), 0);

    let mut t = TextArea::new();
    t.insert_str("prefix ");
    t.insert_element("<element>");
    t.insert_str(" tail");

    // 在元素中间，删除前缀单词会删除该元素
    // cursor in the middle of the element, delete_forward_word deletes the element
    let elem_range = t.elements[0].range.clone();
    t.cursor_pos = elem_range.start + (elem_range.len() / 2);
    t.delete_forward_word();
    assert_eq!(t.text(), "prefix  tail");
    assert_eq!(t.cursor(), elem_range.start);
}

// 验证 `delete_backward_word_respects_word_separators` 场景的触发条件与预期结果。
#[test]
fn delete_backward_word_respects_word_separators() {
    let mut t = ta_with("path/to/file");
    t.set_cursor(t.text().len());
    t.delete_backward_word();
    assert_eq!(t.text(), "path/to/");
    assert_eq!(t.cursor(), t.text().len());

    t.delete_backward_word();
    assert_eq!(t.text(), "path/to");
    assert_eq!(t.cursor(), t.text().len());

    let mut t = ta_with("foo/ ");
    t.set_cursor(t.text().len());
    t.delete_backward_word();
    assert_eq!(t.text(), "foo");
    assert_eq!(t.cursor(), 3);

    let mut t = ta_with("foo /");
    t.set_cursor(t.text().len());
    t.delete_backward_word();
    assert_eq!(t.text(), "foo ");
    assert_eq!(t.cursor(), 4);
}

// 验证 `delete_forward_word_respects_word_separators` 场景的触发条件与预期结果。
#[test]
fn delete_forward_word_respects_word_separators() {
    let mut t = ta_with("path/to/file");
    t.set_cursor(/*pos*/ 0);
    t.delete_forward_word();
    assert_eq!(t.text(), "/to/file");
    assert_eq!(t.cursor(), 0);

    t.delete_forward_word();
    assert_eq!(t.text(), "to/file");
    assert_eq!(t.cursor(), 0);

    let mut t = ta_with("/ foo");
    t.set_cursor(/*pos*/ 0);
    t.delete_forward_word();
    assert_eq!(t.text(), " foo");
    assert_eq!(t.cursor(), 0);

    let mut t = ta_with(" /foo");
    t.set_cursor(/*pos*/ 0);
    t.delete_forward_word();
    assert_eq!(t.text(), "foo");
    assert_eq!(t.cursor(), 0);
}

// 验证 `yank_restores_last_kill` 场景的触发条件与预期结果。
#[test]
fn yank_restores_last_kill() {
    let mut t = ta_with("hello");
    t.set_cursor(/*pos*/ 0);
    t.kill_to_end_of_line();
    assert_eq!(t.text(), "");
    assert_eq!(t.cursor(), 0);

    t.yank();
    assert_eq!(t.text(), "hello");
    assert_eq!(t.cursor(), 5);

    let mut t = ta_with("hello world");
    t.set_cursor(t.text().len());
    t.delete_backward_word();
    assert_eq!(t.text(), "hello ");
    assert_eq!(t.cursor(), 6);

    t.yank();
    assert_eq!(t.text(), "hello world");
    assert_eq!(t.cursor(), 11);

    let mut t = ta_with("hello");
    t.set_cursor(/*pos*/ 5);
    t.kill_to_beginning_of_line();
    assert_eq!(t.text(), "");
    assert_eq!(t.cursor(), 0);

    t.yank();
    assert_eq!(t.text(), "hello");
    assert_eq!(t.cursor(), 5);
}

// 验证 `kill_buffer_persists_across_set_text` 场景的触发条件与预期结果。
#[test]
fn kill_buffer_persists_across_set_text() {
    let mut t = ta_with("restore me");
    t.set_cursor(/*pos*/ 0);
    t.kill_to_end_of_line();
    assert!(t.text().is_empty());

    t.set_text_clearing_elements("/diff");
    t.set_text_clearing_elements("");
    t.yank();

    assert_eq!(t.text(), "restore me");
    assert_eq!(t.cursor(), "restore me".len());
}

// 验证 `cursor_left_and_right_handle_graphemes` 场景的触发条件与预期结果。
#[test]
fn cursor_left_and_right_handle_graphemes() {
    let mut t = ta_with("a👍b");
    t.set_cursor(t.text().len());

    t.move_cursor_left(); // before 'b'
    let after_first_left = t.cursor();
    t.move_cursor_left(); // before '👍'
    let after_second_left = t.cursor();
    t.move_cursor_left(); // before 'a'
    let after_third_left = t.cursor();

    assert!(after_first_left < t.text().len());
    assert!(after_second_left < after_first_left);
    assert!(after_third_left < after_second_left);

    // 安全地回到末尾
    // Move right back to end safely
    t.move_cursor_right();
    t.move_cursor_right();
    t.move_cursor_right();
    assert_eq!(t.cursor(), t.text().len());
}

// 验证 `control_b_and_f_move_cursor` 场景的触发条件与预期结果。
#[test]
fn control_b_and_f_move_cursor() {
    let mut t = ta_with("abcd");
    t.set_cursor(/*pos*/ 1);

    t.input(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
    assert_eq!(t.cursor(), 2);

    t.input(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
    assert_eq!(t.cursor(), 1);
}

// 验证 `control_b_f_fallback_control_chars_move_cursor` 场景的触发条件与预期结果。
#[test]
fn control_b_f_fallback_control_chars_move_cursor() {
    let mut t = ta_with("abcd");
    t.set_cursor(/*pos*/ 2);

    // 模拟发送不带控制字符的终端，并移除 CONTROL 修饰符。
    // Simulate terminals that send C0 control chars without CONTROL modifier.
    // ^B (U+0002) should move left
    t.input(KeyEvent::new(KeyCode::Char('\u{0002}'), KeyModifiers::NONE));
    assert_eq!(t.cursor(), 1);

    // ^F (Unicode U+0006) 应向右移动
    // ^F (U+0006) should move right
    t.input(KeyEvent::new(KeyCode::Char('\u{0006}'), KeyModifiers::NONE));
    assert_eq!(t.cursor(), 2);
}

// 验证 `c0_line_feed_inserts_newline_through_insert_newline_keymap` 场景的触发条件与预期结果。
#[test]
fn c0_line_feed_inserts_newline_through_insert_newline_keymap() {
    let mut t = ta_with("ab");
    t.set_cursor(/*pos*/ 1);

    t.input(KeyEvent::new(KeyCode::Char('\u{000a}'), KeyModifiers::NONE));

    assert_eq!(t.text(), "a\nb");
    assert_eq!(t.cursor(), 2);
}

// 验证 `c0_control_chars_respect_unbound_editor_movement` 场景的触发条件与预期结果。
#[test]
fn c0_control_chars_respect_unbound_editor_movement() {
    let mut t = ta_with("a\nb");
    t.set_cursor(/*pos*/ 2);
    let mut keymap = RuntimeKeymap::defaults().editor;
    Arc::make_mut(&mut keymap).move_up.clear();

    t.input_with_keymap(KeyEvent::new(KeyCode::Char('\u{0010}'), KeyModifiers::NONE), &keymap);

    assert_eq!(t.cursor(), 2);
}

// 验证 `c0_control_chars_respect_remapped_editor_movement` 场景的触发条件与预期结果。
#[test]
fn c0_control_chars_respect_remapped_editor_movement() {
    let mut t = ta_with("a\nb");
    t.set_cursor(/*pos*/ 0);
    let mut keymap = RuntimeKeymap::defaults().editor;
    Arc::make_mut(&mut keymap).move_up.clear();
    Arc::make_mut(&mut keymap).move_down = vec![crate::key_hint::ctrl(KeyCode::Char('p'))];

    t.input_with_keymap(KeyEvent::new(KeyCode::Char('\u{0010}'), KeyModifiers::NONE), &keymap);

    assert_eq!(t.cursor(), 2);
}

// 验证 `delete_backward_word_alt_keys` 场景的触发条件与预期结果。
#[test]
fn delete_backward_word_alt_keys() {
    // 测试自定义 Alt+Ctrl+h 键绑定
    // Test the custom Alt+Ctrl+h binding
    let mut t = ta_with("hello world");
    t.set_cursor(t.text().len()); // cursor at the end
    t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL | KeyModifiers::ALT));
    assert_eq!(t.text(), "hello ");
    assert_eq!(t.cursor(), 6);

    // 测试标准 Alt+Backspace 键绑定
    // Test the standard Alt+Backspace binding
    let mut t = ta_with("hello world");
    t.set_cursor(t.text().len()); // cursor at the end
    t.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT));
    assert_eq!(t.text(), "hello ");
    assert_eq!(t.cursor(), 6);
}

// 验证 `shift_backspace_and_shift_delete_keep_grapheme_delete_behavior` 场景的触发条件与预期结果。
#[test]
fn shift_backspace_and_shift_delete_keep_grapheme_delete_behavior() {
    let mut t = ta_with("abc");
    t.set_cursor(/*pos*/ 2);

    t.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::SHIFT));
    assert_eq!(t.text(), "ac");
    assert_eq!(t.cursor(), 1);

    let mut t = ta_with("abc");
    t.set_cursor(/*pos*/ 1);

    t.input(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
    assert_eq!(t.text(), "ac");
    assert_eq!(t.cursor(), 1);
}

// 验证 `control_backspace_variants_delete_backward_word` 场景的触发条件与预期结果。
#[test]
fn control_backspace_variants_delete_backward_word() {
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::CONTROL | KeyModifiers::SHIFT] {
        let mut t = ta_with("hello world");
        t.set_cursor(t.text().len());

        t.input(KeyEvent::new(KeyCode::Backspace, modifiers));
        assert_eq!(t.text(), "hello ");
        assert_eq!(t.cursor(), 6);
    }
}

// 验证 `control_delete_variants_delete_forward_word` 场景的触发条件与预期结果。
#[test]
fn control_delete_variants_delete_forward_word() {
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::CONTROL | KeyModifiers::SHIFT] {
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ 0);

        t.input(KeyEvent::new(KeyCode::Delete, modifiers));
        assert_eq!(t.text(), " world");
        assert_eq!(t.cursor(), 0);
    }
}

// 验证 `delete_backward_word_handles_narrow_no_break_space` 场景的触发条件与预期结果。
#[test]
fn delete_backward_word_handles_narrow_no_break_space() {
    let mut t = ta_with("32\u{202F}AM");
    t.set_cursor(t.text().len());
    t.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT));
    pretty_assertions::assert_eq!(t.text(), "32\u{202F}");
    pretty_assertions::assert_eq!(t.cursor(), t.text().len());
}

// 验证 `delete_forward_word_with_without_alt_modifier` 场景的触发条件与预期结果。
#[test]
fn delete_forward_word_with_without_alt_modifier() {
    let mut t = ta_with("hello world");
    t.set_cursor(/*pos*/ 0);
    t.input(KeyEvent::new(KeyCode::Delete, KeyModifiers::ALT));
    assert_eq!(t.text(), " world");
    assert_eq!(t.cursor(), 0);

    let mut t = ta_with("hello");
    t.set_cursor(/*pos*/ 0);
    t.input(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(t.text(), "ello");
    assert_eq!(t.cursor(), 0);
}

// 验证 `delete_forward_word_alt_d` 场景的触发条件与预期结果。
#[test]
fn delete_forward_word_alt_d() {
    let mut t = ta_with("hello world");
    t.set_cursor(/*pos*/ 6);
    t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::ALT));
    pretty_assertions::assert_eq!(t.text(), "hello ");
    pretty_assertions::assert_eq!(t.cursor(), 6);
}

// 验证 `control_h_backspace` 场景的触发条件与预期结果。
#[test]
fn control_h_backspace() {
    // 按 Ctrl + H 键作为回退
    // Test Ctrl+H as backspace
    let mut t = ta_with("12345");
    t.set_cursor(/*pos*/ 3); // cursor after '3'
    t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
    assert_eq!(t.text(), "1245");
    assert_eq!(t.cursor(), 2);

    // 在测试 Ctrl+H 键时（应为无操作）
    // Test Ctrl+H at beginning (should be no-op)
    t.set_cursor(/*pos*/ 0);
    t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
    assert_eq!(t.text(), "1245");
    assert_eq!(t.cursor(), 0);

    // 在文件末尾执行 Ctrl+H
    // Test Ctrl+H at end
    t.set_cursor(t.text().len());
    t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
    assert_eq!(t.text(), "124");
    assert_eq!(t.cursor(), 3);
}

#[cfg_attr(not(windows), ignore = "AltGr modifier only applies on Windows")]
// 验证 `altgr_ctrl_alt_char_inserts_literal` 场景的触发条件与预期结果。
#[test]
fn altgr_ctrl_alt_char_inserts_literal() {
    let mut t = ta_with("");
    t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL | KeyModifiers::ALT));
    assert_eq!(t.text(), "c");
    assert_eq!(t.cursor(), 1);
}

// 验证 `cursor_vertical_movement_across_lines_and_bounds` 场景的触发条件与预期结果。
#[test]
fn cursor_vertical_movement_across_lines_and_bounds() {
    let mut t = ta_with("short\nloooooooooong\nmid");
    // 在第二行第 5 列处放置光标
    // Place cursor on second line, column 5
    let second_line_start = 6; // after first '\n'
    t.set_cursor(second_line_start + 5);

    // 上移：列目标保持不变，受行长限制
    // Move up: target column preserved, clamped by line length
    t.move_cursor_up();
    assert_eq!(t.cursor(), 5); // first line has len 5

    // 再次向上移动将回到文本起始处
    // Move up again goes to start of text
    t.move_cursor_up();
    assert_eq!(t.cursor(), 0);

    // 向下移动：从起始列追踪到目标列
    // Move down: from start to target col tracked
    t.move_cursor_down();
    // 第一次下移时，应落在第二行第 0 列（目标列已记住为 0）
    // On first move down, we should land on second line, at col 0 (target col remembered as 0)
    let pos_after_down = t.cursor();
    assert!(pos_after_down >= second_line_start);

    // 再次向下移动至第三行，并限制其长度
    // Move down again to third line; clamp to its length
    t.move_cursor_down();
    let third_line_start = t.text().find("mid").unwrap();
    let third_line_end = third_line_start + 3;
    assert!(t.cursor() >= third_line_start && t.cursor() <= third_line_end);

    // 最后一行向下移动将到达末尾
    // Moving down at last line jumps to end
    t.move_cursor_down();
    assert_eq!(t.cursor(), t.text().len());
}

// 验证 `home_end_and_emacs_style_home_end` 场景的触发条件与预期结果。
#[test]
fn home_end_and_emacs_style_home_end() {
    let mut t = ta_with("one\ntwo\nthree");
    // 第二行中间位置
    // Position at middle of second line
    let second_line_start = t.text().find("two").unwrap();
    t.set_cursor(second_line_start + 1);

    t.move_cursor_to_beginning_of_line(/*move_up_at_bol*/ false);
    assert_eq!(t.cursor(), second_line_start);

    // Ctrl+A 行为：在行首位置时，跳转到上一行的开始
    // Ctrl-A behavior: if at BOL, go to beginning of previous line
    t.move_cursor_to_beginning_of_line(/*move_up_at_bol*/ true);
    assert_eq!(t.cursor(), 0); // beginning of first line

    // 将光标移动到第一行末尾
    // Move to EOL of first line
    t.move_cursor_to_end_of_line(/*move_down_at_eol*/ false);
    assert_eq!(t.cursor(), 3);

    // Ctrl+E：若行尾为换行符，则跳转至下一行的末尾
    // Ctrl-E: if at EOL, go to end of next line
    t.move_cursor_to_end_of_line(/*move_down_at_eol*/ true);
    // 第二行末尾（即 "two"）在 \n 之前是结束符
    // end of second line ("two") is right before its '\n'
    let end_second_nl = t.text().find("\nthree").unwrap();
    assert_eq!(t.cursor(), end_second_nl);
}

// 验证 `end_of_line_or_down_at_end_of_text` 场景的触发条件与预期结果。
#[test]
fn end_of_line_or_down_at_end_of_text() {
    let mut t = ta_with("one\ntwo");
    // 将光标定位到文本末尾
    // Place cursor at absolute end of the text
    t.set_cursor(t.text().len());
    // 保持于结束处，不引发 panic
    // Should remain at end without panicking
    t.move_cursor_to_end_of_line(/*move_down_at_eol*/ true);
    assert_eq!(t.cursor(), t.text().len());

    // 在非最终行末尾时验证行为
    // Also verify behavior when at EOL of a non-final line:
    let eol_first_line = 3; // index of '\n' in "one\ntwo"
    t.set_cursor(eol_first_line);
    t.move_cursor_to_end_of_line(/*move_down_at_eol*/ true);
    assert_eq!(t.cursor(), t.text().len()); // moves to end of next (last) line
}

// 验证 `word_navigation_helpers` 场景的触发条件与预期结果。
#[test]
fn word_navigation_helpers() {
    let t = ta_with("  alpha  beta   gamma");
    let mut t = t; // make mutable for set_cursor
    // 在"alpha"之后放置光标
    // Put cursor after "alpha"
    let after_alpha = t.text().find("alpha").unwrap() + "alpha".len();
    t.set_cursor(after_alpha);
    assert_eq!(t.beginning_of_previous_word(), 2); // skip initial spaces

    // 将光标置于 Beta 的起始位置
    // Put cursor at start of beta
    let beta_start = t.text().find("beta").unwrap();
    t.set_cursor(beta_start);
    assert_eq!(t.end_of_next_word(), beta_start + "beta".len());

    // 如果结束，则返回下一个单词的结束位置长度
    // If at end, end_of_next_word returns len
    t.set_cursor(t.text().len());
    assert_eq!(t.end_of_next_word(), t.text().len());
}

// 验证 `word_navigation_cjk_each_char_is_boundary` 场景的触发条件与预期结果。
#[test]
fn word_navigation_cjk_each_char_is_boundary() {
    let text = "你好世界";
    let mut t = ta_with(text);

    t.set_cursor(/*pos*/ text.len());
    assert_eq!(t.beginning_of_previous_word(), 9);

    t.set_cursor(/*pos*/ 9);
    assert_eq!(t.beginning_of_previous_word(), 6);

    t.set_cursor(/*pos*/ 6);
    assert_eq!(t.beginning_of_previous_word(), 3);

    t.set_cursor(/*pos*/ 3);
    assert_eq!(t.beginning_of_previous_word(), 0);
}

// 验证 `word_navigation_cjk_forward` 场景的触发条件与预期结果。
#[test]
fn word_navigation_cjk_forward() {
    let text = "你好世界";
    let mut t = ta_with(text);

    t.set_cursor(/*pos*/ 0);
    assert_eq!(t.end_of_next_word(), 3);

    t.set_cursor(/*pos*/ 3);
    assert_eq!(t.end_of_next_word(), 6);

    t.set_cursor(/*pos*/ 6);
    assert_eq!(t.end_of_next_word(), 9);

    t.set_cursor(/*pos*/ 9);
    assert_eq!(t.end_of_next_word(), 12);
}

// 验证 `word_navigation_mixed_ascii_cjk` 场景的触发条件与预期结果。
#[test]
fn word_navigation_mixed_ascii_cjk() {
    let text = "hello你好";
    let mut t = ta_with(text);

    t.set_cursor(/*pos*/ 0);
    assert_eq!(t.end_of_next_word(), 5);

    t.set_cursor(/*pos*/ 5);
    assert_eq!(t.end_of_next_word(), 8);

    t.set_cursor(/*pos*/ text.len());
    assert_eq!(t.beginning_of_previous_word(), 8);

    t.set_cursor(/*pos*/ 8);
    assert_eq!(t.beginning_of_previous_word(), 5);

    t.set_cursor(/*pos*/ 5);
    assert_eq!(t.beginning_of_previous_word(), 0);
}

// 验证 `word_navigation_preserves_separator_breaks_within_unicode_segments` 场景的触发条件与预期结果。
#[test]
fn word_navigation_preserves_separator_breaks_within_unicode_segments() {
    let mut t = ta_with("can't 32.3 foo.bar");

    t.set_cursor(/*pos*/ 5);
    assert_eq!(t.beginning_of_previous_word(), 4);

    t.set_cursor(/*pos*/ 4);
    assert_eq!(t.beginning_of_previous_word(), 3);

    t.set_cursor(/*pos*/ 10);
    assert_eq!(t.beginning_of_previous_word(), 9);

    t.set_cursor(/*pos*/ 18);
    assert_eq!(t.beginning_of_previous_word(), 15);
}

// 验证 `wrapping_and_cursor_positions` 场景的触发条件与预期结果。
#[test]
fn wrapping_and_cursor_positions() {
    let mut t = ta_with("hello world here");
    let area = Rect::new(0, 0, 6, 10); // width 6 -> wraps words
    // 期望高度将包裹多行
    // desired height counts wrapped lines
    assert!(t.desired_height(area.width) >= 3);

    // 在"世界"中放置光标
    // Place cursor in "world"
    let world_start = t.text().find("world").unwrap();
    t.set_cursor(world_start + 3);
    let (_x, y) = t.cursor_pos(area).unwrap();
    assert_eq!(y, 1); // world should be on second wrapped line

    // 状态与较小高度下，光标映射到可见行
    // With state and small height, cursor is mapped onto visible row
    let mut state = TextAreaState::default();
    let small_area = Rect::new(0, 0, 6, 1);
    // 第一调用：光标不可见，因此有效滚动确保其可见
    // First call: cursor not visible -> effective scroll ensures it is
    let (_x, y) = t.cursor_pos_with_state(small_area, state).unwrap();
    assert_eq!(y, 0);

    // 以状态渲染实际滚动值
    // Render with state to update actual scroll value
    let mut buf = Buffer::empty(small_area);
    ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), small_area, &mut buf, &mut state);
    // 渲染完成后，滚动状态应调整，使光标行与内容对齐
    // After render, state.scroll should be adjusted so cursor row fits
    let effective_lines = t.desired_height(small_area.width);
    assert!(state.scroll < effective_lines);
}

// 验证 `cursor_wraps_after_filling_the_final_line` 场景的触发条件与预期结果。
#[test]
fn cursor_wraps_after_filling_the_final_line() {
    for text in ["abad", "界界", "abｶﾞ"] {
        let mut t = ta_with(text);
        t.set_cursor(t.text().len());
        let area = Rect::new(2, 3, /*width*/ 4, /*height*/ 2);

        assert_eq!(t.desired_height(area.width), 2);
        assert_eq!(t.cursor_pos(area), Some((2, 4)));
    }
}

// 验证 `full_non_final_lines_reserve_visible_cursor_rows` 场景的触发条件与预期结果。
#[test]
fn full_non_final_lines_reserve_visible_cursor_rows() {
    for text in ["abad\nef", "界界\nef", "abｶﾞ\nef"] {
        let mut t = ta_with(text);
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
        let newline = text.find('\n').unwrap();

        assert_eq!(t.desired_height(area.width), 3);

        t.set_cursor(newline);
        assert_eq!(t.cursor_pos(area), Some((0, 1)));

        t.set_cursor(newline + 1);
        assert_eq!(t.cursor_pos(area), Some((0, 2)));
    }
}

// 验证 `full_non_final_line_cursor_snapshot` 场景的触发条件与预期结果。
#[test]
fn full_non_final_line_cursor_snapshot() {
    let mut t = ta_with("abad\nef");
    t.set_cursor(/*pos*/ 4);
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();

    insta::assert_snapshot!(
        "textarea_full_non_final_line_has_a_visible_cursor_row",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );
}

// 验证 `trailing_spaces_wrap_without_letting_the_cursor_escape` 场景的触发条件与预期结果。
#[test]
fn trailing_spaces_wrap_without_letting_the_cursor_escape() {
    let mut t = ta_with("abad        ");
    t.set_cursor(t.text().len());
    let area = Rect::new(0, 0, /*width*/ 5, /*height*/ 3);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();

    assert_eq!(t.desired_height(area.width), 3);
    insta::assert_snapshot!(
        "textarea_trailing_spaces_wrap_without_cursor_overflow",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );
}

// 验证 `separator_after_full_line_hangs_before_following_text` 场景的触发条件与预期结果。
#[test]
fn separator_after_full_line_hangs_before_following_text() {
    let mut t = ta_with("abad a");
    t.set_cursor(t.text().len());
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 2);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();

    assert_eq!(t.desired_height(area.width), 2);
    assert_eq!(t.cursor_pos(area), Some((1, 1)));
    insta::assert_snapshot!(
        "textarea_separator_after_full_line_hangs_before_following_text",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );

    for text in ["界界 a", "abｶﾞ a", "abad\ta"] {
        let mut t = ta_with(text);
        t.set_cursor(t.text().len());

        assert_eq!(t.desired_height(area.width), 2);
        assert_eq!(t.cursor_pos(area), Some((1, 1)));
    }
}

// 验证 `breakable_unicode_space_hangs_before_following_text` 场景的触发条件与预期结果。
#[test]
fn breakable_unicode_space_hangs_before_following_text() {
    let mut t = ta_with("abad\u{3000}abcde");
    t.set_cursor(t.text().len());
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();

    assert_eq!(t.desired_height(area.width), 3);
    assert_eq!(t.cursor_pos(area), Some((1, 2)));
    insta::assert_snapshot!(
        "textarea_breakable_unicode_space_hangs_before_following_text",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );
}

// 验证 `leading_space_reflows_across_following_wrapped_fragments` 场景的触发条件与预期结果。
#[test]
fn leading_space_reflows_across_following_wrapped_fragments() {
    let mut t = ta_with("abad abcde wxyz");
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 5);
    assert_eq!(t.desired_height(area.width), 5);
    for (cursor, expected) in [(11, (0, 3)), (14, (3, 3)), (15, (0, 4))] {
        t.set_cursor(cursor);
        assert_eq!(t.cursor_pos(area), Some(expected));
    }

    let mut t = ta_with("abad abcde xy zz");
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 4);
    assert_eq!(t.desired_height(area.width), 4);
    for (cursor, expected) in [
        (4, (0, 1)),
        (5, (0, 1)),
        (8, (3, 1)),
        (9, (0, 2)),
        (10, (1, 2)),
        (11, (2, 2)),
        (12, (3, 2)),
        (13, (0, 3)),
        (14, (0, 3)),
        (16, (2, 3)),
    ] {
        t.set_cursor(cursor);
        assert_eq!(t.cursor_pos(area), Some(expected));
    }

    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();

    insta::assert_snapshot!(
        "textarea_leading_space_reflows_across_following_wrapped_fragments",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );
}

// 验证 `reflow_keeps_fitting_remainder_on_current_row` 场景的触发条件与预期结果。
#[test]
fn reflow_keeps_fitting_remainder_on_current_row() {
    for (text, width, expected_height, expected_cursor) in [
        ("  -", 1, 4, (0, 3)),
        ("p    a b", 3, 3, (0, 2)),
        ("p     a b", 5, 2, (3, 1)),
        (" aaaa a   ", 3, 4, (1, 3)),
    ] {
        let mut t = ta_with(text);
        t.set_cursor(t.text().len());
        let area = Rect::new(0, 0, width, expected_height);

        assert_eq!(t.desired_height(area.width), expected_height);
        assert_eq!(t.cursor_pos(area), Some(expected_cursor));
        for range in t.wrapped_lines(width).iter() {
            assert!(
                display_width(text[range.start..range.end - 1].trim_end_matches(' '))
                    <= usize::from(width)
            );
        }
    }

    let mut t = ta_with("p     a b");
    t.set_cursor(t.text().len());
    let area = Rect::new(0, 0, /*width*/ 5, /*height*/ 2);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();

    insta::assert_snapshot!(
        "textarea_reflow_keeps_fitting_remainder_on_current_row",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );
}

// 验证 `reflow_preserves_hyphenated_word_breakpoint_when_rendering` 场景的触发条件与预期结果。
#[test]
fn reflow_preserves_hyphenated_word_breakpoint_when_rendering() {
    let mut t = ta_with("a foo-barbaz");
    t.set_cursor(t.text().len());
    let area = Rect::new(0, 0, /*width*/ 10, /*height*/ 2);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();

    insta::assert_snapshot!(
        "textarea_reflow_preserves_hyphenated_word_breakpoint",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );
}

// 验证 `space_runs_hang_before_later_text` 场景的触发条件与预期结果。
#[test]
fn space_runs_hang_before_later_text() {
    let mut t = ta_with("abad     next");
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);

    assert_eq!(t.desired_height(area.width), 3);

    for cursor in 4..=9 {
        t.set_cursor(cursor);
        assert_eq!(t.cursor_pos(area), Some((0, 1)));
    }
}

// 验证 `full_final_line_scrolls_to_its_visible_cursor_row` 场景的触发条件与预期结果。
#[test]
fn full_final_line_scrolls_to_its_visible_cursor_row() {
    let mut t = ta_with("abcdefgh");
    t.set_cursor(t.text().len());
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 2);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
        })
        .unwrap();

    assert_eq!(t.desired_height(area.width), 3);
    insta::assert_snapshot!(
        "textarea_full_final_line_scrolls_to_visible_cursor_row",
        format!(
            "scroll: {}\ncursor: {:?}\n{}",
            state.scroll,
            t.cursor_pos_with_state(area, state),
            terminal.backend()
        )
    );
}

// 验证 `text_rendering_does_not_overwrite_cells_outside_the_textarea` 场景的触发条件与预期结果。
#[test]
fn text_rendering_does_not_overwrite_cells_outside_the_textarea() {
    let t = ta_with("abad     next");
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
    let mut state = TextAreaState::default();
    let mut buf = Buffer::empty(Rect::new(0, 0, /*width*/ 8, /*height*/ 3));
    let protected_style = Style::default().fg(Color::Red);
    buf.set_style(Rect::new(4, 0, /*width*/ 4, /*height*/ 3), protected_style);
    let protected_cell = buf[(4, 0)].clone();

    StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);

    assert_eq!(buf[(4, 0)], protected_cell);
}

// 验证 `styled_text_rendering_does_not_overwrite_cells_outside_the_textarea` 场景的触发条件与预期结果。
#[test]
fn styled_text_rendering_does_not_overwrite_cells_outside_the_textarea() {
    let mut snapshots = Vec::new();
    for overlay in ["element", "highlight"] {
        let mut t = ta_with("abad     next");
        if overlay == "element" {
            t.add_element_range(0..9);
        }
        let highlights = if overlay == "highlight" {
            vec![(0..9, Style::default().fg(Color::Magenta))]
        } else {
            Vec::new()
        };
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
        let mut terminal = Terminal::new(TestBackend::new(/*width*/ 8, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                frame.buffer_mut().set_string(
                    /*x*/ 4,
                    /*y*/ 0,
                    "KEEP",
                    Style::default().fg(Color::Red),
                );
                t.render_ref_styled_with_highlights(
                    area,
                    frame.buffer_mut(),
                    &mut state,
                    Style::default(),
                    &highlights,
                );
            })
            .unwrap();

        assert_eq!(terminal.backend().buffer()[(4, 0)].symbol(), "K");
        snapshots.push(format!("{overlay}\n{}", terminal.backend()));
    }

    insta::assert_snapshot!(
        "textarea_styled_overlays_stay_inside_the_textarea",
        snapshots.join("\n\n")
    );
}

// 验证 `empty_textarea_areas_do_not_have_a_cursor` 场景的触发条件与预期结果。
#[test]
fn empty_textarea_areas_do_not_have_a_cursor() {
    let t = ta_with("abad");

    assert_eq!(t.cursor_pos(Rect::new(0, 0, /*width*/ 0, /*height*/ 1)), None);
    assert_eq!(t.cursor_pos(Rect::new(0, 0, /*width*/ 1, /*height*/ 0)), None);
}

// 验证 `render_highlights_apply_style_without_mutating_text` 场景的触发条件与预期结果。
#[test]
fn render_highlights_apply_style_without_mutating_text() {
    let t = ta_with("hello world");
    let area = Rect::new(0, 0, 20, 1);
    let mut state = TextAreaState::default();
    let mut buf = Buffer::empty(area);
    let highlight_style = Style::default().add_modifier(ratatui::style::Modifier::REVERSED);

    t.render_ref_styled_with_highlights(
        area,
        &mut buf,
        &mut state,
        Style::default(),
        &[(6..11, highlight_style)],
    );

    assert_eq!(t.text(), "hello world");
    assert!(!buf[(0, 0)].style().add_modifier.contains(ratatui::style::Modifier::REVERSED));
    assert!(buf[(6, 0)].style().add_modifier.contains(ratatui::style::Modifier::REVERSED));
    assert!(buf[(10, 0)].style().add_modifier.contains(ratatui::style::Modifier::REVERSED));
}

// 验证 `tabs_render_as_spaces_and_align_with_cursor_snapshot` 场景的触发条件与预期结果。
#[test]
fn tabs_render_as_spaces_and_align_with_cursor_snapshot() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let text = "❌\tSimulation\tformatter[large/dataset.py]\t7.4 ms\t8.1 ms\t-8.29%";
    let mut t = ta_with(text);
    t.set_cursor(text.len());

    let mut terminal = Terminal::new(TestBackend::new(/*width*/ 100, /*height*/ 1)).unwrap();
    terminal
        .draw(|frame| {
            ratatui::widgets::WidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut());
        })
        .unwrap();

    let cursor = t.cursor_pos(terminal.backend().buffer().area).unwrap();
    assert_eq!(terminal.backend().buffer()[(cursor.0 - 1, cursor.1)].symbol(), "%");
    insta::assert_snapshot!(
        "textarea_tabs_render_as_spaces_and_align_with_cursor",
        format!("cursor: {cursor:?}\n{}", terminal.backend())
    );
}

// 验证 `tabs_wrap_at_their_rendered_width` 场景的触发条件与预期结果。
#[test]
fn tabs_wrap_at_their_rendered_width() {
    let text = "1234\t5";
    let mut t = ta_with(text);
    t.set_cursor(text.len());
    let area = Rect::new(0, 0, /*width*/ 5, /*height*/ 2);
    let mut buf = Buffer::empty(area);

    ratatui::widgets::WidgetRef::render_ref(&(&t), area, &mut buf);

    assert_eq!(t.desired_height(area.width), 2);
    assert_eq!(t.cursor_pos(area), Some((1, 1)));
    assert_eq!(buf[(0, 1)].symbol(), "5");
}

// 验证 `halfwidth_sound_marks_wrap_and_align_with_the_cursor` 场景的触发条件与预期结果。
#[test]
fn halfwidth_sound_marks_wrap_and_align_with_the_cursor() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let mut snapshots = Vec::new();
    for (label, text, width, height, cursor, cells) in [
        ("dakuten", "12ｶﾞx", 4, 2, (1, 1), [(2, 0, "ｶﾞ"), (0, 1, "x")]),
        ("handakuten", "abﾊﾟc", 3, 3, (0, 2), [(0, 1, "ﾊﾟ"), (2, 1, "c")]),
        ("standalone", "a ﾞb", 2, 3, (0, 2), [(0, 1, "ﾞ"), (1, 1, "b")]),
    ] {
        let mut t = ta_with(text);
        t.set_cursor(text.len());
        let area = Rect::new(0, 0, width, height);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| {
                ratatui::widgets::WidgetRef::render_ref(
                    &(&t),
                    frame.area(),
                    frame.buffer_mut(),
                );
            })
            .unwrap();

        assert_eq!(t.desired_height(area.width), height);
        assert_eq!(t.cursor_pos(area), Some(cursor));
        for (x, y, expected) in cells {
            assert_eq!(terminal.backend().buffer()[(x, y)].symbol(), expected);
        }
        snapshots.push(format!(
            "{label}\ncursor: {:?}\n{}",
            t.cursor_pos(area),
            terminal.backend()
        ));
    }

    insta::assert_snapshot!(
        "textarea_halfwidth_sound_marks_wrap_and_align_with_cursor",
        snapshots.join("\n\n")
    );
}

// 验证 `masked_graphemes_align_with_the_cursor` 场景的触发条件与预期结果。
#[test]
fn masked_graphemes_align_with_the_cursor() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let text = "界ﾞa";
    let mut t = ta_with(text);
    t.set_cursor(text.len());
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 2);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    let mut state = TextAreaState::default();
    terminal
        .draw(|frame| {
            t.render_ref_masked(frame.area(), frame.buffer_mut(), &mut state, '*');
        })
        .unwrap();

    assert_eq!(t.cursor_pos(area), Some((0, 1)));
    insta::assert_snapshot!(
        "textarea_masked_graphemes_align_with_cursor",
        format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
    );
}

// 验证 `overwide_halfwidth_sound_marks_keep_the_final_cursor_visible` 场景的触发条件与预期结果。
#[test]
fn overwide_halfwidth_sound_marks_keep_the_final_cursor_visible() {
    let area = Rect::new(0, 0, /*width*/ 2, /*height*/ 3);

    for grapheme in ["ｶﾞﾞ", "界ﾞ"] {
        let text = format!("{grapheme}ab");
        let mut t = ta_with(&text);
        t.set_cursor(text.len());

        assert_eq!(t.desired_height(area.width), 3);
        assert_eq!(t.cursor_pos(area), Some((0, 2)));

        t.set_cursor(grapheme.len());
        assert_eq!(t.cursor_pos(area), Some((0, 1)));
    }
}

// 验证 `cursor_pos_with_state_basic_and_scroll_behaviors` 场景的触发条件与预期结果。
#[test]
fn cursor_pos_with_state_basic_and_scroll_behaviors() {
    // 第 1 项：无需包裹，高度适配——滚动被忽略，Y 值直接映射。
    // Case 1: No wrapping needed, height fits — scroll ignored, y maps directly.
    let mut t = ta_with("hello world");
    t.set_cursor(/*pos*/ 3);
    let area = Rect::new(2, 5, 20, 3);
    // 即使提供荒谬的滚动，当内容填满区域时，有效滚动量为 0，且光标位置与 cursor_pos 一致。
    // Even if an absurd scroll is provided, when content fits the area the
    // effective scroll is 0 and the cursor position matches cursor_pos.
    let bad_state = TextAreaState { scroll: 999 };
    let (x1, y1) = t.cursor_pos(area).unwrap();
    let (x2, y2) = t.cursor_pos_with_state(area, bad_state).unwrap();
    assert_eq!((x2, y2), (x1, y1));

    // 第 2 步：当前窗口下方光标——y 值应在调整有效滚动后，被限制在高度为 (area.height - 1) 的行（即最后一行）
    // Case 2: Cursor below the current window — y should be clamped to the
    // bottom row (area.height - 1) after adjusting effective scroll.
    let mut t = ta_with("one two three four five six");
    // 强制将内容包裹在多条视觉行中。
    // Force wrapping to many visual lines.
    let wrap_width = 4;
    let _ = t.desired_height(wrap_width);
    // 将光标放置在窗口末尾附近，确保其始终位于第一个窗口之下。
    // Put cursor somewhere near the end so it's definitely below the first window.
    t.set_cursor(t.text().len().saturating_sub(2));
    let small_area = Rect::new(0, 0, wrap_width, 2);
    let state = TextAreaState { scroll: 0 };
    let (_x, y) = t.cursor_pos_with_state(small_area, state).unwrap();
    assert_eq!(y, small_area.y + small_area.height - 1);

    // 当提供的滚动量过大时，当前窗口上方的光标应位于第一行（索引为 0）
    // Case 3: Cursor above the current window — y should be top row (0)
    // when the provided scroll is too large.
    let mut t = ta_with("alpha beta gamma delta epsilon zeta");
    let wrap_width = 5;
    let lines = t.desired_height(wrap_width);
    // 将光标置于起始位置，使过度滚动将其移至第一行。
    // Place cursor near start so an excessive scroll moves it to top row.
    t.set_cursor(/*pos*/ 1);
    let area = Rect::new(0, 0, wrap_width, 3);
    let state = TextAreaState { scroll: lines.saturating_mul(2) };
    let (_x, y) = t.cursor_pos_with_state(area, state).unwrap();
    assert_eq!(y, area.y);
}

// 验证 `wrapped_navigation_across_visual_lines` 场景的触发条件与预期结果。
#[test]
fn wrapped_navigation_across_visual_lines() {
    let mut t = ta_with("abcdefghij");
    // 强制在宽度 4 处包裹：行 -> ['abcd', 'efgh', 'ij']
    // Force wrapping at width 4: lines -> ["abcd", "efgh", "ij"]
    let _ = t.desired_height(/*width*/ 4);

    // 从开始，向下移动应前往下一行（索引 4）的起始位置
    // From the very start, moving down should go to the start of the next wrapped line (index 4)
    t.set_cursor(/*pos*/ 0);
    t.move_cursor_down();
    assert_eq!(t.cursor(), 4);

    // 在索引 4 处应显示第二个被包裹行开始的 Cursor
    // Cursor at boundary index 4 should be displayed at start of second wrapped line
    t.set_cursor(/*pos*/ 4);
    let area = Rect::new(0, 0, 4, 10);
    let (x, y) = t.cursor_pos(area).unwrap();
    assert_eq!((x, y), (0, 1));

    // 在状态和较小的高度下，光标应在第 0 行第 0 列可见
    // With state and small height, cursor should be visible at row 0, col 0
    let small_area = Rect::new(0, 0, 4, 1);
    let state = TextAreaState::default();
    let (x, y) = t.cursor_pos_with_state(small_area, state).unwrap();
    assert_eq!((x, y), (0, 0));

    // 在 "efgh" 第二个被包裹的行中间放置光标，在字符 'g' 处
    // Place cursor in the middle of the second wrapped line ("efgh"), at 'g'
    t.set_cursor(/*pos*/ 6);
    // 上一行向上移动应停留在上一行的相同列上，索引为 2（字符 'c'）
    // Move up should go to same column on previous wrapped line -> index 2 ('c')
    t.move_cursor_up();
    assert_eq!(t.cursor(), 2);

    // 向下移动应返回到下一次行末尾的相同位置，即索引 6（'g'）
    // Move down should return to same position on the next wrapped line -> back to index 6 ('g')
    t.move_cursor_down();
    assert_eq!(t.cursor(), 6);

    // 再次向下移动应到达第三行。目标列数为 2，但该行长度为 2，因此限制为末尾
    // Move down again should go to third wrapped line. Target col is 2, but the line has len 2 -> clamp to end
    t.move_cursor_down();
    assert_eq!(t.cursor(), t.text().len());
}

// 验证 `cursor_pos_with_state_after_movements` 场景的触发条件与预期结果。
#[test]
fn cursor_pos_with_state_after_movements() {
    let mut t = ta_with("abcdefghij");
    // 将宽度 4 包裹为视觉行：abcd | efgh | ij
    // Wrap width 4 -> visual lines: abcd | efgh | ij
    let _ = t.desired_height(/*width*/ 4);
    let area = Rect::new(0, 0, 4, 2);
    let mut state = TextAreaState::default();
    let mut buf = Buffer::empty(area);

    // 从开始处开始
    // Start at beginning
    t.set_cursor(/*pos*/ 0);
    ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
    let (x, y) = t.cursor_pos_with_state(area, state).unwrap();
    assert_eq!((x, y), (0, 0));

    // 移动到第二行视觉线；应在视窗内第 1 行的最底部
    // Move down to second visual line; should be at bottom row (row 1) within 2-line viewport
    t.move_cursor_down();
    ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
    let (x, y) = t.cursor_pos_with_state(area, state).unwrap();
    assert_eq!((x, y), (0, 1));

    // 向下移动到第三行视觉线；视口滚动并保持在最底部一行
    // Move down to third visual line; viewport scrolls and keeps cursor on bottom row
    t.move_cursor_down();
    ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
    let (x, y) = t.cursor_pos_with_state(area, state).unwrap();
    assert_eq!((x, y), (0, 1));

    // 向上移动至第二行视觉线；在当前滚动状态下，该行出现在最上方
    // Move up to second visual line; with current scroll, it appears on top row
    t.move_cursor_up();
    ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
    let (x, y) = t.cursor_pos_with_state(area, state).unwrap();
    assert_eq!((x, y), (0, 0));

    // 跨移动保持列：第一行设为第 2 列，向下移动
    // Column preservation across moves: set to col 2 on first line, move down
    t.set_cursor(/*pos*/ 2);
    ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
    let (x0, y0) = t.cursor_pos_with_state(area, state).unwrap();
    assert_eq!((x0, y0), (2, 0));
    t.move_cursor_down();
    ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
    let (x1, y1) = t.cursor_pos_with_state(area, state).unwrap();
    assert_eq!((x1, y1), (2, 1));
}

// 验证 `wrapped_navigation_with_newlines_and_spaces` 场景的触发条件与预期结果。
#[test]
fn wrapped_navigation_with_newlines_and_spaces() {
    // 包含空格并显式换行以测试边界
    // Include spaces and an explicit newline to exercise boundaries
    let mut t = ta_with("word1  word2\nword3");
    // 第一行有两个分隔符，宽度为 6。
    // Width 6 leaves both separators on the first visual row.
    let _ = t.desired_height(/*width*/ 6);

    // 将光标定位到单词 'word2' 的第 1 列。
    // Put the cursor at column 1 of "word2".
    let start_word2 = t.text().find("word2").unwrap();
    t.set_cursor(start_word2 + 1);

    // 向上移动直接指向前一行，并保留列对齐。
    // Up moves directly to the preceding visual line while preserving the visual column.
    t.move_cursor_up();
    assert_eq!(t.cursor(), 1);

    // Down 返回到与 'word2' 相同的视觉列。
    // Down returns to the same visual column of "word2".
    t.move_cursor_down();
    assert_eq!(t.cursor(), start_word2 + 1);

    // 向下跨越逻辑换行符，到达下一个视觉行（"word3"）
    // Down crosses the logical newline to the next visual line ("word3").
    t.move_cursor_down();
    let start_word3 = t.text().find("word3").unwrap();
    assert_eq!(t.cursor(), start_word3 + 1);
}

// 验证 `wrapped_navigation_with_wide_graphemes` 场景的触发条件与预期结果。
#[test]
fn wrapped_navigation_with_wide_graphemes() {
    // 四指向上，每个显示宽度为 2，宽度为 3 以强制在图元边界内包裹
    // Four thumbs up, each of display width 2, with width 3 to force wrapping inside grapheme boundaries
    let mut t = ta_with("👍👍👍👍");
    let _ = t.desired_height(/*width*/ 3);

    // 在第二个 Emoji 之后放置光标
    // Put cursor after the second emoji (which should be on first wrapped line)
    t.set_cursor("👍👍".len());

    // 向下移动应到达下一行开始的位置（保持列对齐，但被限制）
    // Move down should go to the start of the next wrapped line (same column preserved but clamped)
    t.move_cursor_down();
    // 我们期望在第三个 Emoji 中着陆，或者在其开头
    // We expect to land somewhere within the third emoji or at the start of it
    let pos_after_down = t.cursor();
    assert!(pos_after_down >= "👍👍".len());

    // 向上移动将使我们回到原始位置
    // Moving up should take us back to the original position
    t.move_cursor_up();
    assert_eq!(t.cursor(), "👍👍".len());
}

#[cfg(any())]
// 验证 `fuzz_textarea_randomized` 场景的触发条件与预期结果。
#[test]
fn fuzz_textarea_randomized() {
    // 确定种子用于可重复性 在太平洋时区（PST/PDT）中根据当前日期初始化随机数生成器。这确保了 fuzz 测试在一天内保持确定性，同时仍随天而异以提高覆盖率。
    // Deterministic seed for reproducibility
    // Seed the RNG based on the current day in Pacific Time (PST/PDT). This
    // keeps the fuzz test deterministic within a day while still varying
    // day-to-day to improve coverage.
    let pst_today_seed: u64 = (chrono::Utc::now() - chrono::Duration::hours(8))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp() as u64;
    let mut rng = rand::rngs::StdRng::seed_from_u64(pst_today_seed);

    for _case in 0..500 {
        let mut ta = TextArea::new();
        let mut state = TextAreaState::default();
        // 跟踪插入的元素载荷。载荷使用字符'['和']'，这些字符rand_grapheme()不会生成，以避免意外的碰撞。
        // Track element payloads we insert. Payloads use characters '[' and ']' which
        // are not produced by rand_grapheme(), avoiding accidental collisions.
        let mut elem_texts: Vec<String> = Vec::new();
        let mut next_elem_id: usize = 0;
        // 以随机基础字符串开始
        // Start with a random base string
        let base_len = rng.random_range(0..30);
        let mut base = String::new();
        for _ in 0..base_len {
            base.push_str(&rand_grapheme(&mut rng));
        }
        ta.set_text_clearing_elements(&base);
        // 选择初始光标的有效字符边界
        // Choose a valid char boundary for initial cursor
        let mut boundaries: Vec<usize> = vec![0];
        boundaries.extend(ta.text().char_indices().map(|(i, _)| i).skip(1));
        boundaries.push(ta.text().len());
        let init = boundaries[rng.random_range(0..boundaries.len())];
        ta.set_cursor(init);

        let mut width: u16 = rng.random_range(1..=12);
        let mut height: u16 = rng.random_range(1..=4);

        for _step in 0..60 {
            // 宽度与高度通常稳定，偶尔会发生变化
            // Mostly stable width/height, occasionally change
            if rng.random_bool(0.1) {
                width = rng.random_range(1..=12);
            }
            if rng.random_bool(0.1) {
                height = rng.random_range(1..=4);
            }

            // 选择操作
            // Pick an operation
            match rng.random_range(0..18) {
                0 => {
                    // 在光标处插入一个小随机字符串
                    // insert small random string at cursor
                    let len = rng.random_range(0..6);
                    let mut s = String::new();
                    for _ in 0..len {
                        s.push_str(&rand_grapheme(&mut rng));
                    }
                    ta.insert_str(&s);
                }
                1 => {
                    // 使用小随机切片替换
                    // replace_range with small random slice
                    let mut b: Vec<usize> = vec![0];
                    b.extend(ta.text().char_indices().map(|(i, _)| i).skip(1));
                    b.push(ta.text().len());
                    let i1 = rng.random_range(0..b.len());
                    let i2 = rng.random_range(0..b.len());
                    let (start, end) =
                        if b[i1] <= b[i2] { (b[i1], b[i2]) } else { (b[i2], b[i1]) };
                    let insert_len = rng.random_range(0..=4);
                    let mut s = String::new();
                    for _ in 0..insert_len {
                        s.push_str(&rand_grapheme(&mut rng));
                    }
                    let before = ta.text().len();
                    // 如果选择的范围与元素相交，则 replace_range 将扩展至元素边界，因此默认的增量大小断言不再成立。
                    // If the chosen range intersects an element, replace_range will expand to
                    // element boundaries, so the naive size delta assertion does not hold.
                    let intersects_element = elem_texts.iter().any(|payload| {
                        if let Some(pstart) = ta.text().find(payload) {
                            let pend = pstart + payload.len();
                            pstart < end && pend > start
                        } else {
                            false
                        }
                    });
                    ta.replace_range(start..end, &s);
                    if !intersects_element {
                        let after = ta.text().len();
                        assert_eq!(
                            after as isize,
                            before as isize + (s.len() as isize) - ((end - start) as isize)
                        );
                    }
                }
                2 => ta.delete_backward(rng.random_range(0..=3)),
                3 => ta.delete_forward(rng.random_range(0..=3)),
                4 => ta.delete_backward_word(),
                5 => ta.kill_to_beginning_of_line(),
                6 => ta.kill_to_end_of_line(),
                7 => ta.move_cursor_left(),
                8 => ta.move_cursor_right(),
                9 => ta.move_cursor_up(),
                10 => ta.move_cursor_down(),
                11 => ta.move_cursor_to_beginning_of_line(/*move_up_at_bol*/ true),
                12 => ta.move_cursor_to_end_of_line(/*move_down_at_eol*/ true),
                13 => {
                    // 插入一个具有唯一占位符 Payload 的元素
                    // Insert an element with a unique sentinel payload
                    let payload =
                        format!("[[EL#{}:{}]]", next_elem_id, rng.random_range(1000..9999));
                    next_elem_id += 1;
                    ta.insert_element(&payload);
                    elem_texts.push(payload);
                }
                14 => {
                    // 在现有元素内部尝试插入内容（应限制到边界）
                    // Try inserting inside an existing element (should clamp to boundary)
                    if let Some(payload) = elem_texts.choose(&mut rng).cloned()
                        && let Some(start) = ta.text().find(&payload)
                    {
                        let end = start + payload.len();
                        if end - start > 2 {
                            let pos = rng.random_range(start + 1..end - 1);
                            let ins = rand_grapheme(&mut rng);
                            ta.insert_str_at(pos, &ins);
                        }
                    }
                }
                15 => {
                    // 将包含元素范围的替换整个元素
                    // Replace a range that intersects an element -> whole element should be replaced
                    if let Some(payload) = elem_texts.choose(&mut rng).cloned()
                        && let Some(start) = ta.text().find(&payload)
                    {
                        let end = start + payload.len();
                        // 创建重叠的区间 [start-δ, end-δ2)
                        // Create an intersecting range [start-δ, end-δ2)
                        let mut s = start.saturating_sub(rng.random_range(0..=2));
                        let mut e = (end + rng.random_range(0..=2)).min(ta.text().len());
                        // 将字符串对齐以满足 replace_range 契约
                        // Align to char boundaries to satisfy String::replace_range contract
                        let txt = ta.text();
                        while s > 0 && !txt.is_char_boundary(s) {
                            s -= 1;
                        }
                        while e < txt.len() && !txt.is_char_boundary(e) {
                            e += 1;
                        }
                        if s < e {
                            // 小替换文本
                            // Small replacement text
                            let mut srep = String::new();
                            for _ in 0..rng.random_range(0..=2) {
                                srep.push_str(&rand_grapheme(&mut rng));
                            }
                            ta.replace_range(s..e, &srep);
                        }
                    }
                }
                16 => {
                    // 尝试将光标定位到元素内部；它应限制其范围
                    // Try setting the cursor to a position inside an element; it should clamp out
                    if let Some(payload) = elem_texts.choose(&mut rng).cloned()
                        && let Some(start) = ta.text().find(&payload)
                    {
                        let end = start + payload.len();
                        if end - start > 2 {
                            let pos = rng.random_range(start + 1..end - 1);
                            ta.set_cursor(pos);
                        }
                    }
                }
                _ => {
                    // 跳转到单词边界
                    // Jump to word boundaries
                    if rng.random_bool(0.5) {
                        let p = ta.beginning_of_previous_word();
                        ta.set_cursor(p);
                    } else {
                        let p = ta.end_of_next_word();
                        ta.set_cursor(p);
                    }
                }
            }

            // 安全性规范
            // Sanity invariants
            assert!(ta.cursor() <= ta.text().len());

            // 元素不变量
            // Element invariants
            for payload in &elem_texts {
                if let Some(start) = ta.text().find(payload) {
                    let end = start + payload.len();
                    // 元素内部的内容与初始设置的 Payload 一致
                    // 1) Text inside elements matches the initially set payload
                    assert_eq!(&ta.text()[start..end], payload);
                    // 光标始终不严格位于元素内部
                    // 2) Cursor is never strictly inside an element
                    let c = ta.cursor();
                    assert!(
                        c <= start || c >= end,
                        "cursor inside element: {start}..{end} at {c}"
                    );
                }
            }

            // 渲染并计算光标位置；确保其位于有效范围内，避免崩溃
            // Render and compute cursor positions; ensure they are in-bounds and do not panic
            let area = Rect::new(0, 0, width, height);
            // 将状态转换为足够容纳所有嵌套行的区域
            // Stateless render into an area tall enough for all wrapped lines
            let total_lines = ta.desired_height(width);
            let full_area = Rect::new(0, 0, width, total_lines.max(1));
            let mut buf = Buffer::empty(full_area);
            ratatui::widgets::WidgetRef::render_ref(&(&ta), full_area, &mut buf);

            // 光标辅助功能必须保持插入点在视口内。
            // Both cursor helpers must keep the insertion point inside the viewport.
            for (x, y) in [ta.cursor_pos(area), ta.cursor_pos_with_state(area, state)]
                .into_iter()
                .flatten()
            {
                assert!(area.contains((x, y).into()), "cursor escaped {area:?}: ({x}, {y})");
            }

            // 状态渲染不应导致崩溃，且滚动更新应平滑
            // Stateful render should not panic, and updates scroll
            let mut sbuf = Buffer::empty(area);
            ratatui::widgets::StatefulWidgetRef::render_ref(
                &(&ta),
                area,
                &mut sbuf,
                &mut state,
            );

            // 在渲染完成后，所需的行数等于不滚动时显示的行数。
            // After wrapping, desired height equals the number of lines we would render without scroll
            let total_lines = total_lines as usize;
            // 当内容在高度内时，滚动状态不能超过总行数
            // state.scroll must not exceed total_lines when content fits within area height
            if (height as usize) >= total_lines {
                assert_eq!(state.scroll, 0);
            }
        }
    }
}
