use crate::{acp_state::AcpSettingsState, message::Event};
#[derive(Default)]
pub(super) struct AcpFixture {
    pub acp_settings: AcpSettingsState,
    pub status_animation_frame: usize,
}
impl AcpFixture {
    pub fn input(&self) -> crate::acp_view::Input<'_> {
        static THEME: iced::Theme = iced::Theme::Light;
        crate::acp_view::Input {
            help: super::selectable_help,
            state: &self.acp_settings,
            locale: config_ui::DesktopLocale::SimplifiedChinese,
            theme: &THEME,
            frame: self.status_animation_frame,
            icon: |agent, size| {
                desktop_ui_controls::icons::acp_agent_icon::<Event>(agent, size)
            },
        }
    }
}
