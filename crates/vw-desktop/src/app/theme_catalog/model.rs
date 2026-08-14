//! 自定义主题定义与校验。
//!
//! 负责 id/颜色格式约束以及名称、标语截断，不触及磁盘或 Iced 转换。

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

const MAX_NAME_CHARS: usize = 80;
const MAX_TAGLINE_CHARS: usize = 160;

/// Iced Palette 六色的十六进制表示，仅接受 `#rrggbb`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePaletteHex {
    pub background: String,
    pub text: String,
    pub primary: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
}

/// 主题包内封面/背景/分区资源的相对文件名（如 `cover.png`、`chat_header.png`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeAssets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_header: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_input: Option<String>,
}

impl ThemeAssets {
    pub fn is_empty(&self) -> bool {
        self.cover.is_none()
            && self.background.is_none()
            && self.chat_header.is_none()
            && self.chat_body.is_none()
            && self.chat_input.is_none()
    }
}

/// 输入框、面板和主按钮等界面外观覆盖。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ThemeChrome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_bg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_border: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_bg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_border: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button_bg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_px: Option<u32>,
    /// 分区透明度回退值（0–255）；未设分区专用 alpha 时使用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_alpha: Option<u8>,
    /// 聊天顶栏透明度（0–255）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_header_alpha: Option<u8>,
    /// 消息区/聊天外壳透明度（0–255）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_body_alpha: Option<u8>,
    /// 输入卡透明度（0–255）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_input_alpha: Option<u8>,
    /// 聊头顶栏背景图缩放：`cover`/`contain`/`fill`/`scale_down`/`none`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_header_image_fit: Option<String>,
    /// 消息区背景图缩放。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_body_image_fit: Option<String>,
    /// 输入区背景图缩放。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_input_image_fit: Option<String>,
}

/// 反序列化中间态：读取旧全局 `panel_image_fit` 并折叠进未设的分区字段。
#[derive(Debug, Default, Deserialize)]
struct ThemeChromeDe {
    #[serde(default)]
    input_bg: Option<String>,
    #[serde(default)]
    input_border: Option<String>,
    #[serde(default)]
    input_text: Option<String>,
    #[serde(default)]
    panel_bg: Option<String>,
    #[serde(default)]
    panel_border: Option<String>,
    #[serde(default)]
    button_bg: Option<String>,
    #[serde(default)]
    button_text: Option<String>,
    #[serde(default)]
    radius_px: Option<u32>,
    #[serde(default)]
    panel_alpha: Option<u8>,
    #[serde(default)]
    chat_header_alpha: Option<u8>,
    #[serde(default)]
    chat_body_alpha: Option<u8>,
    #[serde(default)]
    chat_input_alpha: Option<u8>,
    #[serde(default)]
    chat_header_image_fit: Option<String>,
    #[serde(default)]
    chat_body_image_fit: Option<String>,
    #[serde(default)]
    chat_input_image_fit: Option<String>,
    /// 旧全局缩放字段；仅反序列化，折叠后丢弃。
    #[serde(default)]
    panel_image_fit: Option<String>,
}

impl From<ThemeChromeDe> for ThemeChrome {
    fn from(raw: ThemeChromeDe) -> Self {
        let mut chrome = Self {
            input_bg: raw.input_bg,
            input_border: raw.input_border,
            input_text: raw.input_text,
            panel_bg: raw.panel_bg,
            panel_border: raw.panel_border,
            button_bg: raw.button_bg,
            button_text: raw.button_text,
            radius_px: raw.radius_px,
            panel_alpha: raw.panel_alpha,
            chat_header_alpha: raw.chat_header_alpha,
            chat_body_alpha: raw.chat_body_alpha,
            chat_input_alpha: raw.chat_input_alpha,
            chat_header_image_fit: raw.chat_header_image_fit,
            chat_body_image_fit: raw.chat_body_image_fit,
            chat_input_image_fit: raw.chat_input_image_fit,
        };
        if let Some(legacy) = raw.panel_image_fit {
            if chrome.chat_header_image_fit.is_none() {
                chrome.chat_header_image_fit = Some(legacy.clone());
            }
            if chrome.chat_body_image_fit.is_none() {
                chrome.chat_body_image_fit = Some(legacy.clone());
            }
            if chrome.chat_input_image_fit.is_none() {
                chrome.chat_input_image_fit = Some(legacy);
            }
        }
        chrome
    }
}

impl<'de> Deserialize<'de> for ThemeChrome {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ThemeChromeDe::deserialize(deserializer).map(Self::from)
    }
}

impl ThemeChrome {
    pub fn is_empty(&self) -> bool {
        self.input_bg.is_none()
            && self.input_border.is_none()
            && self.input_text.is_none()
            && self.panel_bg.is_none()
            && self.panel_border.is_none()
            && self.button_bg.is_none()
            && self.button_text.is_none()
            && self.radius_px.is_none()
            && self.panel_alpha.is_none()
            && self.chat_header_alpha.is_none()
            && self.chat_body_alpha.is_none()
            && self.chat_input_alpha.is_none()
            && self.chat_header_image_fit.is_none()
            && self.chat_body_image_fit.is_none()
            && self.chat_input_image_fit.is_none()
    }
}

/// 本地自定义主题定义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeDefinition {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tagline: Option<String>,
    pub palette: ThemePaletteHex,
    #[serde(default, skip_serializing_if = "ThemeAssets::is_empty")]
    pub assets: ThemeAssets,
    #[serde(default, rename = "appearance", skip_serializing_if = "ThemeChrome::is_empty")]
    pub chrome: ThemeChrome,
}

impl ThemeDefinition {
    /// 截断名称与标语至允许长度。
    pub fn normalize(&mut self) {
        self.name = truncate_chars(&self.name, MAX_NAME_CHARS);
        if let Some(tagline) = self.tagline.as_mut() {
            *tagline = truncate_chars(tagline, MAX_TAGLINE_CHARS);
            if tagline.is_empty() {
                self.tagline = None;
            }
        }
    }

    /// 校验 id、调色板、资源相对名与界面外观格式。
    pub fn validate(&self) -> Result<()> {
        validate_theme_id(&self.id)?;
        validate_hex_color("background", &self.palette.background)?;
        validate_hex_color("text", &self.palette.text)?;
        validate_hex_color("primary", &self.palette.primary)?;
        validate_hex_color("success", &self.palette.success)?;
        validate_hex_color("warning", &self.palette.warning)?;
        validate_hex_color("danger", &self.palette.danger)?;
        validate_asset_name("cover", self.assets.cover.as_deref())?;
        validate_asset_name("background", self.assets.background.as_deref())?;
        validate_asset_name("chat_header", self.assets.chat_header.as_deref())?;
        validate_asset_name("chat_body", self.assets.chat_body.as_deref())?;
        validate_asset_name("chat_input", self.assets.chat_input.as_deref())?;
        validate_optional_hex("appearance.input_bg", self.chrome.input_bg.as_deref())?;
        validate_optional_hex("appearance.input_border", self.chrome.input_border.as_deref())?;
        validate_optional_hex("appearance.input_text", self.chrome.input_text.as_deref())?;
        validate_optional_hex("appearance.panel_bg", self.chrome.panel_bg.as_deref())?;
        validate_optional_hex("appearance.panel_border", self.chrome.panel_border.as_deref())?;
        validate_optional_hex("appearance.button_bg", self.chrome.button_bg.as_deref())?;
        validate_optional_hex("appearance.button_text", self.chrome.button_text.as_deref())?;
        validate_zone_image_fit(
            "appearance.chat_header_image_fit",
            self.chrome.chat_header_image_fit.as_deref(),
        )?;
        validate_zone_image_fit(
            "appearance.chat_body_image_fit",
            self.chrome.chat_body_image_fit.as_deref(),
        )?;
        validate_zone_image_fit(
            "appearance.chat_input_image_fit",
            self.chrome.chat_input_image_fit.as_deref(),
        )?;
        Ok(())
    }
}

fn validate_zone_image_fit(field: &str, value: Option<&str>) -> Result<()> {
    if let Err(err) = super::panel_surface::PanelImageFit::parse(value) {
        bail!("invalid {field}: {err}");
    }
    Ok(())
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn validate_theme_id(id: &str) -> Result<()> {
    let valid = id.len() <= 64
        && id.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if !valid {
        bail!("invalid theme id `{id}`: expected [a-z0-9][a-z0-9_-]{{0,63}}");
    }
    Ok(())
}

/// 资源字段仅允许单层相对文件名，禁止绝对路径、`..` 与目录分隔符。
fn validate_asset_name(field: &str, value: Option<&str>) -> Result<()> {
    let Some(name) = value else {
        return Ok(());
    };
    if name.is_empty() {
        bail!("invalid asset name for {field}: expected non-empty relative filename");
    }
    if name.starts_with('/') || name.starts_with('\\') {
        bail!("invalid asset name for {field}: absolute paths are not allowed, got `{name}`");
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        bail!("invalid asset name for {field}: expected a single relative filename, got `{name}`");
    }
    Ok(())
}

fn validate_optional_hex(field: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value {
        validate_hex_color(field, value)?;
    }
    Ok(())
}

pub(super) fn validate_hex_color(field: &str, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        bail!("invalid hex color for {field}: expected #rrggbb, got `{value}`");
    }
    if !bytes[1..].iter().all(|b| b.is_ascii_hexdigit()) {
        bail!("invalid hex color for {field}: expected #rrggbb, got `{value}`");
    }
    Ok(())
}

pub(super) fn parse_rrggbb(value: &str) -> Result<iced::Color> {
    validate_hex_color("color", value)?;
    let r = u8::from_str_radix(&value[1..3], 16)?;
    let g = u8::from_str_radix(&value[3..5], 16)?;
    let b = u8::from_str_radix(&value[5..7], 16)?;
    Ok(iced::Color::from_rgb8(r, g, b))
}
