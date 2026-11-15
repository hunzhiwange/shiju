//! 应用通知视图。
//!
//! 本模块把通知队列转换为可复制、可删除的紧凑通知列表。

use desktop_assets::assets::{self, Icon};
/// 重新导出 use desktop_ui_controls::system_settings_common::{，让上层模块通过稳定路径访问。
use desktop_ui_controls::system_settings_common::{
    round_icon_btn_style, rounded_action_btn_style,
};
#[derive(Debug, Clone)]
pub enum NotificationMessage { EditorAction(usize, iced::widget::text_editor::Action), Copy(usize), Remove(usize), ClearAll, ToggleExpanded }

pub struct Input<'a> {
    pub desktop_locale: config_ui::DesktopLocale,
    pub notifications: &'a [Notification],
    pub notification_editors: &'a std::collections::HashMap<usize, iced::widget::text_editor::Content>,
    pub copied_notification_id: Option<usize>,
    pub notifications_scroll_id: iced::widget::Id,
}
#[derive(Debug, Clone)]
pub enum Message { Notification(NotificationMessage) }

/// 重新导出 use iced::widget::svg::{self, Svg}，让上层模块通过稳定路径访问。
use iced::widget::svg::{self, Svg};
/// 重新导出 use iced::widget::{，让上层模块通过稳定路径访问。
use iced::widget::{
    Space, button, column, container, mouse_area, opaque, row, scrollable, text, text_editor,
};
/// 重新导出 use iced::{Alignment, Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Alignment, Color, Element, Length, Theme};

pub(crate) use super::notification_styles::*;

pub fn view<'a>(app: Input<'a>) -> Element<'a, Message> {
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
                text(desktop_i18n::I18n::new(app.desktop_locale).__("暂无通知"))
                    .size(14)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        color: Some(if is_dark_theme(theme) {
                            theme.extended_palette().background.base.text.scale_alpha(0.92)
                        } else {
                            theme.palette().text.scale_alpha(0.84)
                        }),
                    }),
                text(
                    desktop_i18n::I18n::new(app.desktop_locale)
                        .__("新的系统消息会在这里汇总展示。")
                )
                .size(12)
                .style(|theme: &Theme| {
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
                    let fmt =
                        time::format_description::parse_borrowed::<2>("[hour]:[minute]:[second]")
                            .unwrap();
                    dt.format(&fmt).unwrap_or_default()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    desktop_i18n::I18n::new(app.desktop_locale).__("刚刚").to_owned()
                }
            };

            let notification_id = n.id;
            let msg_element: Element<'_, Message> = if let Some(editor_content) =
                app.notification_editors.get(&n.id).filter(|_| n.localized_message.is_none())
            {
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
                text(n.render(desktop_i18n::I18n::new(app.desktop_locale)))
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
                        button(
                            text(desktop_i18n::I18n::new(app.desktop_locale).__(copy_label))
                                .size(10)
                        )
                        .on_press(Message::Notification(NotificationMessage::Copy(n.id)))
                        .padding([3, 8])
                        .style(move |theme: &Theme, status| {
                            copy_action_btn_style(theme, status, copied)
                        }),
                        button(
                            text(desktop_i18n::I18n::new(app.desktop_locale).__("删除"))
                                .size(10)
                        )
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
                text(desktop_i18n::I18n::new(app.desktop_locale).__("通知")).size(14).style(
                    |theme: &Theme| {
                        let p = theme.extended_palette();
                        iced::widget::text::Style { color: Some(p.background.strong.text) }
                    }
                ),
                count_badge
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill),
            button(text(desktop_i18n::I18n::new(app.desktop_locale).__("清空")).size(12))
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

/// 通知消息
///
/// 表示应用程序中的一个通知消息，
/// 包含消息 ID、内容和创建时间。
#[derive(Debug, Clone)]
pub struct Notification {
    pub localized_message: Option<desktop_i18n::format::Message>,
    /// 通知的唯一标识符
    pub id: usize,
    /// 通知消息内容
    pub message: String,
    /// 通知创建时间
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub created_at: web_time::SystemTime,
}

impl Notification {
    pub fn render(&self, i18n: desktop_i18n::I18n) -> String {
        self.localized_message
            .as_ref()
            .map(|message| message.render(i18n).expect("valid notification template"))
            .unwrap_or_else(|| self.message.clone())
    }
}
