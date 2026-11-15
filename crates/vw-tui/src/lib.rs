//! TUI 上层交互装配。
//!
//! 保留聊天组件、底部面板、审批交互、onboarding、surface 和终端运行循环。
//! 契约、文本、渲染、主题、流式处理、终端后端及剪贴板由 `tui-*` 库提供；
//! 对外重导出保持既有消费者入口和底层类型所有权。

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![deny(clippy::disallowed_methods)]

pub(crate) use tui_input::{input_types, key_hint, keymap};

// 底部面板与聊天组件构成 v3 交互界面的主要输入出口。
#[path = "bottom_pane/mod.rs"]
#[allow(dead_code)]
pub(crate) mod bottom_pane;
#[allow(dead_code)]
pub mod chatwidget;

// VibeWindow 适配：复制模块先于下游消费者激活，因此阶段性死代码许可只限定于本批次。
// VibeWindow adaptation: copied modules are activated before their downstream
// consumers, so staged dead-code allowances stay scoped to this batch only.
pub(crate) use tui_clipboard::{clipboard_copy, clipboard_image};
#[allow(dead_code)]
pub(crate) use tui_theme::color;
#[allow(dead_code)]
mod custom_terminal;
pub use tui_contracts::diff_model;
#[allow(dead_code)]
pub use tui_cells::exec_cell;
#[allow(dead_code)]
mod git_action_directives;
#[allow(dead_code)]
mod goal_display;
pub use tui_cells::history_cell;
#[allow(dead_code)]
mod insert_history;
#[allow(dead_code)]
pub(crate) use tui_text::line_truncation;
// Markdown 与实时换行模块共同维护流式内容的稳定布局。
#[allow(dead_code)]
pub(crate) use tui_text::live_wrap;
#[allow(dead_code)]
mod markdown;
#[allow(dead_code)]
pub(crate) use tui_markdown::markdown_render;
#[allow(dead_code)]
use tui_cells::motion;
#[allow(dead_code)]
pub mod onboarding;
#[allow(dead_code)]
mod pager_overlay;
#[allow(dead_code, unused_imports)]
mod render;
#[allow(dead_code)]
pub(crate) mod resume_picker;
#[allow(dead_code)]
pub use tui_widgets::selection_list;
#[allow(dead_code)]
pub use tui_cells::status;
#[allow(dead_code)]
mod status_indicator_widget;
#[allow(dead_code)]
pub(crate) use tui_stream::streaming;
#[allow(dead_code)]
pub(crate) use tui_theme::style;
#[allow(dead_code)]
pub(crate) use tui_markdown::table_detect;
// 终端适配层集中处理链接、调色板和启动探测，避免渲染层直接依赖环境细节。
#[allow(dead_code)]
use tui_terminal::resize_reflow_cap;
#[allow(dead_code)]
mod terminal_hyperlinks;
#[allow(dead_code)]
pub(crate) use tui_theme::terminal_palette;
#[allow(dead_code, unused_imports)]
pub(crate) use tui_terminal::terminal_probe;
#[allow(dead_code)]
pub(crate) use tui_text::text_formatting;
#[allow(dead_code)]
pub(crate) mod theme_picker;
#[allow(dead_code)]
use tui_cells::thread_transcript;
#[allow(dead_code)]
use tui_terminal::transcript_reflow;
#[allow(dead_code)]
mod tui;
#[allow(dead_code)]
mod ui_consts;
pub mod vibe;
#[allow(dead_code)]
pub(crate) use tui_text::width;
#[allow(dead_code)]
pub(crate) use tui_text::wrapping;

// 测试后端仅为 crate 内测试提供可控终端缓冲区。
#[cfg(test)]
mod test_backend;

// 上游快照矩阵继续校验迁移模块与原始渲染结果的一致性。
#[cfg(test)]
#[path = "../tests/upstream_snapshot_matrix.rs"]
mod upstream_snapshot_matrix;

// 对外重导出维持 vw-cli 桥接与运行器入口，不包装或复制底层类型。
pub use chatwidget::{ChatWidget, ChatWidgetConfig, V3ChatStatus};
pub use custom_terminal::Terminal as CustomTerminal;
pub use history_cell::{ExecHistoryCell, HistoryCell, history_cell_for, render_display_event};
pub use insert_history::{
    HistoryLineWrapPolicy, insert_history_lines, insert_history_lines_with_wrap_policy,
};
pub use pager_overlay::TranscriptPager;
pub use resize_reflow_cap::{ResizeReflowCap, resize_reflow_max_rows};
pub use status::{StatusHistoryCell, V3SessionDisplayData, V3TokenUsage};
pub use transcript_reflow::{TranscriptReflowState, TranscriptWidthChange};
pub use tui::{TerminalMode, run_terminal};
pub use vibe::action::{
    V3_NON_SECURITY_SELECTION_SURFACES, V3Action, V3ApprovalChoice, V3ApprovalDecision,
    V3ApprovalKind, V3ApprovalRequest, V3FileSearchMatch, V3FooterData, V3SelectionItem,
    V3UserInputAnswer, V3UserInputOption, V3UserInputQuestion, V3UserInputRequest,
};
pub use vibe::event::{
    V3Attachment, V3AttachmentKind, V3DisplayEvent, V3Event, V3FileChange, V3FileDiff,
    V3FileDiffEntry, V3StepStatus,
};
pub use vibe::surface::V3Surface;
