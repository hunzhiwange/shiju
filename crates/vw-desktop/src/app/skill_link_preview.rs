//! Launch links remain queued until handed to the scoped repository confirmation.
use crate::app::{App, Message, deep_links, message::settings::{GeneralSettingsMessage, SettingsMessage}};
use crate::app::message::settings::skills::{deeplink, repo_modal};
use iced::{Element, Task};
use iced::widget::{button, column, row, text};

fn pending() -> Option<deep_links::PendingLink> {
    deep_links::pending().ok()?.into_iter().find(|link| {
        reqwest::Url::parse(&link.url).ok().is_some_and(|url|
            url.query_pairs().any(|(key, value)| key == "resource" && value == "skill"))
    })
}

fn parse(raw: &str, directory: String, host: String) -> Result<deeplink::Request, String> {
    // Both registered schemes share the exact same validated payload contract.
    let normalized = raw.strip_prefix("shiju://")
        .map(|rest| format!("ccswitch://{rest}"));
    deeplink::parse(normalized.as_deref().unwrap_or(raw), directory, host)
}

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
    let mut body = column![text("外部技能链接需要确认后才保存仓库。保存不会安装技能。")].spacing(12);
    let valid = parsed.is_ok();
    match parsed {
        Ok(request) => {
            body = body.push(text(format!("仓库：{}/{}\n分支：{}\n项目目录：{}\n目标主机：{}",
                request.repo.owner, request.repo.name, request.repo.branch,
                request.directory, request.host)));
        }
        Err(error) => body = body.push(settings_error_banner(error)),
    }
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
