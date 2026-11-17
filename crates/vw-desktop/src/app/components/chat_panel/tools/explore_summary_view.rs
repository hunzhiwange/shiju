//! 桌面探索摘要只读投影。
pub fn tool_explore_summary_view<'a>(app: &'a crate::app::App, msg_idx: usize, group_idx: usize, items: &[super::types::ExploreItem], force_running: bool, closed_by_following_block: bool) -> Option<iced::Element<'a, crate::app::Message>> {
 let input = desktop_chat_tools::tools::explore_input::ExploreInput {
  i18n: crate::app::i18n::I18n::new(app.general_settings.desktop_locale), hovered: app.chat_interaction.chat_tool_hovered_idx,
  chat_explore_expanded: &app.chat_view.chat_explore_expanded,
  chat_explore_summary_animations: &app.chat_interaction.chat_explore_summary_animations,
  text: super::super::tool_text_support::text_state(app), now_ms: crate::app::time::now_ms(),
  status_animation_frame: app.status_animation_frame,
 };
 desktop_chat_tools::tools::explore_summary_view::tool_explore_summary_view(&input,msg_idx,group_idx,items,force_running,closed_by_following_block)
 .map(|view| view.map(super::card_adapter::map_card))
}

#[cfg(test)]
pub(crate) use desktop_chat_tools::tools::explore_summary_view::explore_summary_expanded;
#[cfg(test)]
pub(crate) use desktop_chat_tools::tools::explore_summary_view::explore_summary_is_running;
