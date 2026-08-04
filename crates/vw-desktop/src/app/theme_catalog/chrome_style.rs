//! 从活动主题 `ThemeChrome` 与 palette 解析控件覆盖色。

use iced::Color;
use iced::Theme;

use super::model::{ThemeChrome, parse_rrggbb};

/// 存在窗口壁纸且未显式设置分区 alpha 时的默认面板透明度。
pub const DEFAULT_WALLPAPER_PANEL_ALPHA: f32 = 184.0 / 255.0;

/// 已解析的控件皮肤覆盖；字段为 `None` 表示沿用默认样式推导。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChromeStyleOverrides {
    pub input_bg: Option<Color>,
    pub input_border: Option<Color>,
    pub input_text: Option<Color>,
    pub panel_bg: Option<Color>,
    pub panel_border: Option<Color>,
    pub button_bg: Option<Color>,
    pub button_text: Option<Color>,
    pub radius_px: Option<f32>,
    /// 消息区/聊天外壳透明度（0.0–1.0）；兼容旧 `panel_alpha` 回退。
    pub panel_alpha: Option<f32>,
    /// 聊头顶栏透明度（0.0–1.0）。
    pub chat_header_alpha: Option<f32>,
    /// 输入卡透明度（0.0–1.0）。
    pub chat_input_alpha: Option<f32>,
    /// 聊头顶栏半透明色（有壁纸/显式 alpha 时合成）。
    pub chat_header_bg: Option<Color>,
}

fn parse_optional_color(value: Option<&str>) -> Option<Color> {
    value.and_then(|hex| parse_rrggbb(hex).ok())
}

fn with_alpha(color: Color, alpha: Option<f32>) -> Color {
    match alpha {
        Some(a) => Color { a: a.clamp(0.0, 1.0), ..color },
        None => color,
    }
}

fn resolve_zone_alpha(
    specific: Option<u8>,
    fallback: Option<u8>,
    has_wallpaper: bool,
) -> Option<f32> {
    match specific.or(fallback) {
        Some(v) => Some(f32::from(v) / 255.0),
        None if has_wallpaper => Some(DEFAULT_WALLPAPER_PANEL_ALPHA),
        None => None,
    }
}

/// 浅色工作区软底，避免纯白聊天壳在浅灰工作区上过于突兀。
const LIGHT_SOFT_PANEL: Color = Color::from_rgb8(248, 250, 252);

fn is_dark_background(background: Color) -> bool {
    background.r + background.g + background.b < 1.5
}

/// 浅色主题面板底：向软 slate 混合，保留有色浅底的色调；深色原样返回。
fn soft_panel_base(background: Color) -> Color {
    if is_dark_background(background) {
        return background;
    }
    Color {
        r: background.r * 0.40 + LIGHT_SOFT_PANEL.r * 0.60,
        g: background.g * 0.40 + LIGHT_SOFT_PANEL.g * 0.60,
        b: background.b * 0.40 + LIGHT_SOFT_PANEL.b * 0.60,
        a: 1.0,
    }
}

fn default_input_surface(palette: &iced::theme::Palette) -> Color {
    // 浅色用 palette.background（偏白）相对软面板抬升；深色同底。
    palette.background
}

fn default_panel_surface(palette: &iced::theme::Palette) -> Color {
    soft_panel_base(palette.background)
}

/// 内容面板表面：浅色主题软化纯白，深色跟随 `palette.background`。
pub fn panel_surface_color(theme: &Theme, alpha: f32) -> Color {
    with_alpha(soft_panel_base(theme.palette().background), Some(alpha))
}

/// 输入卡表面：深色用 `background.weak` 抬升；浅色用底色（避免 weak 偏深灰）。
pub fn input_surface_color(theme: &Theme, alpha: f32) -> Color {
    let ext = theme.extended_palette();
    let base = if ext.is_dark { ext.background.weak.color } else { theme.palette().background };
    with_alpha(base, Some(alpha))
}

/// 将 `ThemeChrome` 解析为运行时覆盖色。
///
/// 若只设置了输入背景而未设置文字色，回退到 `palette.text` 以保证可读性。
pub fn resolve_chrome_style(
    chrome: &ThemeChrome,
    palette: &iced::theme::Palette,
) -> ChromeStyleOverrides {
    resolve_chrome_style_with_wallpaper(chrome, palette, false)
}

/// 解析 chrome；`has_wallpaper` 为真时，对未显式配色的聊天表面填充默认可调透明度。
///
/// 透明度优先级：分区专用 alpha → 全局 `panel_alpha` → 有壁纸时默认半透明。
pub fn resolve_chrome_style_with_wallpaper(
    chrome: &ThemeChrome,
    palette: &iced::theme::Palette,
    has_wallpaper: bool,
) -> ChromeStyleOverrides {
    if chrome.is_empty() && !has_wallpaper {
        return ChromeStyleOverrides::default();
    }

    let body_alpha = resolve_zone_alpha(chrome.chat_body_alpha, chrome.panel_alpha, has_wallpaper);
    let input_alpha =
        resolve_zone_alpha(chrome.chat_input_alpha, chrome.panel_alpha, has_wallpaper);
    let header_alpha =
        resolve_zone_alpha(chrome.chat_header_alpha, chrome.panel_alpha, has_wallpaper);

    let input_bg = match parse_optional_color(chrome.input_bg.as_deref()) {
        Some(color) => Some(with_alpha(color, input_alpha)),
        None if input_alpha.is_some() => {
            Some(with_alpha(default_input_surface(palette), input_alpha))
        }
        None => None,
    };
    let panel_bg = match parse_optional_color(chrome.panel_bg.as_deref()) {
        Some(color) => Some(with_alpha(color, body_alpha)),
        None if body_alpha.is_some() => {
            Some(with_alpha(default_panel_surface(palette), body_alpha))
        }
        None => None,
    };
    let chat_header_bg = match parse_optional_color(chrome.panel_bg.as_deref()) {
        Some(color) => header_alpha.map(|a| with_alpha(color, Some(a))),
        None if header_alpha.is_some() => {
            Some(with_alpha(default_panel_surface(palette), header_alpha))
        }
        None => None,
    };

    let input_text = match parse_optional_color(chrome.input_text.as_deref()) {
        Some(color) => Some(color),
        None if input_bg.is_some() => Some(palette.text),
        None => None,
    };

    ChromeStyleOverrides {
        input_bg,
        input_border: parse_optional_color(chrome.input_border.as_deref()),
        input_text,
        panel_bg,
        panel_border: parse_optional_color(chrome.panel_border.as_deref()),
        button_bg: parse_optional_color(chrome.button_bg.as_deref()),
        button_text: parse_optional_color(chrome.button_text.as_deref()),
        radius_px: chrome.radius_px.map(|v| v as f32),
        panel_alpha: body_alpha,
        chat_header_alpha: header_alpha,
        chat_input_alpha: input_alpha,
        chat_header_bg,
    }
}

/// 从可选 chrome 与主题解析覆盖；无 chrome 或空 chrome 时返回全 `None`。
pub fn resolve_chrome_style_from_theme(
    chrome: Option<&ThemeChrome>,
    theme: &Theme,
) -> ChromeStyleOverrides {
    match chrome {
        Some(chrome) => resolve_chrome_style(chrome, &theme.palette()),
        None => ChromeStyleOverrides::default(),
    }
}

/// 从活动皮肤快照解析覆盖；存在窗口壁纸或分区图时应用默认/显式面板透明度。
///
/// 分区背景图由 `with_panel_background` 叠在对应表面底层；本函数负责半透明色层。
pub fn resolve_chrome_style_from_skin(
    skin: Option<&super::ActiveThemeSkin>,
    theme: &Theme,
) -> ChromeStyleOverrides {
    match skin {
        Some(skin) => {
            let has_wallpaper = skin.background_path.is_some()
                || skin.chat_header_path.is_some()
                || skin.chat_body_path.is_some()
                || skin.chat_input_path.is_some();
            resolve_chrome_style_with_wallpaper(&skin.chrome, &theme.palette(), has_wallpaper)
        }
        None => ChromeStyleOverrides::default(),
    }
}

#[cfg(test)]
#[path = "chrome_style_tests.rs"]
mod chrome_style_tests;
