pub fn build_usage_view(app: &crate::app::App, data: &super::data::UsageData) -> iced::Element<'static, crate::app::Message> {
 desktop_usage::layout::build_usage_view(&super::view_input(app), data).map(super::map_message)
}
