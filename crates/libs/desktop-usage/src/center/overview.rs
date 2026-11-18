//! Query changes are atomic and always invalidate pagination.
use desktop_assets::assets::{Icon, get_icon, get_provider_icon};
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, SettingsSelectLabel, icon_svg, rounded_action_btn_style,
    settings_modal_card_style,
};
use desktop_ui_controls::{overlays::BelowOverlay, tooltip::tooltip_dark_style};
use iced::{
    Element, Length, Theme,
    widget::{Space, button, column, container, row, rule, scrollable, svg, text, tooltip},
};
use vw_api_usage::{UsageFilter, UsageProvider, UsageQuery, UsageRange, UsageSourceKind};

pub const APPS: [(&str, &str); 11] = [
    ("claude", "Claude"),
    ("codex", "Codex"),
    ("gemini", "Gemini"),
    ("grokbuild", "Grok"),
    ("opencode", "OpenCode"),
    ("pi", "Pi"),
    ("omp", "OMP"),
    ("mcode", "MCode"),
    ("kimi", "Kimi"),
    ("dsh", "DSH"),
    ("agy", "AGY"),
];
pub fn default_query() -> UsageQuery {
    UsageQuery { range: UsageRange::All, filter: UsageFilter::default(), cursor: None, limit: None }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMenu { Scope, Source }

#[derive(Clone, Debug)]
pub enum Change {
    App(String),
    Provider(String),
    Model(String),
    Source(Option<UsageSourceKind>),
    Range(UsageRange),
    Scope(Option<UsageProvider>, Option<String>),
    OpenMenu(Option<FilterMenu>),
    Clear,
}
pub fn apply(query: &mut UsageQuery, change: Change) {
    let optional =
        |value: String| if value.trim().is_empty() { None } else { Some(value.trim().to_owned()) };
    match change {
        Change::App(value) => {
            let app = optional(value);
            if query.filter.app != app {
                query.filter.provider = None;
                query.filter.request_model = None;
            }
            query.filter.app = app;
        }
        Change::Provider(value) => {
            let provider = optional(value).map(|provider_id| UsageProvider::Known { provider_id });
            if query.filter.provider != provider {
                query.filter.request_model = None;
            }
            query.filter.provider = provider;
        }
        Change::Model(value) => query.filter.request_model = optional(value),
        Change::Scope(provider, model) => {
            query.filter.provider = provider;
            query.filter.request_model = model.and_then(optional);
        }
        Change::OpenMenu(_) => return,
        Change::Source(value) => query.filter.source_kind = value,
        Change::Range(value) => query.range = value,
        Change::Clear => query.filter = UsageFilter::default(),
    }
    query.cursor = None;
}
/// Supplied by scoped provider/model queries; never derive options from a paginated log page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeOption {
    pub label: String,
    pub provider: Option<UsageProvider>,
    pub model: Option<String>,
}
const SOURCES: [(&str, Option<UsageSourceKind>); 4] = [
    ("全部来源", None),
    ("原生日志", Some(UsageSourceKind::Native)),
    ("路由", Some(UsageSourceKind::Relay)),
    ("识矩", Some(UsageSourceKind::Shiju)),
];

pub fn view(query: &UsageQuery, open_menu: Option<FilterMenu>) -> Element<'_, Change> {
    primary_filters(query, &[], open_menu)
}

pub(super) fn primary_filters<'a>(
    query: &'a UsageQuery,
    options: &[ScopeOption],
    open_menu: Option<FilterMenu>,
) -> Element<'a, Change> {
    let mut apps = row![
        button(
            text("全部").size(13).height(Length::Fill).align_y(iced::alignment::Vertical::Center)
        )
        .height(30)
        .padding([4, 9])
        .style(move |theme, status| filter_button(theme, status, query.filter.app.is_none()))
        .on_press(Change::App(String::new()))
    ]
    .spacing(2)
    .align_y(iced::Alignment::Center);
    for (id, label) in APPS {
        let icon = match id {
            "claude" => get_icon(Icon::AppClaudeCode),
            "codex" => get_icon(Icon::AppCodex),
            "gemini" => get_icon(Icon::AppGeminiCli),
            "opencode" => get_icon(Icon::AppOpenCode),
            "pi" | "omp" => get_icon(Icon::AppPi),
            "kimi" => get_icon(Icon::AppKimiCode),
            "grokbuild" => get_provider_icon("xai"),
            "mcode" => get_provider_icon("minimax"),
            "dsh" => get_provider_icon("deepseek"),
            "agy" => get_icon(Icon::AppAntigravity),
            _ => get_provider_icon("agent"),
        };
        let selected = query.filter.app.as_deref() == Some(id);
        apps = apps.push(
            tooltip(
                button(container(svg(icon).width(18).height(18)).center_y(Length::Fill))
                    .height(30)
                    .padding([4, 7])
                    .style(move |theme, status| filter_button(theme, status, selected))
                    .on_press(Change::App(id.into())),
                text(label).size(12),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .padding(7)
            .style(tooltip_dark_style)
            .snap_within_viewport(true),
        );
    }
    let apps = scrollable(container(apps).padding([4, 5]).style(|theme: &Theme| {
        container::Style::default().border(iced::Border {
            color: theme.palette().text.scale_alpha(0.14),
            width: 1.0,
            radius: 9.0.into(),
        })
    }))
    .width(Length::Fill)
    .height(SETTINGS_CONTROL_HEIGHT)
    .direction(scrollable::Direction::Horizontal(
        scrollable::Scrollbar::new().width(4).scroller_width(4),
    ));
    row![
        container(apps).width(Length::Fill).max_width(448),
        scope_menu(query, options, open_menu),
        source_menu(query, open_menu),
    ]
    .spacing(9)
    .align_y(iced::Alignment::Center)
    .into()
}

fn scope_menu<'a>(
    query: &UsageQuery,
    options: &[ScopeOption],
    open_menu: Option<FilterMenu>,
) -> Element<'a, Change> {
    let filter = &query.filter;
    let current_scope = options.iter().find(|option| {
        option.provider == filter.provider && option.model == filter.request_model
    }).map(|option| option.label.as_str()).or_else(|| filter.request_model.as_deref()).or_else(|| {
        match filter.provider.as_ref() {
            Some(UsageProvider::Known { provider_id }) => Some(provider_id.as_str()),
            Some(UsageProvider::Unknown) => Some("未知供应商"),
            None => None,
        }
    });
    filter_dropdown(
        FilterMenu::Scope, open_menu, Icon::ListUl, current_scope.unwrap_or("供应商 / 模型"), 164,
        || {
            let mut items = column![menu_item(
                "全部供应商 / 模型".into(), Change::Scope(None, None),
                filter.provider.is_none() && filter.request_model.is_none(),
            )].spacing(2).width(Length::Fill);
            for option in options {
                let selected = option.provider == filter.provider && option.model == filter.request_model;
                let change = Change::Scope(option.provider.clone(), option.model.clone());
                items = items.push(menu_item(option.label.clone(), change, selected));
            }
            column![
                scrollable(items).height(Length::Shrink).direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(3)
                )),
                rule::horizontal(1),
                menu_item("清除所有筛选".into(), Change::Clear, false),
            ].spacing(4).into()
        },
    )
}

fn source_menu<'a>(query: &UsageQuery, open_menu: Option<FilterMenu>) -> Element<'a, Change> {
    let active = query.filter.source_kind;
    let label = SOURCES.iter().find(|(_, source)| *source == active)
        .map_or("全部来源", |(label, _)| *label);
    filter_dropdown(FilterMenu::Source, open_menu, Icon::Funnel, label, 126, || {
        column(SOURCES.map(|(label, source)| {
            menu_item(label.into(), Change::Source(source), source == active)
        })).spacing(2).into()
    })
}

fn filter_dropdown<'a>(
    menu: FilterMenu,
    open_menu: Option<FilterMenu>,
    icon: Icon,
    label: &str,
    width: u32,
    content: impl FnOnce() -> Element<'a, Change>,
) -> Element<'a, Change> {
    let open = open_menu == Some(menu);
    let trigger = button(container(row![
        icon_svg(icon, 15.0),
        SettingsSelectLabel::new(label.to_owned()),
        icon_svg(Icon::ChevronDown, 11.0),
    ].spacing(8).align_y(iced::Alignment::Center)).center_y(Length::Fill))
    .width(width)
    .height(SETTINGS_CONTROL_HEIGHT)
    .padding([0, 9])
    .style(rounded_action_btn_style)
    .on_press(Change::OpenMenu(if open { None } else { Some(menu) }));
    if !open {
        return trigger.into();
    }
    let panel = container(content()).padding(7)
        .width(if menu == FilterMenu::Scope { 292 } else { width })
        .max_height(410)
        .style(filter_popup_style);
    BelowOverlay::new(trigger, panel)
        .show(true)
        .gap(6.0)
        .on_close(Change::OpenMenu(None))
        .capture_outside_click(true)
        .into()
}

pub(super) fn filter_popup_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let surface = iced::Color { a: 1.0, ..palette.background };
    let mut style = settings_modal_card_style(theme).background(surface);
    style.border = iced::Border {
        color: palette.text.scale_alpha(0.13),
        width: 1.0,
        radius: 12.0.into(),
    };
    style.shadow.color = iced::Color::BLACK.scale_alpha(0.12);
    style.shadow.blur_radius = 12.0;
    style.shadow.offset.y = 4.0;
    style
}

fn menu_item<'a>(label: String, change: Change, selected: bool) -> Element<'a, Change> {
    let mark: Element<'a, Change> = if selected {
        icon_svg(Icon::Check, 13.0).into()
    } else {
        Space::new().width(13).into()
    };
    button(row![mark, SettingsSelectLabel::new(label)].spacing(8).align_y(iced::Alignment::Center))
        .width(Length::Fill)
        .padding([6, 8])
        .style(move |theme, status| filter_button(theme, status, selected))
        .on_press(change)
        .into()
}

fn filter_button(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border.width = 0.0;
    style.border.radius = 6.0.into();
    if selected {
        style.background = Some(theme.extended_palette().background.strong.color.into());
    } else if status == button::Status::Active {
        style.background = None;
    }
    style
}
