//! Query changes are atomic and always invalidate pagination.
use vw_api_usage::{UsageFilter, UsageQuery, UsageRange, UsageSourceKind, UsageProvider};
use iced::{Element, widget::{button, column, row, text, pick_list, svg, tooltip}};
use desktop_assets::assets::{get_icon, get_provider_icon, Icon};
use desktop_ui_controls::system_settings_common::rounded_action_btn_style;

pub const APPS: [(&str, &str); 11] = [
    ("claude", "Claude"), ("codex", "Codex"), ("gemini", "Gemini"),
    ("grokbuild", "Grok"), ("opencode", "OpenCode"), ("pi", "Pi"),
    ("omp", "OMP"), ("mcode", "MCode"), ("kimi", "Kimi"), ("dsh", "DSH"), ("agy", "AGY"),
];
pub fn default_query() -> UsageQuery {
    UsageQuery { range: UsageRange::All, filter: UsageFilter::default(), cursor: None, limit: None }
}
#[derive(Clone, Debug)]
pub enum Change {
    App(String), Provider(String), Model(String), Source(Option<UsageSourceKind>), Range(UsageRange),
    Scope(Option<UsageProvider>, Option<String>), Clear,
}
pub fn apply(query: &mut UsageQuery, change: Change) {
    let optional = |value: String| if value.trim().is_empty() { None } else { Some(value.trim().to_owned()) };
    match change {
        Change::App(value) => {
            let app = optional(value);
            if query.filter.app != app { query.filter.provider = None; query.filter.request_model = None; }
            query.filter.app = app;
        }
        Change::Provider(value) => {
            let provider = optional(value).map(|provider_id| UsageProvider::Known { provider_id });
            if query.filter.provider != provider { query.filter.request_model = None; }
            query.filter.provider = provider;
        }
        Change::Model(value) => query.filter.request_model = optional(value),
        Change::Scope(provider, model) => { query.filter.provider = provider; query.filter.request_model = model.and_then(optional); }
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.label) }
}
pub fn view(query: &UsageQuery) -> Element<'_, Change> { view_with_options(query, &[]) }
pub fn view_with_options<'a>(query: &'a UsageQuery, options: &'a [ScopeOption]) -> Element<'a, Change> {
    let action = |label: &'static str, change| button(text(label).size(13)).padding([6,10]).style(rounded_action_btn_style).on_press(change);
    let mut apps = row![action("全部应用", Change::App(String::new()))].spacing(4);
    for (id, label) in APPS {
        let icon = match id {
            "claude" => get_icon(Icon::AppClaudeCode), "codex" => get_icon(Icon::AppCodex),
            "gemini" => get_icon(Icon::AppGeminiCli), "opencode" => get_icon(Icon::AppOpenCode),
            "pi" | "omp" => get_icon(Icon::AppPi), "kimi" => get_icon(Icon::AppKimiCode),
            "grokbuild" => get_provider_icon("xai"), "dsh" => get_provider_icon("deepseek"),
            "agy" => get_provider_icon("google"), _ => get_provider_icon("agent"),
        };
        let selected = query.filter.app.as_deref() == Some(id);
        apps = apps.push(tooltip(button(svg(icon).width(20).height(20))
            .padding([6,8]).style(move |theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                if selected { style.background = Some(theme.extended_palette().background.strong.color.into()); }
                style
            }).on_press(Change::App(id.into())), text(label), tooltip::Position::Bottom));
    }
    let selected = options.iter().find(|o| o.provider == query.filter.provider && o.model == query.filter.request_model).cloned();
    column![apps, row![
        pick_list(options.to_vec(), selected, |o| Change::Scope(o.provider, o.model)).placeholder("供应商 / 模型").padding([6,10]),
        action("清除筛选", Change::Clear), action("全部来源", Change::Source(None)),
        action("原生日志", Change::Source(Some(UsageSourceKind::Native))), action("路由", Change::Source(Some(UsageSourceKind::Relay))), action("识矩", Change::Source(Some(UsageSourceKind::Shiju))),
    ].spacing(8)].spacing(8).into()
}
