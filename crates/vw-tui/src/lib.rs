//! Terminal compatibility exports; implementations belong to the tui-* libraries.
#![deny(clippy::print_stdout, clippy::print_stderr)]
#![deny(clippy::disallowed_methods)]

pub use tui_chat::chatwidget;
pub use tui_onboarding as onboarding;
pub use tui_surface::vibe;
pub use tui_contracts::diff_model;
pub use tui_cells::{exec_cell, history_cell, status};
pub use tui_widgets::selection_list;

#[cfg(test)]
use tui_terminal::custom_terminal;
#[cfg(test)]
use tui_markdown::markdown_render;
#[cfg(test)]
mod test_backend;
#[cfg(test)]
mod insert_history;
#[cfg(test)]
#[path = "vibe/surface_tests.rs"]
mod surface_tests;
#[cfg(test)]
#[path = "../tests/upstream_snapshot_matrix.rs"]
mod upstream_snapshot_matrix;

pub use tui_chat::{ChatWidget, ChatWidgetConfig, V3ChatStatus};
pub use tui_terminal::custom_terminal::Terminal as CustomTerminal;
pub use history_cell::{ExecHistoryCell, HistoryCell, history_cell_for, render_display_event};
pub use tui_terminal::insert_history::{
    HistoryLineWrapPolicy, insert_history_lines, insert_history_lines_with_wrap_policy,
};
pub use tui_pager::TranscriptPager;
pub use tui_terminal::resize_reflow_cap::{ResizeReflowCap, resize_reflow_max_rows};
pub use status::{StatusHistoryCell, V3SessionDisplayData, V3TokenUsage};
pub use tui_terminal::transcript_reflow::{TranscriptReflowState, TranscriptWidthChange};
pub use tui_terminal_lifecycle::{TerminalMode, run_terminal};
pub use vibe::action::{
    V3_NON_SECURITY_SELECTION_SURFACES, V3Action, V3ApprovalChoice, V3ApprovalDecision,
    V3ApprovalKind, V3ApprovalRequest, V3FileSearchMatch, V3FooterData, V3SelectionItem,
    V3UserInputAnswer, V3UserInputOption, V3UserInputQuestion, V3UserInputRequest,
};
pub use vibe::event::{
    V3Attachment, V3AttachmentKind, V3DisplayEvent, V3Event, V3FileChange, V3FileDiff,
    V3FileDiffEntry, V3StepStatus,
};
pub use tui_surface::V3Surface;
