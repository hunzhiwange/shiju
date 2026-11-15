use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::repos_api::SkillReposResponse;
#[derive(Debug, Clone, Default)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub identity: String,
    pub source_link: String,
    pub source_preview: Option<desktop_platform::skill_deeplink::Request>,
    pub branch: String,
    pub snapshot: Option<SkillReposResponse>,
    pub error: Option<String>,
    pub generation: u64,
    pub directory: Option<String>,
    pub context: Option<AgentAppsContext>,
    pub project: Option<String>,
    pub host: String,
}
#[derive(Debug, Clone)]
pub enum Event {
    Open, Close, Identity(String), Branch(String), Refresh, Add,
    Toggle(usize), Remove(usize),
    SourceLink(String), PreviewSource, CancelSource, ConfirmSource,
    Completed { generation: u64, added: bool, result: Result<SkillReposResponse, String> },
}

// Repository settings share the standard theme-aware, scrollable modal layout.

use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, container, row, text};
use iced::Element;
pub fn view<'a, M: Clone + 'static>(s: &'a State, base: Element<'a, M>, message: fn(Event) -> M) -> Element<'a, M> {
    if !s.open { return base; }
    let mut body = column![
        settings_field_label("仓库（owner/name）"),
        settings_text_input("owner/name", &s.identity)
            .on_input_maybe((!s.busy).then_some(move |v| message(Event::Identity(v)))),
        settings_field_label("分支（留空使用默认分支）"),
        settings_text_input("", &s.branch)
            .on_input_maybe((!s.busy).then_some(move |v| message(Event::Branch(v)))),
        button("添加仓库").padding([8, 10]).style(primary_action_btn_style)
            .on_press_maybe((!s.busy && s.snapshot.is_some()).then_some(message(Event::Add))),
        crate::deeplink::view(crate::deeplink::ViewState { source_link: &s.source_link, busy: s.busy, source_preview: &s.source_preview, snapshot: &s.snapshot }, |e| e).map(move_deeplink).map(message),
    ].spacing(10);
    if let Some(snapshot) = &s.snapshot {
        for (index, repo) in snapshot.repos.iter().enumerate() {
            body = body.push(row![
                container(text(format!("{}/{} · {}", repo.owner, repo.name, repo.branch)))
                    .width(iced::Length::Fill),
                button(if repo.enabled { "停用" } else { "启用" }).padding([8, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!s.busy).then_some(message(Event::Toggle(index)))),
                button("删除").padding([8, 10]).style(rounded_action_btn_style)
                    .on_press_maybe((!s.busy).then_some(message(Event::Remove(index)))),
            ].spacing(8).align_y(iced::Alignment::Center));
        }
        if snapshot.repos.is_empty() { body = body.push(text("尚未配置仓库")); }
    }
    if s.busy { body = body.push(text("正在同步仓库…")); }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    let close = message(Event::Close);
    let header = settings_modal_header(text("仓库管理").size(20), settings_close_button(close.clone()));
    let footer = row![
        button("刷新").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(760).height(580);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

fn move_deeplink(e: crate::deeplink::Event) -> Event { match e { crate::deeplink::Event::SourceLink(v) => Event::SourceLink(v), crate::deeplink::Event::PreviewSource => Event::PreviewSource, crate::deeplink::Event::CancelSource => Event::CancelSource, crate::deeplink::Event::ConfirmSource => Event::ConfirmSource } }
