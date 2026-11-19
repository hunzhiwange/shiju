use crate::{Scope, state::list::Snapshot};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, keyed_column, row, text, text_input, tooltip},
};
use std::hash::{DefaultHasher, Hash, Hasher};
use vw_api_app_identity::AppId;
use vw_api_app_provider_edit::catalog::{ProviderMembership, ProviderSelection};
#[derive(Clone)]
pub enum ListMessage {
    Create,
    OpenEdit { id: String, copy: bool, delete: bool },
    Close,
    Refresh,
    QueryInput(String),
    Edit(String),
    Endpoints(String),
    ModelDiagnostics(String),
    FailoverHealth(String),
    Preview(String),
    DesktopAggregation(String),
    Desktop(Option<String>),
    Icons(String),
    ModelCatalog(String),
    Universal,
    Snippets,
    Env,
    Aggregation,
    Failover,
    Mode,
    Paths,
    Omo(bool),
    CodexSubagent,
    PiDirectory,
    OmpDirectory,
    HermesMember { id: String, add: bool },
    PiMember { id: String, add: bool },
    KimiMember { id: String, add: bool },
    McodeMember { id: String, add: bool },
    DshMember { id: String, add: bool },
    OmpMember { id: String, add: bool },
    OpenClawMember { id: String, add: bool },
    OpenCodeMember { id: String, add: bool },
}

pub struct View<'a> {
    pub scope: Option<&'a Scope>,
    pub query: &'a str,
    pub loading: bool,
    pub error: Option<&'a String>,
    pub snapshot: Option<&'a Snapshot>,
    pub editing: bool,
}
fn selection(value: &ProviderSelection) -> String {
    match value {
        ProviderSelection::Selected { id } => id.clone(),
        ProviderSelection::Unconfigured => "未配置".into(),
        ProviderSelection::NotApplicable => "不适用".into(),
        ProviderSelection::Unavailable { reason } => format!("不可读取：{reason:?}"),
    }
}
fn menu<M: Clone + 'static>(
    message: fn(ListMessage) -> M,
    items: Vec<(&'static str, ListMessage)>,
    label: &'static str,
    enabled: bool,
) -> Element<'static, M> {
    let labels = items.iter().map(|(label, _)| *label).collect::<Vec<_>>();
    let select = settings_select(labels, None, move |choice| {
        let (_, event) = items.iter().find(|(label, _)| *label == choice).expect("menu option");
        message(event.clone())
    })
    .menu_icon(Icon::DotsThreeVertical, 184.0)
    .placeholder(label)
    .width(36)
    .height(34.0)
    .disabled(!enabled);

    tooltip(select, container(text(label).size(12)).padding([6, 8]), tooltip::Position::Bottom)
        .style(settings_panel_style)
        .snap_within_viewport(true)
        .into()
}

fn selected(value: &ProviderSelection, provider_id: &str) -> bool {
    matches!(value, ProviderSelection::Selected { id } if id == provider_id)
}

pub fn card<'a, M: Clone + 'static>(
    message: fn(ListMessage) -> M,
    state: View<'a>,
    mut supplements: Vec<(&'a str, Option<Element<'a, M>>, Element<'a, M>)>,
) -> Element<'a, M> {
    let Some(scope) = state.scope else {
        return iced::widget::Space::new().into();
    };
    let mut body = column![].spacing(10).width(Length::Fill);
    if state.loading {
        body = body.push(settings_panel(text("正在读取…").style(settings_muted_text_style)));
    }
    if let Some(error) = state.error {
        body = body.push(settings_error_banner(error.as_str()));
    }
    if let Some(snapshot) = state.snapshot {
        let mut status = column![
            row![
                settings_value_badge(format!("直连 · {}", selection(&snapshot.catalog.direct))),
                settings_value_badge(format!("原生当前 · {}", selection(&snapshot.catalog.in_use))),
            ]
            .spacing(8)
            .wrap()
        ]
        .spacing(8);
        if let Some(reason) = snapshot.catalog.native_read {
            status = status.push(
                text(format!("原生状态不可用：{reason:?}"))
                    .size(13)
                    .style(settings_muted_text_style),
            );
        }
        body = body.push(
            container(status)
                .width(Length::Fill)
                .padding([10, 12])
                .style(settings_content_panel_style),
        );
        let query = state.query.trim().to_lowercase();
        let rows = snapshot
            .catalog
            .rows
            .iter()
            .filter_map(|entry| {
                let provider = &entry.provider;
                let presentation = snapshot
                    .presentation
                    .iter()
                    .find(|item| item.id == provider.id)
                    .map(|item| &item.presentation);
                let matches = query.is_empty()
                    || provider.name.to_lowercase().contains(&query)
                    || provider.id.to_lowercase().contains(&query)
                    || presentation
                        .and_then(|item| item.notes.as_deref())
                        .is_some_and(|notes| notes.to_lowercase().contains(&query));
                (provider.app_id == scope.app && matches).then_some((entry, presentation))
            })
            .collect::<Vec<_>>();
        if rows.is_empty() {
            body = body.push(settings_panel(
                text(if query.is_empty() {
                    "没有已保存的供应商"
                } else {
                    "没有匹配的供应商，请修改或清空搜索"
                })
                .style(settings_muted_text_style),
            ));
        }
        let mut cards = keyed_column![].spacing(8);
        for (entry, presentation) in rows {
            let provider = &entry.provider;
            let id = &provider.id;
            let active = selected(&snapshot.catalog.in_use, id);
            let direct = selected(&snapshot.catalog.direct, id);
            let enabled = !state.editing && !state.loading;
            let open = |copy, delete| ListMessage::OpenEdit { id: id.clone(), copy, delete };
            let mut items = Vec::new();
            if scope.app != AppId::Agy {
                items.push(("复制", open(true, false)));
            }
            items.push(("端点与请求选项", ListMessage::Endpoints(id.clone())));
            if matches!(scope.app, AppId::Claude | AppId::Codex | AppId::Gemini) {
                items.extend([
                    ("模型诊断", ListMessage::ModelDiagnostics(id.clone())),
                    ("健康与实际路由", ListMessage::FailoverHealth(id.clone())),
                ]);
            }
            if crate::state::preview::available(scope.app) {
                items.push(("配置预览", ListMessage::Preview(id.clone())));
            }
            if scope.app == AppId::ClaudeDesktop {
                items.extend([
                    ("聚合映射与应用", ListMessage::DesktopAggregation(id.clone())),
                    ("直连与映射", ListMessage::Desktop(Some(id.clone()))),
                ]);
            }
            if entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                let member = match scope.app {
                    AppId::Hermes => Some(ListMessage::HermesMember { id: id.clone(), add }),
                    AppId::Pi => Some(ListMessage::PiMember { id: id.clone(), add }),
                    AppId::Kimi => Some(ListMessage::KimiMember { id: id.clone(), add }),
                    AppId::Mcode => Some(ListMessage::McodeMember { id: id.clone(), add }),
                    AppId::Dsh => Some(ListMessage::DshMember { id: id.clone(), add }),
                    AppId::Omp => Some(ListMessage::OmpMember { id: id.clone(), add }),
                    AppId::OpenClaw => Some(ListMessage::OpenClawMember { id: id.clone(), add }),
                    AppId::OpenCode => Some(ListMessage::OpenCodeMember { id: id.clone(), add }),
                    _ => None,
                };
                if let Some(member) = member {
                    items.push((if add { "添加到应用" } else { "从应用移除" }, member));
                }
            }
            items.extend([
                ("图标", ListMessage::Icons(id.clone())),
                ("排序与备注", ListMessage::Edit(id.clone())),
                ("模型目录", ListMessage::ModelCatalog(id.clone())),
                ("删除", open(false, true)),
            ]);

            let mut identity =
                row![text(&provider.name).size(16)].spacing(8).align_y(Alignment::Center);
            if active {
                identity = identity.push(settings_value_badge("原生当前"));
            }
            if direct {
                identity = identity.push(settings_value_badge("直连"));
            }
            let mut metadata =
                vec![if provider.configured { "凭据已配置" } else { "凭据未配置" }];
            if let Some(category) =
                presentation.and_then(|item| item.category.as_deref()).filter(|s| !s.is_empty())
            {
                metadata.push(category);
            }
            if scope.app != AppId::Agy {
                metadata.push(match entry.native_member {
                    ProviderMembership::Present => "已添加",
                    ProviderMembership::Absent => "未添加",
                    ProviderMembership::Unknown => "成员状态未知",
                });
            }
            let mut summary = column![
                identity.wrap(),
                text(metadata.join(" · ")).size(12).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill);
            if let Some(notes) =
                presentation.and_then(|item| item.notes.as_deref()).filter(|s| !s.is_empty())
            {
                summary = summary.push(text(notes).size(13).style(settings_muted_text_style));
            }
            let initial = provider
                .name
                .chars()
                .find(|ch| !ch.is_whitespace())
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            let avatar = container(text(initial).size(15))
                .center_x(36)
                .center_y(36)
                .style(settings_content_panel_style);
            let mut details = column![
                row![
                    avatar,
                    summary,
                    icon_btn(
                        Icon::Pencil,
                        "编辑供应商",
                        enabled.then(|| message(open(false, false)))
                    ),
                    menu(message, items, "更多供应商操作", enabled),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
            ]
            .spacing(10);

            let extra = supplements
                .iter()
                .position(|(key, _, _)| *key == id)
                .map(|index| supplements.remove(index));
            let usage = if let Some((_, health, usage)) = extra {
                if let Some(health) = health {
                    details = details.push(health);
                }
                Some(usage)
            } else {
                None
            };
            if let Some(usage) = usage {
                details = details.push(usage);
            }
            let mut key = DefaultHasher::new();
            id.hash(&mut key);
            cards = cards.push(
                key.finish(),
                container(details).padding([10, 12]).width(Length::Fill).style(move |theme| {
                    let mut style = settings_content_panel_style(theme);
                    if active {
                        let accent = theme.palette().primary;
                        style.background = Some(accent.scale_alpha(0.09).into());
                        style.border.color = accent.scale_alpha(0.72);
                        style.border.width = 1.0;
                    }
                    style
                }),
            );
        }
        body = body.push(cards);
    }

    let ready = !state.loading && !state.editing;
    let create = if scope.app == AppId::ClaudeDesktop {
        ListMessage::Desktop(None)
    } else {
        ListMessage::Create
    };
    let mut tools = vec![
        ("刷新", ListMessage::Refresh),
        ("统一供应商投影", ListMessage::Universal),
        ("聚合目录", ListMessage::Aggregation),
        ("备用队列", ListMessage::Failover),
    ];
    if crate::state::snippets::available(scope.app) {
        tools.push(("公共配置片段", ListMessage::Snippets));
    }
    if crate::state::env::available(scope.app) {
        tools.push(("环境诊断", ListMessage::Env));
    }
    if cfg!(target_os = "macos") && scope.app == AppId::Claude {
        tools.push(("配置目录", ListMessage::Paths));
    }
    match scope.app {
        AppId::OpenCode => tools.extend([
            ("OMO 标准目录", ListMessage::Omo(false)),
            ("OMO Slim 目录", ListMessage::Omo(true)),
        ]),
        AppId::Codex => tools.push(("Codex 子代理设置", ListMessage::CodexSubagent)),
        AppId::Pi => tools.push(("Pi 模型条目", ListMessage::PiDirectory)),
        AppId::Omp => tools.push(("OMP 独立模型条目", ListMessage::OmpDirectory)),
        _ => {}
    }
    let toolbar = row![
        button(text("＋ 添加供应商").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(ready.then(|| message(create))),
        menu(message, tools, "供应商工具", ready),
        settings_close_button(message(ListMessage::Close)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    let search = settings_text_input("搜索供应商名称、标识或备注", state.query)
        .on_input(move |query| message(ListMessage::QueryInput(query)))
        .width(Length::Fill);
    let mut filters = row![
        icon_svg(Icon::Search, 15.0)
            .style(desktop_ui_controls::system_settings_common::icons::icon_btn_svg_style),
        search,
        icon_btn(
            Icon::X,
            "清空搜索",
            (!state.query.is_empty()).then(|| message(ListMessage::QueryInput(String::new())))
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if cfg!(target_os = "macos") && scope.app == AppId::Claude {
        filters = filters.push(
            button(text("供应商模式").size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(ready.then(|| message(ListMessage::Mode))),
        );
    }
    let header = column![
        settings_modal_header(text(format!("{:?} 供应商", scope.app)).size(21), toolbar),
        container(filters).padding([0, 7]),
        settings_divider(),
    ]
    .spacing(12);
    settings_modal_layout(header, body, None).width(880).height(680).into()
}

#[derive(Clone)]
pub enum EditorMessage { Sort(String), Category(String), Notes(String), Cancel, Save }
fn message(event: EditorMessage) -> crate::Message { crate::Message::ListEditor(event) }
fn editor_action(label: &str, event: EditorMessage, enabled: bool) -> Element<'_, crate::Message> {
 button(text(label)).padding([8,14]).style(rounded_action_btn_style).on_press_maybe(enabled.then_some(message(event))).into()
}
pub fn editor_card<'a>(sort: &'a str, category: &'a str, notes: &'a str, saving: bool) -> Element<'a, crate::Message> {
    let input = |hint: &'static str, value: &'a str, event: fn(String) -> EditorMessage| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe((!saving).then_some(move |v| message(event(v))))
    };
    let fields = settings_panel(
        column![
            settings_field_label("排序"),
            settings_control_with_hint(
                input("排序", sort, EditorMessage::Sort),
                "非负整数，留空清除",
            ),
            settings_field_label("分类"),
            input("分类", category, EditorMessage::Category),
            settings_field_label("备注"),
            input("备注", notes, EditorMessage::Notes),
        ]
        .spacing(8),
    );
    let cancel = message(EditorMessage::Cancel);
    let header = settings_modal_header(
        text("排序、分类与备注").size(20),
        settings_close_button(cancel.clone()),
    );
    let actions = row![
        editor_action("取消", EditorMessage::Cancel, !saving),
        button(text(if saving { "保存中…" } else { "保存" }))
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe((!saving).then_some(message(EditorMessage::Save))),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, fields, Some(actions.into())).width(560).height(480);
    container(card).padding(12).into()
}

/// Provider-owned modal precedence and close guards, shared by host and view tests.
pub fn local_card<'a, L, F, A, S, P, D, O, M, U>(
    state: &'a crate::state::list::ListState<L, F, A, S, P, D, O, M, U>,
    help: super::help::Help<'_>,
) -> Option<(crate::Message, Element<'a, crate::Message>)> {
    use crate::Message as E;
    let app = state.scope.as_ref()?.app;
    if let Some(form) = &state.paths {
        return Some((
            if form.saving { E::None } else { E::Paths(super::paths::Event::Close) },
            super::paths::card(form, help),
        ));
    }
    if let Some(form) = &state.mode {
        return Some((
            if form.writing() { E::None } else { E::Mode(super::mode::Event::Close) },
            super::mode::card(state.snapshot.as_ref(), form, help),
        ));
    }
    if let Some(form) = &state.env {
        return Some((
            if form.writing() { E::None } else { E::Env(crate::message::env::Event::Close) },
            super::env::card(form, help),
        ));
    }
    if let Some(form) = &state.preview {
        return Some((
            if form.save.is_some() { E::None } else { E::Preview(super::preview::Event::Close) },
            super::preview::card(form, help),
        ));
    }
    if let Some(form) = &state.snippets {
        return Some((
            if form.pending == Some(crate::state::snippets::Operation::Save) {
                E::None
            } else {
                E::Snippets(super::snippets::Event::Close)
            },
            super::snippets::card(state.snapshot.as_ref(), form, app, help),
        ));
    }
    if let Some(form) = &state.universal {
        return Some((
            if form.busy { E::None } else { E::Universal(super::universal::Event::Close) },
            super::universal::card(form, help),
        ));
    }
    if let Some(form) = &state.icons {
        return Some((E::Icons(crate::message::icons::Event::Close), super::icons::card(form)));
    }
    if let Some(form) = &state.endpoints {
        return Some((
            E::Endpoints(crate::message::endpoints::Event::Cancel),
            super::endpoints::card(form, app),
        ));
    }
    if let Some(form) = &state.desktop {
        if form.aggregation.is_none() {
            return Some((
                E::Desktop(crate::message::desktop::Event::Cancel),
                super::desktop::card(form, help),
            ));
        }
    }
    None
}
