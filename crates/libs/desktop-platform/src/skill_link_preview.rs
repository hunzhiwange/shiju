use crate::deep_links;
use crate::skill_deeplink as deeplink;
pub fn pending() -> Option<deep_links::PendingLink> {
    deep_links::pending().ok()?.into_iter().find(|link| {
        reqwest::Url::parse(&link.url).ok().is_some_and(|url|
            url.query_pairs().any(|(key, value)| key == "resource" && value == "skill"))
    })
}

pub fn parse(raw: &str, directory: String, host: String) -> Result<deeplink::Request, String> {
    // Both registered schemes share the exact same validated payload contract.
    let normalized = raw.strip_prefix("shiju://")
        .map(|rest| format!("ccswitch://{rest}"));
    deeplink::parse(normalized.as_deref().unwrap_or(raw), directory, host)
}


use iced::widget::{column, text};
pub fn body(parsed: Result<deeplink::Request, String>) -> iced::Element<'static, ()> {
    let mut body = column![text("外部技能链接需要确认后才保存仓库。保存不会安装技能。")].spacing(12);
    match parsed {
        Ok(request) => {
            body = body.push(text(format!("仓库：{}/{}\n分支：{}\n项目目录：{}\n目标主机：{}",
                request.repo.owner, request.repo.name, request.repo.branch,
                request.directory, request.host)));
        }
        Err(_) => {},
    }
    body.into()
}
