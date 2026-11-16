pub mod state;
pub mod messages;
pub mod common;
pub mod basic_channels;
pub mod overview;
pub mod execution;
#[derive(Clone, Copy)]
pub struct ViewState<'a> {
 pub channels_settings: &'a state::ChannelsSettingsState,
 pub desktop_locale: config_ui::DesktopLocale,
 pub app_theme: &'a iced::Theme,
}
#[cfg(test)]
pub(crate) mod test_support {
    pub struct Fixture {
        pub channels_settings: crate::state::ChannelsSettingsState,
        pub desktop_locale: config_ui::DesktopLocale,
        pub app_theme: iced::Theme,
    }
    impl Fixture {
        pub fn new() -> (Self, ()) { (Self { channels_settings: Default::default(), desktop_locale: config_ui::DesktopLocale::SimplifiedChinese, app_theme: iced::Theme::Light }, ()) }
        pub fn view(&self) -> crate::ViewState<'_> { crate::ViewState { channels_settings: &self.channels_settings, desktop_locale: self.desktop_locale, app_theme: &self.app_theme } }
    }
}
#[cfg(test)]
mod basic_channels_tests;
#[cfg(test)]
mod common_tests;
#[cfg(test)]
mod execution_tests;

pub mod enterprise_channels;
pub mod extended_channels;
pub mod wecom_delivery_modal;
#[cfg(test)]
mod enterprise_channels_tests;
#[cfg(test)]
mod extended_channels_tests;

pub mod update;

pub mod save_feedback;

pub mod app_init;

#[cfg(test)]
mod desktop_split_15_tests;

#[cfg(test)]
mod host_split_4_tests;

#[cfg(test)]
mod channel_render_fixture;
#[cfg(test)]
mod host_split_5_tests;

pub mod view;
mod help;
#[cfg(test)]
mod host_module_18_tests;
