#[derive(Debug, Clone)]
pub enum Message { View(ViewMessage), CopyCode(String), None }
#[derive(Debug, Clone)]
pub enum ViewMessage {
 CloseInstallCliModal, CheckAppUpdate, CheckCliToolUpdate, OpenAppInstaller,
 OpenAppInstallerDirectory, CancelAppUpdate, RunAppUpdate, RunInstallCliTool, SkipAppUpdate,
 AppUpdateSourceSelected(desktop_release::UpdateSource),
 CliCommandsTabSelected(super::cli_command_help::CliCommandsTab),
 #[cfg(not(target_arch="wasm32"))] WebInstall(desktop_web_install::Event),
}
pub type HelpRenderer = for<'a> fn(desktop_i18n::I18n, Option<&iced::Theme>, &'a str, std::borrow::Cow<'a, str>) -> iced::Element<'a, Message>;
pub fn settings_hover_help_label<'a, 'b>(help: HelpRenderer, i18n: desktop_i18n::I18n, theme: impl Into<Option<&'b iced::Theme>>, title: &'a str, label: iced::widget::Text<'a>, description: impl Into<std::borrow::Cow<'a, str>>) -> iced::widget::Row<'a, Message> {
 desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, help(i18n, theme.into(), title, description.into()))
}
