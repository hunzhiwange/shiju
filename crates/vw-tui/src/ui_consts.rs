//! TUI 内布局与对齐所共享的界面常量。
//! Shared UI constants for layout and alignment within the TUI.

/// Width (in terminal columns) reserved for the left gutter/prefix used by
/// 为实时单元和对齐组件的左侧沟槽或前缀预留的终端列数。
/// live cells and aligned widgets.
///
/// 语义约定：
///
/// - 聊天输入框为左边框和内边距保留这些列；
/// - 状态指示行以相同数量的空格起始以保持对齐；
/// - 用户历史行换行时也需计入这些列，例如 `"▌ "`。
///
/// Semantics:
///
/// - Chat composer reserves this many columns for the left border + padding.
/// - Status indicator lines begin with this many spaces for alignment.
/// - User history lines account for this many columns (e.g., "▌ ") when wrapping.
pub(crate) const LIVE_PREFIX_COLS: u16 = 2;
/// 页脚沿用实时内容的左侧缩进列数。
pub(crate) const FOOTER_INDENT_COLS: usize = LIVE_PREFIX_COLS as usize;
/// 记录查看快捷键的统一提示文本。
pub(crate) const TRANSCRIPT_HINT: &str = "ctrl + t to view transcript";
