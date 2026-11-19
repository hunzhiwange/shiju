use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::import_api::{UnmanagedSkillItem, UnmanagedSkillsResponse};
use vw_api_types::agent_apps::AppId;
#[derive(Debug, Clone, Default)]
pub struct State {
    pub query: String,
    pub targets: Vec<vw_api_app_skills::bindings_api::SkillBindingTarget>,
    pub available_targets: Vec<vw_api_app_skills::bindings_api::SkillBindingTarget>,
    pub open: bool,
    pub busy: bool,
    pub apps: Vec<AppId>,
    pub selected: Vec<String>,
    pub report: Option<UnmanagedSkillsResponse>,
    pub outcomes: Vec<(String, Result<(), String>)>,
    pub error: Option<String>,
    pub generation: u64,
    pub context: Option<AgentAppsContext>,
    pub catalog: u64,
    pub project: Option<String>,
    pub host: String,
}
#[derive(Debug, Clone)]
pub enum Event {
    Query(String),
    SelectVisible(bool),
    Target(vw_api_app_skills::bindings_api::SkillBindingTarget, bool),
    Open,
    Close,
    App(AppId, bool),
    Select(String, bool),
    Scan,
    Import,
    Completed(u64, Vec<(String, Result<(), String>)>, Result<UnmanagedSkillsResponse, String>),
}
pub const APPS: &[AppId] = &[
    AppId::Claude,
    AppId::Codex,
    AppId::Gemini,
    AppId::GrokBuild,
    AppId::OpenCode,
    AppId::Hermes,
    AppId::Pi,
    AppId::Mcode,
    AppId::Kimi,
    AppId::Dsh,
    AppId::Agy,
    AppId::Omp,
];

pub fn eligible(row: &UnmanagedSkillItem) -> bool {
    !row.managed
        && !row.conflict
        && !row.external_link
        && row.selection_id.as_ref().is_some_and(|id| !id.trim().is_empty())
        && row.content_hash.as_ref().is_some_and(|hash| !hash.trim().is_empty())
}

pub fn matches_query(row: &UnmanagedSkillItem, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    row.name.to_lowercase().contains(&query)
        || row.directory.to_lowercase().contains(&query)
        || format!("{:?}", row.apps).to_lowercase().contains(&query)
}

// Theme-aware unmanaged import confirmation; source paths are read-only.

use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Element, Length};
pub fn view<'a, M: Clone + 'static>(
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    view_with_icons(s, base, message, |_, _| iced::widget::Space::new().width(0).into())
}

pub fn view_with_icons<'a, M: Clone + 'static>(
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
    icon: fn(AppId, f32) -> Element<'static, M>,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    let mut body = column![text("选择来源应用，扫描后勾选需要导入的技能。")].spacing(10);
    body = body.push(text(format!("扫描来源主机：{}（保留源文件）", s.host)));
    let mut sources = row![].spacing(12);
    for (index, &id) in APPS.iter().enumerate() {
        sources = sources.push(
            row![
                icon(id, 20.0),
                checkbox(s.apps.contains(&id))
                    .label(format!("{id:?}"))
                    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_checkbox_style)
                    .on_toggle_maybe((!s.busy).then_some(move |v| message(Event::App(id, v))))
            ]
            .spacing(6)
            .width(Length::Fill),
        );
        if index % 2 == 1 {
            body = body.push(sources);
            sources = row![].spacing(12);
        }
    }
    body = body.push(sources);
    body = body.push(
        settings_text_input("按名称或来源搜索", &s.query)
            .on_input(move |v| message(Event::Query(v))),
    );
    body = body.push(
        row![
            button("全选筛选结果")
                .on_press_maybe((!s.busy).then_some(message(Event::SelectVisible(true)))),
            button("全不选筛选结果")
                .on_press_maybe((!s.busy).then_some(message(Event::SelectVisible(false))))
        ]
        .spacing(8),
    );
    body = body.push(text(format!("已选择 {} 项（包含隐藏项）", s.selected.len())));
    body = body.push(text("导入后启用目标（独立于扫描来源）"));
    for target in &s.available_targets {
        let target = target.clone();
        body = body.push(
            checkbox(s.targets.contains(&target))
                .label(format!("{:?} / {}", target.app_id, target.installation_id))
                .style(settings_checkbox_style)
                .on_toggle_maybe(
                    (!s.busy).then_some(move |v| message(Event::Target(target.clone(), v))),
                ),
        );
    }
    if let Some(report) = &s.report {
        for skill in report.skills.iter().filter(|skill| matches_query(skill, &s.query)) {
            let id = skill.directory.clone();
            let reason = if skill.managed {
                "已托管，跳过"
            } else if skill.conflict {
                "来源冲突，跳过"
            } else if skill.external_link {
                "外部链接，跳过"
            } else if !eligible(skill) {
                "缺少可用扫描令牌，跳过"
            } else {
                "可导入"
            };
            body = body.push(
                column![
                    checkbox(s.selected.contains(&id))
                        .label(skill.name.clone())
                        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_checkbox_style)
                        .on_toggle_maybe(
                            (!s.busy && eligible(skill))
                                .then_some(move |v| message(Event::Select(id.clone(), v)))
                        ),
                    text(format!("来源目录：{}", skill.directory)),
                    text(format!("foundIn: {:?} · {}", skill.apps, reason)),
                ]
                .spacing(5),
            );
        }
        if report.skills.is_empty() {
            body = body.push(text("未发现未托管技能"));
        }
    }
    for (id, result) in &s.outcomes {
        body = body.push(text(format!(
            "{}：{}",
            id,
            match result {
                Ok(()) => "已导入".to_owned(),
                Err(error) => format!("失败，可重试：{error}"),
            }
        )));
    }
    if s.busy {
        body = body.push(text("正在扫描或导入…"));
    }
    if let Some(error) = &s.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        text("导入未托管技能").size(20),
        settings_close_button(close.clone()),
    );
    let footer = row![
        button("扫描 / 刷新")
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy && !s.apps.is_empty()).then_some(message(Event::Scan))),
        button(text(format!("导入所选（{}）", s.selected.len())))
            .padding([8, 10])
            .style(primary_action_btn_style)
            .on_press_maybe((!s.busy && !s.selected.is_empty()).then_some(message(Event::Import))),
        button("关闭")
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ]
    .spacing(8);
    let card = settings_modal_layout(header, body.width(Length::Fill), Some(footer.into()))
        .style(super::modal_card_style)
        .width(760)
        .height(620);
    settings_modal_overlay_with_escape_and_backdrop_dismiss(
        base,
        close,
        container(card).padding(12),
    )
}
