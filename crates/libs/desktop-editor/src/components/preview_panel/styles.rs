//! 预览面板样式模块
//!
//! 本模块提供预览面板相关的样式定义、图标渲染和宽度计算辅助功能。
//! 主要包含：
//! - 菜单按钮样式配置
//! - 文件图标选择逻辑
//! - 标签页标题截断与宽度估算
//! - 滚动条显示判断逻辑

use desktop_assets::assets::{self, Icon};
use iced::widget::svg::{self, Svg};
use iced::{Background, Border, Color, Length};
use unicode_width::UnicodeWidthChar;

/// 标签栏布局需要的窗口和分栏参数。
#[derive(Clone, Copy)]
pub struct PreviewTabsLayout {
    pub window_width: f32,
    pub show_file_manager: bool,
    pub file_manager_width: f32,
    pub show_diff: bool,
    pub split_ratio: f32,
}

/// 生成菜单按钮的样式配置
///
/// 根据当前主题和按钮状态（如悬停），生成对应的按钮样式。
/// 悬停时会显示带透明度的主题色背景，并应用圆角边框。
///
/// # 参数
/// - `theme`: 当前的 Iced 主题引用，用于获取调色板信息
/// - `status`: 按钮的当前状态（如 Active、Hovered、Pressed 等）
///
/// # 返回值
/// 返回配置好的 `iced::widget::button::Style` 实例
///
/// # 示例
/// ```ignore
/// let style = menu_button_style(&theme, iced::widget::button::Status::Hovered);
/// ```
pub fn menu_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    // 检测按钮是否处于悬停状态
    let hovered = matches!(status, iced::widget::button::Status::Hovered);

    // 悬停时使用主题色的半透明背景
    let bg = if hovered {
        Some(Background::Color(Color::from_rgba(
            theme.palette().primary.r,
            theme.palette().primary.g,
            theme.palette().primary.b,
            0.10, // 10% 不透明度
        )))
    } else {
        None
    };

    iced::widget::button::Style {
        background: bg,
        border: Border {
            radius: 6.0.into(), // 6像素圆角
            width: 0.0,         // 无边框宽度
            color: theme.palette().primary,
        },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

/// 创建小尺寸图标的 SVG 组件
///
/// 将指定图标渲染为固定尺寸（14x14 像素）的 SVG 组件。
///
/// # 参数
/// - `icon`: 图标枚举值，指定要渲染的图标类型
///
/// # 返回值
/// 返回配置好尺寸的 `Svg<'static>` 组件
pub fn small_icon_svg(icon: Icon) -> Svg<'static> {
    Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(|theme: &iced::Theme, _status| svg::Style { color: Some(theme.palette().text) })
}

pub fn file_tab_icon_svg(icon: Icon) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(
        |theme: &iced::Theme, _status| {
            let palette = theme.extended_palette();
            let bg = theme.palette().background;
            let is_dark = bg.r + bg.g + bg.b < 1.5;
            let color = if is_dark {
                palette.background.strong.text.scale_alpha(0.78)
            } else {
                Color::from_rgba8(71, 85, 105, 0.72)
            };
            svg::Style { color: Some(color) }
        },
    )
}

/// 根据文件名获取对应的文件图标
///
/// 通过文件扩展名匹配，返回适合该文件类型的图标。
/// 支持常见的编程语言、配置文件、图片等格式。
///
/// # 参数
/// - `name`: 文件名字符串（包含扩展名）
///
/// # 返回值
/// 返回匹配的 `Icon` 枚举值，若无法识别则返回 `Icon::Document`
///
/// # 示例
/// ```ignore
/// assert_eq!(file_icon_for("main.rs"), Icon::Rust);
/// assert_eq!(file_icon_for("config.json"), Icon::Json);
/// assert_eq!(file_icon_for("readme.md"), Icon::Markdown);
/// assert_eq!(file_icon_for("unknown.xyz"), Icon::Document);
/// ```
pub fn file_icon_for(name: &str) -> Icon {
    // 统一转换为小写以进行不区分大小写的匹配
    let lower = name.to_lowercase();

    // 按扩展名匹配对应的图标
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
    } else if matches!(
        lower.rsplit('.').next(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg")
    ) {
        Icon::Image
    } else {
        // 默认图标
        Icon::Document
    }
}

/// 截断标题字符串到指定最大字符数
///
/// 当标题超过最大长度时，在末尾添加省略号（…）。
/// 保留完整字符，不会截断多字节字符的中间。
///
/// # 参数
/// - `s`: 原始标题字符串
/// - `max`: 最大字符数限制
///
/// # 返回值
/// 返回截断后的字符串，若超出限制则在末尾添加省略号
///
/// # 示例
/// ```ignore
/// assert_eq!(truncate_title("Hello World", 5), "Hello…");
/// assert_eq!(truncate_title("Hi", 5), "Hi");
/// ```
pub fn truncate_title(s: &str, max: usize) -> String {
    let mut out = String::new();

    // 按字符遍历，确保多字节字符完整性
    for (i, ch) in s.chars().enumerate() {
        if i >= max {
            // 超出限制时添加省略号并终止
            out.push('…');
            break;
        }
        out.push(ch);
    }

    out
}

/// 估算标签标题的像素宽度
///
/// 基于 Unicode 字符宽度计算标题的近似显示宽度。
/// 限制标题最多考虑前 40 个字符，以避免过长的计算。
///
/// # 参数
/// - `title`: 标签标题字符串
///
/// # 返回值
/// 返回估算的像素宽度值（至少 32.0 像素）
pub(crate) fn estimate_tab_title_width_px(title: &str) -> f32 {
    // 计算前40个字符的 Unicode 宽度总和
    let width_units = title
        .chars()
        .take(40)
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(1) as f32)
        .sum::<f32>();

    // 基于 UI 中 13 号字体的近似换算系数
    // 每个宽度单位约等于 7.2 像素
    (width_units * 7.2).max(32.0)
}

/// 估算标签页的总像素宽度
///
/// 计算一个标签页按钮的完整宽度，包括图标、标题、关闭按钮和间距。
///
/// # 参数
/// - `title`: 标签标题字符串
///
/// # 返回值
/// 返回估算的标签页总宽度（像素）
pub(crate) fn estimate_tab_width_px(title: &str) -> f32 {
    // 宽度组成：
    // - 图标: 16px
    // - 关闭按钮区域: 约22px
    // - 行间距: 6px * 2 = 12px
    // - 按钮容器水平内边距: 6px * 2 = 12px
    // - 标题宽度: 根据内容估算
    16.0 + 22.0 + 12.0 + 12.0 + estimate_tab_title_width_px(title)
}

/// 估算预览标签视口的像素宽度
///
/// 根据窗口大小和面板布局状态，计算预览标签栏的实际可用宽度。
/// 考虑文件管理器面板和差异对比视图的分隔占用。
///
/// # 参数
/// - `layout`: 窗口大小和面板配置
///
/// # 返回值
/// 返回估算的视口宽度（像素），至少为 1.0
pub fn estimate_preview_tabs_viewport_width(layout: PreviewTabsLayout) -> f32 {
    // 从窗口宽度开始计算
    let mut width = layout.window_width.max(1.0);

    // 减去文件管理器面板占用的宽度
    if layout.show_file_manager {
        // 文件管理器分隔线命中宽度(8px) + 面板宽度
        width -= 8.0 + layout.file_manager_width;
    }

    // 在差异对比视图中，进一步分配右侧空间
    if layout.show_diff {
        // 减去分隔线命中宽度(8px)，然后将剩余宽度分配给预览面板
        width = (width - 8.0).max(1.0);

        // 计算右侧比例（限制在 20%-80% 之间）
        let right_ratio = (1.0 - layout.split_ratio).clamp(0.2, 0.8);
        width *= right_ratio;
    }

    // 减去预览面板在项目视图中的内边距与右上角按钮预留（近似值）
    (width - 18.0).max(1.0)
}

/// 判断是否应该显示预览标签栏的滚动条
///
/// 通过估算所有标签页的总宽度和视口可用宽度进行比较，
/// 判断是否需要显示水平滚动条。
///
/// # 参数
/// - `layout`: 面板布局；`titles`: 借用的标签标题
///
/// # 返回值
/// - `true`: 标签页总宽度超过视口宽度，需要显示滚动条
/// - `false`: 标签页可完全显示，无需滚动条
///
/// # 计算逻辑
/// 总宽度 = 所有标签页宽度之和 + 标签间距(每个标签间4px) + 末尾边距(4px)
pub fn should_show_preview_tabs_scrollbar<'a>(
    layout: PreviewTabsLayout,
    titles: impl ExactSizeIterator<Item = &'a str>,
) -> bool {
    let count = titles.len();
    if count == 0 {
        return false;
    }
    let total_tabs_width = titles.map(estimate_tab_width_px).sum::<f32>()
        + count.saturating_sub(1) as f32 * 4.0 + 4.0;
    total_tabs_width > estimate_preview_tabs_viewport_width(layout)
}
