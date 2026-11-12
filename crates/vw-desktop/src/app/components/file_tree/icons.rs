//! 文件树图标模块
//!
//! 本模块提供文件树组件中使用的图标渲染功能。主要包含以下功能：
//! - 创建静态 SVG 图标
//! - 创建主题感知的 SVG 图标（根据主题自动调整颜色）
//! - 根据文件扩展名自动选择对应的图标
//!
//! # 主要组件
//!
//! - [`static_icon_svg`]: 创建固定颜色的静态 SVG 图标
//! - [`themed_icon_svg`]: 创建可随主题变化的 SVG 图标
//! - [`file_icon_for`]: 根据文件名返回对应的图标类型

use iced::widget::svg::Svg;
use iced::{Length, Theme};

use crate::app::assets::{self, Icon};

pub(super) const ICON_SIZE: f32 = 14.0;

/// 创建静态 SVG 图标
///
/// 生成一个固定尺寸（14x14 像素）的 SVG 图标，不随主题变化。
/// 适用于需要固定颜色显示的图标场景。
///
/// # 参数
///
/// - `icon`: 图标枚举类型，指定要渲染的图标
///
/// # 返回值
///
/// 返回一个 `Svg<'static>` 组件，可直接用于 iced UI 布局
///
/// # 示例
///
/// ```ignore
/// use crate::app::assets::Icon;
/// use crate::app::components::file_tree::icons::static_icon_svg;
///
/// let rust_icon = static_icon_svg(Icon::Rust);
/// // 在 UI 中使用该图标
/// ```
pub fn static_icon_svg(icon: Icon) -> Svg<'static> {
    Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
}

/// 创建主题感知的 SVG 图标
///
/// 生成一个固定尺寸（14x14 像素）的 SVG 图标，其颜色会根据当前主题自动调整。
/// 图标颜色将使用主题的文本颜色（`theme.palette().text`）。
///
/// # 参数
///
/// - `icon`: 图标枚举类型，指定要渲染的图标
///
/// # 返回值
///
/// 返回一个 `Svg<'static>` 组件，颜色会随主题自动变化
///
/// # 示例
///
/// ```ignore
/// use crate::app::assets::Icon;
/// use crate::app::components::file_tree::icons::themed_icon_svg;
///
/// let folder_icon = themed_icon_svg(Icon::Folder);
/// // 在 UI 中使用该图标，颜色会随主题自动调整
/// ```
pub fn themed_icon_svg(icon: Icon) -> Svg<'static> {
    Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .style(|theme: &Theme, _status| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        })
}

/// 根据文件名返回对应的图标类型
///
/// 通过分析文件名的扩展名，自动选择合适的图标类型。
/// 扩展名匹配不区分大小写。
///
/// # 参数
///
/// - `name`: 文件名字符串（包含扩展名）
///
/// # 返回值
///
/// 返回与文件类型匹配的 `Icon` 枚举值
///
/// # 支持的文件类型
///
/// | 扩展名 | 图标 |
/// |--------|------|
/// | `.rs` | Rust |
/// | `.ts`, `.tsx` | Typescript |
/// | `.js`, `.jsx` | Javascript |
/// | `.json` | Json |
/// | `.toml` | Toml |
/// | `.yaml`, `.yml` | Yaml |
/// | `.md` | Markdown |
/// | `.html`, `.htm` | Html |
/// | `.css` | Css |
/// | `.py` | Python |
/// | `.go` | Go |
/// | `.sh` | Console |
/// | 其他 | Document |
///
/// # 示例
///
/// ```ignore
/// use crate::app::components::file_tree::icons::file_icon_for;
/// use crate::app::assets::Icon;
///
/// assert!(matches!(file_icon_for("main.rs"), Icon::Rust));
/// assert!(matches!(file_icon_for("config.JSON"), Icon::Json));
/// assert!(matches!(file_icon_for("unknown.xyz"), Icon::Document));
/// ```
pub fn file_icon_for(name: &str) -> Icon {
    let lower = name.to_lowercase();

    if lower.ends_with(".rs") {
        Icon::Rust
    } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
        Icon::Typescript
    } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
        Icon::Javascript
    } else if lower.ends_with(".json") {
        Icon::Json
    } else if lower.ends_with(".toml") {
        Icon::Toml
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        Icon::Yaml
    } else if lower.ends_with(".md") {
        Icon::Markdown
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        Icon::Html
    } else if lower.ends_with(".css") {
        Icon::Css
    } else if lower.ends_with(".py") {
        Icon::Python
    } else if lower.ends_with(".go") {
        Icon::Go
    } else if lower.ends_with(".sh") {
        Icon::Console
    } else {
        Icon::Document
    }
}
