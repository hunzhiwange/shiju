//! 队列面板组件模块
//!
//! 本模块提供任务队列的可视化面板组件，用于显示待处理任务的列表。
//! 面板支持显示任务序号、单行内容预览以及执行状态指示器。
//!
//! # 主要功能
//!
//! - 渲染任务队列列表，每个任务项仅展示单行内容预览与图标操作
//! - 自动截断过长的查询文本（超过42个字符时显示省略号）
//! - 悬停时在右侧浮层展示完整内容
//! - 区分引导与下一条任务，使用不同的图标和颜色
//! - 支持引导、再次编辑、复制、上下移动和删除队列项
//! - 支持滚动浏览，限制最大显示高度以保持界面整洁
//!
//! # 组件结构
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ ● 1. 这是一个示例查询文本...            │
//! │ ○ 2. 另一个查询任务...                  │
//! └─────────────────────────────────────────┘
//! ```

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::icon_svg;
use crate::app::state::{ChatSendBehavior, QueueItem};
use crate::app::{Message, message};

use super::dark_tooltip_content;
use super::styles::{self, square_icon_button_style};

const MAX_QUEUE_PANEL_HEIGHT: f32 = 120.0;
const QUEUE_SCROLLBAR_WIDTH: u32 = 4;
const MAX_QUEUE_LABEL_CHARS: usize = 42;
const QUEUE_ACTION_BUTTON_SIZE: f32 = 22.0;
const QUEUE_CONTENT_TOOLTIP_MAX_WIDTH: f32 = 360.0;

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

pub(super) fn queue_item_label(query: &str) -> String {
    let label = query.lines().next().unwrap_or("").trim();

    if label.chars().count() > MAX_QUEUE_LABEL_CHARS {
        format!("{}…", label.chars().take(MAX_QUEUE_LABEL_CHARS).collect::<String>())
    } else {
        label.to_string()
    }
}

fn queue_content_tooltip(query: String) -> Element<'static, Message> {
    container(
        text(query)
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(styles::tooltip_dark_style)
    .padding([6, 8])
    .max_width(QUEUE_CONTENT_TOOLTIP_MAX_WIDTH)
    .into()
}

pub(super) fn queue_move_availability(index: usize, queue_len: usize) -> (bool, bool) {
    if index >= queue_len {
        return (false, false);
    }
    (index > 0, index + 1 < queue_len)
}

fn queue_action_button(
    icon: Icon,
    tooltip_label: &'static str,
    action: Option<message::ChatMessage>,
) -> Element<'static, Message> {
    let enabled = action.is_some();
    let button_content = container(icon_svg(icon, 12.0))
        .width(Length::Fixed(QUEUE_ACTION_BUTTON_SIZE))
        .height(Length::Fixed(QUEUE_ACTION_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);
    let mut action_button = button(button_content)
        .padding(0)
        .style(move |theme: &Theme, status| square_icon_button_style(theme, status, enabled));
    if let Some(action) = action {
        action_button = action_button.on_press(Message::Chat(action));
    }

    Tooltip::new(action_button, dark_tooltip_content(tooltip_label), TooltipPosition::Top)
        .gap(4.0)
        .into()
}

/// 构建并渲染任务队列面板
///
/// 该函数创建一个可滚动的任务队列列表，显示所有待处理的查询任务。
/// 每个任务项包含：
/// - 状态图标（引导 / 下一条 / 等待）
/// - 任务序号和单行查询内容预览
/// - 悬停时右侧展示完整内容
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
/// └── 滚动容器（内容自适应高度，最大 120px）
///     └── 任务列表（垂直排列，间距 4px）
///         └── 任务卡片（主题自适应背景 + 圆角边框）
///             └── 行布局
///                 ├── 状态图标（13px）
///                 ├── 单行标题（自动填充宽度，悬停右侧展示全文）
///                 └── 操作按钮
/// ```
pub fn queue_panel(
    queue: Vec<QueueItem>,
    is_requesting: bool,
) -> Element<'static, crate::app::Message> {
    // 构建任务列表容器，设置项间距为 4 像素
    let mut queue_items = column![].spacing(4);

    // 遍历队列中的每个任务项，构建对应的 UI 卡片
    for (i, item) in queue.iter().enumerate() {
        // 获取查询文本的第一行作为显示标签
        // 如果第一行为空则使用空字符串
        let label = queue_item_label(&item.query);
        let full_query = item.query.clone();

        let is_next = is_requesting && i == 0;
        let is_guide = item.send_behavior == ChatSendBehavior::Guide;

        // 队列不包含当前执行中的请求，仅高亮队首项表示“下一条”
        let status_icon = if is_guide {
            Icon::ChatTextFill
        } else if is_next {
            Icon::ChevronUp
        } else {
            Icon::Circle
        };
        let status_color = if is_guide || is_next {
            // 队首项使用蓝色，表示当前请求结束后会优先发送
            Color::from_rgba8(32, 118, 255, 1.0)
        } else {
            // 其他等待项使用中性灰色
            Color::from_rgba8(136, 142, 152, 1.0)
        };

        // 构建状态图标，应用对应的颜色
        let status = icon_svg(status_icon, 13.0)
            .style(move |_theme: &Theme, _| svg::Style { color: Some(status_color) });

        // 构建标题文本，格式为 "序号. 查询内容"；悬停时在右侧展示完整内容
        let title = text(format!("{}. {}", i + 1, label)).size(12).line_height(1.0);
        let info = container(
            Tooltip::new(title, queue_content_tooltip(full_query), TooltipPosition::Right).gap(8.0),
        )
        .width(Length::Fill);
        let (can_move_up, can_move_down) = queue_move_availability(i, queue.len());
        let guide_button = queue_action_button(
            Icon::ChatTextFill,
            "立即引导",
            Some(message::ChatMessage::QueueGuide(i)),
        );
        let edit_button = queue_action_button(
            Icon::Pencil,
            "再次编辑文字",
            Some(message::ChatMessage::QueueEdit(i)),
        );
        let copy_button =
            queue_action_button(Icon::Copy, "复制", Some(message::ChatMessage::QueueCopy(i)));
        let move_up_button = queue_action_button(
            Icon::ArrowUp,
            "上移",
            can_move_up.then_some(message::ChatMessage::QueueUp(i)),
        );
        let move_down_button = queue_action_button(
            Icon::ArrowDown,
            "下移",
            can_move_down.then_some(message::ChatMessage::QueueDown(i)),
        );
        let remove_button =
            queue_action_button(Icon::Trash, "删除", Some(message::ChatMessage::QueueRemove(i)));
        let actions = row![
            guide_button,
            edit_button,
            copy_button,
            move_up_button,
            move_down_button,
            remove_button
        ]
        .spacing(3)
        .align_y(Alignment::Center);

        // 构建行布局：状态图标 + 单行内容（填充剩余宽度）+ 图标操作
        let row = row![status, info, actions].spacing(8).align_y(Alignment::Center);

        let item_card = container(row)
            .padding([5, 10])
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
