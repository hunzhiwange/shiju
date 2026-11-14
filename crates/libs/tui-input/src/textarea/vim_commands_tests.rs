//! 验证文本域 Vim 普通模式命令、待定操作符状态、按键重绑与字素边界。

use super::super::TextArea;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;

// 构造指定光标位置的 Vim 普通模式文本域，供命令测试复用。
fn vim_textarea(text: &str, cursor: usize) -> TextArea {
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.set_cursor(cursor);
    textarea.set_vim_enabled(/*enabled*/ true);
    textarea
}

// 将紧凑命令字符串逐键送入文本域，其中换行字符模拟 Enter。
fn keys(textarea: &mut TextArea, keys: &str) {
    for key in keys.chars() {
        let code = if key == '\n' { KeyCode::Enter } else { KeyCode::Char(key) };
        textarea.input(KeyEvent::new(code, KeyModifiers::NONE));
    }
}

// 发送 Esc 以取消待定命令或从插入模式返回普通模式。
fn escape(textarea: &mut TextArea) {
    textarea.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
}

#[test]
// `r` 替换完整字素并留在普通模式，也支持换行及取消未完成替换。
fn replace_character_preserves_normal_mode_and_grapheme_boundaries() {
    let mut textarea = vim_textarea("a👩‍💻c", /*cursor*/ 1);
    keys(&mut textarea, "rZ");
    assert_eq!(textarea.text(), "aZc");
    assert_eq!(textarea.cursor(), 1);
    assert_eq!(textarea.vim_mode_label(), Some("Normal"));

    keys(&mut textarea, "r");
    assert_eq!(textarea.keymap_context(), crate::keymap::KeymapContext::Editor);
    escape(&mut textarea);
    assert_eq!(textarea.text(), "aZc");

    keys(&mut textarea, "r\n");
    assert_eq!(textarea.text(), "a\nc");
    assert_eq!(textarea.cursor(), 2);

    #[cfg(windows)]
    {
        keys(&mut textarea, "r");
        textarea
            .input(KeyEvent::new(KeyCode::Char('@'), KeyModifiers::CONTROL | KeyModifiers::ALT));
        assert_eq!(textarea.text(), "a\n@");
    }
}

#[test]
// 替换字符命令服从运行时重绑，清空绑定后原按键不再触发替换。
fn replace_character_uses_configured_binding_and_supports_unbinding() {
    let mut textarea = vim_textarea("abc", /*cursor*/ 0);
    let mut keymap = crate::keymap::RuntimeKeymap::defaults();
    keymap.vim_normal.replace_char = vec![crate::key_hint::plain(KeyCode::Char('z'))];
    textarea.set_keymap_bindings(&keymap);

    keys(&mut textarea, "zQ");
    assert_eq!(textarea.text(), "Qbc");

    keymap.vim_normal.replace_char.clear();
    textarea.set_keymap_bindings(&keymap);
    keys(&mut textarea, "zR");
    assert_eq!(textarea.text(), "Qbc");
}

#[test]
// `c` 覆盖单词、行尾、整行和上下行移动，并在有效修改后进入插入模式。
fn change_accepts_word_line_and_repeated_operator_motions() {
    let mut textarea = vim_textarea("hello world\nnext", /*cursor*/ 0);
    keys(&mut textarea, "cw");
    assert_eq!(textarea.text(), " world\nnext");
    assert_eq!(textarea.vim_mode_label(), Some("Insert"));

    let mut textarea = vim_textarea("hello world\nnext", /*cursor*/ 1);
    keys(&mut textarea, "c$");
    assert_eq!(textarea.text(), "h\nnext");
    assert_eq!(textarea.vim_mode_label(), Some("Insert"));

    let mut textarea = vim_textarea("first\nsecond\nthird", /*cursor*/ 8);
    keys(&mut textarea, "cc");
    assert_eq!(textarea.text(), "first\n\nthird");
    assert_eq!(textarea.cursor(), "first\n".len());
    keys(&mut textarea, "X");
    escape(&mut textarea);
    keys(&mut textarea, "p");
    assert_eq!(textarea.text(), "first\nX\nsecond\nthird");

    for (text, cursor, command, expected) in [
        ("a b", 0, "cw", " b"),
        ("\nnext", 0, "cw", "\nnext"),
        ("one\ntwo\nthree", 0, "cj", "\nthree"),
        ("one\ntwo\nthree", 8, "ck", "one\n"),
        ("one\ntwo\n", 8, "ck", "one\n"),
        ("\nnext", 0, "c$", "\nnext"),
    ] {
        let mut textarea = vim_textarea(text, cursor);
        keys(&mut textarea, command);
        assert_eq!(textarea.text(), expected);
        assert_eq!(textarea.vim_mode_label(), Some("Insert"));
    }
    for (cursor, command) in [(0, "ck"), (4, "cj")] {
        let mut textarea = vim_textarea("one\ntwo", cursor);
        keys(&mut textarea, command);
        assert_eq!((textarea.text(), textarea.vim_mode_label()), ("one\ntwo", Some("Normal")));
    }

    let mut textarea = vim_textarea("hello world", /*cursor*/ 0);
    textarea.vim_operator_keymap.motion_word_forward =
        vec![crate::key_hint::plain(KeyCode::Char('c'))];
    keys(&mut textarea, "cc");
    assert_eq!(textarea.text(), " world");
}

#[test]
// 待输入替换字符时 Esc 由文本域消费并取消命令，不能上浮为会话中断。
fn pending_replacement_owns_escape_before_turn_interruption() {
    let mut textarea = vim_textarea("alpha", /*cursor*/ 0);
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert!(!textarea.should_handle_vim_insert_escape(escape));

    keys(&mut textarea, "r");
    assert!(textarea.should_handle_vim_insert_escape(escape));
    textarea.input(escape);
    assert!(!textarea.should_handle_vim_insert_escape(escape));
    assert_eq!(textarea.text(), "alpha");
}

#[test]
// 快照记录连续替换与修改命令后的正文和光标位置，提供可读回归基线。
fn editor_commands_have_visual_snapshot_coverage() {
    let mut textarea = vim_textarea("alpha beta\ngamma delta", /*cursor*/ 0);
    let mut states = Vec::new();
    for command in ["rZ", "cw"] {
        keys(&mut textarea, command);
        states.push(format!(
            "{command}: {}\n{}^",
            textarea.text().replace('\n', "\\n"),
            " ".repeat(textarea.cursor())
        ));
    }
    insta::assert_snapshot!(states.join("\n\n"), @r###"
    rZ: Zlpha beta\ngamma delta
    ^

    cw:  beta\ngamma delta
    ^
    "###);
}
