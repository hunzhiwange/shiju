//! 聊天分区背景图：铺在内容下方，不参与尺寸计算。

use std::path::Path;

use iced::widget::image::Handle as ImageHandle;
use iced::widget::{Image, Stack};
use iced::{ContentFit, Element, Length};

/// 分区背景图缩放策略（对齐 Iced [`ContentFit`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelImageFit {
    /// 铺满并裁剪，适合作背景。
    #[default]
    Cover,
    /// 完整显示，可能留白。
    Contain,
    /// 拉伸填满，可能变形。
    Fill,
    /// 仅缩小不放大。
    ScaleDown,
    /// 原始尺寸。
    None,
}

impl PanelImageFit {
    pub const ALL: [Self; 5] =
        [Self::Cover, Self::Contain, Self::Fill, Self::ScaleDown, Self::None];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cover => "cover",
            Self::Contain => "contain",
            Self::Fill => "fill",
            Self::ScaleDown => "scale_down",
            Self::None => "none",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cover => "覆盖裁剪",
            Self::Contain => "完整容纳",
            Self::Fill => "拉伸填满",
            Self::ScaleDown => "仅缩小",
            Self::None => "原始尺寸",
        }
    }

    pub fn to_content_fit(self) -> ContentFit {
        match self {
            Self::Cover => ContentFit::Cover,
            Self::Contain => ContentFit::Contain,
            Self::Fill => ContentFit::Fill,
            Self::ScaleDown => ContentFit::ScaleDown,
            Self::None => ContentFit::None,
        }
    }

    /// 解析配置值；空/`None` 默认 Cover。非法值返回错误。
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        let Some(raw) = value.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(Self::Cover);
        };
        match raw.to_ascii_lowercase().as_str() {
            "cover" => Ok(Self::Cover),
            "contain" => Ok(Self::Contain),
            "fill" => Ok(Self::Fill),
            "scale_down" | "scaledown" => Ok(Self::ScaleDown),
            "none" => Ok(Self::None),
            other => Err(format!(
                "invalid panel_image_fit `{other}`: expected cover|contain|fill|scale_down|none"
            )),
        }
    }
}

impl std::fmt::Display for PanelImageFit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// 若存在分区背景图，将其铺在内容下方；尺寸仅由内容决定。
///
/// 使用 [`Stack::push_under`]：底层图片不改变 Stack 固有尺寸，避免输入区等被撑高。
/// 层叠顺序：底层图片 → 上层内容（可带半透明色）。
pub fn with_panel_background<'a, Message: 'a>(
    content: Element<'a, Message>,
    path: Option<&Path>,
    fit: PanelImageFit,
) -> Element<'a, Message> {
    let Some(path) = path.filter(|p| p.is_file()) else {
        return content;
    };

    let image: Element<'a, Message> = Image::new(ImageHandle::from_path(path.to_path_buf()))
        .content_fit(fit.to_content_fit())
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    // 先 push 内容作为尺寸基准，再 push_under 背景图。
    Stack::new().push(content).push_under(image).width(Length::Fill).clip(true).into()
}

#[cfg(test)]
#[path = "panel_surface_tests.rs"]
mod panel_surface_tests;
