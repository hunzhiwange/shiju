use desktop_i18n::I18n;
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
pub fn body(parsed: Result<deeplink::Request, String>, i18n: I18n) -> iced::Element<'static, ()> {
    let mut body =
        column![text(i18n.__("外部技能链接需要确认后才保存仓库。保存不会安装技能。"))].spacing(12);
    if let Ok(request) = parsed {
        body = body.push(text(
            i18n.format(
                "仓库：%s/%s\n分支：%s\n项目目录：%s",
                &[
                    request.repo.owner.into(),
                    request.repo.name.into(),
                    request.repo.branch.into(),
                    request.directory.into(),
                ],
            )
            .expect("valid source template"),
        ));
    }
    body.into()
}
