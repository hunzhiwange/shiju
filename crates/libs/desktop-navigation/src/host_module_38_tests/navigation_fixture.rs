use crate::components::{
    cli_command_help::{self, CliCommandsTab},
    install_cli_modal,
    message::{Message, ViewMessage},
};
use desktop_ui_controls::selectable_text::markdown_selection::{
    SelectionMessage, selectable_titled_text,
};
#[derive(Clone)]
struct Help(Message);
impl SelectionMessage for Help {
    fn none() -> Self {
        Self(Message::None)
    }
    fn copy(text: String) -> Self {
        Self(Message::CopyCode(text))
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        Self(match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Message::CopyCode(text)
            }
            _ => Message::None,
        })
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::none()
    }
}
pub(super) fn help<'a>(
    i18n: desktop_i18n::I18n,
    theme: Option<&iced::Theme>,
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
) -> iced::Element<'a, Message> {
    selectable_titled_text::<Help>(title, body, theme.map(|theme| theme.palette().text), i18n)
        .map(|event| event.0)
}
pub(super) struct Fixture {
    pub app_theme: iced::Theme,
    pub desktop_locale: config_ui::DesktopLocale,
    pub update_panel: desktop_update::panel::PanelState,
    pub desktop_update: desktop_update::app::desktop_update::DesktopUpdate,
    pub cli_commands_tab: CliCommandsTab,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            app_theme: iced::Theme::Dark,
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            update_panel: Default::default(),
            desktop_update: desktop_update::app::desktop_update::DesktopUpdate::from_config(
                &serde_json::json!({}),
            ),
            cli_commands_tab: CliCommandsTab::Common,
        }
    }
}
impl Fixture {
    pub fn view(&self) -> iced::Element<'_, Message> {
        install_cli_modal::view_inner(
            desktop_i18n::I18n::new(self.desktop_locale),
            &self.update_panel.cli_install_modal_title,
            &self.update_panel.cli_install_modal_message,
            Some(&self.update_panel.cli_install_modal_current_version),
            Some(&self.update_panel.cli_install_modal_server_version),
            self.update_panel.cli_install_modal_is_checking_update,
            self.update_panel.cli_install_modal_show_install_action,
            self.update_panel.cli_install_modal_use_app_update_action,
            false,
            self.update_panel.app_update_progress.as_ref(),
            None,
            Some(&self.desktop_update),
            (!self.update_panel.cli_install_modal_use_app_update_action).then(|| {
                cli_command_help::view(
                    desktop_i18n::I18n::new(self.desktop_locale),
                    self.cli_commands_tab,
                    None,
                    help,
                )
            }),
            0,
            help,
        )
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::View(ViewMessage::CliCommandsTabSelected(tab)) => self.cli_commands_tab = tab,
            Message::View(ViewMessage::CloseInstallCliModal) => {
                let mut effects = Vec::new();
                let _ = desktop_update::panel::update(
                    &mut self.update_panel,
                    &mut self.desktop_update,
                    &mut effects,
                    env!("CARGO_PKG_VERSION"),
                    #[cfg(not(target_arch = "wasm32"))]
                    self.desktop_locale,
                    desktop_update::panel::PanelMessage::CloseInstallCliModal,
                );
            }
            message => panic!("unexpected fixture event: {message:?}"),
        }
    }
}
