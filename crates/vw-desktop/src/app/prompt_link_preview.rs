//! Launch payloads stay inert until handed to the scoped prompt import confirmation.
use crate::app::{
    App, Message, deep_links,
    message::settings::{GeneralSettingsMessage, SettingsMessage, prompts},
};
use iced::{Element, Task};

use desktop_platform::prompt_link_preview::{parse, pending};

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    if app.apps_catalog.prompts.shared_import.is_some() {
        return Task::none();
    }
    let Some(link) = pending().filter(|link| link.id == id) else { return Task::none() };
    let payload = match parse(&link.url) {
        Ok(payload) => payload,
        Err(error) => return app.show_error_toast(error),
    };
    if app.apps_catalog.prompts.scope.is_none()
        || app.apps_catalog.prompts.catalog.is_none()
        || app.apps_catalog.prompts.preview.is_none()
    {
        if app.apps_catalog.prompts.loading {
            return app.show_error_toast("提示词正在加载，请稍后重试");
        }
        return prompts::update(app, prompts::Event::Open(payload.source_app));
    }
    let task = prompts::update(
        app,
        prompts::Event::SharedImport(prompts::shared_import::Event::Open(payload)),
    );
    if app.apps_catalog.prompts.shared_import.is_some() {
        let _ = deep_links::acknowledge(id);
        task
    } else {
        Task::batch([
            task,
            app.show_error_toast("请先在目标应用中打开提示词页面，等待加载完成后重试"),
        ])
    }
}

pub(crate) fn overlay<'a>(app: &App, base: Element<'a, Message>) -> Element<'a, Message> {
    if app.apps_catalog.prompts.shared_import.is_some() {
        return base;
    }
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::PromptLinkRejected(link.id),
    ));
    let accept = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::PromptLinkPreview(link.id),
    ));
    desktop_navigation::view_layout::link_preview::prompt(base, &link.url, close, accept)
}
