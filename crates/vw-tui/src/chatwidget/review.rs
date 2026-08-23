//! Code-review flow state for `ChatWidget`.
//! 本模块负责代码审查流程的本地状态。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use crate::auto_review_denials::RecentAutoReviewDenials;
use crate::token_usage::TokenUsageInfo;

#[derive(Debug, Default)]
pub(super) struct ReviewState {
    pub(super) recent_auto_review_denials: RecentAutoReviewDenials,
    /// Simple review mode flag; used to adjust layout and banners.
    /// 简单审查模式旗;用于调整布局和横幅.
    pub(super) is_review_mode: bool,
    /// Snapshot of token usage to restore after review mode exits.
    /// 复检模式退出后恢复的符号使用快照。
    pub(super) pre_review_token_info: Option<Option<TokenUsageInfo>>,
}
