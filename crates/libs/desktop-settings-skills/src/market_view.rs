use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::{discovery_api::DiscoverSkillsResponse, search_api::MarketSearchResponse};
#[derive(Debug, Clone)]
pub enum Results { Discovery(DiscoverSkillsResponse), Search(MarketSearchResponse) }
#[derive(Debug, Clone, Default)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub query: String,
    pub results: Option<Results>,
    pub error: Option<String>,
    pub generation: u64,
    pub context: Option<AgentAppsContext>,
    pub catalog: u64,
    pub project: Option<String>,
    pub host: String,
    pub version: u64,
}
#[derive(Debug, Clone)]
pub enum Event {
    Open, Close, Query(String), Refresh,
    Completed(u64, Result<Results, String>),
}

// Read-only market inside the existing skills settings entry point.

use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

pub fn view<'a, M: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    let mut body = column![
        settings_field_label(i18n.__("市场搜索（留空发现已配置仓库）")),
        settings_text_input(i18n.__("搜索技能"), &s.query)
            .on_input(move |q| message(Event::Query(q)))
            .on_submit(message(Event::Refresh)),
    ]
    .spacing(10)
    .width(Length::Fill);
    if s.busy {
        body = body.push(text(i18n.__("正在加载…")));
    }
    if let Some(error) = &s.error {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
            i18n,
            &error.clone(),
        )));
    }
    match &s.results {
        Some(Results::Discovery(report)) => {
            for failure in &report.failures {
                body = body.push(settings_error_banner(
                    desktop_i18n::__!(
                        i18n,
                        "仓库 %s/%s（%s）发现失败：%s",
                        (failure.owner).to_string(),
                        (failure.name).to_string(),
                        (failure.branch).to_string(),
                        (format!("{:?}", failure.error)).to_string()
                    )
                    .expect("valid extension display template"),
                ));
            }
            for skill in &report.skills {
                body = body.push(settings_group_card(column![text(skill.name.clone()),
                    text(skill.description.clone()),
                    text(format!("{}/{} · {}", skill.repo_owner, skill.repo_name, skill.repo_branch))].spacing(6)));
            }
            if report.skills.is_empty() {
                body = body.push(text(if report.failures.is_empty() {
                    i18n.__("未发现技能")
                } else {
                    i18n.__("没有可展示的技能；部分仓库失败，请查看上方原因")
                }));
            }
        }
        Some(Results::Search(report)) => {
            body = body.push(text(
                desktop_i18n::__!(
                    i18n,
                    "共 %s 项，当前显示 %s 项",
                    (report.total_count).to_string(),
                    (report.skills.len()).to_string()
                )
                .expect("valid extension display template"),
            ));
            for skill in &report.skills {
                body = body.push(settings_group_card(
                    column![
                        text(skill.name.clone()),
                        text(
                            desktop_i18n::__!(
                                i18n,
                                "%s/%s · %s · 安装次数 %s",
                                (skill.repo_owner).to_string(),
                                (skill.repo_name).to_string(),
                                (skill.repo_branch).to_string(),
                                (skill.installs).to_string()
                            )
                            .expect("valid extension display template")
                        )
                    ]
                    .spacing(6),
                ));
            }
            if report.skills.is_empty() {
                body = body.push(text(i18n.__("没有匹配的技能")));
            }
        }
        None => {}
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        text(i18n.__("技能市场")).size(20),
        settings_close_button(close.clone()),
    );
    let footer = row![
        button(if s.query.trim().is_empty() {
            i18n.__("发现仓库技能")
        } else {
            i18n.__("搜索")
        })
        .padding([8, 10])
        .style(primary_action_btn_style)
        .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button(i18n.__("关闭"))
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(760).height(580);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
