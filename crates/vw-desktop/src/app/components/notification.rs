//! 应用通知视图。
//!
//! 本模块把通知队列转换为可复制、可删除的紧凑通知列表。

use crate::app::assets::{self, Icon};
/// 重新导出 use crate::app::components::system_settings_common::{，让上层模块通过稳定路径访问。
use crate::app::components::system_settings_common::{
    round_icon_btn_style, rounded_action_btn_style, settings_panel_style,
};
/// 重新导出 use crate::app::message::NotificationMessage，让上层模块通过稳定路径访问。
use crate::app::message::NotificationMessage;
/// 重新导出 use crate::app::{App, Message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message};
/// 重新导出 use iced::widget::svg::{self, Svg}，让上层模块通过稳定路径访问。
use iced::widget::svg::{self, Svg};
/// 重新导出 use iced::widget::{，让上层模块通过稳定路径访问。
use iced::widget::{
    Space, button, column, container, mouse_area, opaque, row, scrollable, text, text_editor,
};
/// 重新导出 use iced::{Alignment, Background, Border, Color, Element, Length, Theme, Vector}，让上层模块通过稳定路径访问。
use iced::{Alignment, Background, Border, Color, Element, Length, Theme, Vector};

/// 处理 is dark theme 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

/// 根据主题与状态计算 panel style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn panel_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    let is_dark = is_dark_theme(theme);
    style.border.radius = 22.0.into();
    style.background = Some(Background::Color(if is_dark {
        theme.extended_palette().background.base.color.scale_alpha(0.96)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::WHITE.scale_alpha(0.95)
    }));
    style.shadow = iced::Shadow {
        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        color: Color::BLACK.scale_alpha(if is_dark { 0.16 } else { 0.08 }),
        // offset 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        offset: Vector::new(0.0, 16.0),
        // blur_radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        blur_radius: 28.0,
    };
    style
}

/// 根据主题与状态计算 accent badge style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn accent_badge_style(theme: &Theme, accent: Color) -> iced::widget::container::Style {
    // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(if is_dark_theme(theme) {
            accent.scale_alpha(0.18)
        } else {
            accent.scale_alpha(0.12)
        })),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: accent.scale_alpha(if is_dark_theme(theme) { 0.34 } else { 0.18 }),
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

/// 根据主题与状态计算 item style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn item_style(theme: &Theme, accent: Color) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(if is_dark {
            palette.background.weak.color.scale_alpha(0.26)
        } else {
            // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Color::WHITE.scale_alpha(0.82)
        })),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: if is_dark {
                accent.scale_alpha(0.22)
            } else {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgba8(15, 23, 42, 0.06)
            },
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 16.0.into(),
        },
        // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shadow: iced::Shadow {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Color::BLACK.scale_alpha(if is_dark { 0.07 } else { 0.025 }),
            // offset 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            offset: Vector::new(0.0, 8.0),
            // blur_radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

/// 根据主题与状态计算 compact action btn style。
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
pub(super) fn compact_action_btn_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border.radius = 999.0.into();
    style
}

/// 根据主题与状态计算 copy action btn style。
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
pub(super) fn copy_action_btn_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
    // copied 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    copied: bool,
) -> iced::widget::button::Style {
    let mut style = compact_action_btn_style(theme, status);
    if copied {
        let success = theme.extended_palette().success.base.color;
        style.text_color = success;
        style.border.color = success.scale_alpha(if is_dark_theme(theme) { 0.42 } else { 0.24 });
    }
    style
}

/// 根据主题与状态计算 delete action btn style。
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
pub(super) fn delete_action_btn_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = compact_action_btn_style(theme, status);
    let danger = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xF5, 0xA3, 0xA3, 0.96)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xB4, 0x23, 0x18, 0.94)
    };
    style.text_color = danger;
    style.border.color = danger.scale_alpha(if is_dark_theme(theme) { 0.32 } else { 0.18 });
    style
}

pub(super) fn notification_copy_label(copied: bool) -> &'static str {
    if copied { "✓" } else { "复制" }
}

pub(super) fn notification_text_editor_style(theme: &Theme) -> iced::widget::text_editor::Style {
    let value = theme.palette().text;
    iced::widget::text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        value,
        selection: if is_dark_theme(theme) {
            Color::from_rgba8(0x8B, 0x93, 0x9C, 0.34)
        } else {
            Color::from_rgba8(0xD9, 0xDE, 0xE5, 0.92)
        },
        placeholder: value.scale_alpha(0.7),
    }
}

/// 处理 view 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn view(app: &App) -> Element<'_, Message> {
    /// 处理 icon svg 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn icon_svg(icon: Icon, size: f32) -> Svg<'static> {
        // Svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Svg::new(assets::get_icon(icon))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(|theme: &Theme, _status| svg::Style { color: Some(theme.palette().text) })
    }

    let accent = Color::from_rgb8(0x4D, 0x7C, 0xD6);
    let total_notifications = app.notifications.len();

    let list: Element<'_, Message> = if app.notifications.is_empty() {
        container(
            column![
                container(icon_svg(Icon::Journals, 16.0).style(move |_theme: &Theme, _status| {
                    // svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    svg::Style { color: Some(accent) }
                }),)
                .width(Length::Fixed(38.0))
                .height(Length::Fixed(38.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(move |theme: &Theme| accent_badge_style(theme, accent)),
                text("暂无通知").size(14).style(|theme: &Theme| iced::widget::text::Style {
                    // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    color: Some(if is_dark_theme(theme) {
                        theme.extended_palette().background.base.text.scale_alpha(0.92)
                    } else {
                        theme.palette().text.scale_alpha(0.84)
                    }),
                }),
                text("新的系统消息会在这里汇总展示。").size(12).style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    iced::widget::text::Style {
                        color: Some(p.background.weak.text.scale_alpha(0.72)),
                    }
                }),
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .padding([28, 20])
        .into()
    } else {
        column(app.notifications.iter().rev().map(|n| {
            let time_str = {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let duration =
                        n.created_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    let dt = time::OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64)
                        .unwrap();
                    let fmt = time::format_description::parse("[hour]:[minute]:[second]").unwrap();
                    dt.format(&fmt).unwrap_or_default()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    "Just now".to_string()
                }
            };

            let notification_id = n.id;
            let msg_element: Element<'_, Message> =
                if let Some(editor_content) = app.notification_editors.get(&n.id) {
                    container(
                        text_editor(editor_content)
                            .on_action(move |a| {
                                // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                                Message::Notification(NotificationMessage::EditorAction(
                                    notification_id,
                                    a,
                                ))
                            })
                            .size(13)
                            .padding(0)
                            .height(Length::Shrink)
                            .style(|theme: &Theme, _status| notification_text_editor_style(theme)),
                    )
                    .width(Length::Fill)
                    .into()
                } else {
                    text(&n.message)
                        .size(13)
                        .width(Length::Fill)
                        .style(|theme: &Theme| iced::widget::text::Style {
                            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                            color: Some(if is_dark_theme(theme) {
                                theme.extended_palette().background.base.text.scale_alpha(0.92)
                            } else {
                                theme.palette().text.scale_alpha(0.86)
                            }),
                        })
                        .into()
                };

            let copied = app.copied_notification_id == Some(n.id);
            let copy_label = notification_copy_label(copied);

            container(
                column![
                    row![
                        text(time_str).size(10).width(Length::Fill).style(|theme: &Theme| {
                            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                            iced::widget::text::Style {
                                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                                color: Some(
                                    theme.extended_palette().background.weak.text.scale_alpha(0.72),
                                ),
                            }
                        }),
                        button(text(copy_label).size(10))
                            .on_press(Message::Notification(NotificationMessage::Copy(n.id)))
                            .padding([3, 8])
                            .style(move |theme: &Theme, status| {
                                copy_action_btn_style(theme, status, copied)
                            }),
                        button(text("删除").size(10))
                            .on_press(Message::Notification(NotificationMessage::Remove(n.id)))
                            .padding([3, 8])
                            .style(delete_action_btn_style)
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    msg_element
                ]
                .spacing(8),
            )
            .padding([11, 12])
            .style(move |theme: &Theme| item_style(theme, accent))
            .into()
        }))
        .spacing(8)
        .into()
    };

    let count_badge: Element<'_, Message> = if total_notifications > 0 {
        container(
            text(total_notifications.to_string())
                .size(11)
                .style(move |_theme: &Theme| iced::widget::text::Style { color: Some(accent) }),
        )
        .padding([4, 10])
        .style(move |theme: &Theme| accent_badge_style(theme, accent))
        .into()
    } else {
        // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Space::new().width(Length::Fixed(0.0)).height(Length::Fixed(0.0)).into()
    };

    let content = column![
        row![
            row![
                text("通知").size(14).style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    iced::widget::text::Style { color: Some(p.background.strong.text) }
                }),
                count_badge
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill),
            button(text("清空").size(12))
                .on_press_maybe(
                    (total_notifications > 0)
                        .then_some(Message::Notification(NotificationMessage::ClearAll)),
                )
                .padding([6, 10])
                .style(rounded_action_btn_style),
            button(icon_svg(Icon::X, 12.0))
                .on_press(Message::Notification(NotificationMessage::ToggleExpanded))
                .padding(6)
                .style(round_icon_btn_style)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        scrollable(
            container(list)
                .width(Length::Fill)
                .padding(iced::Padding::default().left(3.0).right(3.0)),
        )
        .id(app.notifications_scroll_id.clone())
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fixed(312.0))
    ]
    .spacing(10);

    let panel = container(content).width(Length::Fixed(368.0)).padding(14).style(panel_style);

    opaque(mouse_area(panel))
}
