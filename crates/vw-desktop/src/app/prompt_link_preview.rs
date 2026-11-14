//! Launch payloads stay inert until handed to the scoped prompt import confirmation.
use base64::{Engine as _, prelude::*};
use crate::app::{App, Message, deep_links, message::settings::{GeneralSettingsMessage, SettingsMessage, prompts}};
use iced::{Element, Task, widget::{button, column, row, text}};
use vw_gateway_client::vw_api_types::agent_apps::{PromptResource, PromptSharePayload};

pub(crate) fn pending() -> Option<deep_links::PendingLink> {
    deep_links::pending().ok()?.into_iter().find(|link| {
        reqwest::Url::parse(&link.url).ok().is_some_and(|url|
            url.query_pairs().any(|(key, value)| key == "resource" && value == "prompt"))
    })
}

fn parse(raw: &str) -> Result<PromptSharePayload, &'static str> {
    let invalid = "提示词链接格式无效";
    if raw.len() > 16 * 1024 || raw.chars().any(char::is_control) { return Err(invalid); }
    let url = reqwest::Url::parse(raw).map_err(|_| invalid)?;
    if !matches!(url.scheme(), "shiju" | "ccswitch") || url.host_str() != Some("v1")
        || url.path() != "/import" || url.fragment().is_some() || url.port().is_some()
        || !url.username().is_empty() || url.password().is_some() { return Err(invalid); }
    let mut fields = std::collections::BTreeMap::new();
    for (key, value) in url.query_pairs() {
        if !matches!(key.as_ref(), "resource" | "app" | "name" | "content" | "description" | "enabled")
            || fields.insert(key.into_owned(), value.into_owned()).is_some() { return Err(invalid); }
    }
    if fields.get("resource").map(String::as_str) != Some("prompt") { return Err(invalid); }
    let source_app = serde_json::from_value(serde_json::Value::String(fields.remove("app").ok_or(invalid)?)).map_err(|_| invalid)?;
    (PromptResource::Application { app_id: source_app }).validate_application_access().map_err(|_| "不支持该提示词应用")?;
    let name = fields.remove("name").ok_or(invalid)?;
    let raw = fields.remove("content").ok_or(invalid)?;
    let normalized = raw.trim_matches(['\r', '\n']).replace(' ', "+");
    let mut padded = normalized.clone();
    if padded.len() % 4 != 0 { padded.extend(std::iter::repeat_n('=', 4 - padded.len() % 4)); }
    let bytes = [&normalized, &padded].into_iter().find_map(|value| {
        [BASE64_STANDARD, BASE64_STANDARD_NO_PAD, BASE64_URL_SAFE, BASE64_URL_SAFE_NO_PAD]
            .into_iter().find_map(|engine| engine.decode(value).ok())
    }).ok_or("提示词正文 Base64 无效")?;
    let content = String::from_utf8(bytes).map_err(|_| "提示词正文必须为 UTF-8")?;
    if name.trim().is_empty() || content.trim().is_empty() || name.chars().any(char::is_control) { return Err(invalid); }
    let activation_suggested = match fields.remove("enabled").as_deref() {
        None | Some("false") => false, Some("true") => true, _ => return Err(invalid),
    };
    Ok(PromptSharePayload { source_app, name, content, description: fields.remove("description"), activation_suggested })
}

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
    let mut body = column![text("目标使用当前提示词页面；尚未打开时，继续将加载来源应用的提示词，再次继续即可确认导入。默认保持停用。")].spacing(12);
    match parsed {
        Ok(payload) => {
            body = body.push(text(format!("来源应用：{:?}", payload.source_app)))
                .push(text(payload.name)).push(text(payload.description.unwrap_or_default())).push(text(payload.content));
        }
        Err(error) => body = body.push(settings_error_banner(error)),
    }
    let footer = row![
        button("继续确认导入").padding([8, 12]).style(primary_action_btn_style)
            .on_press_maybe(valid.then_some(Message::Settings(SettingsMessage::General(GeneralSettingsMessage::PromptLinkPreview(link.id))))),
        button("取消").padding([8, 12]).style(rounded_action_btn_style).on_press(close.clone()),
    ].spacing(10);
    settings_modal_overlay(Some(base), close.clone(), settings_modal_layout(
        settings_modal_header(text("提示词链接预览"), settings_close_button(close)), body, Some(footer.into()),
    ).style(settings_modal_card_style).width(700).height(620))
}
