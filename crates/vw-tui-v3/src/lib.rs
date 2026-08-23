//! Compilation shell for the copied VibeWindow TUI v3 sources.
//!
//! VibeWindow adaptation: activate copied Codex modules in migration batches so
//! their original rendering behavior and tests remain the source of truth.

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![deny(clippy::disallowed_methods)]

#[allow(dead_code)]
mod key_hint;

/// Minimal runtime keymap for the copied editor. The full Codex keymap owns
/// configuration parsing and is intentionally outside the v3 input-core
/// boundary; these defaults keep the editor's behavior self-contained.
#[allow(dead_code)]
mod keymap {
    use crate::key_hint::{self, KeyBinding};
    use crossterm::event::KeyCode;
    use std::sync::Arc;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum KeymapContext {
        Editor,
        VimNormal,
        VimOperator,
        VimTextObject,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct EditorKeymap {
        pub(crate) insert_newline: Vec<KeyBinding>,
        pub(crate) move_left: Vec<KeyBinding>,
        pub(crate) move_right: Vec<KeyBinding>,
        pub(crate) move_up: Vec<KeyBinding>,
        pub(crate) move_down: Vec<KeyBinding>,
        pub(crate) move_word_left: Vec<KeyBinding>,
        pub(crate) move_word_right: Vec<KeyBinding>,
        pub(crate) move_line_start: Vec<KeyBinding>,
        pub(crate) move_line_end: Vec<KeyBinding>,
        pub(crate) delete_backward: Vec<KeyBinding>,
        pub(crate) delete_forward: Vec<KeyBinding>,
        pub(crate) delete_backward_word: Vec<KeyBinding>,
        pub(crate) delete_forward_word: Vec<KeyBinding>,
        pub(crate) kill_line_start: Vec<KeyBinding>,
        pub(crate) kill_whole_line: Vec<KeyBinding>,
        pub(crate) kill_line_end: Vec<KeyBinding>,
        pub(crate) yank: Vec<KeyBinding>,
    }

    #[derive(Clone, Debug, Default)]
    pub(crate) struct VimNormalKeymap {
        pub(crate) enter_insert: Vec<KeyBinding>,
        pub(crate) append_after_cursor: Vec<KeyBinding>,
        pub(crate) append_line_end: Vec<KeyBinding>,
        pub(crate) insert_line_start: Vec<KeyBinding>,
        pub(crate) open_line_below: Vec<KeyBinding>,
        pub(crate) open_line_above: Vec<KeyBinding>,
        pub(crate) move_left: Vec<KeyBinding>,
        pub(crate) move_right: Vec<KeyBinding>,
        pub(crate) move_up: Vec<KeyBinding>,
        pub(crate) move_down: Vec<KeyBinding>,
        pub(crate) move_word_forward: Vec<KeyBinding>,
        pub(crate) move_word_backward: Vec<KeyBinding>,
        pub(crate) move_word_end: Vec<KeyBinding>,
        pub(crate) move_line_start: Vec<KeyBinding>,
        pub(crate) move_line_end: Vec<KeyBinding>,
        pub(crate) delete_char: Vec<KeyBinding>,
        pub(crate) replace_char: Vec<KeyBinding>,
        pub(crate) substitute_char: Vec<KeyBinding>,
        pub(crate) delete_to_line_end: Vec<KeyBinding>,
        pub(crate) change_to_line_end: Vec<KeyBinding>,
        pub(crate) yank_line: Vec<KeyBinding>,
        pub(crate) paste_after: Vec<KeyBinding>,
        pub(crate) start_delete_operator: Vec<KeyBinding>,
        pub(crate) start_yank_operator: Vec<KeyBinding>,
        pub(crate) start_change_operator: Vec<KeyBinding>,
        pub(crate) cancel_operator: Vec<KeyBinding>,
    }

    #[derive(Clone, Debug, Default)]
    pub(crate) struct VimOperatorKeymap {
        pub(crate) delete_line: Vec<KeyBinding>,
        pub(crate) yank_line: Vec<KeyBinding>,
        pub(crate) motion_left: Vec<KeyBinding>,
        pub(crate) motion_right: Vec<KeyBinding>,
        pub(crate) motion_up: Vec<KeyBinding>,
        pub(crate) motion_down: Vec<KeyBinding>,
        pub(crate) motion_word_forward: Vec<KeyBinding>,
        pub(crate) motion_word_backward: Vec<KeyBinding>,
        pub(crate) motion_word_end: Vec<KeyBinding>,
        pub(crate) motion_line_start: Vec<KeyBinding>,
        pub(crate) motion_line_end: Vec<KeyBinding>,
        pub(crate) select_inner_text_object: Vec<KeyBinding>,
        pub(crate) select_around_text_object: Vec<KeyBinding>,
        pub(crate) cancel: Vec<KeyBinding>,
    }

    #[derive(Clone, Debug, Default)]
    pub(crate) struct VimTextObjectKeymap {
        pub(crate) word: Vec<KeyBinding>,
        pub(crate) big_word: Vec<KeyBinding>,
        pub(crate) parentheses: Vec<KeyBinding>,
        pub(crate) brackets: Vec<KeyBinding>,
        pub(crate) braces: Vec<KeyBinding>,
        pub(crate) double_quote: Vec<KeyBinding>,
        pub(crate) single_quote: Vec<KeyBinding>,
        pub(crate) backtick: Vec<KeyBinding>,
        pub(crate) cancel: Vec<KeyBinding>,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct RuntimeKeymap {
        pub(crate) editor: Arc<EditorKeymap>,
        pub(crate) vim_normal: VimNormalKeymap,
        pub(crate) vim_operator: VimOperatorKeymap,
        pub(crate) vim_text_object: VimTextObjectKeymap,
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
        pub(crate) fn defaults() -> Self {
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
}

#[allow(dead_code)]
mod input_types {
    pub(crate) const MAX_USER_INPUT_TEXT_CHARS: usize = 1 << 20;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct ByteRange {
        pub(crate) start: usize,
        pub(crate) end: usize,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct TextElement {
        pub(crate) byte_range: ByteRange,
        placeholder: Option<String>,
    }

    impl TextElement {
        pub(crate) fn new(byte_range: ByteRange, placeholder: Option<String>) -> Self {
            Self { byte_range, placeholder }
        }

        pub(crate) fn placeholder<'a>(&'a self, text: &'a str) -> Option<&'a str> {
            self.placeholder
                .as_deref()
                .or_else(|| text.get(self.byte_range.start..self.byte_range.end))
        }
    }
}

#[path = "bottom_pane/mod.rs"]
#[allow(dead_code)]
pub(crate) mod bottom_pane;
#[allow(dead_code)]
pub mod chatwidget;

// VibeWindow adaptation: copied modules are activated before their downstream
// consumers, so staged dead-code allowances stay scoped to this batch only.
#[allow(dead_code)]
mod color;
#[allow(dead_code)]
mod custom_terminal;
#[allow(dead_code)]
pub mod diff_model;
#[allow(dead_code)]
mod diff_render;
#[allow(dead_code)]
pub mod exec_cell;
#[allow(dead_code)]
mod git_action_directives;
#[allow(dead_code)]
mod goal_display;
pub mod history_cell;
#[allow(dead_code)]
mod insert_history;
#[allow(dead_code)]
mod line_truncation;
#[allow(dead_code)]
pub(crate) mod live_wrap;
#[allow(dead_code)]
mod markdown;
#[allow(dead_code)]
mod markdown_render;
#[allow(dead_code)]
mod markdown_stream;
#[allow(dead_code)]
mod markdown_text_merge;
#[allow(dead_code)]
mod motion;
#[allow(dead_code)]
mod pager_overlay;
#[allow(dead_code)]
pub mod onboarding;
#[allow(dead_code, unused_imports)]
mod render;
#[allow(dead_code)]
pub(crate) mod resume_picker;
#[allow(dead_code)]
pub mod selection_list;
#[allow(dead_code)]
mod streaming;
#[allow(dead_code)]
mod style;
#[allow(dead_code)]
pub mod status;
#[allow(dead_code)]
mod status_indicator_widget;
#[allow(dead_code)]
mod table_detect;
#[allow(dead_code)]
mod terminal_hyperlinks;
#[allow(dead_code)]
mod terminal_palette;
#[allow(dead_code, unused_imports)]
mod terminal_probe;
#[allow(dead_code)]
mod text_formatting;
#[allow(dead_code)]
mod thread_transcript;
#[allow(dead_code)]
mod transcript_reflow;
#[allow(dead_code)]
mod resize_reflow_cap;
#[allow(dead_code)]
mod tui;
#[allow(dead_code)]
pub(crate) mod theme_picker;
#[allow(dead_code)]
mod ui_consts;
pub mod vibe;
#[allow(dead_code)]
mod width;
#[allow(dead_code)]
mod wrapping;

#[cfg(test)]
mod test_backend;

#[cfg(test)]
#[path = "../tests/upstream_snapshot_matrix.rs"]
mod upstream_snapshot_matrix;

pub use chatwidget::{ChatWidget, ChatWidgetConfig, V3ChatStatus};
pub use custom_terminal::Terminal as CustomTerminal;
pub use history_cell::{ExecHistoryCell, HistoryCell, history_cell_for, render_display_event};
pub use insert_history::{HistoryLineWrapPolicy, insert_history_lines, insert_history_lines_with_wrap_policy};
pub use pager_overlay::TranscriptPager;
pub use resize_reflow_cap::{ResizeReflowCap, resize_reflow_max_rows};
pub use status::{StatusHistoryCell, V3SessionDisplayData, V3TokenUsage};
pub use tui::{TerminalMode, run_terminal};
pub use transcript_reflow::{TranscriptReflowState, TranscriptWidthChange};
pub use vibe::action::{
    V3_NON_SECURITY_SELECTION_SURFACES, V3Action, V3ApprovalChoice, V3ApprovalDecision,
    V3ApprovalKind, V3ApprovalRequest, V3FileSearchMatch, V3FooterData, V3SelectionItem,
    V3UserInputAnswer, V3UserInputOption, V3UserInputQuestion, V3UserInputRequest,
};
pub use vibe::event::{V3Attachment, V3DisplayEvent, V3Event, V3StepStatus};
pub use vibe::surface::V3Surface;
