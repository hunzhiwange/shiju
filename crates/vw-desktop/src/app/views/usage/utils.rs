//! Usage 视图工具函数模块
//!
//! 本模块提供用于构建使用情况统计视图的辅助函数和 UI 组件。
//! 主要功能包括：
//! - 节标题和键值对显示组件
//! - 图标按钮及其工具提示
//! - 路径格式化显示
//! - 时间戳和金额格式化

use iced::widget::svg;
use iced::widget::{
    Space, Svg, button, container, row, text,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Alignment, Color, Length, Theme};

use crate::app::Message;
use crate::app::assets::{self, Icon};
use crate::app::message::view::ViewMessage;

fn muted_label_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().background.weak.text.scale_alpha(0.9)),
    }
}

fn primary_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().text) }
}

fn icon_svg_style(theme: &Theme) -> svg::Style {
    svg::Style { color: Some(theme.palette().text) }
}

fn icon_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let bg = match status {
        iced::widget::button::Status::Hovered => Some(palette.background.weak.color.into()),
        iced::widget::button::Status::Pressed => Some(palette.background.strong.color.into()),
        _ => None,
    };

    iced::widget::button::Style {
        background: bg,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 6.0.into() },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn tooltip_container_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(iced::Background::Color(theme.palette().background)),
        border: iced::Border {
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
            radius: 8.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

/// 创建节标题文本元素
///
/// 用于在视图中显示带有一致样式的节标题文本。
///
/// # 参数
///
/// * `label` - 标题文本内容
///
/// # 返回值
///
/// 返回一个 iced Element，渲染为 14 号字体的文本
///
/// # 示例
///
/// ```ignore
/// let title = section_title("使用统计");
/// ```
pub fn section_title<'a>(label: &'a str) -> iced::Element<'a, Message> {
    text(label).size(14).into()
}

/// 创建键值对显示行
///
/// 生成一行布局，左侧显示标签，右侧显示值，中间用空白填充。
/// 标签使用弱化的文本颜色，值使用正常的文本颜色。
///
/// # 参数
///
/// * `label` - 左侧标签文本
/// * `value` - 右侧值文本
///
/// # 返回值
///
/// 返回一个 iced Element，包含标签、空白填充和值的水平布局
///
/// # 示例
///
/// ```ignore
/// let row = kv("总请求数", "1234".to_string());
/// ```
pub fn kv<'a>(label: &'a str, value: String) -> iced::Element<'a, Message> {
    row![
        text(label).size(12).style(muted_label_text_style),
        Space::new().width(Length::Fill),
        text(value).size(12).style(primary_text_style)
    ]
    .align_y(Alignment::Center)
    .into()
}

/// 创建 SVG 图标组件
///
/// 根据给定的图标类型创建一个固定尺寸（14x14 像素）的 SVG 图标。
///
/// # 参数
///
/// * `icon` - 图标枚举值，指定要渲染的图标类型
///
/// # 返回值
///
/// 返回一个 Svg 组件，宽度和高度均为 14.0 像素
///
/// # 示例
///
/// ```ignore
/// let icon = icon_svg(Icon::Copy);
/// ```
pub fn icon_svg(icon: Icon) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(14.0)).height(Length::Fixed(14.0))
}

/// 创建带工具提示的图标按钮
///
/// 生成一个可点击的图标按钮，带有底部显示的工具提示。
/// 按钮具有悬停和按压状态的视觉反馈。
///
/// # 参数
///
/// * `icon` - 图标枚举值，指定按钮上显示的图标
/// * `tip` - 工具提示文本内容
/// * `msg` - 按钮点击时发送的消息
///
/// # 返回值
///
/// 返回一个 iced Element，包含按钮和工具提示的组合组件
///
/// # 样式特性
///
/// - 按钮高度：24 像素，内边距：上下 4 像素，左右 6 像素
/// - 悬停状态：使用弱背景色
/// - 按压状态：使用强背景色
/// - 工具提示：底部显示，与按钮间距 8 像素
///
/// # 示例
///
/// ```ignore
/// let btn = icon_btn(Icon::Copy, "复制内容", Message::CopyCode(text));
/// ```
pub fn icon_btn<'a>(icon: Icon, tip: &'a str, msg: Message) -> iced::Element<'a, Message> {
    // 创建图标按钮，设置尺寸、内边距和样式
    let btn = button(icon_svg(icon).style(|theme: &Theme, _status| icon_svg_style(theme)))
        .on_press(msg)
        .height(Length::Fixed(24.0))
        .padding([4, 6])
        .style(icon_button_style);

    // 创建工具提示容器，设置文本和样式
    let tip_content =
        container(text(tip.to_string()).size(12)).padding([6, 10]).style(tooltip_container_style);
    Tooltip::new(btn, tip_content, TooltipPosition::Bottom).gap(8).into()
}

/// 格式化显示路径为简短形式
///
/// 将完整路径转换为更简洁的显示形式，仅保留父目录名和文件名。
/// 格式为："…/父目录/文件名"，如果路径没有父目录则仅显示文件名。
///
/// # 参数
///
/// * `path` - 完整文件路径字符串
///
/// # 返回值
///
/// 返回简化后的路径字符串，格式如 "…/config/settings.json" 或 "file.txt"
///
/// # 示例
///
/// ```ignore
/// let short = display_path("/home/user/project/config/settings.json");
/// // 返回 "…/config/settings.json"
///
/// let short = display_path("README.md");
/// // 返回 "README.md"
/// ```
pub fn display_path(path: &str) -> String {
    let p = std::path::Path::new(path);
    // 获取文件名（路径最后一部分）
    let file = p.file_name().and_then(|s| s.to_str()).unwrap_or(path);
    // 获取父目录名（路径倒数第二部分）
    let parent = p.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str());
    match parent {
        Some(dir) => format!("…/{}/{}", dir, file),
        None => file.to_string(),
    }
}

/// 创建键值对路径显示行（带操作按钮）
///
/// 生成一行布局，左侧显示标签，右侧显示简化的路径，
/// 如果路径存在，还会添加"复制路径"和"打开"两个操作按钮。
///
/// # 参数
///
/// * `label` - 左侧标签文本
/// * `path` - 可选的文件路径，None 时显示"暂无"
///
/// # 返回值
///
/// 返回一个 iced Element，包含标签、路径显示和可选的操作按钮
///
/// # 功能特性
///
/// - 路径自动简化显示（仅显示父目录和文件名）
/// - 路径不存在时显示"暂无"
/// - 路径存在时显示：
///   - 复制按钮：点击复制完整路径到剪贴板
///   - 打开按钮：在文件管理器中打开路径
///
/// # 示例
///
/// ```ignore
/// let row = kv_path("配置文件", Some("/path/to/config.json".to_string()));
/// let row = kv_path("日志文件", None); // 显示"暂无"
/// ```
pub fn kv_path<'a>(label: &'a str, path: Option<String>) -> iced::Element<'a, Message> {
    // 格式化路径显示，无路径时显示"暂无"
    let value = path.as_deref().map(display_path).unwrap_or_else(|| "暂无".to_string());
    // 创建基础键值对行布局
    let mut r = row![
        text(label).size(12).style(muted_label_text_style),
        Space::new().width(Length::Fill),
        text(value).size(12).style(primary_text_style),
    ]
    .align_y(Alignment::Center)
    .spacing(10);

    // 如果路径存在，添加复制和打开按钮
    if let Some(p) = path {
        r = r.push(icon_btn(Icon::Copy, "复制路径", Message::CopyCode(p.clone())));
        r = r.push(icon_btn(
            Icon::FolderOpen,
            "打开",
            Message::View(ViewMessage::OpenPathInFinder(p)),
        ));
    }

    r.into()
}

/// 将毫秒时间戳格式化为可读的日期时间字符串
///
/// 将 Unix 毫秒时间戳转换为 "YYYY-MM-DD HH:MM" 格式的字符串。
/// 如果转换失败，返回"暂无"。
///
/// # 参数
///
/// * `ms` - Unix 时间戳（毫秒）
///
/// # 返回值
///
/// 返回格式化后的日期时间字符串，如 "2024-03-19 14:30"
/// 转换失败时返回 "暂无"
///
/// # 示例
///
/// ```ignore
/// let time_str = fmt_ms(1710844800000);
/// // 返回类似 "2024-03-19 12:00"
///
/// let time_str = fmt_ms(0);
/// // 返回 "1970-01-01 08:00"（取决于时区）
/// ```
pub fn fmt_ms(ms: u64) -> String {
    // 将毫秒转换为秒级时间戳
    let ts = (ms / 1000) as i64;
    // 从 Unix 时间戳创建日期时间对象
    let Ok(dt) = time::OffsetDateTime::from_unix_timestamp(ts) else {
        return "暂无".to_string();
    };
    // 定义日期时间格式：年-月-日 时:分
    let Ok(fmt) = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]") else {
        return "暂无".to_string();
    };
    dt.format(&fmt).unwrap_or_else(|_| "暂无".to_string())
}

/// 将金额格式化为美元字符串
///
/// 将浮点数金额格式化为带 "US$" 前缀的字符串，保留 4 位小数。
///
/// # 参数
///
/// * `v` - 金额数值（美元）
///
/// # 返回值
///
/// 返回格式化后的金额字符串，如 "US$0.0012"
///
/// # 示例
///
/// ```ignore
/// let amount = fmt_usd(0.00123456);
/// // 返回 "US$0.0012"
///
/// let amount = fmt_usd(12.5);
/// // 返回 "US$12.50"
/// ```
pub fn fmt_usd(v: f64) -> String {
    format!("US${:.4}", v)
}

#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;
