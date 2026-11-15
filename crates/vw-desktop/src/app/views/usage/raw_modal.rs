pub fn view(app: &crate::app::App) -> Option<iced::Element<'_, crate::app::Message>> {
 desktop_usage::raw_modal::view(&super::view_input(app)).map(|element| element.map(super::map_message))
}
