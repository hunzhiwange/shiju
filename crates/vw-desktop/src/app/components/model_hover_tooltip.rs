//! 模型悬浮提示组件。
//!
//! 本模块将模型或文本触发区与悬浮提示内容绑定，供模型选择控件展示补充说明。

use iced::widget::{MouseArea, Space, column, container, text};
/// 重新导出 use iced::{Color, Element, Length, Point, Theme}，让上层模块通过稳定路径访问。
use iced::{Color, Element, Length, Point, Theme};

/// 重新导出模型提示使用的指针左侧浮层。
use crate::app::components::overlays::PointLeftOverlay;
/// 重新导出 use crate::app::state::ModelPopoverHover，让上层模块通过稳定路径访问。
use crate::app::state::ModelPopoverHover;
/// 重新导出 use crate::app::{App, Message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message};

/// MODEL_HOVER_TOOLTIP_WIDTH 是当前模块共享的固定参数。
const MODEL_HOVER_TOOLTIP_WIDTH: f32 = 240.0;

/// HoverAnchor 保存 model_hover_tooltip 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
#[derive(Debug, Clone, Copy)]
pub struct HoverAnchor {
    // x 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub x: f32,
    // y 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub y: f32,
}

/// 处理 hover text trigger 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn hover_text_trigger<'a>(
    trigger: impl Into<Element<'a, Message>>,
    // label 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    label: impl Into<String>,
    // on_enter 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_enter: impl Fn(Option<ModelPopoverHover>) -> Message + 'a,
    // on_exit 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_exit: Message,
) -> Element<'a, Message> {
    let label = label.into();

    MouseArea::new(trigger)
        .on_enter(on_enter(Some(ModelPopoverHover::Text { text: label.clone(), anchor: None })))
        .on_move(move |position| {
            on_enter(Some(ModelPopoverHover::Text {
                // text 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                text: label.clone(),
                // anchor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                anchor: Some(HoverAnchor { x: position.x, y: position.y }),
            }))
        })
        .on_exit(on_exit)
        .into()
}

/// 处理 hover model trigger 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn hover_model_trigger<'a>(
    trigger: impl Into<Element<'a, Message>>,
    // provider_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    provider_id: &str,
    // model_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    model_id: &str,
    // on_enter 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_enter: impl Fn(Option<ModelPopoverHover>) -> Message + 'a,
    // on_exit 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_exit: Message,
) -> Element<'a, Message> {
    let provider_id = provider_id.to_string();
    let model_id = model_id.to_string();
    let hover = ModelPopoverHover::Model {
        // provider_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        provider_id: provider_id.clone(),
        // model_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        model_id: model_id.clone(),
        // anchor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        anchor: None,
    };

    MouseArea::new(trigger)
        .on_enter(on_enter(Some(hover)))
        .on_move(move |position| {
            on_enter(Some(ModelPopoverHover::Model {
                // provider_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                provider_id: provider_id.to_string(),
                // model_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                model_id: model_id.to_string(),
                // anchor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                anchor: Some(HoverAnchor { x: position.x, y: position.y }),
            }))
        })
        .on_exit(on_exit)
        .into()
}

/// 构建或定位 hover tooltip overlay，用于把浮层稳定附着到目标控件。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn hover_tooltip_overlay<'a>(
    app: &'a App,
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: impl Into<Element<'a, Message>>,
    // tooltip_style 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tooltip_style: fn(&Theme) -> iced::widget::container::Style,
) -> Element<'a, Message> {
    let content = content.into();
    let anchor = hover_anchor(&app.model_popover_hover);
    let tooltip = build_hover_tip(app, tooltip_style);
    let show = tooltip.is_some() && anchor.is_some();
    let tooltip = tooltip.unwrap_or_else(|| Space::new().into());
    let anchor = anchor.unwrap_or(HoverAnchor { x: 0.0, y: 0.0 });

    // 始终保留同一种根组件，只切换浮层可见性。若根据 hover 状态替换根组件，
    // Iced 会重建其下的模型列表，导致滚动状态在 tooltip 更新时回到顶部。
    PointLeftOverlay::new(content, tooltip)
        .show(show)
        .anchor(Point::new(anchor.x, anchor.y))
        .gap(12.0)
        .snap_within_viewport(true)
        .capture_events(false)
        .into()
}

/// 处理 hover anchor 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn hover_anchor(hover: &Option<ModelPopoverHover>) -> Option<HoverAnchor> {
    match hover {
        Some(ModelPopoverHover::Model { anchor, .. }) => *anchor,
        Some(ModelPopoverHover::Text { anchor, .. }) => *anchor,
        None => None,
    }
}

/// 构建 hover tip 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn build_hover_tip<'a>(
    app: &'a App,
    // tooltip_style 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tooltip_style: fn(&Theme) -> iced::widget::container::Style,
) -> Option<Element<'a, Message>> {
    match &app.model_popover_hover {
        Some(ModelPopoverHover::Text { text: label, .. }) => {
            Some(tooltip_text(label, tooltip_style))
        }
        Some(ModelPopoverHover::Model { provider_id, model_id, .. }) => {
            let provider = app.model_settings.providers.iter().find(|p| &p.id == provider_id);

            if let Some(provider) = provider
                && let Some(model) = provider.models.iter().find(|m| &m.id == model_id)
            {
                let meta = format!(
                    "工具 {} · 附件 {} · 上下文限制 {}",
                    if model.toolcall { "✓" } else { "✕" },
                    if model.attachment { "✓" } else { "✕" },
                    model.context_limit
                );

                Some(
                    container(
                        column![
                            text(model.name.clone()).size(12).style(|_t: &Theme| {
                                // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                                iced::widget::text::Style { color: Some(Color::WHITE) }
                            }),
                            text(format!("{} / {}", provider.name, provider.id))
                                .size(11)
                                .wrapping(iced::widget::text::Wrapping::Word)
                                .style(|_t: &Theme| iced::widget::text::Style {
                                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                                    color: Some(Color::WHITE.scale_alpha(0.72)),
                                }),
                            text(meta).size(11).wrapping(iced::widget::text::Wrapping::Word).style(
                                |_t: &Theme| iced::widget::text::Style {
                                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                                    color: Some(Color::from_rgb8(255, 225, 80)),
                                }
                            ),
                        ]
                        .spacing(2)
                        .max_width(MODEL_HOVER_TOOLTIP_WIDTH),
                    )
                    .style(tooltip_style)
                    .padding([6, 8])
                    .width(Length::Fixed(MODEL_HOVER_TOOLTIP_WIDTH))
                    .into(),
                )
            } else {
                None
            }
        }
        None => None,
    }
}

/// 处理 tooltip text 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn tooltip_text<'a>(
    label: &'a str,
    // tooltip_style 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tooltip_style: fn(&Theme) -> iced::widget::container::Style,
) -> Element<'a, Message> {
    container(
        text(label)
            .size(12)
            .wrapping(iced::widget::text::Wrapping::Word)
            .style(|_t: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(tooltip_style)
    .padding([6, 8])
    .width(Length::Fixed(MODEL_HOVER_TOOLTIP_WIDTH))
    .into()
}
