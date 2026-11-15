use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Default)]
pub struct ExploreSummaryAnimationState {
 pub previous_summary_text: String,
 pub current_summary_text: String,
 pub changed_at_ms: Option<u64>,
}
pub struct ExploreInput<'a> {
 pub i18n: desktop_i18n::I18n,
 pub hovered: Option<u64>,
 pub chat_explore_expanded: &'a HashSet<u64>,
 pub chat_explore_summary_animations: &'a HashMap<u128, ExploreSummaryAnimationState>,
 pub text: crate::tool_text_support::ToolTextState<'a>,
 pub now_ms: u64,
 pub status_animation_frame: usize,
}
