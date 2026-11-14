//! 复制编辑器使用的最小运行时键位；完整 Codex 键位负责配置解析，刻意位于 v3 输入核心边界之外，
//! 此处默认值只用于保持编辑器行为自包含。
//! Minimal runtime keymap for the copied editor. The full Codex keymap owns
//! configuration parsing and is intentionally outside the v3 input-core
//! boundary; these defaults keep the editor's behavior self-contained.

use crate::key_hint::{self, KeyBinding};
use crossterm::event::KeyCode;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeymapContext {
    Editor,
    VimNormal,
    VimOperator,
    VimTextObject,
}

#[derive(Clone, Debug)]
pub struct EditorKeymap {
    pub insert_newline: Vec<KeyBinding>,
    pub move_left: Vec<KeyBinding>,
    pub move_right: Vec<KeyBinding>,
    pub move_up: Vec<KeyBinding>,
    pub move_down: Vec<KeyBinding>,
    pub move_word_left: Vec<KeyBinding>,
    pub move_word_right: Vec<KeyBinding>,
    pub move_line_start: Vec<KeyBinding>,
    pub move_line_end: Vec<KeyBinding>,
    pub delete_backward: Vec<KeyBinding>,
    pub delete_forward: Vec<KeyBinding>,
    pub delete_backward_word: Vec<KeyBinding>,
    pub delete_forward_word: Vec<KeyBinding>,
    pub kill_line_start: Vec<KeyBinding>,
    pub kill_whole_line: Vec<KeyBinding>,
    pub kill_line_end: Vec<KeyBinding>,
    pub yank: Vec<KeyBinding>,
}

#[derive(Clone, Debug, Default)]
pub struct VimNormalKeymap {
    pub enter_insert: Vec<KeyBinding>,
    pub append_after_cursor: Vec<KeyBinding>,
    pub append_line_end: Vec<KeyBinding>,
    pub insert_line_start: Vec<KeyBinding>,
    pub open_line_below: Vec<KeyBinding>,
    pub open_line_above: Vec<KeyBinding>,
    pub move_left: Vec<KeyBinding>,
    pub move_right: Vec<KeyBinding>,
    pub move_up: Vec<KeyBinding>,
    pub move_down: Vec<KeyBinding>,
    pub move_word_forward: Vec<KeyBinding>,
    pub move_word_backward: Vec<KeyBinding>,
    pub move_word_end: Vec<KeyBinding>,
    pub move_line_start: Vec<KeyBinding>,
    pub move_line_end: Vec<KeyBinding>,
    pub delete_char: Vec<KeyBinding>,
    pub replace_char: Vec<KeyBinding>,
    pub substitute_char: Vec<KeyBinding>,
    pub delete_to_line_end: Vec<KeyBinding>,
    pub change_to_line_end: Vec<KeyBinding>,
    pub yank_line: Vec<KeyBinding>,
    pub paste_after: Vec<KeyBinding>,
    pub start_delete_operator: Vec<KeyBinding>,
    pub start_yank_operator: Vec<KeyBinding>,
    pub start_change_operator: Vec<KeyBinding>,
    pub cancel_operator: Vec<KeyBinding>,
}

#[derive(Clone, Debug, Default)]
pub struct VimOperatorKeymap {
    pub delete_line: Vec<KeyBinding>,
    pub yank_line: Vec<KeyBinding>,
    pub motion_left: Vec<KeyBinding>,
    pub motion_right: Vec<KeyBinding>,
    pub motion_up: Vec<KeyBinding>,
    pub motion_down: Vec<KeyBinding>,
    pub motion_word_forward: Vec<KeyBinding>,
    pub motion_word_backward: Vec<KeyBinding>,
    pub motion_word_end: Vec<KeyBinding>,
    pub motion_line_start: Vec<KeyBinding>,
    pub motion_line_end: Vec<KeyBinding>,
    pub select_inner_text_object: Vec<KeyBinding>,
    pub select_around_text_object: Vec<KeyBinding>,
    pub cancel: Vec<KeyBinding>,
}

#[derive(Clone, Debug, Default)]
pub struct VimTextObjectKeymap {
    pub word: Vec<KeyBinding>,
    pub big_word: Vec<KeyBinding>,
    pub parentheses: Vec<KeyBinding>,
    pub brackets: Vec<KeyBinding>,
    pub braces: Vec<KeyBinding>,
    pub double_quote: Vec<KeyBinding>,
    pub single_quote: Vec<KeyBinding>,
    pub backtick: Vec<KeyBinding>,
    pub cancel: Vec<KeyBinding>,
}

#[derive(Clone, Debug)]
pub struct RuntimeKeymap {
    pub editor: Arc<EditorKeymap>,
    pub vim_normal: VimNormalKeymap,
    pub vim_operator: VimOperatorKeymap,
    pub vim_text_object: VimTextObjectKeymap,
}

fn plain(key: KeyCode) -> KeyBinding {
    key_hint::plain(key)
}
fn ctrl(key: KeyCode) -> KeyBinding {
    key_hint::ctrl(key)
}
fn alt(key: KeyCode) -> KeyBinding {
    key_hint::alt(key)
}
fn shift(key: KeyCode) -> KeyBinding {
    key_hint::shift(key)
}

impl RuntimeKeymap {
    pub fn defaults() -> Self {
        Self {
            editor: Arc::new(EditorKeymap {
                insert_newline: vec![
                    ctrl(KeyCode::Char('j')),
                    ctrl(KeyCode::Char('m')),
                    plain(KeyCode::Enter),
                    shift(KeyCode::Enter),
                    alt(KeyCode::Enter),
                ],
                move_left: vec![plain(KeyCode::Left), ctrl(KeyCode::Char('b'))],
                move_right: vec![plain(KeyCode::Right), ctrl(KeyCode::Char('f'))],
                move_up: vec![plain(KeyCode::Up), ctrl(KeyCode::Char('p'))],
                move_down: vec![plain(KeyCode::Down), ctrl(KeyCode::Char('n'))],
                move_word_left: vec![
                    alt(KeyCode::Char('b')),
                    alt(KeyCode::Left),
                    ctrl(KeyCode::Left),
                ],
                move_word_right: vec![
                    alt(KeyCode::Char('f')),
                    alt(KeyCode::Right),
                    ctrl(KeyCode::Right),
                ],
                move_line_start: vec![plain(KeyCode::Home), ctrl(KeyCode::Char('a'))],
                move_line_end: vec![plain(KeyCode::End), ctrl(KeyCode::Char('e'))],
                delete_backward: vec![
                    plain(KeyCode::Backspace),
                    shift(KeyCode::Backspace),
                    ctrl(KeyCode::Char('h')),
                ],
                delete_forward: vec![
                    plain(KeyCode::Delete),
                    shift(KeyCode::Delete),
                    ctrl(KeyCode::Char('d')),
                ],
                delete_backward_word: vec![
                    alt(KeyCode::Backspace),
                    ctrl(KeyCode::Backspace),
                    KeyBinding::new(
                        KeyCode::Backspace,
                        crossterm::event::KeyModifiers::CONTROL
                            | crossterm::event::KeyModifiers::SHIFT,
                    ),
                    ctrl(KeyCode::Char('w')),
                    KeyBinding::new(
                        KeyCode::Char('h'),
                        crossterm::event::KeyModifiers::CONTROL
                            | crossterm::event::KeyModifiers::ALT,
                    ),
                ],
                delete_forward_word: vec![
                    alt(KeyCode::Delete),
                    ctrl(KeyCode::Delete),
                    KeyBinding::new(
                        KeyCode::Delete,
                        crossterm::event::KeyModifiers::CONTROL
                            | crossterm::event::KeyModifiers::SHIFT,
                    ),
                    alt(KeyCode::Char('d')),
                ],
                kill_line_start: vec![ctrl(KeyCode::Char('u'))],
                kill_whole_line: Vec::new(),
                kill_line_end: vec![ctrl(KeyCode::Char('k'))],
                yank: vec![ctrl(KeyCode::Char('y'))],
            }),
            vim_normal: VimNormalKeymap {
                enter_insert: vec![plain(KeyCode::Char('i')), plain(KeyCode::Insert)],
                append_after_cursor: vec![plain(KeyCode::Char('a'))],
                append_line_end: vec![shift(KeyCode::Char('a')), plain(KeyCode::Char('A'))],
                insert_line_start: vec![shift(KeyCode::Char('i')), plain(KeyCode::Char('I'))],
                open_line_below: vec![plain(KeyCode::Char('o'))],
                open_line_above: vec![shift(KeyCode::Char('o')), plain(KeyCode::Char('O'))],
                move_left: vec![plain(KeyCode::Char('h')), plain(KeyCode::Left)],
                move_right: vec![plain(KeyCode::Char('l')), plain(KeyCode::Right)],
                move_up: vec![plain(KeyCode::Char('k')), plain(KeyCode::Up)],
                move_down: vec![plain(KeyCode::Char('j')), plain(KeyCode::Down)],
                move_word_forward: vec![plain(KeyCode::Char('w'))],
                move_word_backward: vec![plain(KeyCode::Char('b'))],
                move_word_end: vec![plain(KeyCode::Char('e'))],
                move_line_start: vec![plain(KeyCode::Char('0'))],
                move_line_end: vec![plain(KeyCode::Char('$')), shift(KeyCode::Char('$'))],
                delete_char: vec![plain(KeyCode::Char('x'))],
                replace_char: vec![plain(KeyCode::Char('r'))],
                substitute_char: vec![plain(KeyCode::Char('s'))],
                delete_to_line_end: vec![shift(KeyCode::Char('d')), plain(KeyCode::Char('D'))],
                change_to_line_end: vec![shift(KeyCode::Char('c')), plain(KeyCode::Char('C'))],
                yank_line: vec![shift(KeyCode::Char('y')), plain(KeyCode::Char('Y'))],
                paste_after: vec![plain(KeyCode::Char('p'))],
                start_delete_operator: vec![plain(KeyCode::Char('d'))],
                start_yank_operator: vec![plain(KeyCode::Char('y'))],
                start_change_operator: vec![plain(KeyCode::Char('c'))],
                cancel_operator: vec![plain(KeyCode::Esc)],
            },
            vim_operator: VimOperatorKeymap {
                delete_line: vec![plain(KeyCode::Char('d'))],
                yank_line: vec![plain(KeyCode::Char('y'))],
                motion_left: vec![plain(KeyCode::Char('h'))],
                motion_right: vec![plain(KeyCode::Char('l'))],
                motion_up: vec![plain(KeyCode::Char('k'))],
                motion_down: vec![plain(KeyCode::Char('j'))],
                motion_word_forward: vec![plain(KeyCode::Char('w'))],
                motion_word_backward: vec![plain(KeyCode::Char('b'))],
                motion_word_end: vec![plain(KeyCode::Char('e'))],
                motion_line_start: vec![plain(KeyCode::Char('0'))],
                motion_line_end: vec![plain(KeyCode::Char('$')), shift(KeyCode::Char('$'))],
                select_inner_text_object: vec![plain(KeyCode::Char('i'))],
                select_around_text_object: vec![plain(KeyCode::Char('a'))],
                cancel: vec![plain(KeyCode::Esc)],
            },
            vim_text_object: VimTextObjectKeymap {
                word: vec![plain(KeyCode::Char('w'))],
                big_word: vec![shift(KeyCode::Char('w')), plain(KeyCode::Char('W'))],
                parentheses: vec![
                    plain(KeyCode::Char('(')),
                    shift(KeyCode::Char('(')),
                    plain(KeyCode::Char(')')),
                    shift(KeyCode::Char(')')),
                    plain(KeyCode::Char('b')),
                ],
                brackets: vec![plain(KeyCode::Char('[')), plain(KeyCode::Char(']'))],
                braces: vec![
                    plain(KeyCode::Char('{')),
                    shift(KeyCode::Char('{')),
                    plain(KeyCode::Char('}')),
                    shift(KeyCode::Char('}')),
                    shift(KeyCode::Char('b')),
                    plain(KeyCode::Char('B')),
                ],
                double_quote: vec![plain(KeyCode::Char('"')), shift(KeyCode::Char('"'))],
                single_quote: vec![plain(KeyCode::Char('\''))],
                backtick: vec![plain(KeyCode::Char('`'))],
                cancel: vec![plain(KeyCode::Esc)],
            },
        }
    }
}
