//! Launch links remain queued until handed to the scoped repository confirmation.
use crate::app::{App, Message, deep_links, message::settings::{GeneralSettingsMessage, SettingsMessage}};
use crate::app::message::settings::skills::repo_modal;
use iced::{Element, Task};
use iced::widget::{button, column, row, text};

use desktop_platform::skill_link_preview::{parse, pending};

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    let Some(link) = pending().filter(|link| link.id == id) else { return Task::none() };
    if app.skills_settings.repositories.open {
        return app.show_error_toast("请先关闭仓库管理弹窗");
    }
    let Some(directory) = app.project_path.clone() else {
        return app.show_error_toast("请先打开项目，再预览技能链接");
    };
    let request = match parse(&link.url, directory, app.apps_catalog.host.clone()) {
        Ok(request) => request,
        Err(error) => return app.show_error_toast(error),
    };
    let task = repo_modal::update(app, repo_modal::Event::Open);
    if !app.skills_settings.repositories.open { return task; }
    // Open captures gateway/project/host and starts a read only SDK list request.
    // The existing ConfirmSource action alone may persist this repository.
    app.skills_settings.repositories.source_link = link.url;
    app.skills_settings.repositories.source_preview = Some(request);
    let _ = deep_links::acknowledge(id);
    task
}

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::*;
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(GeneralSettingsMessage::SkillLinkRejected(link.id)));
    let parsed = parse(&link.url, app.project_path.clone().unwrap_or_default(), app.apps_catalog.host.clone());
    let valid = parsed.is_ok();
    let error = parsed.as_ref().err().map(|error| error.to_string());
    let content = desktop_platform::skill_link_preview::body(parsed).map(|_| unreachable!());
    let body = column![content].spacing(12);
    let body = if let Some(error) = error { body.push(settings_error_banner(error)) } else { body };
    let footer = row![
        button("继续确认仓库").padding([8, 10]).style(primary_action_btn_style)
            .on_press_maybe((valid && app.project_path.is_some() && !app.skills_settings.repositories.open)
                .then_some(Message::Settings(SettingsMessage::General(GeneralSettingsMessage::SkillLinkPreview(link.id))))),
        button("拒绝链接").padding([8, 10]).style(rounded_action_btn_style).on_press(close.clone()),
    ].spacing(8);
    let card = settings_modal_layout(
        settings_modal_header(text("技能链接预览"), settings_close_button(close.clone())),
        body, Some(footer.into()),
    ).style(settings_modal_card_style).width(640).height(420);
    settings_modal_overlay(Some(base), close, card)
}
