//! 阅读列、虚拟窗口切片与问题导航。桌面注入跨域内容。
use iced::widget::{Space, button, column, container, text, svg, row};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::{Background, Border, Color, Element, Length, Padding, Theme};
use desktop_assets::assets::Icon;
use crate::utils::theme::icon_svg;
use crate::height_index::CHAT_MESSAGE_GAP;
#[cfg(test)]
use crate::virtualizer::ChatVirtualizer;
use session_ui_types::{ChatMessage, ChatRole};
use desktop_session::session::active::UserQuestionMarker;
#[derive(Debug, Clone)]
pub enum Message { ScrollToBottom, LocateChatMessageIndex(usize) }
const MAX_USER_QUESTION_MARKERS: usize = 40;
pub const CONVERSATION_READING_COLUMN_MAX_WIDTH: f32 = 880.0;

pub fn chat_message_column<'a, Message: Clone + 'static>() -> iced::widget::Column<'a, Message> {
    column![]
        .spacing(CHAT_MESSAGE_GAP)
        .width(Length::Fill)
        .max_width(CONVERSATION_READING_COLUMN_MAX_WIDTH)
}

// Spacers must not participate in the message column's inter-message spacing.
pub fn chat_virtual_content<'a, Message: Clone + 'static>(
    messages: iced::widget::Column<'a, Message>,
    top: f32,
    bottom: f32,
) -> iced::widget::Column<'a, Message> {
    column![Space::new().height(top), messages, Space::new().height(bottom)].width(Length::Fill)
}

pub fn conversation_reading_column<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    padding: Padding,
) -> iced::widget::Container<'a, Message> {
    container(
        container(content).width(Length::Fill).max_width(CONVERSATION_READING_COLUMN_MAX_WIDTH),
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
    .padding(padding)
}


#[cfg(test)]
pub(crate) fn compute_visible_message_window_for_chat(
    chat_len: usize,
    message_heights: &[f32],
    scroll_offset_y: f32,
    viewport_h: f32,
) -> (usize, usize) {
    let clamped_len = chat_len.min(message_heights.len());
    if clamped_len == 0 {
        return (0, 0);
    }

    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&message_heights[..clamped_len]);
    virtualizer.set_viewport_height(viewport_h);
    virtualizer.set_relative_y(scroll_offset_y);
    let window = virtualizer.compute_window(0.0);
    (window.visible_start_idx, window.visible_end_idx)
}


pub fn chat_render_window<T>(
    chat: &[T],
    start_idx: usize,
    end_idx: usize,
) -> impl ExactSizeIterator<Item = (usize, &T)> {
    let start_idx = start_idx.min(chat.len());
    let end_idx = end_idx.max(start_idx).min(chat.len());
    chat[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(move |(offset, message)| (start_idx + offset, message))
}


pub fn user_question_indices(chat: &[ChatMessage]) -> Vec<usize> {
    chat.iter()
        .enumerate()
        .filter_map(|(idx, message)| (message.role == ChatRole::User).then_some(idx))
        .collect()
}

#[cfg(test)]
pub(crate) fn is_chat_message_idx_visible(
    target_idx: usize,
    visible_start_idx: usize,
    visible_end_idx: usize,
    viewport_h: f32,
) -> bool {
    if viewport_h <= 0.0 || visible_end_idx <= visible_start_idx {
        return false;
    }

    (visible_start_idx..visible_end_idx).contains(&target_idx)
}

#[cfg(test)]
pub(crate) fn active_user_question_idx(
    user_question_idxs: &[usize],
    visible_start_idx: usize,
    visible_end_idx: usize,
    viewport_h: f32,
) -> Option<usize> {
    if viewport_h <= 0.0 || visible_end_idx <= visible_start_idx {
        return None;
    }

    let position = user_question_idxs.partition_point(|&idx| idx < visible_start_idx);
    user_question_idxs.get(position).copied().filter(|&idx| idx < visible_end_idx)
}

pub fn active_user_question_marker_position(
    user_question_markers: &[UserQuestionMarker],
    visible_start_idx: usize,
    visible_end_idx: usize,
    viewport_h: f32,
) -> Option<usize> {
    if viewport_h <= 0.0 || visible_end_idx <= visible_start_idx {
        return None;
    }

    let position =
        user_question_markers.partition_point(|marker| marker.message_idx < visible_start_idx);
    user_question_markers
        .get(position)
        .filter(|marker| marker.message_idx < visible_end_idx)
        .map(|_| position)
}

pub fn project_user_question_marker_positions(
    marker_len: usize,
    active_position: Option<usize>,
) -> Vec<usize> {
    if marker_len <= MAX_USER_QUESTION_MARKERS {
        return (0..marker_len).collect();
    }

    let mut positions = Vec::with_capacity(MAX_USER_QUESTION_MARKERS);
    push_projected_question_marker_position(&mut positions, marker_len, 0);
    push_projected_question_marker_position(&mut positions, marker_len, marker_len - 1);
    if let Some(active_position) = active_position {
        push_projected_question_marker_position(&mut positions, marker_len, active_position);
        push_projected_question_marker_position(
            &mut positions,
            marker_len,
            active_position.saturating_sub(1),
        );
        push_projected_question_marker_position(
            &mut positions,
            marker_len,
            active_position.saturating_add(1),
        );
    }

    let sample_slots = MAX_USER_QUESTION_MARKERS - positions.len();
    for slot in 0..sample_slots {
        push_projected_question_marker_position(
            &mut positions,
            marker_len,
            slot * (marker_len - 1) / (sample_slots - 1),
        );
    }
    positions.sort_unstable();
    positions
}

fn push_projected_question_marker_position(
    positions: &mut Vec<usize>,
    marker_len: usize,
    position: usize,
) {
    if position < marker_len
        && !positions.contains(&position)
        && positions.len() < MAX_USER_QUESTION_MARKERS
    {
        positions.push(position);
    }
}

#[cfg(test)]
pub(crate) use desktop_session::session::active::user_question_preview;

pub fn live_message_meta_fallback(
    i18n: desktop_i18n::I18n,
    role: ChatRole,
    is_requesting: bool,
    is_last: bool,
    model: &str,
) -> Option<String> {
    (is_requesting && is_last && matches!(role, ChatRole::Assistant | ChatRole::User))
        .then(|| i18n.format("%s · 刚刚", &[model.into()]).expect("valid live metadata template"))
}

fn user_question_tooltip_content<Message: 'static>(label: String) -> Element<'static, Message> {
    container(text(label).size(11))
        .padding([4, 8])
        .style(|_theme: &Theme| iced::widget::container::Style {
            text_color: Some(Color::from_rgb8(0xF3, 0xF4, 0xF6)),
            background: Some(Background::Color(Color::from_rgb8(0x0B, 0x0B, 0x0B))),
            border: Border {
                width: 1.0,
                color: Color::from_rgb8(0x14, 0x14, 0x14),
                radius: 6.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.25),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            },
            ..Default::default()
        })
        .into()
}


pub fn rough_message_heights(chat: &[ChatMessage]) -> Vec<f32> {
    chat.iter().map(|message| crate::message_view::text::estimate_message_height_rough(&message.content)).collect()
}
pub fn jump_to_bottom_button<'a>(auto_scroll: bool, viewport_h: f32) -> Element<'a, Message> {
    // 如果处于自动滚动模式或视口未初始化，不显示按钮
    if auto_scroll || viewport_h == 0.0 {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    // 创建向下箭头图标，使用主题文字颜色
    let chevron = icon_svg(Icon::ChevronDown)
        .style(|theme: &Theme, _status| svg::Style { color: Some(theme.palette().text) });

    // 创建圆形气泡容器，包含箭头图标
    let bubble = container(chevron)
        .width(Length::Fixed(30.0))
        .height(Length::Fixed(30.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|theme: &Theme| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let bg = if is_dark {
                Color::from_rgba8(24, 25, 29, 0.96)
            } else {
                Color::from_rgba8(252, 252, 253, 1.0)
            };
            let border = if is_dark {
                Color::from_rgba8(45, 48, 54, 0.94)
            } else {
                Color::from_rgba8(226, 231, 237, 1.0)
            };

            iced::widget::container::Style {
                background: Some(Background::Color(bg)),
                border: Border { width: 1.0, color: border, radius: 999.0.into() },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.08 }),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 18.0,
                },
                ..Default::default()
            }
        });

    let button = button(bubble)
        .padding(0)
        .width(Length::Fixed(30.0))
        .height(Length::Fixed(30.0))
        .style(iced::widget::button::text)
        .on_press(Message::ScrollToBottom);

    // 最终容器：底部居中定位，底部留 12px 间距
    container(button)
        .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 18.0, left: 0.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Bottom)
        .into()
}

pub fn user_question_nav_overlay<'a>(
    i18n: desktop_i18n::I18n,
    viewport_h: f32,
    user_question_markers: &[UserQuestionMarker],
    visible_start_idx: usize,
    visible_end_idx: usize,
) -> Element<'a, Message> {
    if user_question_markers.is_empty() || viewport_h <= 0.0 {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let mut dots = column![].spacing(5).align_x(iced::alignment::Horizontal::Center);
    let active_question_position = active_user_question_marker_position(
        user_question_markers,
        visible_start_idx,
        visible_end_idx,
        viewport_h,
    );

    for marker_position in project_user_question_marker_positions(
        user_question_markers.len(),
        active_question_position,
    ) {
        let marker = &user_question_markers[marker_position];
        let is_active = active_question_position == Some(marker_position);
        let tooltip_label = i18n
            .format(
                "问题 %u: %s",
                &[
                    (marker_position + 1).into(),
                    (if marker.preview.is_empty() {
                        i18n.__("空提问")
                    } else {
                        marker.preview.as_str()
                    })
                    .into(),
                ],
            )
            .expect("valid question marker template");

        let dot = container(Space::new().width(Length::Shrink).height(Length::Shrink))
            .width(Length::Fixed(if is_active { 7.0 } else { 5.0 }))
            .height(Length::Fixed(if is_active { 7.0 } else { 5.0 }))
            .style(move |theme: &Theme| {
                let primary = theme.extended_palette().primary.base.color;
                let muted = if theme.palette().background.r
                    + theme.palette().background.g
                    + theme.palette().background.b
                    < 1.5
                {
                    Color::from_rgba8(92, 97, 108, 0.92)
                } else {
                    Color::from_rgba8(162, 168, 178, 0.96)
                };
                iced::widget::container::Style {
                    background: Some(Background::Color(if is_active {
                        primary.scale_alpha(0.96)
                    } else {
                        muted
                    })),
                    border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                    ..Default::default()
                }
            });

        let bubble = container(dot)
            .width(Length::Fixed(15.0))
            .height(Length::Fixed(15.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center);

        let button = button(bubble)
            .padding(0)
            .width(Length::Fixed(15.0))
            .height(Length::Fixed(15.0))
            .style(iced::widget::button::text)
            .on_press(Message::LocateChatMessageIndex(marker.message_idx));

        let button_with_tooltip: Element<'a, Message> = Tooltip::new(
            button,
            user_question_tooltip_content(tooltip_label),
            TooltipPosition::Left,
        )
        .gap(6)
        .into();

        dots = dots.push(button_with_tooltip);
    }

    let rail = container(dots).padding([8, 4]).style(|theme: &Theme| {
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        let bg = if is_dark {
            Color::from_rgba8(20, 22, 27, 0.84)
        } else {
            Color::from_rgba8(244, 246, 249, 0.92)
        };
        let border = if is_dark {
            Color::from_rgba8(42, 45, 52, 0.92)
        } else {
            Color::from_rgba8(225, 230, 237, 1.0)
        };

        iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            border: Border { width: 1.0, color: border, radius: 14.0.into() },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.08 }),
                offset: iced::Vector::new(0.0, 8.0),
                blur_radius: 18.0,
            },
            ..Default::default()
        }
    });

    container(rail)
        .padding(iced::Padding { top: 0.0, right: 7.0, bottom: 0.0, left: 0.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}
pub fn empty_fullscreen_controls<'a, Message: Clone + 'static>(i18n: desktop_i18n::I18n, fullscreen: bool, half_fullscreen: bool, show_half: bool, toggle_full: Message, toggle_half: Message) -> Element<'a, Message> {
    let fullscreen_icon =
        if fullscreen { Icon::FullscreenExit } else { Icon::Fullscreen };
    let fullscreen_label = if fullscreen {
        i18n.__("退出全屏")
    } else {
        i18n.__("全屏")
    };

    let mut controls = row![].spacing(6).align_y(iced::Alignment::Center);
    if show_half {
        controls = controls.push(crate::header::header_icon_button(
            Icon::LayoutTextWindow,
            if half_fullscreen {
                i18n.__("退出半屏")
            } else {
                i18n.__("半屏")
            },
            toggle_half,
        ));
    }
    let controls = controls.push(crate::header::header_icon_button(
        fullscreen_icon,
        fullscreen_label,
        toggle_full,
    ));

    container(controls)
        .width(Length::Fill)
        .height(Length::Shrink)
        .align_x(iced::alignment::Horizontal::Right)
        .padding(iced::Padding { top: 16.0, right: 22.0, bottom: 0.0, left: 0.0 })
        .into()
}


/// 固定页头与状态栏，正文由宿主选择会话或轨迹内容。
pub fn started_view<'a, Message: Clone + 'static>(header: Element<'a, Message>, body: Element<'a, Message>, status: Element<'a, Message>) -> iced::widget::Column<'a, Message> {
    column![header, body, status].spacing(6).width(Length::Fill).height(Length::Fill)
}
/// 空会话和准备中的会话共享阅读列与全屏控件布局。
pub fn placeholder_view<'a, Message: Clone + 'static>(content: Element<'a, Message>, controls: Element<'a, Message>, status: Element<'a, Message>) -> Element<'a, Message> {
    iced::widget::column![
        iced::widget::stack![
            conversation_reading_column(content, Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 }).height(Length::Fill), controls
        ].width(Length::Fill).height(Length::Fill), status
    ].width(Length::Fill).height(Length::Fill).into()
}
pub fn todo_overlay<'a, Message: Clone + 'static>(panel: Element<'a, Message>) -> Element<'a, Message> {
    container(iced::widget::opaque(panel))
        .padding(Padding { top: 14.0, right: 22.0, bottom: 0.0, left: 0.0 })
        .width(Length::Fill).height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Top).into()
}

/// A borrowed conversation window; the host supplies only the message renderer.
pub struct ConversationInput<'a> {
    pub chat: &'a [ChatMessage],
    pub cache: &'a std::collections::HashMap<usize, crate::render_cache::ChatRenderCacheEntry>,
    pub metadata: &'a [Option<desktop_i18n::chat::MessageMeta>],
    pub timeline: Option<&'a desktop_session::session::chat_timeline::PagedChatTimeline>,
    pub window: crate::virtualizer::ChatVirtualWindow,
    pub scroll_id: iced::widget::Id,
    pub i18n: desktop_i18n::I18n,
    pub requesting: bool,
    pub model: &'a str,
    pub width: f32,
    pub dark: bool,
    pub reasoning_summary: bool,
}

pub fn conversation_window<'a, M: Clone + 'static>(
    input: ConversationInput<'a>,
    mut render: impl FnMut(
        usize,
        &'a ChatMessage,
        Option<std::borrow::Cow<'a, str>>,
        &'a crate::render_cache::ChatRenderCacheEntry,
        bool,
        Option<&crate::render_cache::RevisionRenderKey>,
    ) -> Element<'a, M>,
    scroll: impl Fn(iced::widget::scrollable::Viewport) -> M + 'a,
) -> Element<'a, M> {
    static FALLBACK: once_cell::sync::Lazy<crate::render_cache::ChatRenderCacheEntry> =
        once_cell::sync::Lazy::new(Default::default);
    let window = input.window;
    #[cfg(test)]
    crate::performance::record_mounted_rows(
        window.render_end_idx.saturating_sub(window.render_start_idx),
    );
    let mut messages = chat_message_column();
    for (i, message) in
        chat_render_window(input.chat, window.render_start_idx, window.render_end_idx)
    {
        let target = input.timeline.and_then(|timeline| timeline.target_at(i));
        let revision = target.as_ref().map(|target| {
            crate::render_cache::RevisionRenderKey::new(
                target.key.clone(),
                target.revision,
                input.width,
                input.dark,
                input.reasoning_summary,
            )
        });
        let meta = input
            .metadata
            .get(i)
            .and_then(Option::as_ref)
            .map(|meta| meta.render(input.i18n))
            .or_else(|| {
                live_message_meta_fallback(
                    input.i18n,
                    message.role,
                    input.requesting,
                    i + 1 == input.chat.len(),
                    input.model,
                )
            })
            .map(std::borrow::Cow::Owned);
        let content = render(
            i,
            message,
            meta,
            input.cache.get(&i).unwrap_or(&FALLBACK),
            i >= window.visible_start_idx && i < window.visible_end_idx,
            revision.as_ref(),
        );
        messages = messages
            .push(crate::message_measure::MeasuredChatMessage::new(i, content).with_target(target));
    }
    iced::widget::scrollable(conversation_reading_column(
        chat_virtual_content(messages, window.top_spacer_h, window.bottom_spacer_h),
        Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
    ))
    .direction(crate::utils::theme::chat_scroll_direction())
    .id(input.scroll_id)
    .on_scroll(scroll)
    .into()
}

pub fn conversation_body<'a, M: 'a>(
    content: Element<'a, M>,
    questions: Element<'a, M>,
    jump: Element<'a, M>,
    todo: Element<'a, M>,
) -> Element<'a, M> {
    iced::widget::stack![content, questions, jump, todo]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// A single context overlay owns closing and anchoring for the conversation.
pub fn context_menu_overlay<'a, M: Clone + 'static>(
    content: impl Into<Element<'a, M>>,
    menu: Element<'a, M>,
    open: bool,
    anchor: Option<(f32, f32)>,
    close: M,
) -> Element<'a, M> {
    let anchor = anchor.unwrap_or((12.0, 26.0));
    desktop_ui_controls::overlays::PointBelowOverlay::new(content, menu)
        .show(open)
        .global_anchor(iced::Point::new(anchor.0, anchor.1))
        .gap(0.0)
        .close_on_target_click(true)
        .on_close(close)
        .into()
}

/// Todo placement belongs to the conversation surface; the host supplies its view.
pub fn todo_is_visible(
    top_right: bool,
    todo_session: Option<&str>,
    active_session: Option<&str>,
    item_count: usize,
) -> bool {
    top_right && todo_session == active_session && item_count > 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPhase {
    Loading,
    Empty,
    Started,
}
pub fn panel_phase(
    active: bool,
    preparing: bool,
    base_ready: bool,
    message_count: usize,
) -> PanelPhase {
    if active && preparing && !base_ready {
        PanelPhase::Loading
    } else if active && message_count > 0 {
        PanelPhase::Started
    } else {
        PanelPhase::Empty
    }
}
pub fn status_reading_column<'a, M: Clone + 'static>(
    recovery: Element<'a, M>,
    status: Element<'a, M>,
) -> Element<'a, M> {
    conversation_reading_column(
        column![recovery, status].spacing(6),
        Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
    )
    .into()
}
