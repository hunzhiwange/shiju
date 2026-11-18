//! Server-paged ledger requests. Column preferences never change the query.
use crate::Identity;

use desktop_assets::assets::{Icon, get_icon};
use desktop_i18n::I18n;
use desktop_ui_controls::{
    selectable_text::markdown_selection::SelectionMessage,
    system_settings_common::help::settings_selectable_help_label,
    text_editor_context_menu::TextContextEffect,
};
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE,
    rounded_action_btn_style, settings_close_button,
    settings_modal_card_style, settings_modal_header, settings_modal_layout,
    settings_pick_list_menu_style, settings_select, settings_text_input_style,
};

use gateway_usage_read::{UsageLogDetail, UsageLogResult, UsageReadScope};
use iced::{
    Element, Font, Length, Task, Theme, font,
    widget::{Space, button, column, container, row, rule, scrollable, svg, text, text_input, tooltip},
};
use vw_api_usage::*;

const PAGE_SIZES: [u32; 4] = [10, 20, 50, 100];
const DEFAULT_PAGE_SIZE: u32 = 100;

#[derive(Default)]
pub struct State {
    ticket: u64,
    detail_ticket: u64,
    pub query: Option<UsageQuery>,
    page: u64,
    page_input: Option<String>,
    loading: bool,
    result: Option<Result<UsageLogResult, String>>,
    detail: Option<Result<UsageLogDetail, String>>,
    detail_open: bool,
    columns: super::log_columns::Preferences,
    columns_open: bool,
    preference_error: Option<String>,
    page_size: u32,
    initialized: bool,
}

impl State {
    pub(super) fn first_page(&mut self) {
        self.page = 1;
        self.page_input = None;
    }

    fn page(&self) -> u64 {
        self.page.max(1)
    }

    fn page_size(&self) -> u32 {
        if self.page_size == 0 { DEFAULT_PAGE_SIZE } else { self.page_size }
    }

    fn last_page(&self) -> u64 {
        request_count(self).unwrap_or(0).div_ceil(u64::from(self.page_size())).max(1)
    }

    fn jump_target(&self) -> Option<u64> {
        self.page_input.as_deref()?.parse::<u64>().ok()
            .filter(|&page| page > 0 && page <= self.last_page() && page != self.page())
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Help(TextContextEffect),
    None,
    Reload,
    Next,
    Previous,
    PageInput(String),
    Jump,
    Toggle(usize),
    Columns,
    CloseColumns,
    Status(Option<UsageRequestStatus>),
    PageSize(u32),
    Open(String),
    CloseDetail,
    Loaded(u64, u64, Identity, UsageQuery, Result<UsageLogResult, String>),
    DetailLoaded(u64, u64, Identity, UsageQuery, Result<UsageLogDetail, String>),
}
impl SelectionMessage for Event {
    fn none() -> Self { Self::None }

    fn copy(text: String) -> Self { Self::Help(TextContextEffect::Copy(text)) }

    fn context(effect: TextContextEffect) -> Self { Self::Help(effect) }

    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self { Self::None }
}

fn message(event: Event) -> Event {
    event
}

/// Called by the shared filter loader, with its exact query snapshot.
pub fn refresh(
    state: &mut State,
    current: Option<Identity>,
    visible: bool,
    open: bool,
    generation: u64,
    client: Result<vw_gateway_client::GatewayClient, String>,
    query: UsageQuery,
) -> Task<Event> {
    state.query = Some(query);
    update(state, current, visible, open, generation, client, Event::Reload)
}

pub fn update(
    state: &mut State,
    current: Option<Identity>,
    visible: bool,
    open: bool,
    generation: u64,
    client: Result<vw_gateway_client::GatewayClient, String>,
    event: Event,
) -> Task<Event> {
    let scope = current.clone();
    let active = open && visible;

    match event {
        Event::None | Event::Help(_) => {},
        Event::Toggle(index) => {
            state.columns.toggle(index);
            state.preference_error = state.columns.save().err();
        }
        Event::Columns => state.columns_open = !state.columns_open,
        Event::CloseColumns => state.columns_open = false,
        Event::Status(status) => {
            let Some(mut query) = state.query.clone() else {
                return Task::none();
            };
            query.filter.status = status;
            state.first_page();
            return refresh(state, current, visible, open, generation, client, query);
        }
        Event::PageSize(size) => {
            if !PAGE_SIZES.contains(&size) {
                return Task::none();
            }
            state.page_size = size;
            let Some(query) = state.query.clone() else {
                return Task::none();
            };
            state.first_page();
            return refresh(state, current, visible, open, generation, client, query);
        }
        Event::PageInput(value) => {
            if value.len() <= 20 && value.bytes().all(|byte| byte.is_ascii_digit()) {
                state.page_input = Some(value);
            }
        }
        Event::CloseDetail => {
            state.detail_open = false;
            state.detail_ticket = state.detail_ticket.wrapping_add(1);
        }
        Event::Loaded(ticket, seq, expected, query, result) => {
            if active
                && generation == seq
                && scope.as_ref() == Some(&expected)
                && state.ticket == ticket
                && state.query.as_ref() == Some(&query)
            {
                state.loading = false;
                state.result = Some(result);
                if state.result.as_ref().is_some_and(Result::is_ok) {
                    state.page = state.page().min(state.last_page());
                }
            }
        }
        Event::DetailLoaded(ticket, seq, expected, query, result) => {
            if active
                && generation == seq
                && scope.as_ref() == Some(&expected)
                && state.detail_ticket == ticket
                && state.detail_open
                && state.query.as_ref() == Some(&query)
            {
                state.detail = Some(result);
            }
        }
        Event::Open(id) => {
            if !active {
                return Task::none();
            }
            let Some(scope) = scope else {
                return Task::none();
            };
            let Some(query) = state.query.clone() else {
                return Task::none();
            };
            state.detail_ticket = state.detail_ticket.wrapping_add(1);
            let ticket = state.detail_ticket;
            state.detail_open = true;
            state.detail = None;
            let client = client.map(|client| {
                gateway_usage_read::GatewayUsageReadClient::new(client.transport().clone())
            });
            return Task::perform(
                async move {
                    let result = match client {
                        Ok(client) => client
                            .usage_log_detail(&read_scope(&scope), &query, &id)
                            .await
                            .map_err(|e| e.to_string()),
                        Err(error) => Err(error),
                    };
                    (scope, query, result)
                },
                move |(scope, query, result)| {
                    message(Event::DetailLoaded(ticket, generation, scope, query, result))
                },
            );
        }
        Event::Reload | Event::Next | Event::Previous | Event::Jump => {
            if !active {
                return Task::none();
            }
            if state.loading && !matches!(event, Event::Reload) {
                return Task::none();
            }
            let page = match event {
                Event::Reload => state.page(),
                Event::Next if state.page() < state.last_page() => state.page() + 1,
                Event::Previous if state.page() > 1 => state.page() - 1,
                Event::Jump => {
                    let Some(page) = state.jump_target() else {
                        return Task::none();
                    };
                    page
                }
                _ => return Task::none(),
            };
            if !matches!(event, Event::Reload) {
                state.page_input = None;
            }
            state.page = page;
            if !state.initialized {
                match super::log_columns::Preferences::load() {
                    Ok(columns) => state.columns = columns,
                    Err(error) => state.preference_error = Some(error),
                }
                state.initialized = true;
            }
            state.detail_open = false;
            state.detail = None;
            state.detail_ticket = state.detail_ticket.wrapping_add(1);
            state.ticket = state.ticket.wrapping_add(1);
            let ticket = state.ticket;
            state.result = None;
            let Some(scope) = scope else {
                state.loading = false;
                state.result = Some(Err("网关不可用，请检查连接".into()));
                return Task::none();
            };
            let Some(query) = state.query.clone() else {
                return Task::none();
            };
            let mut page_query = query.clone();
            page_query.cursor = None;
            page_query.limit = Some(state.page_size());
            state.loading = true;
            let client = client.map(|client| {
                gateway_usage_read::GatewayUsageReadClient::new(client.transport().clone())
            });
            return Task::perform(
                async move {
                    let result = match client {
                        Ok(client) if page == 1 => client
                            .usage_logs(&read_scope(&scope), &page_query)
                            .await
                            .map_err(|e| e.to_string()),
                        Ok(client) => client
                            .usage_log_page(&read_scope(&scope), &page_query, page)
                            .await
                            .map_err(|e| e.to_string()),
                        Err(error) => Err(error),
                    };
                    (scope, query, result)
                },
                move |(scope, query, result)| {
                    message(Event::Loaded(ticket, generation, scope, query, result))
                },
            );
        }
    }
    Task::none()
}
fn read_scope(scope: &Identity) -> UsageReadScope {
    UsageReadScope {
        project_id: scope.project.clone().into(),
        target_host: scope.host.clone(),
        time_zone: "UTC".into(),
    }
}
fn unknown(value: Option<&str>) -> String {
    value.unwrap_or("未知").to_owned()
}
pub fn cost(value: &UsageCost) -> String {
    let source = match value.source {
        UsageCostSource::Unknown => "未知",
        UsageCostSource::Native => "原生",
        UsageCostSource::Calculated => "计算",
        UsageCostSource::Estimated => "估算",
    };
    format!("{} USD（{source}）", unknown(value.amount_usd.as_deref()))
}
pub fn metric(value: Option<&UsageMeasurement>, unit: &str) -> String {
    value
        .filter(|v| v.value.is_finite() && v.value >= 0.0)
        .map(|v| {
            format!(
                "{}{:.1} {unit}",
                if v.kind == UsageMeasurementKind::Estimated { "≈" } else { "" },
                v.value
            )
        })
        .unwrap_or_else(|| "—".into())
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct RequestStatusOption(Option<UsageRequestStatus>);

impl std::fmt::Display for RequestStatusOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            None => "全部状态",
            Some(UsageRequestStatus::Succeeded) => "成功",
            Some(UsageRequestStatus::Failed) => "失败",
            Some(UsageRequestStatus::Cancelled) => "取消",
            Some(UsageRequestStatus::Unknown) => "未知",
        })
    }
}

pub(super) fn toolbar(state: &State) -> Element<'_, Event> {
    use super::log_columns::COLUMNS;
    let status = state.query.as_ref().and_then(|q| q.filter.status);
    let statuses = [
        None,
        Some(UsageRequestStatus::Succeeded),
        Some(UsageRequestStatus::Failed),
        Some(UsageRequestStatus::Cancelled),
        Some(UsageRequestStatus::Unknown),
    ]
    .map(RequestStatusOption);
    let icon = |icon| {
        svg(get_icon(icon))
            .width(16)
            .height(16)
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
    };
    let mut options = column![].spacing(2);
    for (index, (_, label, _, _)) in COLUMNS.iter().enumerate().skip(1) {
        let check: Element<'_, Event> = if state.columns.visible(index) {
            icon(Icon::Check).into()
        } else {
            Space::new().width(16).into()
        };
        options = options.push(
            button(row![check, text(*label).size(13)].spacing(9).align_y(iced::Alignment::Center))
                .width(Length::Fill)
                .padding([6, 9])
                .style(|theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    style.border = iced::border::rounded(7);
                    if status == button::Status::Active {
                        style.background = None;
                    }
                    style
                })
                .on_press(Event::Toggle(index)),
        );
    }
    let menu =
        container(scrollable(container(options).padding([0, 7]).width(Length::Fill)).direction(
            scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ),
        ))
        .padding([5, 0])
        .width(192)
        .max_height(458)
        .style(|theme| {
            let menu_style = settings_pick_list_menu_style(theme);
            let mut style = settings_modal_card_style(theme);
            style.background = Some(menu_style.background);
            style.border = menu_style.border.rounded(12);
            style.shadow.offset.y = 4.0;
            style.shadow.blur_radius = 18.0;
            style
        });
    let trigger = tooltip(
        button(container(icon(Icon::Columns)).center_x(Length::Fill).center_y(Length::Fill))
            .width(32)
            .height(30)
            .padding(0)
            .style(move |theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                style.border = iced::border::rounded(7);
                let ink = theme.palette().text;
                style.background = match status {
                    _ if state.columns_open || status == button::Status::Pressed => Some(ink.scale_alpha(0.10).into()),
                    button::Status::Hovered => Some(ink.scale_alpha(0.06).into()),
                    _ => None,
                };
                style
            })
            .on_press(Event::Columns),
        text("显示列").size(12),
        tooltip::Position::Top,
    )
    .snap_within_viewport(true);
    let columns = desktop_ui_controls::overlays::BelowOverlay::new(trigger, menu)
        .show(state.columns_open)
        .gap(5.0)
        .on_close(Event::CloseColumns)
        .capture_outside_click(true);
    row![
        settings_select(statuses, Some(RequestStatusOption(status)), |option| {
            Event::Status(option.0)
        })
        .toolbar_icon(Icon::Funnel)
        .width(120),
        container(rule::vertical(1)).height(18),
        columns
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

pub(super) fn request_count(state: &State) -> Option<u64> {
    state.result.as_ref()?.as_ref().ok().map(|result| result.counts.total_requests)
}

pub fn view(state: &State, i18n: I18n) -> Element<'_, Event> {
    column![container(toolbar(state)).align_right(Length::Fill), table(state, i18n)].spacing(8).into()
}

pub(super) fn table(state: &State, i18n: I18n) -> Element<'_, Event> {
    use super::log_columns::COLUMNS;
    let widths: Vec<f32> = COLUMNS
        .iter()
        .enumerate()
        .map(|(index, (_, _, _, minimum))| {
            state
                .result
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .map(|r| {
                    r.page
                        .items
                        .iter()
                        .map(|d| cell(d, index).chars().count() as f32 * 9.0 + 24.0)
                        .fold(*minimum, f32::max)
                })
                .unwrap_or(*minimum)
        })
        .collect();
    let mut rows = column![].spacing(0);
    let mut header = row![].spacing(12).align_y(iced::Alignment::Center);
    for (index, (_, label, _, _)) in COLUMNS.iter().enumerate() {
        if state.columns.visible(index) {
            let title = text(*label).size(12);
            let help = match index {
                5 => Some("新增输入按来源语义扣除缓存；缺少必要数据时显示 —"),
                9 => Some("缓存读取 /（新增输入 + 缓存读取 + 缓存创建）"),
                10 => Some(
                    "速度怎么算：每秒输出的 token 数。≈ 表示根据会话日志时间戳估算，可能包含等待首字的时间；来源未提供时显示 —。",
                ),
                11 | 12 => Some(
                    "时间怎么算：单位为秒。首字表示开始输出前的等待时间，总耗时表示请求持续时间。≈ 为日志时间戳估算，可能包含客户端准备时间；来源未提供时显示 —。",
                ),
                _ => None,
            };
            let title: Element<'_, Event> = match help {
                Some(help) => settings_selectable_help_label(i18n, None, label, title, help).into(),
                None => title.into(),
            };
            header = header.push(container(title).width(Length::Fixed(widths[index])));
        }
    }
    rows = rows.push(container(header).padding([10, 14]).style(|theme: &Theme| {
        container::Style::default().background(theme.extended_palette().background.weak.color)
    }));
    if state.loading {
        rows = rows.push(text("正在加载请求…"));
    }
    match &state.result {
        Some(Err(error)) => rows = rows.push(text(format!("请求加载失败：{error}"))),
        Some(Ok(result)) => {
            if result.page.items.is_empty() {
                rows = rows.push(container(text("暂无请求")).padding(24));
            }
            for detail in &result.page.items {
                let mut cells = row![].spacing(12);
                for (index, (_, _, _, _)) in COLUMNS.iter().enumerate() {
                    if state.columns.visible(index) {
                        let value = if index == 0 && state.columns.visible(13) {
                            cell(detail, index).split(" · ").next().unwrap_or("—").to_owned()
                        } else {
                            cell(detail, index)
                        };
                        if index == 1 {
                            let app = detail
                                .record
                                .sources
                                .first()
                                .map(|s| s.app.as_str())
                                .unwrap_or("agent");
                            cells = cells.push(
                                row![
                                    iced::widget::svg(desktop_assets::assets::get_provider_icon(
                                        app
                                    ))
                                    .width(14)
                                    .height(14),
                                    text(value).size(13)
                                ]
                                .spacing(6)
                                .width(Length::Fixed(widths[index])),
                            );
                        } else {
                            cells = cells
                                .push(text(value).size(13).width(Length::Fixed(widths[index])));
                        }
                    }
                }
                rows = rows.push(
                    button(cells)
                        .padding([10, 14])
                        .style(|theme: &iced::Theme, status| {
                            let mut style = button::text(theme, status);
                            style.border = iced::Border {
                                width: 0.5,
                                color: theme.extended_palette().background.strong.color,
                                ..Default::default()
                            };
                            style
                        })
                        .on_press(Event::Open(detail.record.id.clone())),
                );
            }
        }
        None => {}
    }
    let nav = |label: &'static str, event: Event, enabled: bool| {
        button(container(text(label).size(13)).center_y(Length::Fill))
            .height(SETTINGS_CONTROL_HEIGHT)
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(event))
    };
    let request_count = request_count(state);
    let total = request_count.map_or_else(|| "—".into(), |count| count.to_string());
    let page_size = state.page_size();
    let page = state.page();
    let last_page = state.last_page();
    let page_count = request_count.map_or_else(|| "/ —".into(), |_| format!("/ {last_page}"));
    let can_jump = !state.loading && state.jump_target().is_some();
    let current_page = page.to_string();
    let page_input = text_input("页码", state.page_input.as_deref().unwrap_or(&current_page))
        .width(82)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .padding(SETTINGS_CONTROL_PADDING)
        .style(settings_text_input_style)
        .on_input(Event::PageInput)
        .on_submit_maybe(can_jump.then_some(Event::Jump));
    let footer = container(
        row![
            text(format!("共 {total} 条记录")).size(13).style(text::secondary),
            settings_select(PAGE_SIZES, Some(page_size), Event::PageSize).width(108),
            text("条 / 页").size(13).style(text::secondary),
            iced::widget::Space::new().width(Length::Fill),
            nav("‹", Event::Previous, !state.loading && page > 1),
            page_input,
            text(page_count).size(13),
            nav("›", Event::Next, !state.loading && page < last_page),
            nav("跳转", Event::Jump, can_jump)
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .padding([12, 16])
    .width(Length::Fill)
    .style(container::rounded_box);
    let table = scrollable(container(rows).padding([0, 7]))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(3),
        ))
        .height(Length::Shrink);
    let mut content = column![].spacing(8);
    if let Some(error) = &state.preference_error {
        content = content
            .push(text(format!("列设置保存/加载失败：{error}")).size(12).style(text::danger));
    }
    content.push(table).push(footer).into()
}

const DETAIL_FONT: Font = Font {
    #[cfg(target_os = "macos")]
    family: font::Family::Name("Hiragino Sans GB"),
    weight: font::Weight::Light,
    ..Font::DEFAULT
};

pub(super) fn detail_drawer(state: &State) -> Option<(Event, Element<'_, Event>)> {
    if !state.detail_open {
        return None;
    }
    let body: Element<'_, Event> = match &state.detail {
        None => text("正在加载详情…").font(DETAIL_FONT).style(text::secondary).into(),
        Some(Err(error)) => text(format!("请求详情加载失败：{error}"))
            .font(DETAIL_FONT)
            .style(text::danger)
            .into(),
        Some(Ok(detail)) => detail_view(detail),
    };
    let header = column![
        settings_modal_header(
            text("请求详情").size(21).font(DETAIL_FONT),
            settings_close_button(Event::CloseDetail),
        ),
        rule::horizontal(1)
    ]
    .spacing(12);
    let drawer = settings_modal_layout(header, body, None).width(500).style(|theme| {
        let mut style = settings_modal_card_style(theme).background(theme.palette().background);
        style.border.radius = iced::border::left(18);
        style
    });
    Some((Event::CloseDetail, drawer.into()))
}

fn detail_view(detail: &UsageLogDetail) -> Element<'_, Event> {
    fn section(
        title: &'static str,
        values: Vec<(&'static str, String)>,
    ) -> Element<'static, Event> {
        let mut rows = column![].spacing(12);
        for (label, value) in values {
            rows = rows.push(
                row![
                    text(label).size(13).font(DETAIL_FONT).style(text::secondary).width(76),
                    text(value)
                        .size(14)
                        .font(DETAIL_FONT)
                        .width(Length::Fill)
                        .wrapping(iced::advanced::text::Wrapping::WordOrGlyph)
                        .align_x(iced::alignment::Horizontal::Right)
                ]
                .spacing(16),
            );
        }
        column![
            text(title).size(17).font(DETAIL_FONT),
            container(rows).padding(14).width(Length::Fill).style(super::view::panel_style)
        ]
        .spacing(10)
        .into()
    }
    let r = &detail.record;
    let exact = |n: Option<u64>| n.map_or_else(|| "—".into(), |n| n.to_string());
    let t = r.tokens.as_ref();
    let time = chrono::DateTime::from_timestamp(r.started_at_unix_seconds.0, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%Y/%m/%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "—".into());
    let mut body = column![
        section(
            "基本信息",
            vec![
                ("时间", time),
                ("应用", cell(detail, 1)),
                ("供应商", cell(detail, 2)),
                ("模型", cell(detail, 3)),
                ("请求 ID", r.id.clone())
            ]
        ),
        section(
            "Tokens 用量",
            vec![
                ("新增输入", exact(t.and_then(fresh))),
                ("原始输入", exact(t.and_then(|t| t.input))),
                ("输出", exact(t.and_then(|t| t.output))),
                ("缓存命中", exact(t.and_then(|t| t.cache_read))),
                ("缓存写入", exact(t.and_then(|t| t.cache_creation)))
            ]
        ),
        section(
            "成本（USD）",
            vec![
                ("合计", cell(detail, 4)),
                ("计价说明", cost(&r.cost)),
                ("定价模型", unknown(r.models.pricing_model.as_deref()))
            ]
        ),
        section(
            "性能",
            vec![
                ("速度", cell(detail, 10)),
                ("首字", cell(detail, 11)),
                ("总耗时", cell(detail, 12)),
                ("状态码", cell(detail, 13))
            ]
        ),
        text("≈ 表示来源提供的估算值；— 表示暂无数据。费用分项尚未由账本提供。")
            .size(12)
            .font(DETAIL_FONT)
            .style(text::secondary)
    ]
    .spacing(18);
    if let Some(error) = &detail.error_message {
        body = body.push(section("错误信息", vec![("详情", error.clone())]));
    }
    body.into()
}

fn count(value: Option<u64>) -> String {
    match value {
        Some(n) if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
        Some(n) if n >= 1000 => format!("{:.1}K", n as f64 / 1000.0),
        Some(n) => n.to_string(),
        None => "—".into(),
    }
}
fn fresh(tokens: &UsageTokens) -> Option<u64> {
    match tokens.input_semantics {
        UsageInputSemantics::Fresh => tokens.input,
        UsageInputSemantics::Total => {
            tokens.input?.checked_sub(tokens.cache_read?.checked_add(tokens.cache_creation?)?)
        }
        UsageInputSemantics::Unknown => None,
    }
}
fn seconds(value: Option<&UsageMeasurement>) -> String {
    let converted = value.map(|v| UsageMeasurement { value: v.value / 1000.0, kind: v.kind });
    metric(converted.as_ref(), "s")
}
fn cell(detail: &UsageLogDetail, index: usize) -> String {
    let r = &detail.record;
    let t = r.tokens.as_ref();
    match index {
        0 => {
            let time = chrono::DateTime::from_timestamp(r.started_at_unix_seconds.0, 0)
                .map(|t| t.with_timezone(&chrono::Local).format("%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "未知时间".into());
            if r.status == UsageRequestStatus::Succeeded {
                time
            } else {
                format!(
                    "{time} · {}",
                    detail
                        .status_code
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{:?}", r.status))
                )
            }
        }
        1 => {
            let mut apps: Vec<_> = r.sources.iter().map(|s| s.app.as_str()).collect();
            apps.sort_unstable();
            apps.dedup();
            if apps.is_empty() { "未知".into() } else { apps.join(" / ") }
        }
        2 => super::groups::provider_label(
            &r.provider,
            !r.sources.is_empty() && r.sources.iter().all(|source| source.kind == UsageSourceKind::Native),
        )
        .into(),
        3 => unknown(r.models.response_model.as_deref().or(r.models.request_model.as_deref())),
        4 => r
            .cost
            .amount_usd
            .as_ref()
            .map(|v| {
                let amount = v
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .map(|v| format!("${v:.4}"))
                    .unwrap_or_else(|| format!("{v} USD"));
                format!(
                    "{}{amount}",
                    if r.cost.source == UsageCostSource::Estimated { "≈" } else { "" }
                )
            })
            .unwrap_or_else(|| "未定价".into()),
        5 => count(t.and_then(fresh)),
        6 => count(t.and_then(|t| t.output)),
        7 => count(t.and_then(|t| t.cache_read)),
        8 => count(t.and_then(|t| t.cache_creation)),
        9 => t
            .and_then(|t| {
                Some((
                    t.cache_read?,
                    fresh(t)?.checked_add(t.cache_read?)?.checked_add(t.cache_creation?)?,
                ))
            })
            .filter(|(_, total)| *total > 0)
            .map(|(read, total)| format!("{:.1}%", read as f64 / total as f64 * 100.0))
            .unwrap_or_else(|| "—".into()),
        10 => metric(r.performance.output_tokens_per_second.as_ref(), "tok/s"),
        11 => seconds(r.performance.first_token_ms.as_ref()),
        12 => seconds(r.performance.latency_ms.as_ref()),
        13 => detail.status_code.map(|s| s.to_string()).unwrap_or_else(|| "—".into()),
        _ => String::new(),
    }
}
