//! Launch payloads stay inert until handed to the scoped prompt import confirmation.
use crate::app::{App, Message, deep_links, message::settings::{GeneralSettingsMessage, SettingsMessage, prompts}};
use iced::{Element, Task, widget::{button, column, row, text}};


use desktop_platform::prompt_link_preview::{parse, pending};

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    if app.apps_catalog.prompts.shared_import.is_some() { return Task::none(); }
    let Some(link) = pending().filter(|link| link.id == id) else { return Task::none() };
    let payload = match parse(&link.url) { Ok(payload) => payload, Err(error) => return app.show_error_toast(error) };
    if app.apps_catalog.prompts.scope.is_none() || app.apps_catalog.prompts.catalog.is_none() || app.apps_catalog.prompts.preview.is_none() {
        if app.apps_catalog.prompts.loading { return app.show_error_toast("提示词正在加载，请稍后重试"); }
        return prompts::update(app, prompts::Event::Open(payload.source_app));
    }
    let task = prompts::update(app, prompts::Event::SharedImport(prompts::shared_import::Event::Open(payload)));
    if app.apps_catalog.prompts.shared_import.is_some() {
        let _ = deep_links::acknowledge(id);
        task
    } else {
        Task::batch([task, app.show_error_toast("请先在目标应用中打开提示词页面，等待加载完成后重试")])
    }
}

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::*;
    if app.apps_catalog.prompts.shared_import.is_some() { return base; }
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(GeneralSettingsMessage::PromptLinkRejected(link.id)));
    let parsed = parse(&link.url);
    let valid = parsed.is_ok();
    let error = parsed.as_ref().err().map(|error| error.to_string());
    let content = desktop_platform::prompt_link_preview::body(parsed).map(|_| unreachable!());
    let body = column![content].spacing(12);
    let body = if let Some(error) = error { body.push(settings_error_banner(error)) } else { body };
    let footer = row![
        button("继续确认导入").padding([8, 12]).style(primary_action_btn_style)
            .on_press_maybe(valid.then_some(Message::Settings(SettingsMessage::General(GeneralSettingsMessage::PromptLinkPreview(link.id))))),
        button("取消").padding([8, 12]).style(rounded_action_btn_style).on_press(close.clone()),
    ].spacing(10);
    settings_modal_overlay(Some(base), close.clone(), settings_modal_layout(
        settings_modal_header(text("提示词链接预览"), settings_close_button(close)), body, Some(footer.into()),
    ).style(settings_modal_card_style).width(700).height(620))
}
