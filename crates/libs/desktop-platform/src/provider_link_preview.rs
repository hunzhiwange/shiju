use std::collections::BTreeMap;
pub struct Preview {
    pub app: String,
    pub fields: usize,
    pub has_credentials: bool,
}

fn fields(raw: &str) -> Result<BTreeMap<String, String>, &'static str> {
    let invalid = "供应商链接格式无效";
    if raw.len() > 16 * 1024 || raw.chars().any(char::is_control) { return Err(invalid); }
    let url = reqwest::Url::parse(raw).map_err(|_| invalid)?;
    if !matches!(url.scheme(), "ccswitch" | "shiju") || url.host_str() != Some("v1")
        || url.path() != "/import" || url.fragment().is_some() || url.port().is_some()
        || !url.username().is_empty() || url.password().is_some() { return Err(invalid); }
    let mut fields = BTreeMap::new();
    for (key, value) in url.query_pairs() {
        if value.chars().any(char::is_control) || fields.insert(key.into_owned(), value.into_owned()).is_some() {
            return Err(invalid);
        }
    }
    Ok(fields)
}

pub fn parse(raw: &str) -> Result<Preview, &'static str> {
    let fields = fields(raw)?;
    if fields.get("resource").map(String::as_str) != Some("provider") {
        return Err("链接不是供应商资源");
    }
    let app = fields.get("app").ok_or("供应商链接缺少应用")?;
    // AppId is the production catalog identity; preview does not imply import support.
    let _: vw_api_app_identity::AppId =
        serde_json::from_value(serde_json::Value::String(app.clone()))
            .map_err(|_| "供应商链接应用无效")?;
    if fields.get("name").is_none_or(|name| name.trim().is_empty()) {
        return Err("供应商链接缺少名称");
    }
    for key in fields.keys() {
        if !matches!(key.as_str(), "resource" | "app" | "name" | "homepage" | "endpoint"
            | "apiKey" | "model" | "notes" | "haikuModel" | "sonnetModel" | "opusModel"
            | "icon" | "config" | "configUrl" | "configFormat") {
            return Err("供应商链接包含未知字段");
        }
    }
    Ok(Preview { app: app.clone(), fields: fields.len(), has_credentials: fields.contains_key("apiKey") })
}

pub fn pending() -> Option<super::deep_links::PendingLink> {
    super::deep_links::pending().ok()?.into_iter().find(|link| {
        reqwest::Url::parse(&link.url).ok().is_some_and(|url|
            url.query_pairs().any(|(key, value)| key == "resource" && value == "provider"))
    })
}


use iced::widget::{column, text};
pub fn body(parsed: Result<Preview, &'static str>) -> iced::Element<'static, ()> {
    let body: iced::Element<'static, ()> = match parsed {
        Ok(preview) => column![
            text(format!("目标应用：{}", preview.app)),
            text(format!("配置字段：{} 项（内容已隐藏）", preview.fields)),
            text(if preview.has_credentials { "API 凭据：已提供（已隐藏）" } else { "API 凭据：未提供" }),
            text("名称、端点、模型及其他载荷均已隐藏。链接仍在待确认队列中；未写入供应商，未读取远程配置。"),
            text("关闭将拒绝此链接。导入确认由后续导入流程处理。"),
        ].spacing(12).into(),
        Err(error) => text(error).into(),
    };
    body.into()
}
