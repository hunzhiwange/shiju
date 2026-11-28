use crate::{Scope, state::list::Snapshot};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, keyed_column, row, scrollable, text, text_input, tooltip},
};
use std::hash::{DefaultHasher, Hash, Hasher};
use vw_api_app_identity::AppId;
use vw_api_app_provider_edit::catalog::{ProviderMembership, ProviderSelection};
#[derive(Clone)]
pub enum ListMessage {
    Create,
    Presets(crate::presets::Event),
    OpenEdit { id: String, copy: bool, delete: bool },
    Refresh,
    QueryInput(String),
    Terminal(String),
    UseProvider(String),
    #[cfg(not(target_arch = "wasm32"))]
    UsageSettings(AppId, String),
    Edit(String),
    Endpoints(String),
    ModelDiagnostics(String),
    FailoverHealth(String),
    Preview(String),
    Desktop(Option<String>),
    Icons(String),
    ModelCatalog(String),
    Universal,
    Snippets,
    Failover,
    Paths,
    Omo(bool),
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
fn menu<M: Clone + 'static>(i18n: desktop_i18n::I18n,
    items: Vec<(&'static str, M)>,
    label: &'static str,
    enabled: bool,
) -> Element<'static, M> {
    let labels = items.iter().map(|(label, _)| *label).collect::<Vec<_>>();
    let select = settings_select(labels, None, move |choice| {
        let (_, event) = items.iter().find(|(label, _)| *label == choice).expect("menu option");
        event.clone()
    })
    .format_labels(|label| i18n.__(label).to_owned())
    .menu_icon(Icon::DotsThreeVertical, 184.0)
    .placeholder(i18n.__(label))
    .width(36)
    .height(34.0)
    .disabled(!enabled);

    tooltip(select, container(text(i18n.__(label)).size(12)).padding([6, 8]), tooltip::Position::Bottom)
        .style(settings_panel_style)
        .snap_within_viewport(true)
        .into()
}

fn selected(value: &ProviderSelection, provider_id: &str) -> bool {
    matches!(value, ProviderSelection::Selected { id } if id == provider_id)
}

pub struct Page<'a, M> {
    pub tabs: Element<'a, M>,
    pub lead: Option<Element<'a, M>>,
    pub can_launch: bool,
    pub can_switch: bool,
    pub tools: Vec<(&'static str, M)>,
    pub search: Option<(bool, M)>,
}

pub fn page<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    message: fn(ListMessage) -> M,
    state: View<'a>,
    supplements: Vec<(&'a str, Option<Element<'a, M>>, Element<'a, M>)>,
    title: Element<'a, M>,
    page: Page<'a, M>,
) -> Element<'a, M> {
    let (header, body) = content(i18n, message, state, supplements, title, page);
    let list = scrollable(container(body).padding([0, 7]).width(Length::Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Length::Fill)
        .height(Length::Fill);
    column![header, list].spacing(12).width(Length::Fill).height(Length::Fill).into()
}

fn content<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    message: fn(ListMessage) -> M,
    state: View<'a>,
    mut supplements: Vec<(&'a str, Option<Element<'a, M>>, Element<'a, M>)>,
    title: Element<'a, M>,
    page: Page<'a, M>,
) -> (Element<'a, M>, Element<'a, M>) {
    let Some(scope) = state.scope else {
        return (title, iced::widget::Space::new().into());
    };
    let Page {
        tabs,
        lead,
        can_launch,
        can_switch,
        tools: page_tools,
        search: search_toggle,
    } = page;
    let mut body = column![].spacing(10).width(Length::Fill);
    if state.loading {
        body = body.push(settings_panel(text(i18n.__("正在读取…")).style(settings_muted_text_style)));
    }
    if let Some(error) = state.error {
        body = body.push(settings_error_banner(i18n.__(error.as_str())));
    }
    if let Some(lead) = lead {
        body = body.push(lead);
    }
    if let Some(snapshot) = state.snapshot {
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
                    i18n.__("没有已保存的供应商")
                } else {
                    i18n.__("没有匹配的供应商，请修改或清空搜索")
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
                items.push((i18n.__("复制"), open(true, false)));
            }
            #[cfg(not(target_arch = "wasm32"))]
            items.push((i18n.__("配置用量查询"), ListMessage::UsageSettings(scope.app, id.clone())));
            items.push((i18n.__("端点与请求选项"), ListMessage::Endpoints(id.clone())));
            if matches!(scope.app, AppId::Claude | AppId::Codex | AppId::Gemini) {
                items.extend([
                    (i18n.__("模型诊断"), ListMessage::ModelDiagnostics(id.clone())),
                    (i18n.__("健康与实际路由"), ListMessage::FailoverHealth(id.clone())),
                ]);
            }
            if crate::state::preview::available(scope.app) {
                items.push((i18n.__("配置预览"), ListMessage::Preview(id.clone())));
            }
            if scope.app == AppId::ClaudeDesktop {
                items.push((i18n.__("直连与映射"), ListMessage::Desktop(Some(id.clone()))));
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
                    items.push((if add { i18n.__("添加到应用") } else { i18n.__("从应用移除") }, member));
                }
            }
            items.extend([
                (i18n.__("图标"), ListMessage::Icons(id.clone())),
                (i18n.__("排序与备注"), ListMessage::Edit(id.clone())),
                (i18n.__("模型目录"), ListMessage::ModelCatalog(id.clone())),
            ]);

            if !active && !direct {
                items.push((i18n.__("删除"), open(false, true)));
            }
            let items = items.into_iter().map(|(label, event)| (label, message(event))).collect();
            let mut identity =
                row![text(&provider.name).size(15)].spacing(8).align_y(Alignment::Center);
            if direct {
                identity = identity.push(settings_value_badge(i18n.__("直连")));
            }
            let category = presentation.and_then(|item| item.category.as_deref());
            let mut metadata = vec![match (category, provider.configured) {
                (Some("official"), _) => i18n.__("官方"),
                (_, true) => i18n.__("凭据已配置"),
                (_, false) => i18n.__("凭据未配置"),
            }];
            if let Some(category) = category.filter(|s| !s.is_empty() && *s != "official") {
                metadata.push(category);
            }
            if scope.app != AppId::Agy {
                metadata.push(match entry.native_member {
                    ProviderMembership::Present => i18n.__("已添加"),
                    ProviderMembership::Absent => i18n.__("未添加"),
                    ProviderMembership::Unknown => i18n.__("成员状态未知"),
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
            let avatar_glyph: Element<'_, M> =
                if matches!(id.as_str(), "openai" | "openai-official" | "codex-official") {
                    provider_logo_svg("openai", 21.0).into()
                } else {
                    text(initial).size(14).into()
                };
            let avatar = container(avatar_glyph)
                .center_x(36)
                .center_y(36)
                .style(settings_content_panel_style);
            let mut controls = row![].spacing(6).align_y(Alignment::Center);
            if active {
                controls = controls.push(text(i18n.__("● 当前")).size(12).style(|theme: &iced::Theme| {
                    text::Style { color: Some(theme.extended_palette().primary.strong.color) }
                }));
            }
            if !active && can_switch && crate::tray_switch::supports(scope.app) {
                controls = controls.push(icon_btn(
                    Icon::ArrowRight,
                    i18n.__("切换到此供应商"),
                    enabled.then(|| message(ListMessage::UseProvider(id.clone()))),
                ));
            }
            if can_launch {
                controls = controls.push(icon_btn(
                    Icon::Terminal,
                    i18n.__("用此供应商打开终端"),
                    enabled.then(|| message(ListMessage::Terminal(id.clone()))),
                ));
            }
            let mut details = column![
                row![
                    avatar,
                    summary,
                    controls,
                    icon_btn(
                        Icon::Pencil,
                        i18n.__("编辑供应商"),
                        enabled.then(|| message(open(false, false)))
                    ),
                    menu(i18n, items, i18n.__("更多供应商操作"), enabled),
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
    let create = if crate::presets::app_name(scope.app).is_some() {
        ListMessage::Presets(crate::presets::Event::Open)
    } else {
        ListMessage::Create
    };
    let mut tools = vec![
        (i18n.__("刷新"), ListMessage::Refresh),
        (i18n.__("添加到其他智能体"), ListMessage::Universal),
        (i18n.__("备用队列"), ListMessage::Failover),
    ];
    if crate::state::snippets::available(scope.app) {
        tools.push((i18n.__("公共配置片段"), ListMessage::Snippets));
    }
    if cfg!(target_os = "macos") && scope.app == AppId::Claude {
        tools.push((i18n.__("配置目录"), ListMessage::Paths));
    }
    match scope.app {
        AppId::OpenCode => tools.extend([
            (i18n.__("OMO 标准目录"), ListMessage::Omo(false)),
            (i18n.__("OMO Slim 目录"), ListMessage::Omo(true)),
        ]),
        AppId::Pi => tools.push((i18n.__("Pi 模型条目"), ListMessage::PiDirectory)),
        AppId::Omp => tools.push((i18n.__("OMP 独立模型条目"), ListMessage::OmpDirectory)),
        _ => {}
    }
    let tools = page_tools
        .into_iter()
        .chain(tools.into_iter().map(|(label, event)| (label, message(event))))
        .collect();
    let show_search = search_toggle.as_ref().is_none_or(|(open, _)| *open);
    let mut toolbar = row![].spacing(6).align_y(Alignment::Center);
    if let Some((open, toggle)) = search_toggle {
        toolbar = toolbar.push(icon_btn(
            if open { Icon::X } else { Icon::Search },
            if open { i18n.__("收起搜索") } else { i18n.__("搜索供应商") },
            Some(toggle),
        ));
    }
    toolbar = toolbar
        .push(
            button(text(i18n.__("＋ 添加供应商")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(primary_action_btn_style)
                .on_press_maybe(ready.then(|| message(create))),
        )
        .push(menu(i18n, tools, i18n.__("供应商工具"), ready));
    let mut filters = row![].spacing(8).align_y(Alignment::Center);
    if show_search {
        filters = filters.push(
            settings_text_input(i18n.__("搜索供应商名称、标识或备注"), state.query)
                .on_input(move |query| message(ListMessage::QueryInput(query)))
                .width(Length::Fill),
        );
    }
    let heading = settings_modal_header(title, toolbar);
    let filters = row![tabs, container(filters).width(Length::Fill).max_width(420)]
        .spacing(14)
        .align_y(Alignment::Center)
        .wrap();
    let header = column![
        container(heading).padding([0, 7]),
        settings_divider(),
        container(filters).padding([0, 7]),
    ]
    .spacing(12);
    (header.into(), body.into())
}

#[derive(Clone)]
pub enum EditorMessage { Sort(String), Category(String), Notes(String), Cancel, Save }
fn message(event: EditorMessage) -> crate::Message { crate::Message::ListEditor(event) }
fn editor_action(label: &str, event: EditorMessage, enabled: bool) -> Element<'_, crate::Message> {
 button(text(label)).padding([8,14]).style(rounded_action_btn_style).on_press_maybe(enabled.then_some(message(event))).into()
}
pub fn editor_card<'a>(i18n: desktop_i18n::I18n, sort: &'a str, category: &'a str, notes: &'a str, saving: bool) -> Element<'a, crate::Message> {
    let input = |hint: &'static str, value: &'a str, event: fn(String) -> EditorMessage| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe((!saving).then_some(move |v| message(event(v))))
    };
    let fields = settings_section_content(
        column![
            settings_field_label(i18n.__("排序")),
            settings_control_with_hint(
                input(i18n.__("排序"), sort, EditorMessage::Sort),
                i18n.__("非负整数，留空清除"),
            ),
            settings_field_label(i18n.__("分类")),
            input(i18n.__("分类"), category, EditorMessage::Category),
            settings_field_label(i18n.__("备注")),
            input(i18n.__("备注"), notes, EditorMessage::Notes),
        ]
        .spacing(8),
    );
    let cancel = message(EditorMessage::Cancel);
    let header = settings_modal_header(
        text(i18n.__("排序、分类与备注")).size(20),
        settings_close_button(cancel.clone()),
    );
    let actions = row![
        editor_action(i18n.__("取消"), EditorMessage::Cancel, !saving),
        button(text(if saving { i18n.__("保存中…") } else { i18n.__("保存") }))
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
pub fn local_card<'a, L, F, P, D, O, M, U>(
    state: &'a crate::state::list::ListState<L, F, P, D, O, M, U>,
    help: super::help::Help<'_>,
) -> Option<(crate::Message, Element<'a, crate::Message>)> {
    let i18n = help.i18n;
    use crate::Message as E;
    let app = state.scope.as_ref()?.app;
    if let Some(form) = &state.paths {
        return Some((
            if form.saving { E::None } else { E::Paths(super::paths::Event::Close) },
            super::paths::card(form, help),
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
        return Some((E::Icons(crate::message::icons::Event::Close), super::icons::card(i18n, form)));
    }
    if let Some(form) = &state.endpoints {
        return Some((
            E::Endpoints(crate::message::endpoints::Event::Cancel),
            super::endpoints::card(i18n, form, app),
        ));
    }
    if let Some(form) = &state.desktop {
        return Some((
            E::Desktop(crate::message::desktop::Event::Cancel),
            super::desktop::card(form, help),
        ));
    }
    None
}
