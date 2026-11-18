//! Query changes are atomic and always invalidate pagination.
use desktop_assets::assets::{Icon, get_icon, get_provider_icon};
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, rounded_action_btn_style, settings_select,
};
use iced::{
    Element, Length, Theme,
    widget::{button, column, container, row, scrollable, svg, text, tooltip},
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
#[derive(Clone, Debug)]
pub enum Change {
    App(String),
    Provider(String),
    Model(String),
    Source(Option<UsageSourceKind>),
    Range(UsageRange),
    Scope(Option<UsageProvider>, Option<String>),
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
impl std::fmt::Display for ScopeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
pub fn view(query: &UsageQuery) -> Element<'_, Change> {
    view_with_options(query, &[])
}
pub fn view_with_options<'a>(
    query: &'a UsageQuery,
    options: &[ScopeOption],
) -> Element<'a, Change> {
    column![primary_filters(query, options), source_filters(query)].spacing(8).into()
}

pub(super) fn primary_filters<'a>(
    query: &'a UsageQuery,
    options: &[ScopeOption],
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
            "dsh" => get_provider_icon("deepseek"),
            "agy" => get_provider_icon("google"),
            _ => get_provider_icon("agent"),
        };
        let selected = query.filter.app.as_deref() == Some(id);
        apps = apps.push(tooltip(
            button(container(svg(icon).width(18).height(18)).center_y(Length::Fill))
                .height(30)
                .padding([4, 7])
                .style(move |theme, status| filter_button(theme, status, selected))
                .on_press(Change::App(id.into())),
            text(label),
            tooltip::Position::Bottom,
        ));
    }
    let selected = options
        .iter()
        .find(|o| o.provider == query.filter.provider && o.model == query.filter.request_model)
        .cloned();
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
        settings_select(options.iter().cloned(), selected, |o| Change::Scope(o.provider, o.model))
            .placeholder("供应商 / 模型")
            .width(158),
    ]
    .spacing(9)
    .align_y(iced::Alignment::Center)
    .into()
}

pub(super) fn source_filters(query: &UsageQuery) -> Element<'_, Change> {
    let action = |label: &'static str, change, selected| {
        button(text(label).size(12))
            .padding([4, 8])
            .style(move |theme, status| filter_button(theme, status, selected))
            .on_press(change)
    };
    let mut sources = row![].spacing(3);
    for (label, source) in [
        ("全部来源", None),
        ("原生日志", Some(UsageSourceKind::Native)),
        ("路由", Some(UsageSourceKind::Relay)),
        ("识矩", Some(UsageSourceKind::Shiju)),
    ] {
        let selected = query.filter.source_kind == source;
        sources = sources.push(action(label, Change::Source(source), selected));
    }
    sources = sources.push(action("清除筛选", Change::Clear, false));
    sources.wrap().into()
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
