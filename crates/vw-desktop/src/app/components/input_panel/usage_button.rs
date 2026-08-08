//! 使用率按钮组件
//!
//! 本模块提供用于显示和交互 Token 使用率信息的按钮组件。该组件以圆环形式
//! 可视化当前上下文使用率，并通过工具提示展示详细信息，包括：
//! - 上下文使用情况（输入 token 数 / 上下文限制）
//! - 使用率百分比
//! - 累计 Token 数量
//! - 预估成本
//!
//! 点击按钮可打开详细的使用率统计视图。

use iced::widget::tooltip::Position;
use iced::widget::{button, canvas, column, container, text, tooltip};
use iced::{Color, Element, Length, Theme};

use crate::app::components::input_panel::styles::{
    BOTTOM_BAR_ICON_BUTTON_SIZE, round_icon_button_style, tooltip_dark_style,
};
use crate::app::components::input_panel::usage::{
    UsageRing, get_usage_details, get_usage_rate_percent,
};
use crate::app::{App, Message, message};

/// 创建使用率按钮组件
///
/// 该函数构建一个带有圆环可视化效果的使用率按钮，显示当前 Token 使用率。
/// 按钮包含一个工具提示，鼠标悬停时显示详细的使用统计信息。
///
/// # 参数
///
/// * `app` - 应用状态引用，用于获取当前的使用率数据
///
/// # 返回值
///
/// 返回一个 `Element<Message>` 类型的 UI 元素，包含：
/// - 一个可视化圆环，显示使用率百分比
/// - 工具提示，展示详细的统计信息
/// - 点击交互，打开详细使用率视图
///
/// # 示例
///
/// ```ignore
/// let button = usage_button(&app);
/// // 在 UI 布局中使用该按钮
/// ```
///
/// # 实现细节
///
/// 1. 通过 `get_usage_rate_percent` 获取使用率百分比
/// 2. 通过 `get_usage_details` 获取详细的 token 统计和成本信息
/// 3. 创建 `UsageRing` 画布组件，以圆环形式可视化使用率
/// 4. 构建工具提示容器，显示多行统计信息
/// 5. 创建按钮，设置样式和点击事件处理器
/// 6. 使用 `tooltip` 组件包装按钮和提示内容
pub fn usage_button(app: &App) -> Element<'_, Message> {
    // 获取当前使用率百分比和详细统计信息
    let usage_percent = get_usage_rate_percent(app);
    let (input_tokens, context_limit, estimated_cost, total_tokens) = get_usage_details(app);
    let input_limit = app.usage_model_info.as_ref().map(|info| info.input_limit).unwrap_or(0);

    // 创建圆环画布，用于可视化使用率
    let usage_ring = canvas(UsageRing { percent: usage_percent })
        .width(Length::Fixed(22.0))
        .height(Length::Fixed(22.0));

    // 构建工具提示内容，显示详细的使用统计信息
    let usage_tip = container(
        column![
            // 输入使用情况：输入 token 数 / 当前请求输入窗口
            text(format!("输入窗口: {} / {}", input_tokens, input_limit))
                .size(12)
                .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
            text(format!("上下文上限: {}", context_limit))
                .size(12)
                .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
            // 使用率百分比，使用黄色高亮显示
            text(format!("使用率: {:.1}%", usage_percent)).size(12).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::from_rgb8(255, 225, 80)) }
            }),
            // 累计 Token 数量
            text(format!("累计 Token: {}", total_tokens)).size(12).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.9)) }
            }),
            // 预估成本
            text(format!("预估成本: ${:.4}", estimated_cost)).size(12).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.9)) }
            }),
            if app.usage_model_info.as_ref().is_some_and(|info| info.limit_is_default) {
                text("限制：使用保守默认值").size(12).style(|_theme: &Theme| {
                    iced::widget::text::Style { color: Some(Color::from_rgb8(255, 225, 80)) }
                })
            } else {
                text("").size(12)
            },
        ]
        .spacing(4),
    )
    .style(tooltip_dark_style)
    .padding([8, 10]);

    let usage_btn = button(
        container(usage_ring)
            .width(Length::Fixed(BOTTOM_BAR_ICON_BUTTON_SIZE))
            .height(Length::Fixed(BOTTOM_BAR_ICON_BUTTON_SIZE))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .style(|theme: &Theme, status| round_icon_button_style(theme, status, true))
    .on_press(Message::View(message::ViewMessage::OpenUsage));

    // 创建带工具提示的按钮并返回
    tooltip(usage_btn, usage_tip, Position::Top).into()
}
