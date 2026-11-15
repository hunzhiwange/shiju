use crate::deep_links;
use vw_api_app_operations::{PromptResource, PromptSharePayload};
use base64::{Engine as _, prelude::*};
pub fn pending() -> Option<deep_links::PendingLink> {
    deep_links::pending().ok()?.into_iter().find(|link| {
        reqwest::Url::parse(&link.url).ok().is_some_and(|url|
            url.query_pairs().any(|(key, value)| key == "resource" && value == "prompt"))
    })
}

pub fn parse(raw: &str) -> Result<PromptSharePayload, &'static str> {
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


use iced::widget::{column, text};
pub fn body(parsed: Result<PromptSharePayload, &'static str>) -> iced::Element<'static, ()> {
    let mut body = column![text("目标使用当前提示词页面；尚未打开时，继续将加载来源应用的提示词，再次继续即可确认导入。默认保持停用。")].spacing(12);
    match parsed {
        Ok(payload) => {
            body = body.push(text(format!("来源应用：{:?}", payload.source_app)))
                .push(text(payload.name)).push(text(payload.description.unwrap_or_default())).push(text(payload.content));
        }
        Err(_) => {},
    }
    body.into()
}
