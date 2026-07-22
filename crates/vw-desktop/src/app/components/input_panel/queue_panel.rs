//! 队列面板组件模块
//!
//! 本模块提供任务队列的可视化面板组件，用于显示待处理任务的列表。
//! 面板支持显示任务序号、查询内容预览、创建时间以及执行状态指示器。
//!
//! # 主要功能
//!
//! - 渲染任务队列列表，每个任务项包含状态图标、标题和时间戳
//! - 自动截断过长的查询文本（超过42个字符时显示省略号）
//! - 区分正在执行和等待中的任务，使用不同的图标和颜色
//! - 支持滚动浏览，限制最大显示高度以保持界面整洁
//!
//! # 组件结构
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ ● 1. 这是一个示例查询文本...  2026-03-19 14:30:00 │
//! │ ⟳ 2. 另一个查询任务...     2026-03-19 14:31:00 │
//! └─────────────────────────────────────────┘
//! ```

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use once_cell::sync::Lazy;
use time::format_description::FormatItem;

use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::icon_svg;
use crate::app::state::QueueItem;

const MAX_QUEUE_PANEL_HEIGHT: f32 = 132.0;
const QUEUE_SCROLLBAR_WIDTH: u32 = 4;
const MAX_QUEUE_LABEL_CHARS: usize = 42;

pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub(super) fn queue_item_style(theme: &Theme, is_next: bool) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    let background = if is_next {
        if is_dark {
            Color::from_rgba8(27, 35, 50, 0.98)
        } else {
            Color::from_rgba8(242, 247, 255, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(25, 27, 32, 0.96)
    } else {
        Color::from_rgba8(251, 252, 253, 1.0)
    };
    let border_color = if is_next {
        if is_dark {
            Color::from_rgba8(93, 140, 255, 0.78)
        } else {
            Color::from_rgba8(111, 150, 255, 0.42)
        }
    } else if is_dark {
        Color::from_rgba8(54, 58, 66, 0.94)
    } else {
        theme.extended_palette().background.strong.color.scale_alpha(0.72)
    };

    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border_color, radius: 12.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.04 }),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

pub(super) fn queue_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(15, 17, 21, 0.96)
        } else {
            palette.background.base.color.scale_alpha(0.94)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(47, 51, 59, 0.96)
            } else {
                palette.background.strong.color.scale_alpha(0.82)
            },
            radius: 14.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.22 } else { 0.06 }),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}

/// 将毫秒时间戳格式化为可读的日期时间字符串
///
/// 该函数将 Unix 时间戳（毫秒精度）转换为 `YYYY-MM-DD HH:MM:SS` 格式的字符串。
/// 使用静态变量缓存格式描述符以避免重复解析，提高性能。
///
/// # 参数
///
/// - `created_ms`: 创建时间的 Unix 时间戳（毫秒）
///
/// # 返回值
///
/// 返回格式化后的日期时间字符串，格式为 `[年]-[月]-[日] [时]:[分]:[秒]`。
/// 如果时间戳无效或格式化失败，返回空字符串。
///
/// # 示例
///
/// ```ignore
/// let formatted = format_queue_time(1740675600000);
/// // 返回类似 "2025-02-27 18:20:00" 的字符串
/// ```
pub(super) fn format_queue_time(created_ms: u64) -> String {
    /// 时间格式描述符，用于将日期时间格式化为 "YYYY-MM-DD HH:MM:SS" 格式
    ///
    /// 使用 `Lazy` 延迟初始化，避免每次调用函数时重新解析格式字符串。
    /// 格式：`[year]-[month]-[day] [hour]:[minute]:[second]`
    static FMT: Lazy<Vec<FormatItem<'static>>> = Lazy::new(|| {
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .unwrap_or_default()
    });

    // 将毫秒时间戳转换为纳秒精度，用于构建 OffsetDateTime
    let nanos = (created_ms as i128).saturating_mul(1_000_000);

    // 从 Unix 纳秒时间戳构建 OffsetDateTime，并格式化为字符串
    // 任一步骤失败均返回空字符串
    time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|dt| dt.format(&FMT).ok())
        .unwrap_or_default()
}

pub(super) fn queue_item_label(query: &str) -> String {
    let label = query.lines().next().unwrap_or("").trim();

    if label.chars().count() > MAX_QUEUE_LABEL_CHARS {
        format!("{}…", label.chars().take(MAX_QUEUE_LABEL_CHARS).collect::<String>())
    } else {
        label.to_string()
    }
}

pub(super) fn queue_item_meta_label(created_ms: u64, is_next: bool) -> String {
    let time_label = format_queue_time(created_ms);

    if is_next {
        if time_label.is_empty() {
            "下一条".to_string()
        } else {
            format!("下一条 · {}", time_label)
        }
    } else {
        time_label
    }
}

/// 构建并渲染任务队列面板
///
/// 该函数创建一个可滚动的任务队列列表，显示所有待处理的查询任务。
/// 每个任务项包含：
/// - 状态图标（执行中使用旋转箭头，等待中使用圆点）
/// - 任务序号和查询内容预览
/// - 任务创建时间
///
/// # 参数
///
/// - `queue`: 队列项列表，包含所有待处理的查询任务
/// - `is_requesting`: 标识当前是否有请求正在执行。
///   当为 `true` 时，队列第一项会被标记为"下一条"，表示当前请求结束后优先发送
///
/// # 返回值
///
/// 返回一个 `Element`，包含完整的队列面板 UI 组件。
/// 面板特性：
/// - 仅设置最大高度，内容较少时自动收缩，超出部分可滚动
/// - 任务项样式随明暗主题自适应
/// - 队首任务高亮显示，表示会在当前请求结束后优先发送
///
/// # 示例
///
/// ```ignore
/// let queue = vec![
///     QueueItem { query: "查询示例".to_string(), created_ms: 1740675600000 },
/// ];
/// let panel = queue_panel(queue, true);
/// ```
///
/// # UI 结构
///
/// ```text
/// 面板容器（主题自适应背景 + 圆角 + 阴影）
/// └── 滚动容器（内容自适应高度，最大 132px）
///     └── 任务列表（垂直排列，间距 6px）
///         └── 任务卡片（主题自适应背景 + 圆角边框）
///             └── 行布局
///                 ├── 状态图标（13px）
///                 ├── 标题文本（自动填充宽度）
///                 └── 时间文本（11px，次要颜色）
/// ```
pub fn queue_panel(
    queue: Vec<QueueItem>,
    is_requesting: bool,
) -> Element<'static, crate::app::Message> {
    // 构建任务列表容器，设置项间距为 6 像素
    let mut queue_items = column![].spacing(6);

    // 遍历队列中的每个任务项，构建对应的 UI 卡片
    for (i, item) in queue.iter().enumerate() {
        // 获取查询文本的第一行作为显示标签
        // 如果第一行为空则使用空字符串
        let label = queue_item_label(&item.query);

        let is_next = is_requesting && i == 0;

        // 格式化创建时间并构建时间文本组件
        let meta_label = queue_item_meta_label(item.created_ms, is_next);
        let time_text =
            text(meta_label).size(11).style(|theme: &Theme| iced::widget::text::Style {
                // 使用次要颜色的 75% 透明度，使时间文本不那么突出
                color: Some(theme.extended_palette().secondary.base.text.scale_alpha(0.75)),
            });

        // 队列不包含当前执行中的请求，仅高亮队首项表示“下一条”
        let status_icon = if is_next { Icon::ChevronUp } else { Icon::Circle };
        let status_color = if is_next {
            // 队首项使用蓝色，表示当前请求结束后会优先发送
            Color::from_rgba8(32, 118, 255, 1.0)
        } else {
            // 其他等待项使用中性灰色
            Color::from_rgba8(136, 142, 152, 1.0)
        };

        // 构建状态图标，应用对应的颜色
        let status = icon_svg(status_icon, 13.0)
            .style(move |_theme: &Theme, _| svg::Style { color: Some(status_color) });

        // 构建标题文本，格式为 "序号. 查询内容"
        let title = text(format!("{}. {}", i + 1, label)).size(12);

        // 构建行布局：状态图标 + 标题（填充剩余宽度）+ 时间
        let row = row![status, container(title).width(Length::Fill), time_text]
            .spacing(8)
            .align_y(Alignment::Center);

        let item_card = container(row)
            .padding([6, 10])
            .width(Length::Fill)
            .style(move |theme: &Theme| queue_item_style(theme, is_next));

        // 将卡片添加到任务列表中
        queue_items = queue_items.push(item_card);
    }

    // 构建滚动容器，内容较少时自适应，高度超出后再滚动
    let queue_list = scrollable(container(queue_items).width(Length::Fill).padding([0, 0]))
        .width(Length::Fill)
        .direction(Direction::Vertical(
            Scrollbar::new().width(QUEUE_SCROLLBAR_WIDTH).scroller_width(QUEUE_SCROLLBAR_WIDTH),
        ))
        .height(Length::Shrink);

    // 构建外层面板容器，仅限制最大高度，避免少量任务时出现空白区域
    container(queue_list)
        .width(Length::Fill)
        .max_height(MAX_QUEUE_PANEL_HEIGHT)
        .padding(iced::Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 })
        .style(queue_panel_style)
        .into()
}
