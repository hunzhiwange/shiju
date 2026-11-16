//! Launch links remain queued until handed to the scoped repository confirmation.
use crate::app::message::settings::skills::repo_modal;
use crate::app::{
    App, Message, deep_links,
    message::settings::{GeneralSettingsMessage, SettingsMessage},
};
use iced::{Element, Task};

use desktop_platform::skill_link_preview::{parse, pending};

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    let Some(link) = pending().filter(|link| link.id == id) else { return Task::none() };
    if app.skills_settings.repositories.open {
        return app.show_error_toast("请先关闭仓库管理弹窗");
    }
    let Some(directory) = app.navigation_tabs.project_path.clone() else {
        return app.show_error_toast("请先打开项目，再预览技能链接");
    };
    let request = match parse(&link.url, directory, app.apps_catalog.host.clone()) {
        Ok(request) => request,
        Err(error) => return app.show_error_toast(error),
    };
    let task = repo_modal::update(app, repo_modal::Event::Open);
    if !app.skills_settings.repositories.open {
        return task;
    }
    // Open captures gateway/project/host and starts a read only SDK list request.
    // The existing ConfirmSource action alone may persist this repository.
    app.skills_settings.repositories.source_link = link.url;
    app.skills_settings.repositories.source_preview = Some(request);
    let _ = deep_links::acknowledge(id);
    task
}

pub(crate) fn overlay<'a>(app: &App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::SkillLinkRejected(link.id),
    ));
    let accept = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::SkillLinkPreview(link.id),
    ));
    desktop_navigation::view_layout::link_preview::skill(
        base,
        &link.url,
        close,
        &app.apps_catalog.host,
        app.navigation_tabs.project_path.as_deref(),
        app.skills_settings.repositories.open,
        accept,
    )
}
