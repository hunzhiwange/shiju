//! Syntax highlighting engine for the TUI.
//!
//! Wraps [syntect] with the [two_face] grammar and theme bundles to provide
//! ~250-language syntax highlighting and 32 bundled color themes.  The module
//! owns five process-global singletons:
//!
//! | Singleton | Type | Purpose |
//! |---|---|---|
//! | `SYNTAX_SET` | `OnceLock<SyntaxSet>` | Grammar database, immutable after init |
//! | `THEME` | `OnceLock<RwLock<Theme>>` | Active color theme, swappable at runtime |
//! | `THEME_REVISION` | `AtomicU64` | Invalidates rendered-content caches after theme swaps |
//! | `THEME_OVERRIDE` | `OnceLock<Option<String>>` | Persisted user preference (write-once) |
//! | `CODEX_HOME` | `OnceLock<Option<PathBuf>>` | Root for custom `.tmTheme` discovery |
//!
//! **Lifecycle:** call [`set_theme_override`] once at startup (after the final
//! config is resolved) to persist the user preference and seed the `THEME`
//! lock.  After that, [`set_syntax_theme`] and [`current_syntax_theme`] can
//! swap/snapshot the theme for live preview.  All highlighting functions read
//! the theme via `theme_lock()`.
//!
//! **Guardrails:** inputs exceeding 512 KB or 10 000 lines, or containing an
//! individual line longer than 4 KiB, are rejected early (returns `None`) to
//! prevent pathological CPU/memory usage.  Callers must fall back to plain
//! unstyled text.
//!
//! 本模块以 `syntect` 与 `two_face` 提供语法高亮，并集中持有语法集、可热切换主题、
//! 主题修订号、持久化覆盖项及自定义主题根目录五个进程级单例。启动时应在配置最终确定后
//! 调用 `set_theme_override`；运行中主题切换会递增修订号以使渲染缓存失效。超过大小、
//! 行数或单行长度护栏的输入会返回 `None`，调用方必须退回无样式文本。

use ratatui::style::Color as RtColor;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use syntect::easy::HighlightLines;
use syntect::highlighting::Color as SyntectColor;
use syntect::highlighting::FontStyle;
use syntect::highlighting::Highlighter;
use syntect::highlighting::Style as SyntectStyle;
use syntect::highlighting::Theme;
use syntect::highlighting::ThemeSet;
use syntect::parsing::Scope;
use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use two_face::theme::EmbeddedThemeName;

#[path = "highlight_streaming.rs"]
mod streaming;

pub use streaming::StreamingCodeHighlighter;

// -- Global singletons -------------------------------------------------------
// -- 进程级全局单例 ---------------------------------------------------------

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME: OnceLock<RwLock<Theme>> = OnceLock::new();
static THEME_REVISION: AtomicU64 = AtomicU64::new(0);
static THEME_OVERRIDE: OnceLock<Option<String>> = OnceLock::new();
static CODEX_HOME: OnceLock<Option<PathBuf>> = OnceLock::new();

// Syntect/bat encode ANSI palette semantics in alpha:
// `a=0` => indexed ANSI palette via RGB payload, `a=1` => terminal default.
// Syntect/bat 借用 alpha 通道编码 ANSI 调色板语义：0 表示 RGB 载荷中的索引，1 表示终端默认色。
const ANSI_ALPHA_INDEX: u8 = 0x00;
const ANSI_ALPHA_DEFAULT: u8 = 0x01;
const OPAQUE_ALPHA: u8 = 0xFF;

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

// NOTE: We intentionally do NOT emit a runtime diagnostic when an ANSI-family
// theme (ansi, base16, base16-256) lacks the expected alpha-channel marker
// encoding.  If the upstream two_face/syntect theme format changes, the
// `ansi_themes_use_only_ansi_palette_colors` test will catch it at build
// time — long before it reaches users.  A runtime warning would be
// unactionable noise since users can't fix upstream themes.
// 这里刻意不产生运行时诊断；上游主题格式变化由测试在构建期捕获，运行时警告对用户不可操作。

/// Set the user-configured syntax theme override and codex home path.
///
/// Call this with the **final resolved config** (after onboarding, resume, and
/// fork reloads complete). The first call persists `name` and `codex_home` in
/// `OnceLock`s used by startup/default theme resolution.
///
/// Subsequent calls cannot change the persisted `OnceLock` values, but they
/// still update the runtime theme immediately for live preview flows.
///
/// Returns user-facing warnings for actionable configuration issues, such as
/// unknown/invalid theme names or duplicate override persistence.
/// 设置用户配置的主题覆盖与 CODEX_HOME；首次调用固定持久化来源，后续调用仍可更新实时预览主题。
/// 返回未知、无效主题或重复持久化等用户可处理的警告。
pub fn set_theme_override(
    name: Option<String>,
    codex_home: Option<PathBuf>,
) -> Option<String> {
    let warning = validate_theme_name(name.as_deref(), codex_home.as_deref());
    let override_set_ok = THEME_OVERRIDE.set(name.clone()).is_ok();
    let codex_home_set_ok = CODEX_HOME.set(codex_home.clone()).is_ok();
    if THEME.get().is_some() {
        set_syntax_theme(resolve_theme_with_override(name.as_deref(), codex_home.as_deref()));
    }
    if !override_set_ok || !codex_home_set_ok {
        // This should never happen in practice — set_theme_override is only
        // called once at startup.  Keep as a debug breadcrumb in case a second
        // call site is added in the future.
        // 正常启动只会调用一次；保留调试痕迹以发现未来误加的第二个调用点。
        tracing::debug!("set_theme_override called more than once; OnceLock values unchanged");
    }
    warning
}

/// Check whether a theme name resolves to a bundled theme or a custom
/// `.tmTheme` file.  Returns a user-facing warning when it does not.
/// 检查主题名能否解析为内置主题或自定义 `.tmTheme`，失败时生成面向用户的警告。
pub fn validate_theme_name(name: Option<&str>, codex_home: Option<&Path>) -> Option<String> {
    let name = name?;
    let custom_theme_path_display = codex_home
        .map(|home| custom_theme_path(name, home).display().to_string())
        .unwrap_or_else(|| format!("$CODEX_HOME/themes/{name}.tmTheme"));
    // Bundled themes always resolve.
    // 内置主题始终可解析，无需访问磁盘。
    if parse_theme_name(name).is_some() {
        return None;
    }
    // Custom themes must parse successfully; an unreadable/invalid file should
    // still surface a startup warning so users can diagnose configuration issues.
    // 自定义主题必须可成功解析；文件不可读或格式无效时仍应在启动阶段明确告警。
    if let Some(home) = codex_home {
        let custom_path = custom_theme_path(name, home);
        if custom_path.is_file() {
            if load_custom_theme(name, home).is_some() {
                return None;
            }
            return Some(format!(
                "Custom theme \"{name}\" at {custom_theme_path_display} could not \
                 be loaded (invalid .tmTheme format). Falling back to the default theme."
            ));
        }
    }
    Some(format!(
        "Theme \"{name}\" not found. Using the default theme. \
         To use a custom theme, place a .tmTheme file at \
         {custom_theme_path_display}."
    ))
}

/// Map a kebab-case theme name to the corresponding `EmbeddedThemeName`.
/// 将 kebab-case 主题名映射到对应的内置主题枚举。
fn parse_theme_name(name: &str) -> Option<EmbeddedThemeName> {
    match name {
        "ansi" => Some(EmbeddedThemeName::Ansi),
        "base16" => Some(EmbeddedThemeName::Base16),
        "base16-eighties-dark" => Some(EmbeddedThemeName::Base16EightiesDark),
        "base16-mocha-dark" => Some(EmbeddedThemeName::Base16MochaDark),
        "base16-ocean-dark" => Some(EmbeddedThemeName::Base16OceanDark),
        "base16-ocean-light" => Some(EmbeddedThemeName::Base16OceanLight),
        "base16-256" => Some(EmbeddedThemeName::Base16_256),
        "catppuccin-frappe" => Some(EmbeddedThemeName::CatppuccinFrappe),
        "catppuccin-latte" => Some(EmbeddedThemeName::CatppuccinLatte),
        "catppuccin-macchiato" => Some(EmbeddedThemeName::CatppuccinMacchiato),
        "catppuccin-mocha" => Some(EmbeddedThemeName::CatppuccinMocha),
        "coldark-cold" => Some(EmbeddedThemeName::ColdarkCold),
        "coldark-dark" => Some(EmbeddedThemeName::ColdarkDark),
        "dark-neon" => Some(EmbeddedThemeName::DarkNeon),
        "dracula" => Some(EmbeddedThemeName::Dracula),
        "github" => Some(EmbeddedThemeName::Github),
        "gruvbox-dark" => Some(EmbeddedThemeName::GruvboxDark),
        "gruvbox-light" => Some(EmbeddedThemeName::GruvboxLight),
        "inspired-github" => Some(EmbeddedThemeName::InspiredGithub),
        "1337" => Some(EmbeddedThemeName::Leet),
        "monokai-extended" => Some(EmbeddedThemeName::MonokaiExtended),
        "monokai-extended-bright" => Some(EmbeddedThemeName::MonokaiExtendedBright),
        "monokai-extended-light" => Some(EmbeddedThemeName::MonokaiExtendedLight),
        "monokai-extended-origin" => Some(EmbeddedThemeName::MonokaiExtendedOrigin),
        "nord" => Some(EmbeddedThemeName::Nord),
        "one-half-dark" => Some(EmbeddedThemeName::OneHalfDark),
        "one-half-light" => Some(EmbeddedThemeName::OneHalfLight),
        "solarized-dark" => Some(EmbeddedThemeName::SolarizedDark),
        "solarized-light" => Some(EmbeddedThemeName::SolarizedLight),
        "sublime-snazzy" => Some(EmbeddedThemeName::SublimeSnazzy),
        "two-dark" => Some(EmbeddedThemeName::TwoDark),
        "zenburn" => Some(EmbeddedThemeName::Zenburn),
        _ => None,
    }
}

/// Build the expected path for a custom theme file.
/// 生成自定义主题文件约定的磁盘路径。
fn custom_theme_path(name: &str, codex_home: &Path) -> PathBuf {
    codex_home.join("themes").join(format!("{name}.tmTheme"))
}

/// Try to load a custom `.tmTheme` file from `{codex_home}/themes/{name}.tmTheme`.
/// 尝试从 `{codex_home}/themes/{name}.tmTheme` 加载自定义主题。
fn load_custom_theme(name: &str, codex_home: &Path) -> Option<Theme> {
    ThemeSet::get_theme(custom_theme_path(name, codex_home)).ok()
}

fn adaptive_default_theme_selection() -> (EmbeddedThemeName, &'static str) {
    match tui_theme::terminal_palette::default_bg() {
        Some(bg) if tui_theme::color::is_light(bg) => {
            (EmbeddedThemeName::CatppuccinLatte, "catppuccin-latte")
        }
        _ => (EmbeddedThemeName::CatppuccinMocha, "catppuccin-mocha"),
    }
}

fn adaptive_default_embedded_theme_name() -> EmbeddedThemeName {
    adaptive_default_theme_selection().0
}

/// Return the kebab-case name of the adaptive default syntax theme selected
/// from terminal background lightness.
/// 根据终端背景明暗返回自适应默认主题的 kebab-case 名称。
pub fn adaptive_default_theme_name() -> &'static str {
    adaptive_default_theme_selection().1
}

/// Build the theme from current override/default-theme settings.
/// Extracted from the old `theme()` init closure so it can be reused.
/// 根据当前覆盖项或默认策略构造主题；从原初始化闭包提取以供复用。
fn resolve_theme_with_override(name: Option<&str>, codex_home: Option<&Path>) -> Theme {
    let ts = two_face::theme::extra();

    // Honor user-configured theme if valid.
    // 有效的用户主题覆盖优先于自适应默认值。
    if let Some(name) = name {
        // 1. Try bundled theme by kebab-case name.
        // 第一步按 kebab-case 名称解析内置主题。
        if let Some(theme_name) = parse_theme_name(name) {
            return ts.get(theme_name).clone();
        }
        // 2. Try loading {CODEX_HOME}/themes/{name}.tmTheme from disk.
        // 第二步从 CODEX_HOME 的 themes 目录尝试加载自定义主题。
        if let Some(home) = codex_home
            && let Some(theme) = load_custom_theme(name, home)
        {
            return theme;
        }
        tracing::debug!("Theme \"{name}\" not recognized; using default theme");
    }

    ts.get(adaptive_default_embedded_theme_name()).clone()
}

/// Build the theme from current override/default-theme settings.
/// Extracted from the old `theme()` init closure so it can be reused.
/// 按当前配置创建主题，并复用旧初始化闭包的同一解析路径。
fn build_default_theme() -> Theme {
    let name = THEME_OVERRIDE.get().and_then(|name| name.as_deref());
    let codex_home = CODEX_HOME.get().and_then(|codex_home| codex_home.as_deref());
    resolve_theme_with_override(name, codex_home)
}

fn theme_lock() -> &'static RwLock<Theme> {
    THEME.get_or_init(|| RwLock::new(build_default_theme()))
}

/// Swap the active syntax theme at runtime and invalidate rendered-content caches.
/// 在运行时替换活动语法主题，并使依赖主题的渲染缓存失效。
pub fn set_syntax_theme(theme: Theme) {
    let mut guard = match theme_lock().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = theme;
    THEME_REVISION.fetch_add(1, Ordering::Release);
}

/// Return the revision of the active syntax theme for rendered-content caches.
/// 返回活动主题修订号，供渲染内容缓存校验新鲜度。
pub fn syntax_theme_revision() -> u64 {
    THEME_REVISION.load(Ordering::Acquire)
}

/// Clone the current syntax theme (e.g. to save for cancel-restore).
/// 克隆当前主题，例如在预览取消时恢复原主题。
pub fn current_syntax_theme() -> Theme {
    match theme_lock().read() {
        Ok(theme) => theme.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// Raw RGB background colors extracted from syntax theme diff/markup scopes.
///
/// These are theme-provided colors, not yet adapted for any particular color
/// depth.  The upper-layer diff renderer converts them to ratatui
/// `Color` values via `color_from_rgb_for_level` after deciding whether to
/// emit truecolor or quantized ANSI-256.
///
/// Both fields are `None` when the active theme defines no relevant scope
/// backgrounds, in which case the diff renderer falls back to its hardcoded
/// palette.
/// 保存主题为 diff/markup 作用域提供的原始 RGB 背景色；尚未按终端色深转换。
/// 两个字段均为空时，差异渲染器使用内建回退调色板。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DiffScopeBackgroundRgbs {
    pub inserted: Option<(u8, u8, u8)>,
    pub deleted: Option<(u8, u8, u8)>,
}

/// Query the active syntax theme for diff-scope background colors.
///
/// Prefers `markup.inserted` / `markup.deleted` (the TextMate convention used
/// by most VS Code themes) and falls back to `diff.inserted` / `diff.deleted`
/// (used by some older `.tmTheme` files).
/// 查询活动主题的差异背景色，优先采用通行的 `markup.*` 作用域，再兼容旧式 `diff.*`。
pub fn diff_scope_background_rgbs() -> DiffScopeBackgroundRgbs {
    let theme = current_syntax_theme();
    diff_scope_background_rgbs_for_theme(&theme)
}

/// Pure extraction helper, separated from the global theme singleton so tests
/// can pass arbitrary themes.
/// 纯提取辅助函数与全局单例分离，使测试可传入任意主题。
fn diff_scope_background_rgbs_for_theme(theme: &Theme) -> DiffScopeBackgroundRgbs {
    let highlighter = Highlighter::new(theme);
    let inserted = scope_background_rgb(&highlighter, "markup.inserted")
        .or_else(|| scope_background_rgb(&highlighter, "diff.inserted"));
    let deleted = scope_background_rgb(&highlighter, "markup.deleted")
        .or_else(|| scope_background_rgb(&highlighter, "diff.deleted"));
    DiffScopeBackgroundRgbs { inserted, deleted }
}

/// Extract the background color for a single TextMate scope, if defined.
/// 提取单个 TextMate 作用域定义的背景色。
fn scope_background_rgb(highlighter: &Highlighter<'_>, scope_name: &str) -> Option<(u8, u8, u8)> {
    let scope = Scope::new(scope_name).ok()?;
    let bg = highlighter.style_mod_for_stack(&[scope]).background?;
    Some((bg.r, bg.g, bg.b))
}

/// Query the active syntax theme for the first foreground style provided by the
/// supplied TextMate scopes.
/// 按给定 TextMate 作用域顺序返回首个前景样式。
pub fn foreground_style_for_scopes(scope_names: &[&str]) -> Option<Style> {
    let theme = current_syntax_theme();
    foreground_style_for_scopes_with_theme(&theme, scope_names)
}

fn foreground_style_for_scopes_with_theme(theme: &Theme, scope_names: &[&str]) -> Option<Style> {
    let highlighter = Highlighter::new(theme);
    scope_names.iter().find_map(|scope_name| {
        let scope = Scope::new(scope_name).ok()?;
        let fg = highlighter.style_mod_for_stack(&[scope]).foreground?;
        convert_syntect_color(fg).map(|fg| Style::default().fg(fg))
    })
}

/// Return the configured kebab-case theme name when it resolves; otherwise
/// return the adaptive auto-detected default theme name.
///
/// This intentionally reflects persisted configuration/default selection, not
/// transient runtime swaps applied via `set_syntax_theme`.
/// 返回可解析的持久化主题名，否则返回自适应默认名；刻意不反映实时预览造成的临时切换。
pub fn configured_theme_name() -> String {
    // Explicit user override?
    // 先检查显式用户覆盖是否仍可解析。
    if let Some(Some(name)) = THEME_OVERRIDE.get() {
        if parse_theme_name(name).is_some() {
            return name.clone();
        }
        if let Some(Some(home)) = CODEX_HOME.get()
            && load_custom_theme(name, home).is_some()
        {
            return name.clone();
        }
    }
    adaptive_default_theme_name().to_string()
}

/// Resolve a theme name to a `Theme` (bundled or custom). Returns `None`
/// when the name is unknown and no matching `.tmTheme` file exists.
/// 将名称解析为内置或自定义主题；两者均不存在时返回 `None`。
pub fn resolve_theme_by_name(name: &str, codex_home: Option<&Path>) -> Option<Theme> {
    let ts = two_face::theme::extra();
    // Bundled theme?
    // 优先匹配内置主题。
    if let Some(embedded) = parse_theme_name(name) {
        return Some(ts.get(embedded).clone());
    }
    // Custom .tmTheme file?
    // 未命中时再尝试自定义 `.tmTheme` 文件。
    if let Some(home) = codex_home
        && let Some(theme) = load_custom_theme(name, home)
    {
        return Some(theme);
    }
    None
}

/// A theme available in the picker, either bundled or loaded from a custom
/// `.tmTheme` file under `{CODEX_HOME}/themes/`.
/// 主题选择器中的可用条目，可来自内置包或 CODEX_HOME 下的自定义文件。
pub struct ThemeEntry {
    /// Kebab-case identifier used for config persistence and theme resolution.
    /// 用于配置持久化及解析的 kebab-case 标识符。
    pub name: String,
    /// `true` when this entry was discovered from a `.tmTheme` file on disk
    /// rather than the embedded two-face bundle.
    /// 为真表示条目来自磁盘自定义主题，而非内嵌 two-face 主题包。
    pub is_custom: bool,
}

/// List all available theme names: bundled themes + custom `.tmTheme` files
/// found in `{codex_home}/themes/`.
/// 列出全部内置主题及 `{codex_home}/themes/` 中发现的自定义主题。
pub fn list_available_themes(codex_home: Option<&Path>) -> Vec<ThemeEntry> {
    let mut entries: Vec<ThemeEntry> = BUILTIN_THEME_NAMES
        .iter()
        .map(|name| ThemeEntry { name: name.to_string(), is_custom: false })
        .collect();

    // Discover custom themes on disk, deduplicating against builtins.
    // 扫描磁盘自定义主题，并与内置名称去重。
    if let Some(home) = codex_home {
        let themes_dir = home.join("themes");
        if let Ok(read_dir) = std::fs::read_dir(&themes_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("tmTheme")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    let name = stem.to_string();
                    let is_valid_theme = ThemeSet::get_theme(&path).is_ok();
                    if is_valid_theme && !entries.iter().any(|e| e.name == name) {
                        entries.push(ThemeEntry { name, is_custom: true });
                    }
                }
            }
        }
    }

    // Keep picker ordering stable across platforms/filesystems while sorting
    // custom and bundled themes together, case-insensitively.
    // 自定义与内置主题统一按不区分大小写的名称排序，确保跨平台顺序稳定。
    entries.sort_by_cached_key(|entry| (entry.name.to_ascii_lowercase(), entry.name.clone()));

    entries
}

/// All 32 bundled theme names in kebab-case, ordered alphabetically.
/// 以字母顺序返回 32 个内置主题的 kebab-case 名称。
const BUILTIN_THEME_NAMES: &[&str] = &[
    "1337",
    "ansi",
    "base16",
    "base16-256",
    "base16-eighties-dark",
    "base16-mocha-dark",
    "base16-ocean-dark",
    "base16-ocean-light",
    "catppuccin-frappe",
    "catppuccin-latte",
    "catppuccin-macchiato",
    "catppuccin-mocha",
    "coldark-cold",
    "coldark-dark",
    "dark-neon",
    "dracula",
    "github",
    "gruvbox-dark",
    "gruvbox-light",
    "inspired-github",
    "monokai-extended",
    "monokai-extended-bright",
    "monokai-extended-light",
    "monokai-extended-origin",
    "nord",
    "one-half-dark",
    "one-half-light",
    "solarized-dark",
    "solarized-light",
    "sublime-snazzy",
    "two-dark",
    "zenburn",
];

// -- Style conversion (syntect -> ratatui) ------------------------------------
// -- 样式转换（syntect -> ratatui）------------------------------------------

/// Map a low ANSI palette index (0–7) to ratatui's named color variants,
/// falling back to `Indexed(n)` for indices 8–255.
///
/// Named variants are preferred over `Indexed(0)`…`Indexed(7)` because many
/// terminals apply bold/bright treatment differently for named vs indexed
/// colors, and ANSI themes expect the named behavior.
///
/// `clippy::disallowed_methods` is explicitly allowed here because this helper
/// intentionally constructs `ratatui::style::Color::Indexed`.
/// 将低位 ANSI 索引映射为 ratatui 命名色，其余索引保留为 `Indexed`；命名色能维持终端的加粗/亮色习惯。
/// 此处显式允许受限构造方法，因为创建索引色正是函数职责。
#[allow(clippy::disallowed_methods)]
fn ansi_palette_color(index: u8) -> RtColor {
    match index {
        0x00 => RtColor::Black,
        0x01 => RtColor::Red,
        0x02 => RtColor::Green,
        0x03 => RtColor::Yellow,
        0x04 => RtColor::Blue,
        0x05 => RtColor::Magenta,
        0x06 => RtColor::Cyan,
        // ANSI code 37 is "white", represented as `Gray` in ratatui.
        // ANSI 37 的“白色”在 ratatui 中对应 `Gray`。
        0x07 => RtColor::Gray,
        n => RtColor::Indexed(n),
    }
}

/// Decode a syntect foreground `Color` into a ratatui color, respecting the
/// alpha-channel encoding that bat's `ansi`, `base16`, and `base16-256` themes
/// use to signal ANSI palette semantics instead of true RGB.
///
/// Returns `None` when the color signals "use the terminal's default
/// foreground", allowing the caller to omit the foreground attribute entirely.
///
/// Passing a color from a standard RGB theme (alpha 0xFF) returns
/// `Some(Rgb(..))`, so this function is backward-compatible with non-ANSI
/// themes. Unexpected intermediate alpha values are treated as RGB.
///
/// `clippy::disallowed_methods` is explicitly allowed here because this helper
/// intentionally constructs `ratatui::style::Color::Rgb`.
/// 按 bat 的 alpha 标记解码 syntect 前景色：可返回 ANSI 索引、终端默认色或普通 RGB。
/// 返回 `None` 表示调用方应省略前景属性；显式构造 RGB 是该转换函数的必要行为。
#[allow(clippy::disallowed_methods)]
fn convert_syntect_color(color: SyntectColor) -> Option<RtColor> {
    match color.a {
        // Bat-compatible encoding used by `ansi`, `base16`, and `base16-256`:
        // alpha 0x00 means `r` stores an ANSI palette index, not RGB red.
        // bat 兼容编码中 alpha 为 0 时，`r` 存放 ANSI 调色板索引而非红色分量。
        ANSI_ALPHA_INDEX => Some(ansi_palette_color(color.r)),
        // alpha 0x01 means "use terminal default foreground/background".
        // alpha 为 1 表示使用终端默认前景色或背景色。
        ANSI_ALPHA_DEFAULT => None,
        OPAQUE_ALPHA => Some(RtColor::Rgb(color.r, color.g, color.b)),
        // Non-ANSI alpha values appear in some bundled themes; treat as plain RGB.
        // 某些内置主题含其他 alpha 值，按普通 RGB 处理以保持兼容。
        _ => Some(RtColor::Rgb(color.r, color.g, color.b)),
    }
}

/// Convert a syntect `Style` to a ratatui `Style`.
///
/// Most themes produce RGB colors. The built-in `ansi`/`base16`/`base16-256`
/// themes encode ANSI palette semantics in the alpha channel, matching bat.
/// 将 syntect 样式转换为 ratatui 样式；ANSI 系列主题沿用 bat 的 alpha 通道编码约定。
fn convert_style(syn_style: SyntectStyle) -> Style {
    let mut rt_style = Style::default();

    if let Some(fg) = convert_syntect_color(syn_style.foreground) {
        rt_style = rt_style.fg(fg);
    }
    // Intentionally skip background to avoid overwriting terminal bg.
    // If background support is added later, decode with `convert_syntect_color`
    // to reuse the same alpha-marker semantics as foreground.
    // 刻意跳过背景色以保留终端背景；若以后支持，应复用与前景色相同的 alpha 标记解码。

    if syn_style.font_style.contains(FontStyle::BOLD) {
        rt_style.add_modifier |= Modifier::BOLD;
    }
    // Intentionally skip italic — many terminals render it poorly or not at all.
    // Intentionally skip underline — themes like Dracula use underline on type
    // scopes (entity.name.type, support.class) which produces distracting
    // underlines on type/module names in terminal output.
    // 斜体在许多终端表现不佳，类型作用域的下划线也会造成干扰，因此两者均刻意抑制。

    rt_style
}

// -- Syntax lookup ------------------------------------------------------------
// -- 语法查找 ----------------------------------------------------------------

/// Try to find a syntect `SyntaxReference` for the given language identifier.
///
/// two-face's extended syntax set (~250 languages) resolves most names and
/// extensions directly.  We only patch the few aliases it cannot handle.
/// 查找语言标识对应的 syntect 语法；two-face 已覆盖大多数名称和扩展名，这里只修补少数别名。
fn find_syntax(lang: &str) -> Option<&'static SyntaxReference> {
    let ss = syntax_set();

    // Aliases that two-face does not resolve on its own.
    // 仅列出 two-face 自身无法识别的别名。
    let normalized = lang.to_ascii_lowercase();
    let patched = match normalized.as_str() {
        "csharp" | "c-sharp" => "c#",
        // CUDA source (.cu) and header (.cuh) files use C++ highlighting as a fallback.
        // CUDA 源文件与头文件回退到 C++ 高亮。
        "cu" | "cuh" => "cpp",
        "cppm" | "cxxm" | "ixx" => "cpp",
        "golang" => "go",
        "python3" => "python",
        "shell" => "bash",
        _ => lang,
    };

    // Try by token (matches file_extensions case-insensitively).
    // 先按 token 匹配，文件扩展名比较不区分大小写。
    if let Some(s) = ss.find_syntax_by_token(patched) {
        return Some(s);
    }
    // Try by exact syntax name (e.g. "Rust", "Python").
    // 再按完整语法名称精确匹配。
    if let Some(s) = ss.find_syntax_by_name(patched) {
        return Some(s);
    }
    // Try case-insensitive name match (e.g. "rust" -> "Rust").
    // 然后尝试不区分大小写的语法名称匹配。
    let lower = patched.to_ascii_lowercase();
    if let Some(s) = ss.syntaxes().iter().find(|s| s.name.to_ascii_lowercase() == lower) {
        return Some(s);
    }
    // Try raw input as file extension.
    // 最后把原始输入当作文件扩展名尝试。
    if let Some(s) = ss.find_syntax_by_extension(lang) {
        return Some(s);
    }
    None
}

// -- Guardrail constants ------------------------------------------------------
// -- 资源护栏常量 ------------------------------------------------------------

/// Skip highlighting for inputs larger than 512 KB to avoid excessive memory
/// and CPU usage.  Callers fall back to plain unstyled text.
/// 输入超过 512 KiB 时跳过高亮以限制内存和 CPU，调用方回退到纯文本。
const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;

/// Skip highlighting for inputs with more than 10,000 lines.
/// 输入超过一万行时跳过高亮。
const MAX_HIGHLIGHT_LINES: usize = 10_000;

/// Skip highlighting when an individual line is longer than 4 KiB.
/// 任一单行超过 4 KiB 时跳过高亮。
pub const MAX_HIGHLIGHT_LINE_BYTES: usize = 4 * 1024;

/// Check whether an input exceeds the safe highlighting limits.
///
/// Callers that highlight content in a loop (e.g. per diff-line) should
/// pre-check the aggregate size with this function and skip highlighting
/// entirely when it returns `true`.
/// 判断输入是否突破安全高亮限制；循环高亮的调用方应先对整体输入执行此检查。
pub fn exceeds_highlight_limits(total_bytes: usize, total_lines: usize) -> bool {
    total_bytes > MAX_HIGHLIGHT_BYTES || total_lines > MAX_HIGHLIGHT_LINES
}

// -- Core highlighting --------------------------------------------------------
// -- 核心高亮 ----------------------------------------------------------------

/// Core highlighter that accepts an explicit theme reference.
///
/// This keeps production behavior and test behavior on the same code path:
/// production callers pass the global theme lock, while tests can pass a
/// concrete theme without mutating process-global state.
/// 使用显式主题的核心高亮器，让生产代码和测试共用同一路径且测试无需修改全局状态。
fn highlight_to_line_spans_with_theme(
    code: &str,
    lang: &str,
    theme: &Theme,
) -> Option<Vec<Vec<Span<'static>>>> {
    // Empty input has nothing to highlight; fall back to the plain text path
    // which correctly produces a single empty Line.
    // 空输入没有可高亮内容，交给纯文本路径生成规范的单个空行。
    if code.is_empty() {
        return None;
    }

    // Bail out early for oversized inputs to avoid excessive resource usage.
    // Count actual lines (not newline bytes) to avoid an off-by-one when
    // the input does not end with a newline.
    // 超限时尽早退出；按真实行数而非换行字节计数，避免无尾换行输入的差一错误。
    if code.len() > MAX_HIGHLIGHT_BYTES
        || code.lines().count() > MAX_HIGHLIGHT_LINES
        || code.lines().any(|line| line.len() > MAX_HIGHLIGHT_LINE_BYTES)
    {
        return None;
    }

    let syntax = find_syntax(lang)?;
    let mut h = HighlightLines::new(syntax, theme);
    let mut lines: Vec<Vec<Span<'static>>> = Vec::new();

    for line in LinesWithEndings::from(code) {
        let ranges = h.highlight_line(line, syntax_set()).ok()?;
        lines.push(highlighted_line_spans(ranges));
    }

    Some(lines)
}

/// Convert one highlighted line into the spans shared by full and streaming renders.
///
/// Source line endings are omitted, while empty lines retain their canonical empty span.
/// 将一条高亮源行转换为完整渲染与流式渲染共享的 spans；去除行尾但保留规范空 span。
fn highlighted_line_spans(ranges: Vec<(SyntectStyle, &str)>) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (style, text) in ranges {
        // Line breaks are represented by the surrounding Line, not its spans.
        // 换行由外层 `Line` 表示，不应留在 span 文本中。
        let text = text.trim_end_matches(['\n', '\r']);
        if !text.is_empty() {
            spans.push(Span::styled(text.to_string(), convert_style(style)));
        }
    }
    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }
    spans
}

/// Parse `code` using syntect for `lang` and return per-line styled spans.
/// Each inner Vec represents one source line.  Returns None when the language
/// is not recognized or the input exceeds safety limits.
/// 用 syntect 解析代码并返回逐行 spans；语言未知或超过护栏时返回 `None`。
fn highlight_to_line_spans(code: &str, lang: &str) -> Option<Vec<Vec<Span<'static>>>> {
    let theme_guard = match theme_lock().read() {
        Ok(theme_guard) => theme_guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    highlight_to_line_spans_with_theme(code, lang, &theme_guard)
}

// -- Public API ---------------------------------------------------------------
// -- 公开 API ----------------------------------------------------------------

/// Highlight code in any supported language, returning styled ratatui `Line`s.
///
/// Falls back to plain unstyled text when the language is not recognized or the
/// input exceeds safety guardrails.  Callers can always render the result
/// directly -- the fallback path produces equivalent plain-text lines.
///
/// Used by `markdown_render` for fenced code blocks and by `exec_cell` for bash
/// command highlighting.
/// 高亮任意支持的语言；未知语言或超限时返回等价的逐行无样式文本，可直接渲染。
/// 围栏代码块与命令执行单元共用此入口。
pub fn highlight_code_to_lines(code: &str, lang: &str) -> Vec<Line<'static>> {
    if let Some(line_spans) = highlight_to_line_spans(code, lang) {
        line_spans.into_iter().map(Line::from).collect()
    } else {
        // Fallback: plain text, one Line per source line.
        // Use `lines()` instead of `split('\n')` to avoid a phantom trailing
        // empty element when the input ends with '\n' (as pulldown-cmark emits).
        // 回退路径用 `lines()` 避免 pulldown-cmark 尾换行产生幽灵空行。
        let mut result: Vec<Line<'static>> =
            code.lines().map(|l| Line::from(l.to_string())).collect();
        if result.is_empty() {
            result.push(Line::from(String::new()));
        }
        result
    }
}

/// Backward-compatible wrapper for bash highlighting used by exec cells.
/// 为执行单元保留的 Bash 高亮兼容包装器。
pub fn highlight_bash_to_lines(script: &str) -> Vec<Line<'static>> {
    highlight_code_to_lines(script, "bash")
}

/// Highlight code and return per-line styled spans for diff integration.
///
/// Returns `None` when the language is unrecognized or the input exceeds
/// guardrails.  The caller (`diff_render`) uses this signal to fall back to
/// plain diff coloring.
///
/// Each inner `Vec<Span>` corresponds to one source line.  Styles are derived
/// from the active theme but backgrounds are intentionally omitted so the
/// terminal's own background shows through.
/// 为差异渲染返回逐行 spans；未知语言或超限时由调用方退回纯 diff 着色。
/// 样式刻意省略背景色，以保留终端自身背景。
pub fn highlight_code_to_styled_spans(
    code: &str,
    lang: &str,
) -> Option<Vec<Vec<Span<'static>>>> {
    highlight_to_line_spans(code, lang)
}

#[cfg(test)]
#[path = "highlight_tests.rs"]
mod tests;
