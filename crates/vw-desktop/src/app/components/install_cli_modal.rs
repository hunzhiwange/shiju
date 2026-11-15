use crate::app::{App, Message, message::ViewMessage};
use desktop_navigation::components::{install_cli_modal as shared, message as domain};
pub(super) fn map(event: domain::Message) -> Message {
    use domain::ViewMessage as E;
    match event {
        domain::Message::None => Message::None,
        domain::Message::CopyCode(text) => Message::CopyCode(text),
        domain::Message::View(event) => Message::View(match event {
            E::CloseInstallCliModal => ViewMessage::CloseInstallCliModal,
            E::CheckAppUpdate => ViewMessage::CheckAppUpdate,
            E::CheckCliToolUpdate => ViewMessage::CheckCliToolUpdate,
            E::OpenAppInstaller => ViewMessage::OpenAppInstaller,
            E::OpenAppInstallerDirectory => ViewMessage::OpenAppInstallerDirectory,
            E::CancelAppUpdate => ViewMessage::CancelAppUpdate,
            E::RunAppUpdate => ViewMessage::RunAppUpdate,
            E::RunInstallCliTool => ViewMessage::RunInstallCliTool,
            E::SkipAppUpdate => ViewMessage::SkipAppUpdate,
            E::AppUpdateSourceSelected(value) => ViewMessage::AppUpdateSourceSelected(value),
            E::CliCommandsTabSelected(value) => ViewMessage::CliCommandsTabSelected(value),
            #[cfg(not(target_arch="wasm32"))]
            E::WebInstall(event) => ViewMessage::WebInstall(event),
        }),
    }
}
pub(super) fn help<'a>(i18n: desktop_i18n::I18n, theme: Option<&iced::Theme>, title: &'a str, body: std::borrow::Cow<'a, str>) -> iced::Element<'a, domain::Message> {
    super::chat_panel::message_view::selectable_titled_text(title, body, theme.map(|theme| theme.palette().text), i18n)
        .map(|message| match message { Message::CopyCode(text) => domain::Message::CopyCode(text), Message::None => domain::Message::None, _ => unreachable!("standalone help emits only copy or no-op") })
}
pub fn view<'a>(title: &'a str, content: &'a str) -> iced::Element<'a, Message> {
    shared::view(title, content, help).map(map)
}
pub fn view_localized<'a>(
    i18n: desktop_i18n::I18n,
    title: &'a str,
    content: &'a str,
) -> iced::Element<'a, Message> {
    shared::view_localized(i18n, title, content, help).map(map)
}
pub fn view_update_check<'a>(
    title: &'a str,
    content: &'a str,
    current_version: &'a str,
    server_version: &'a str,
    is_checking: bool,
    show_install_action: bool,
    use_app_update_action: bool,
) -> iced::Element<'a, Message> {
    shared::view_update_check(title, content, current_version, server_version, is_checking, show_install_action, use_app_update_action, help).map(map)
}
pub fn view_update_check_with_progress<'a>(
    title: &'a str,
    content: &'a str,
    current_version: &'a str,
    server_version: &'a str,
    is_checking: bool,
    show_install_action: bool,
    use_app_update_action: bool,
    update_progress: Option<&'a update_types::UpdateProgress>,
    last_copied_code_hash: Option<u64>,
) -> iced::Element<'a, Message> {
    shared::view_update_check_with_progress(title, content, current_version, server_version, is_checking, show_install_action, use_app_update_action, update_progress, last_copied_code_hash, help).map(map)
}
pub fn view_app_update(app: &App) -> iced::Element<'_, Message> {
    shared::view_inner(
        crate::app::i18n::I18n::new(app.desktop_locale),
        &app.cli_install_modal_title,
        &app.cli_install_modal_message,
        Some(&app.cli_install_modal_current_version),
        Some(&app.cli_install_modal_server_version),
        app.cli_install_modal_is_checking_update,
        app.cli_install_modal_show_install_action,
        app.cli_install_modal_use_app_update_action,
        false,
        app.app_update_progress.as_ref(),
        app.last_copied_code_hash,
        Some(&app.desktop_update),
        (!app.cli_install_modal_use_app_update_action).then(|| {
            desktop_navigation::components::cli_command_help::view(
                crate::app::i18n::I18n::new(app.desktop_locale),
                app.cli_commands_tab,
                app.last_copied_code_hash,
                help,
            )
        }),
        app.status_animation_frame,
        help,
    ).map(map)
}
