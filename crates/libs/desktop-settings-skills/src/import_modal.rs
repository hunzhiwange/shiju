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
#[derive(Clone, PartialEq)]
struct ScanSource {
    id: AppId,
    label: String,
}

impl std::fmt::Display for ScanSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
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

use crate::third_party::style;
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{Space, button, checkbox, column, container, row, rule, scrollable, text};
use iced::{Element, Length};
pub fn view<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    view_with_icons(i18n, s, base, message, |_, _| iced::widget::Space::new().width(0).into())
}

pub fn view_with_icons<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
    icon: fn(AppId, f32) -> Element<'static, M>,
) -> Element<'a, M> {
    view_with_app_names(i18n, s, base, message, icon, |id| format!("{id:?}"))
}

pub fn view_with_app_names<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
    icon: fn(AppId, f32) -> Element<'static, M>,
    name: impl Fn(AppId) -> String,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    let search = settings_text_input(i18n.__("搜索名称或来源"), &s.query)
        .style(style::input)
        .on_input(move |v| message(Event::Query(v)));
    let selection = |label, value| {
        button(text(label).size(13))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::SelectVisible(value))))
    };
    let mut body = column![
        text(i18n.__("导入会复制技能，保留来源文件。")).size(13).style(settings_muted_text_style),
        row![style::search(search), selection(i18n.__("全选"), true), selection(i18n.__("全不选"), false)]
            .spacing(8)
            .align_y(iced::Alignment::Center),
    ]
    .spacing(12);
    let mut candidates = column![];
    if let Some(report) = &s.report {
        for skill in report.skills.iter().filter(|skill| matches_query(skill, &s.query)) {
            let id = skill.directory.clone();
            let reason = if skill.managed {
                i18n.__(" · 已托管")
            } else if skill.conflict {
                i18n.__(" · 来源冲突")
            } else if skill.external_link {
                i18n.__(" · 外部链接")
            } else if !eligible(skill) {
                i18n.__(" · 无法导入")
            } else {
                ""
            };
            let source = skill.apps.iter().copied().map(&name).collect::<Vec<_>>().join("、");
            candidates = candidates
                .push(
                    row![
                        checkbox(s.selected.contains(&id))
                            .size(15)
                            .style(style::checkbox)
                            .on_toggle_maybe(
                                (!s.busy && eligible(skill)).then_some(move |value| message(
                                    Event::Select(id.clone(), value)
                                ))
                            ),
                        column![
                            text(skill.name.clone()).size(14),
                            text(desktop_i18n::__!(i18n, "来源 · %s%s", (if source.is_empty() { skill.directory.as_str() } else { &source }).to_string(), (reason).to_string()).expect("valid extension display template"))
                            .size(12)
                            .style(settings_muted_text_style),
                        ]
                        .spacing(3),
                    ]
                    .spacing(9)
                    .padding([8, 10])
                    .align_y(iced::Alignment::Center),
                )
                .push(rule::horizontal(1));
        }
        if !report.skills.iter().any(|skill| matches_query(skill, &s.query)) {
            candidates = candidates.push(
                container(
                    text(if report.skills.is_empty() {
                        i18n.__("未发现可导入的技能")
                    } else {
                        i18n.__("没有匹配的技能")
                    })
                    .size(13)
                    .style(settings_muted_text_style),
                )
                .center_x(Length::Fill)
                .padding(18),
            );
        }
    } else {
        candidates = candidates.push(
            container(
                text(if s.busy { i18n.__("正在扫描技能…") } else { i18n.__("选择来源后扫描技能") })
                    .size(13)
                    .style(settings_muted_text_style),
            )
            .center_x(Length::Fill)
            .padding(18),
        );
    }
    body = body.push(
        container(
            scrollable(container(candidates).padding([0, 7]))
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
                .height(Length::Shrink),
        )
        .width(Length::Fill)
        .max_height(192)
        .style(style::table),
    );
    body = body.push(settings_field_label(i18n.__("导入后启用的应用")));
    if s.available_targets.is_empty() {
        body = body.push(
            text(i18n.__("没有可选应用，只导入主副本。")).size(12).style(settings_muted_text_style),
        );
    }
    for pair in s.available_targets.chunks(2) {
        let mut choices = row![].spacing(10);
        for target in pair {
            let target = target.clone();
            let app = target.app_id;
            let label =
                if s.available_targets.iter().filter(|entry| entry.app_id == app).count() > 1 {
                    format!("{} · {}", name(app), target.installation_id)
                } else {
                    name(app)
                };
            choices = choices.push(
                row![
                    checkbox(s.targets.contains(&target))
                        .size(15)
                        .style(style::checkbox)
                        .on_toggle_maybe(
                            (!s.busy).then_some(move |v| message(Event::Target(target.clone(), v)))
                        ),
                    icon(app, 17.0),
                    text(label).size(13),
                ]
                .spacing(7)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill),
            );
        }
        if pair.len() == 1 {
            choices = choices.push(Space::new().width(Length::Fill));
        }
        body = body.push(choices);
    }
    let sources = settings_select(
        APPS.iter().copied().map(|id| ScanSource {
            id,
            label: format!("{}{}", if s.apps.contains(&id) { "✓  " } else { "" }, name(id)),
        }),
        None,
        move |source| message(Event::App(source.id, !s.apps.contains(&source.id))),
    )
    .placeholder(i18n.__("切换扫描来源"))
    .disabled(s.busy);
    let source_names = s.apps.iter().copied().map(&name).collect::<Vec<_>>().join("、");
    body = body.push(rule::horizontal(1)).push(
        row![
            column![
                text(i18n.__("扫描来源")).size(13),
                text(if source_names.is_empty() {
                    s.host.clone()
                } else {
                    format!("{} · {source_names}", s.host)
                })
                .size(12)
                .style(settings_muted_text_style),
            ]
            .spacing(3)
            .width(Length::Fill),
            sources,
            button(text(i18n.__("扫描")).size(13))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press_maybe((!s.busy && !s.apps.is_empty()).then_some(message(Event::Scan))),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    );
    for (id, result) in &s.outcomes {
        body = body.push(text(format!(
            "{}：{}",
            id,
            match result {
                Ok(()) => i18n.__("已导入").to_owned(),
                Err(error) => desktop_i18n::__!(i18n, "失败，可重试：%s", (error).to_string()).expect("valid extension display template"),
            }
        )));
    }
    if s.busy {
        body = body.push(text(i18n.__("正在扫描或导入…")));
    }
    if let Some(error) = &s.error {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(i18n, error)));
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        text(i18n.__("导入已有 Skill")).size(20),
        settings_close_button(close.clone()),
    );
    let footer = row![
        Space::new().width(Length::Fill),
        button(i18n.__("取消"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
        button(text(desktop_i18n::__!(i18n, "导入 %s 个", (s.selected.len()).to_string()).expect("valid extension display template")))
            .padding([8, 12])
            .style(style::primary)
            .on_press_maybe((!s.busy && !s.selected.is_empty()).then_some(message(Event::Import))),
    ]
    .spacing(8);
    let card = settings_modal_layout(header, body.width(Length::Fill), Some(footer.into()))
        .style(style::dialog)
        .width(600)
        .height(620);
    settings_modal_overlay_with_escape_and_backdrop_dismiss(
        base,
        close,
        container(card).padding(12),
    )
}
