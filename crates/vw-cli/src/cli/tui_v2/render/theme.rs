use super::{Color, Span, Style};
use crate::cli::tui_v2::terminal_capabilities::{
    TerminalCapabilities, TuiBackground, TuiColorLevel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiThemeName {
    Dark,
    Light,
    LightDaltonized,
    DarkDaltonized,
    LightAnsi,
    DarkAnsi,
}

/// 统一的 pill 色调枚举，避免 header/footer/prompt 各自散落颜色判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiTone {
    Accent,
    Muted,
    Success,
    Warning,
    Danger,
}

/// 可复用的轻量状态 pill。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiPill {
    pub(crate) label: String,
    pub(crate) tone: TuiTone,
}

impl TuiPill {
    pub(crate) fn new(label: impl Into<String>, tone: TuiTone) -> Self {
        Self { label: label.into(), tone }
    }
}

/// tui_v2 当前阶段使用的主题 token。
#[derive(Debug, Clone, Copy)]
pub(crate) struct TuiTheme {
    pub(crate) background: Color,
    pub(crate) surface: Color,
    pub(crate) surface_alt: Color,
    pub(crate) border: Color,
    pub(crate) accent: Color,
    pub(crate) text: Color,
    pub(crate) muted: Color,
    pub(crate) success: Color,
    pub(crate) warning: Color,
    pub(crate) danger: Color,
}

impl Default for TuiTheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// panel 使用的背景层级，避免 surface 判断散落在 renderer 各处。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TuiPanelSurface {
    Base,
    Raised,
}

/// panel 边框语义，仅区分普通信息面与当前强调面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TuiPanelBorder {
    Standard,
    Focused,
}

impl TuiTheme {
    pub(crate) fn from_capabilities(capabilities: TerminalCapabilities, daltonized: bool) -> Self {
        let name = match (capabilities.background, daltonized, capabilities.color_level) {
            (TuiBackground::Light, _, TuiColorLevel::Ansi) => TuiThemeName::LightAnsi,
            (_, _, TuiColorLevel::Ansi) => TuiThemeName::DarkAnsi,
            (TuiBackground::Light, true, _) => TuiThemeName::LightDaltonized,
            (TuiBackground::Light, false, _) => TuiThemeName::Light,
            (_, true, _) => TuiThemeName::DarkDaltonized,
            (_, false, _) => TuiThemeName::Dark,
        };
        let mut theme = Self::for_name(name, capabilities);
        if matches!(capabilities.background, TuiBackground::Unknown)
            && !matches!(capabilities.color_level, TuiColorLevel::None)
        {
            theme.background = Color::Reset;
            theme.surface = Color::Reset;
            theme.surface_alt = Color::Reset;
        }
        theme
    }

    pub(crate) fn for_name(name: TuiThemeName, capabilities: TerminalCapabilities) -> Self {
        if matches!(capabilities.color_level, TuiColorLevel::None) {
            return Self::reset();
        }

        let name = match (name, capabilities.color_level) {
            (TuiThemeName::Light | TuiThemeName::LightDaltonized, TuiColorLevel::Ansi) => {
                TuiThemeName::LightAnsi
            }
            (TuiThemeName::Dark | TuiThemeName::DarkDaltonized, TuiColorLevel::Ansi) => {
                TuiThemeName::DarkAnsi
            }
            _ => name,
        };
        let theme = match name {
            TuiThemeName::Dark => Self::dark(),
            TuiThemeName::Light => Self::light(),
            TuiThemeName::LightDaltonized => Self::light_daltonized(),
            TuiThemeName::DarkDaltonized => Self::dark_daltonized(),
            TuiThemeName::LightAnsi => Self::light_ansi(),
            TuiThemeName::DarkAnsi => Self::dark_ansi(),
        };

        match capabilities.color_level {
            TuiColorLevel::Ansi256 => theme.map_colors(rgb_to_ansi256),
            TuiColorLevel::None => Self::reset(),
            TuiColorLevel::Ansi | TuiColorLevel::TrueColor => theme,
        }
    }

    pub(crate) fn colors(self) -> [Color; 10] {
        [
            self.background,
            self.surface,
            self.surface_alt,
            self.border,
            self.accent,
            self.text,
            self.muted,
            self.success,
            self.warning,
            self.danger,
        ]
    }

    fn dark() -> Self {
        Self {
            background: Color::Rgb(0, 0, 0),
            surface: Color::Rgb(55, 55, 55),
            surface_alt: Color::Rgb(44, 50, 62),
            border: Color::Rgb(136, 136, 136),
            accent: Color::Rgb(215, 119, 87),
            text: Color::Rgb(255, 255, 255),
            muted: Color::Rgb(153, 153, 153),
            success: Color::Rgb(78, 186, 101),
            warning: Color::Rgb(255, 193, 7),
            danger: Color::Rgb(255, 107, 128),
        }
    }

    fn light() -> Self {
        Self {
            background: Color::Rgb(255, 255, 255),
            surface: Color::Rgb(240, 240, 240),
            surface_alt: Color::Rgb(232, 236, 244),
            border: Color::Rgb(153, 153, 153),
            accent: Color::Rgb(215, 119, 87),
            text: Color::Rgb(0, 0, 0),
            muted: Color::Rgb(102, 102, 102),
            success: Color::Rgb(44, 122, 57),
            warning: Color::Rgb(150, 108, 30),
            danger: Color::Rgb(171, 43, 63),
        }
    }

    fn dark_daltonized() -> Self {
        Self {
            background: Color::Rgb(0, 0, 0),
            surface: Color::Rgb(55, 55, 55),
            surface_alt: Color::Rgb(44, 50, 62),
            border: Color::Rgb(136, 136, 136),
            accent: Color::Rgb(255, 153, 51),
            text: Color::Rgb(255, 255, 255),
            muted: Color::Rgb(153, 153, 153),
            success: Color::Rgb(51, 153, 255),
            warning: Color::Rgb(255, 204, 0),
            danger: Color::Rgb(255, 102, 102),
        }
    }

    fn light_daltonized() -> Self {
        Self {
            background: Color::Rgb(255, 255, 255),
            surface: Color::Rgb(220, 220, 220),
            surface_alt: Color::Rgb(210, 216, 226),
            border: Color::Rgb(153, 153, 153),
            accent: Color::Rgb(255, 153, 51),
            text: Color::Rgb(0, 0, 0),
            muted: Color::Rgb(102, 102, 102),
            success: Color::Rgb(0, 102, 153),
            warning: Color::Rgb(255, 153, 0),
            danger: Color::Rgb(204, 0, 0),
        }
    }

    fn dark_ansi() -> Self {
        Self {
            background: Color::Black,
            surface: Color::DarkGray,
            surface_alt: Color::Black,
            border: Color::Gray,
            accent: Color::LightRed,
            text: Color::White,
            muted: Color::Gray,
            success: Color::LightGreen,
            warning: Color::LightYellow,
            danger: Color::LightRed,
        }
    }

    fn light_ansi() -> Self {
        Self {
            background: Color::White,
            surface: Color::White,
            surface_alt: Color::Gray,
            border: Color::Gray,
            accent: Color::LightRed,
            text: Color::Black,
            muted: Color::DarkGray,
            success: Color::Green,
            warning: Color::Yellow,
            danger: Color::Red,
        }
    }

    fn reset() -> Self {
        Self {
            background: Color::Reset,
            surface: Color::Reset,
            surface_alt: Color::Reset,
            border: Color::Reset,
            accent: Color::Reset,
            text: Color::Reset,
            muted: Color::Reset,
            success: Color::Reset,
            warning: Color::Reset,
            danger: Color::Reset,
        }
    }

    fn map_colors(self, map: impl Fn(Color) -> Color) -> Self {
        Self {
            background: map(self.background),
            surface: map(self.surface),
            surface_alt: map(self.surface_alt),
            border: map(self.border),
            accent: map(self.accent),
            text: map(self.text),
            muted: map(self.muted),
            success: map(self.success),
            warning: map(self.warning),
            danger: map(self.danger),
        }
    }

    pub(super) fn panel_style(self, _surface: TuiPanelSurface) -> Style {
        Style::default()
    }

    pub(super) fn panel_border_style(self, border: TuiPanelBorder) -> Style {
        let color = match border {
            TuiPanelBorder::Standard => self.border,
            TuiPanelBorder::Focused => self.accent,
        };
        Style::default().fg(color)
    }

    pub(super) fn panel_title(self, title: impl Into<String>) -> Span<'static> {
        Span::styled(format!(" {} ", title.into()), self.accent_style())
    }

    pub(super) fn fg_style(self, color: Color) -> Style {
        Style::default().fg(color)
    }

    pub(super) fn accent_style(self) -> Style {
        self.fg_style(self.accent)
    }

    pub(super) fn body_style(self) -> Style {
        self.fg_style(self.text)
    }

    pub(super) fn muted_style(self) -> Style {
        self.fg_style(self.muted)
    }
}

fn rgb_to_ansi256(color: Color) -> Color {
    let Color::Rgb(red, green, blue) = color else {
        return color;
    };
    let channel =
        |value: u8| -> u8 { u8::try_from((u16::from(value) * 5 + 127) / 255).unwrap_or(5) };
    Color::Indexed(
        16u8.saturating_add(36u8.saturating_mul(channel(red)))
            .saturating_add(6u8.saturating_mul(channel(green)))
            .saturating_add(channel(blue)),
    )
}
