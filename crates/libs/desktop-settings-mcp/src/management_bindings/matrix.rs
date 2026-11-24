//! Matrix inputs carry exact record revisions and installation identities.
use super::*;
use vw_api_app_identity::AppId;

pub const APPS: [AppId; 12] = [
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
const MIN_NAME_WIDTH: f32 = 180.0;
const CELL_WIDTH: f32 = 36.0;
const ACTIONS_WIDTH: f32 = 66.0;
const TABLE_INSET: f32 = 14.0;

#[derive(Debug, Clone)]
pub enum Action {
    Search(String),
    Cell { record: McpRecordRef, target: McpAppTarget, enabled: bool },
    Row { record: McpRecordRef, enabled: bool },
    Column { target: McpAppTarget, enabled: bool },
    Retry,
    Undo,
}
#[derive(Debug, Clone)]
pub struct Change {
    pub record: McpCatalogRecord,
    pub target: McpAppTarget,
    pub enabled: bool,
    pub previous: bool,
}
#[derive(Debug, Clone)]
pub struct Attempt {
    pub change: Change,
    pub result: Result<McpDistributionItem, String>,
}
#[derive(Debug, Clone, Default)]
pub struct State {
    pub query: String,
    pub column_menu: Option<McpAppTarget>,
    pub pending: Vec<Change>,
    pub attempts: Vec<Attempt>,
    pub undo: Vec<Change>,
}
pub fn visible_records<'a>(
    catalog: &'a McpCatalogResponse,
    query: &str,
) -> Vec<&'a McpCatalogRecord> {
    let query = query.trim().to_lowercase();
    catalog
        .records
        .iter()
        .filter(|r| {
            r.record.id.0.to_lowercase().contains(&query)
                || r.name.to_lowercase().contains(&query)
                || r.public_summary.as_deref().unwrap_or("").to_lowercase().contains(&query)
        })
        .collect()
}
/// Each trusted installation gets its own column; preferences never alter stored bindings.
pub fn columns(
    targets: &[McpAppTarget],
    shown: &[AppId],
    host: &str,
) -> Vec<(AppId, Option<McpAppTarget>)> {
    let mut columns = Vec::new();
    for app in APPS.into_iter().filter(|app| shown.contains(app)) {
        let mut found = Vec::new();
        for target in targets.iter().filter(|t| t.app_id == app && t.target_host_id == host) {
            if !found.contains(target) {
                found.push(target.clone());
            }
        }
        if found.is_empty() {
            columns.push((app, None));
        } else {
            columns.extend(found.into_iter().map(|t| (app, Some(t))));
        }
    }
    columns
}
pub fn plan(
    catalog: &McpCatalogResponse,
    targets: &[McpAppTarget],
    state: &State,
    action: &Action,
) -> Result<Vec<Change>, String> {
    let mut changes: Vec<Change> = Vec::new();
    let mut add = |record: &McpCatalogRecord,
                   target: &McpAppTarget,
                   enabled|
     -> Result<(), String> {
        if !targets.contains(target)
            || !crate::distribution::supported(catalog, std::slice::from_ref(record), target.app_id)
        {
            return Err("目标无可信实例或传输能力不可用".into());
        }
        if changes.iter().any(|c| {
            c.record.record == record.record
                && c.record.scope == record.scope
                && c.target == *target
        }) {
            return Ok(());
        }
        let previous = catalog
            .bindings
            .iter()
            .find(|b| b.record.id == record.record.id && b.target == *target)
            .is_some_and(|b| b.enabled);
        changes.push(Change { record: record.clone(), target: target.clone(), enabled, previous });
        Ok(())
    };
    match action {
        Action::Cell { record, target, enabled } => {
            let r = catalog.records.iter().find(|r| r.record == *record).ok_or("目录修订已变更")?;
            add(r, target, *enabled)?;
        }
        Action::Row { record, enabled } => {
            let r = catalog.records.iter().find(|r| r.record == *record).ok_or("目录修订已变更")?;
            for target in targets.iter().filter(|t| APPS.contains(&t.app_id)) {
                if crate::distribution::supported(catalog, std::slice::from_ref(r), target.app_id) {
                    add(r, target, *enabled)?;
                }
            }
        }
        Action::Column { target, enabled } => {
            for r in visible_records(catalog, &state.query) {
                if crate::distribution::supported(catalog, std::slice::from_ref(r), target.app_id) {
                    add(r, target, *enabled)?;
                }
            }
        }
        Action::Retry => {
            for attempt in &state.attempts {
                let Ok(item) = &attempt.result else {
                    continue;
                }; // Unknown writes require a refresh, never blind replay.
                if !matches!(item.native_outcome, McpDistributionOutcome::Failed { .. }) {
                    continue;
                }
                let latest = state
                    .attempts
                    .iter()
                    .rev()
                    .filter(|a| a.change.record.scope == attempt.change.record.scope)
                    .filter_map(|a| a.result.as_ref().ok())
                    .find(|i| i.record.id == item.record.id && i.desired_committed)
                    .map(|i| &i.record)
                    .unwrap_or(&item.record);
                let r = catalog
                    .records
                    .iter()
                    .find(|r| r.record == *latest && r.scope == attempt.change.record.scope)
                    .ok_or("失败项修订已变更，请刷新核实")?;
                add(r, &item.target, item.enabled)?;
            }
        }
        Action::Undo => {
            for change in &state.undo {
                let r = catalog
                    .records
                    .iter()
                    .find(|r| r.record == change.record.record && r.scope == change.record.scope)
                    .ok_or("撤销遇到中途修订冲突，请刷新核实")?;
                add(r, &change.target, change.enabled)?;
            }
        }
        Action::Search(_) => {}
    }
    if changes.is_empty() {
        return Err("没有可提交的目标".into());
    }
    Ok(changes)
}
/// Single-record requests are below every bulk limit and preserve actual scope.
/// Carry forward only revisions observed from a committed write in this operation.
pub async fn execute(client: GatewayClient, changes: Vec<Change>, token: String) -> Vec<Attempt> {
    let mut attempts: Vec<Attempt> = Vec::new();
    for mut change in changes {
        for previous in &attempts {
            if previous.change.record.scope == change.record.scope
                && previous.change.record.record.id == change.record.record.id
                && let Ok(item) = &previous.result
                    && item.desired_committed {
                        change.record.record = item.record.clone();
                    }
        }
        let result = apply(
            client.clone(),
            vec![change.record.clone()],
            change.target.clone(),
            change.enabled,
            token.clone(),
        )
        .await
        .and_then(|r| r.items.into_iter().next().ok_or_else(|| "响应缺少目标".into()));
        attempts.push(Attempt { change, result });
    }
    attempts
}
impl State {
    pub fn finish(&mut self, attempts: Vec<Attempt>) {
        self.pending.clear();
        self.undo = attempts
            .iter()
            .filter_map(|a| {
                let item = a.result.as_ref().ok()?;
                if !item.desired_committed || a.change.previous == item.enabled {
                    return None;
                }
                let mut change = a.change.clone();
                // Other targets in the same operation may have advanced this record again.
                change.record.record = attempts
                    .iter()
                    .rev()
                    .filter(|b| b.change.record.scope == a.change.record.scope)
                    .filter_map(|b| b.result.as_ref().ok())
                    .find(|b| b.record.id == item.record.id && b.desired_committed)?
                    .record
                    .clone();
                change.enabled = a.change.previous;
                change.previous = item.enabled;
                Some(change)
            })
            .collect();
        self.attempts = attempts;
    }
}

/// Toolbar stays outside the horizontal scroller; task 7 maps edit/menu actions and icons.
#[derive(Debug, Clone)]
pub enum ViewMessage {
    Binding(Action),
    Edit(McpRecordRef),
    Menu(McpRecordRef),
    MenuAction(crate::management_probe::actions::MenuMessage),
    ColumnMenu(Option<McpAppTarget>),
}
pub fn panel<'a>(
    i18n: desktop_i18n::I18n,
    catalog: Option<&'a Result<McpCatalogResponse, String>>,
    state: &'a State,
    targets: &[McpAppTarget],
    shown: &[AppId],
    host: &str,
    loading: bool,
    running: bool,
    menu: Option<&'a crate::management_probe::actions::State>,
) -> iced::Element<'a, ViewMessage> {
    use desktop_assets::assets::Icon;
    use desktop_ui_controls::system_settings_common::{icons::icon_btn_svg_style, *};
    use iced::{
        Alignment, Length,
        widget::{column, container, responsive, row, text, text_input},
    };

    let count = catalog.and_then(|c| c.as_ref().ok()).map(|c| {
        let visible = visible_records(c, &state.query).len();
        if state.query.trim().is_empty() {
            desktop_i18n::__!(i18n, "%s 个 MCP", (visible).to_string())
                .expect("valid extension display template")
        } else {
            desktop_i18n::__!(
                i18n,
                "%s / %s 个 MCP",
                (visible).to_string(),
                (c.records.len()).to_string()
            )
            .expect("valid extension display template")
        }
    });
    let search = container(
        row![
            icon_svg(Icon::Search, 16.0).style(icon_btn_svg_style),
            text_input(i18n.__("搜索名称、命令或地址"), &state.query)
                .on_input(|q| ViewMessage::Binding(Action::Search(q)))
                .style(|theme, status| {
                    let mut style = settings_text_input_style(theme, status);
                    style.background = iced::Color::TRANSPARENT.into();
                    if !matches!(status, iced::widget::text_input::Status::Focused { .. }) {
                        style.border = iced::Border::default();
                    }
                    style
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fill),
        ]
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding::ZERO.left(10))
    .style(table_frame);
    let mut toolbar = row![container(search).width(Length::Fill).max_width(332)]
        .spacing(12)
        .width(Length::Fill);
    if let Some(count) = count {
        toolbar = toolbar.push(text(count).size(13).style(settings_muted_text_style));
    }
    let mut body = column![toolbar.align_y(Alignment::Center)].spacing(12).width(Length::Fill);
    if loading {
        return body.push(empty(i18n.__("正在加载目录…"))).into();
    }
    let catalog = match catalog {
        Some(Ok(c)) => c,
        Some(Err(e)) => {
            return body
                .push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
                    i18n,
                    &e.clone(),
                )))
                .into();
        }
        None => return body.push(empty(i18n.__("尚未加载目录"))).into(),
    };
    let records = visible_records(catalog, &state.query);
    let columns = columns(targets, shown, host);
    let table = responsive(move |size| {
        matrix_table(i18n, catalog, state, &records, &columns, running, menu, size.width)
    })
    .height(Length::Shrink);
    body = body.push(container(table).padding(1).width(Length::Fill).style(table_frame));
    let failures = state
        .attempts
        .iter()
        .filter(|a| match &a.result {
            Ok(item) => matches!(item.native_outcome, McpDistributionOutcome::Failed { .. }),
            Err(_) => true,
        })
        .count();
    if failures > 0 || !state.undo.is_empty() {
        let mut feedback = row![].spacing(8).align_y(Alignment::Center);
        if failures > 0 {
            feedback = feedback
                .push(
                    text(
                        desktop_i18n::__!(i18n, "%s 项写入失败", (failures).to_string())
                            .expect("valid extension display template"),
                    )
                    .size(13),
                )
                .push(crate::toolbar_button(
                    i18n.__("重试失败项"),
                    (!running
                        && state.attempts.iter().any(|a| {
                            a.result.as_ref().is_ok_and(|i| {
                                matches!(i.native_outcome, McpDistributionOutcome::Failed { .. })
                            })
                        }))
                    .then_some(ViewMessage::Binding(Action::Retry)),
                ),
            );
        }
        if !state.undo.is_empty() {
            feedback = feedback.push(crate::toolbar_button(
                i18n.__("撤销本次改动"),
                (!running).then_some(ViewMessage::Binding(Action::Undo)),
            ));
        }
        body = body.push(feedback);
    }
    body.into()
}

fn matrix_table<'a>(
    i18n: desktop_i18n::I18n,
    catalog: &'a McpCatalogResponse,
    state: &'a State,
    records: &[&'a McpCatalogRecord],
    columns: &[(AppId, Option<McpAppTarget>)],
    running: bool,
    menu: Option<&'a crate::management_probe::actions::State>,
    viewport_width: f32,
) -> iced::Element<'a, ViewMessage> {
    use desktop_assets::assets::Icon;
    use desktop_settings_apps::system_settings_apps::list::brand_icon;
    use desktop_ui_controls::{
        single_line_label::SingleLineLabel,
        system_settings_common::{icons::icon_btn_style, *},
    };
    use iced::{
        Alignment, Length,
        widget::{Space, checkbox, column, container, row, rule, scrollable, text},
    };

    let mut header =
        row![text(i18n.__("名称")).size(13).style(settings_muted_text_style).width(Length::Fill)]
            .align_y(Alignment::Center);
    for (app, target) in columns {
        let count = target.as_ref().map_or(0, |t| {
            records.iter().filter(|r| binding(catalog, r, t).is_some_and(|b| b.enabled)).count()
        });
        let label = target.as_ref().map_or_else(
            || {
                desktop_i18n::__!(i18n, "%s · 无可信安装", (app_label(*app)).to_string())
                    .expect("valid extension display template")
            },
            |t| format!("{} · {}", app_label(*app), t.installation_id.0),
        );
        let content = column![
            brand_icon(*app, 17.0),
            text(count.to_string()).size(12).style(settings_muted_text_style),
        ]
        .spacing(3)
        .align_x(Alignment::Center);
        let cell = match target {
            Some(target) if !running && !records.is_empty() => {
                let trigger = hint(
                    iced::widget::button(content)
                        .padding([3, 5])
                        .style(icon_btn_style)
                        .on_press(ViewMessage::ColumnMenu(Some(target.clone()))),
                    label,
                );
                if state.column_menu.as_ref() == Some(target) {
                    let items = column![
                        text(app_label(*app)).size(12).style(settings_muted_text_style),
                        crate::toolbar_button(
                            i18n.__("启用筛选项"),
                            Some(ViewMessage::Binding(Action::Column {
                                target: target.clone(),
                                enabled: true,
                            }))
                        ),
                        crate::toolbar_button(
                            i18n.__("关闭筛选项"),
                            Some(ViewMessage::Binding(Action::Column {
                                target: target.clone(),
                                enabled: false,
                            }))
                        ),
                    ]
                    .spacing(4);
                    desktop_ui_controls::overlays::BelowOverlay::new(
                        trigger,
                        container(items).padding(8).width(170).style(settings_modal_card_style),
                    )
                    .show(true)
                    .gap(5.0)
                    .dismiss_on_escape()
                    .on_close(ViewMessage::ColumnMenu(None))
                    .into()
                } else {
                    trigger
                }
            }
            _ => hint(content, label),
        };
        header = header.push(center_cell(cell));
    }
    header = header.push(Space::new().width(ACTIONS_WIDTH));
    let mut grid = column![
        container(header).padding([8.0, TABLE_INSET]).width(Length::Fill).style(table_header),
    ];
    if records.is_empty() {
        grid = grid.push(rule::horizontal(1)).push(empty(if catalog.records.is_empty() {
            i18n.__("还没有 MCP，点击「添加 MCP」或从应用导入")
        } else {
            i18n.__("没有匹配的 MCP")
        }));
    }
    for &record in records {
        let summary = record.public_summary.as_deref().filter(|s| !s.is_empty()).unwrap_or(
            match record.transport {
                vw_api_mcp_management::McpTransport::Stdio => i18n.__("本地命令 · stdio"),
                vw_api_mcp_management::McpTransport::StreamableHttp => "HTTP",
                vw_api_mcp_management::McpTransport::Sse => "SSE",
            },
        );
        let mut line = row![
            container(
                column![
                    hint(
                        SingleLineLabel::new(record.name.clone(), 14.0),
                        if record.name == record.record.id.0 {
                            record.name.clone()
                        } else {
                            format!("{} · {}", record.name, record.record.id.0)
                        },
                    ),
                    hint(
                        container(SingleLineLabel::new(summary.to_owned(), 12.0)).style(
                            |theme| iced::widget::container::Style {
                                text_color: settings_muted_text_style(theme).color,
                                ..Default::default()
                            }
                        ),
                        summary.to_owned(),
                    ),
                ]
                .spacing(3)
            )
            .width(Length::Fill)
            .padding(iced::Padding::ZERO.right(10)),
        ]
        .align_y(Alignment::Center);
        for (app, target) in columns {
            let mut label = app_label(*app).to_owned();
            let mut failed = false;
            let control: iced::Element<'_, ViewMessage> = if let Some(target) = target {
                let observed = binding(catalog, record, target);
                let enabled = observed.is_some_and(|b| b.enabled);
                let supported = crate::distribution::supported(
                    catalog,
                    std::slice::from_ref(record),
                    target.app_id,
                );
                let pending = running
                    && state
                        .pending
                        .iter()
                        .any(|c| c.record.record.id == record.record.id && c.target == *target);
                let result = state.attempts.iter().rev().find(|a| {
                    a.change.record.record.id == record.record.id && a.change.target == *target
                });
                label = format!(
                    "{label} · {}\n{}",
                    target.installation_id.0,
                    if enabled { i18n.__("期望启用") } else { i18n.__("期望关闭") }
                );
                if !supported {
                    label.push_str(i18n.__("\n传输不支持或能力未验证"));
                }
                if pending {
                    label.push_str(i18n.__("\n正在写入应用配置"));
                } else if let Some(attempt) = result {
                    failed = match &attempt.result {
                        Ok(item) => {
                            matches!(item.native_outcome, McpDistributionOutcome::Failed { .. })
                        }
                        Err(_) => true,
                    };
                    label.push('\n');
                    label.push_str(&match &attempt.result {
                        Ok(item) => crate::distribution::item_label(i18n, item),
                        Err(error) => error.clone(),
                    });
                }
                if let Some(binding) = observed {
                    use vw_api_mcp_management::McpNativeStatus::*;
                    label.push_str(match binding.native_status {
                        Present => i18n.__("\n上次观察：已存在"),
                        Missing => i18n.__("\n上次观察：未找到"),
                        Unreadable => i18n.__("\n上次观察：无法读取"),
                        NotVerified => i18n.__("\n上次观察：未验证"),
                        Unsupported => i18n.__("\n上次观察：不支持"),
                    });
                }
                if pending {
                    text("…").size(16).style(settings_muted_text_style).into()
                } else {
                    let target = target.clone();
                    let record = record.record.clone();
                    checkbox(enabled)
                        .size(17)
                        .style(settings_checkbox_style)
                        .on_toggle_maybe((!running && supported).then_some(move |enabled| {
                            ViewMessage::Binding(Action::Cell {
                                record: record.clone(),
                                target: target.clone(),
                                enabled,
                            })
                        }))
                        .into()
                }
            } else {
                label.push_str(i18n.__("\n无可信安装"));
                checkbox(false).size(17).style(settings_checkbox_style).into()
            };
            let control = if failed {
                row![
                    control,
                    text("•").size(14).style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(theme.extended_palette().danger.base.color),
                    }),
                ]
                .spacing(2)
                .align_y(Alignment::Center)
                .into()
            } else {
                control
            };
            line = line.push(center_cell(hint(control, label)));
        }
        use crate::management_probe::actions::{self, MenuMessage};
        let trigger = icon_btn(
            Icon::DotsThreeVertical,
            i18n.__("MCP 操作"),
            (!running).then(|| ViewMessage::Menu(record.record.clone())),
        );
        let trigger = if let Some(state) = menu.filter(|s| s.row.as_ref() == Some(record)) {
            let reference = record.record.clone();
            let content = column![
                crate::toolbar_button(
                    i18n.__("启用整行"),
                    (!running).then_some(ViewMessage::Binding(Action::Row {
                        record: reference.clone(),
                        enabled: true,
                    }))
                ),
                crate::toolbar_button(
                    i18n.__("关闭整行"),
                    (!running).then_some(ViewMessage::Binding(Action::Row {
                        record: reference,
                        enabled: false,
                    }))
                ),
                rule::horizontal(1),
                actions::menu(i18n, state).map(ViewMessage::MenuAction),
            ]
            .spacing(4);
            desktop_ui_controls::overlays::BelowOverlay::new(
                trigger,
                container(content).width(238).padding(7).style(settings_modal_card_style),
            )
            .show(true)
            .gap(5.0)
            .dismiss_on_escape()
            .on_close(ViewMessage::MenuAction(MenuMessage::Action(actions::Message::Close)))
            .into()
        } else {
            trigger
        };
        line = line.push(
            container(
                row![
                    icon_btn(
                        Icon::Pencil,
                        i18n.__("编辑 MCP"),
                        (!running).then(|| ViewMessage::Edit(record.record.clone()))
                    ),
                    trigger,
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .align_right(ACTIONS_WIDTH),
        );
        grid = grid
            .push(rule::horizontal(1))
            .push(container(line).padding([10.0, TABLE_INSET]).width(Length::Fill));
    }
    // Horizontal scrolling gives its content an unbounded width during layout.
    let min_width =
        MIN_NAME_WIDTH + CELL_WIDTH * columns.len() as f32 + ACTIONS_WIDTH + 2.0 * TABLE_INSET;
    scrollable(container(grid).width(viewport_width.max(min_width)))
        .width(Length::Fill)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(0),
        ))
        .into()
}

fn binding<'a>(
    catalog: &'a McpCatalogResponse,
    record: &McpCatalogRecord,
    target: &McpAppTarget,
) -> Option<&'a vw_api_mcp_management::McpCliBinding> {
    catalog.bindings.iter().find(|b| b.record.id == record.record.id && b.target == *target)
}

fn center_cell<'a>(
    content: impl Into<iced::Element<'a, ViewMessage>>,
) -> iced::Element<'a, ViewMessage> {
    iced::widget::container(content).center_x(CELL_WIDTH).into()
}

fn hint<'a>(
    content: impl Into<iced::Element<'a, ViewMessage>>,
    label: String,
) -> iced::Element<'a, ViewMessage> {
    use desktop_ui_controls::system_settings_common::icons::icon_btn_tooltip_style;
    use iced::widget::{text, tooltip};
    tooltip(content, text(label).size(12), tooltip::Position::Top)
        .style(icon_btn_tooltip_style)
        .padding(8)
        .into()
}

fn empty(label: &'static str) -> iced::Element<'static, ViewMessage> {
    use desktop_ui_controls::system_settings_common::settings_muted_text_style;
    iced::widget::container(iced::widget::text(label).size(13).style(settings_muted_text_style))
        .padding([16, 12])
        .into()
}

fn table_frame(theme: &iced::Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style::default().background(palette.background.base.color).border(
        iced::Border { color: palette.background.strong.color, width: 1.0, radius: 9.0.into() },
    )
}

fn table_header(theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style::default()
        .background(theme.extended_palette().background.weak.color)
        .border(iced::Border { radius: iced::border::top(8), ..iced::Border::default() })
}

pub(crate) fn app_label(app: AppId) -> &'static str {
    match app {
        AppId::Claude => "Claude Code",
        AppId::Codex => "Codex",
        AppId::Gemini => "Gemini CLI",
        AppId::GrokBuild => "Grok Build",
        AppId::OpenCode => "OpenCode",
        AppId::Hermes => "Hermes",
        AppId::Pi => "Pi",
        AppId::Mcode => "MiniMax Code",
        AppId::Kimi => "Kimi Code",
        AppId::Dsh => "DeepSeek Harness",
        AppId::Agy => "Antigravity",
        AppId::Omp => "Oh My Pi",
        AppId::ClaudeDesktop => "Claude Desktop",
        AppId::OpenClaw => "OpenClaw",
    }
}
