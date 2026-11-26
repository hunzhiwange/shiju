//! Mini task cards and their bounded collapsed stack.
use crate::pet_state::{
    TASK_PET_CARD_HEIGHT as PET_CARD_HEIGHT, TASK_PET_CARD_REPLY_HEIGHT as PET_CARD_REPLY_HEIGHT,
    TASK_PET_CARD_SPACING as PET_CARD_SPACING, TASK_PET_LIST_HEIGHT as PET_LIST_HEIGHT,
    TASK_PET_STACK_DEPTH, TASK_PET_STACK_OFFSET, TaskPetItem, TaskPetStatus,
};
use crate::pet_view::{PetMessage as Message, PetView};
use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::status_animation::spinner_frame;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_text_input_style,
};
use desktop_i18n::I18n;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, svg, text, text_input, tooltip,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

const REMINDER_DOCK_GAP: f32 = 8.0;
const REMOVE_BUTTON_SIZE: f32 = 22.0;
const CARD_ACTION_SIZE: f32 = 36.0;
const CARD_ACTION_GAP: f32 = 6.0;
static REPLY_ARROW: once_cell::sync::Lazy<svg::Handle> =
    once_cell::sync::Lazy::new(|| super::controls::stroke("<path d='M21 5v6a3 3 0 0 1-3 3H4m5-5-5 5 5 5'/>"));
static STOP_ICON: once_cell::sync::Lazy<svg::Handle> = once_cell::sync::Lazy::new(|| {
    svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/square-fill.svg"))
});

#[cfg(test)]
pub(super) const TASK_STATUS_SLOT_WIDTH: f32 = 20.0;
#[cfg(test)]
pub(super) const TASK_STATUS_SLOT_HEIGHT: f32 = 52.0;
pub(super) const TASK_STATUS_ICON_SIZE: f32 = 18.0;
pub(super) const TASK_TITLE_SIZE: u32 = 15;
pub(super) const TASK_DETAIL_SIZE: u32 = 13;

/// Rear layers are paint-only containers; only the front card owns event handlers.
pub(super) fn collapsed_stack<'a>(app: PetView<'a>) -> Element<'a, Message> {
    let Some(front) = app.task_pet_items.first() else {
        return Space::new().height(0).into();
    };
    let depth = app.task_pet_items.len().min(TASK_PET_STACK_DEPTH);
    let card_height = if app.task_pet_reply_request_id == Some(front.request_id) {
        PET_CARD_REPLY_HEIGHT
    } else {
        PET_CARD_HEIGHT
    };
    let height = card_height + (depth - 1) as f32 * TASK_PET_STACK_OFFSET;
    let mut layers = iced::widget::Stack::new();
    for layer in (1..depth).rev() {
        layers = layers.push(
            container(
                container(Space::new())
                    .width(Length::Fill)
                    .height(card_height)
                    .style(task_card_style),
            )
            .padding(
                iced::Padding::default()
                    .top(layer as f32 * TASK_PET_STACK_OFFSET)
                    .left(layer as f32 * TASK_PET_STACK_OFFSET)
                    .right(layer as f32 * TASK_PET_STACK_OFFSET),
            )
            .width(Length::Fill),
        );
    }
    layers = layers.push(task_card(app, front));
    container(layers.width(Length::Fill).height(height))
        .width(Length::Fill)
        .height(height + REMINDER_DOCK_GAP)
        .padding(iced::Padding::default().left(7).right(7).bottom(REMINDER_DOCK_GAP))
        .into()
}

pub fn reminder_height(items: &[TaskPetItem], reply: Option<u64>, expanded: bool) -> f32 {
    let Some(front) = items.first() else {
        return 0.0;
    };
    let reply_extra = PET_CARD_REPLY_HEIGHT - PET_CARD_HEIGHT;
    REMINDER_DOCK_GAP
        + if expanded {
            (items.len() as f32 * (PET_CARD_HEIGHT + PET_CARD_SPACING) - PET_CARD_SPACING
                + if reply.is_some() { reply_extra } else { 0.0 })
            .min(PET_LIST_HEIGHT)
        } else {
            PET_CARD_HEIGHT
                + (items.len().min(TASK_PET_STACK_DEPTH) - 1) as f32 * TASK_PET_STACK_OFFSET
                + if reply == Some(front.request_id) { reply_extra } else { 0.0 }
        }
}

pub(super) fn expanded_list<'a>(app: PetView<'a>, height: f32) -> Element<'a, Message> {
    let cards = app
        .task_pet_items
        .iter()
        .fold(column![].spacing(PET_CARD_SPACING), |list, item| list.push(task_card(app, item)));
    container(
        scrollable(container(cards).padding([0, 7]).width(Length::Fill))
            .id(app.task_pet_scroll_id.clone())
            .height((height - REMINDER_DOCK_GAP).max(0.0))
            .width(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4).spacing(3))),
    )
    .padding(iced::Padding::default().bottom(REMINDER_DOCK_GAP))
    .into()
}

pub(super) fn task_card<'a>(app: PetView<'a>, item: &'a TaskPetItem) -> Element<'a, Message> {
    let status_icon = match item.status {
        TaskPetStatus::Running => running_status_icon(app.status_animation_frame),
        TaskPetStatus::Completed => completed_status_icon(),
    };
    let request_id = item.request_id;
    let hovered = app.task_pet_hovered_request_id == Some(request_id);
    let replying = app.task_pet_reply_request_id == Some(request_id);
    let expanded = !app.render_collapsed;
    let can_stop = item.status == TaskPetStatus::Running;
    let can_remove = expanded || can_stop;
    let can_reply = can_remove || replying;
    let action_count = u8::from(can_reply) + u8::from(can_stop);
    let right_padding = 16.0
        + f32::from(action_count) * CARD_ACTION_SIZE
        + f32::from(action_count.saturating_sub(1)) * CARD_ACTION_GAP;
    let i18n = I18n::new(app.desktop_locale);

    let title = text(truncate_for_card(&item.title, 18))
        .wrapping(iced::widget::text::Wrapping::None)
        .font(iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT })
        .size(TASK_TITLE_SIZE)
        .width(Length::Fill);
    let detail = item
        .detail_feedback
        .as_ref()
        .map(|feedback| feedback.render(i18n, &item.detail))
        .unwrap_or_else(|| item.detail.clone());
    let detail_limit = if action_count > 0 { 16 } else { 20 };
    let detail = detail_line(&detail, detail_limit);
    let body =
        row![status_icon, column![title, detail].spacing(3)].spacing(8).align_y(Alignment::Start);

    let action = if app.render_collapsed {
        Message::TaskPetExpandReminders
    } else {
        Message::TaskPetOpenReminder(request_id)
    };
    let face = button(
        container(body)
            .width(Length::Fill)
            .height(PET_CARD_HEIGHT)
            .padding(
                iced::Padding::default()
                    .top(10)
                    .bottom(10)
                    .left(if can_remove { 26.0 } else { 16.0 })
                    .right(right_padding),
            )
            .clip(true)
            .style(move |theme: &Theme| {
                let mut style = task_card_style(theme);
                if hovered {
                    style.border.color = theme.palette().primary.scale_alpha(0.65);
                }
                style
            }),
    )
    .width(Length::Fill)
    .padding(0)
    .style(|_, _| button::Style::default())
    .on_press(action);
    let mut card = iced::widget::stack![face];
    if can_remove && hovered {
        let remove = icon_action(
            super::controls::vector(assets::get_icon(Icon::X), 11.0, 1.0),
            i18n.__("移除提醒"),
            Message::TaskPetRemove(request_id),
            REMOVE_BUTTON_SIZE,
            tiny_action_button_style,
        );
        card = card.push(container(remove));
    }
    if action_count > 0 && hovered {
        let mut actions = row![].spacing(CARD_ACTION_GAP);
        if can_reply {
            actions = actions.push(reply_button(i18n, request_id, replying));
        }
        if can_stop {
            actions = actions.push(icon_action(
                super::controls::vector((*STOP_ICON).clone(), 14.0, 1.0),
                i18n.__("停止会话"),
                Message::TaskPetStop(request_id),
                CARD_ACTION_SIZE,
                reply_button_style,
            ));
        }
        card = card.push(
            container(actions)
                .width(Length::Fill)
                .height(Length::Fixed(PET_CARD_HEIGHT))
                .align_x(Alignment::End)
                .align_y(Alignment::Center)
                .padding(iced::Padding::default().right(12)),
        );
    }

    let content: Element<'a, Message> = if replying {
        column![
            container(card).height(Length::Fixed(PET_CARD_HEIGHT)),
            reply_input(i18n, app.task_pet_reply_input),
        ]
        .spacing(6)
        .into()
    } else {
        container(card).height(Length::Fixed(PET_CARD_HEIGHT)).into()
    };
    let height = if replying { PET_CARD_REPLY_HEIGHT } else { PET_CARD_HEIGHT };

    let area = mouse_area(container(content).width(Length::Fill).height(Length::Fixed(height)))
        .on_exit(Message::TaskPetHoverExited(request_id));
    if hovered {
        area.into()
    } else {
        area.on_move(move |_| Message::TaskPetHover(Some(request_id))).into()
    }
}

pub(super) fn reply_button(i18n: I18n, request_id: u64, replying: bool) -> Element<'static, Message> {
    let (icon, label, intent) = if replying {
        (assets::get_icon(Icon::X), i18n.__("关闭回复"), Message::TaskPetReplyClose)
    } else {
        ((*REPLY_ARROW).clone(), i18n.__("回复"), Message::TaskPetReplyPressed(request_id))
    };
    icon_action(
        super::controls::vector(icon, 18.0, 1.0),
        label,
        intent,
        CARD_ACTION_SIZE,
        reply_button_style,
    )
}

fn icon_action(
    icon: Element<'static, Message>,
    label: &'static str,
    intent: Message,
    size: f32,
    style: fn(&Theme, button::Status) -> button::Style,
) -> Element<'static, Message> {
    tooltip(
        button(container(icon).center_x(Length::Fill).center_y(Length::Fill))
            .width(size)
            .height(size)
            .padding(0)
            .style(style)
            .on_press(intent),
        text(label).size(11),
        tooltip::Position::Top,
    )
    .style(container::rounded_box)
    .delay(iced::time::Duration::from_millis(450))
    .padding(5)
    .into()
}

pub fn reply_id() -> iced::widget::Id {
    iced::widget::Id::new("task-pet-reply")
}

pub(super) fn reply_input<'a>(i18n: I18n, value: &'a str) -> Element<'a, Message> {
    row![
        text_input(i18n.__("继续说点什么..."), value)
            .id(reply_id())
            .on_input(Message::TaskPetReplyInputChanged)
            .on_submit(Message::TaskPetReplySubmit)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill),
        button(centered_button_label(i18n.__("发送"), 12))
            .width(Length::Fixed(42.0))
            .height(Length::Fixed(30.0))
            .padding(0)
            .style(primary_action_btn_style)
            .on_press(Message::TaskPetReplySubmit),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn centered_button_label(label: &'static str, size: u32) -> Element<'static, Message> {
    container(text(label).size(size))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(test)]
pub(super) fn task_detail_line(detail: &str) -> Element<'static, Message> {
    detail_line(detail, 20)
}

fn detail_line(detail: &str, limit: usize) -> Element<'static, Message> {
    text(truncate_for_card(detail, limit))
        .wrapping(iced::widget::text::Wrapping::None)
        .size(TASK_DETAIL_SIZE)
        .line_height(1.12)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().background.base.text.scale_alpha(0.75)),
        })
        .width(Length::Fill)
        .into()
}

pub(super) fn truncate_for_card(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let collected = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{collected}...") } else { collected }
}

pub(super) fn completed_status_icon() -> Element<'static, Message> {
    container(
        svg::Svg::new(assets::get_icon(Icon::Check))
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .style(|_theme: &Theme, _status| svg::Style {
                color: Some(Color::from_rgb8(4, 24, 22)),
            }),
    )
    .width(Length::Fixed(15.0))
    .height(Length::Fixed(15.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(Color::from_rgb8(70, 208, 120))),
        border: Border { radius: 7.5.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    })
    .into()
}

pub(super) fn running_status_icon(animation_frame: usize) -> Element<'static, Message> {
    container(
        text(spinner_frame(animation_frame))
            .size(14)
            .style(|theme: &Theme| iced::widget::text::Style { color: Some(theme.palette().text) }),
    )
    .width(Length::Fixed(TASK_STATUS_ICON_SIZE))
    .height(Length::Fixed(TASK_STATUS_ICON_SIZE))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

#[cfg(test)]
pub(super) fn task_status_slot(
    status_icon: Element<'static, Message>,
) -> Element<'static, Message> {
    container(status_icon)
        .width(Length::Fixed(TASK_STATUS_SLOT_WIDTH))
        .height(Length::Fixed(TASK_STATUS_SLOT_HEIGHT))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}

pub(super) fn task_card_style(theme: &Theme) -> container::Style {
    super::controls::surface(theme, 23.0, false)
}

pub(super) fn tiny_action_button_style(theme: &Theme, status: button::Status) -> button::Style {
    circle_action_style(theme, status, REMOVE_BUTTON_SIZE, 0.20)
}

pub(super) fn reply_button_style(theme: &Theme, status: button::Status) -> button::Style {
    circle_action_style(theme, status, CARD_ACTION_SIZE, 0.05)
}

fn circle_action_style(
    theme: &Theme,
    status: button::Status,
    size: f32,
    rest_alpha: f32,
) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    let alpha = match status {
        button::Status::Hovered => 0.30,
        button::Status::Pressed => 0.38,
        _ => rest_alpha,
    };
    style.background = Some(theme.palette().text.scale_alpha(alpha).into());
    style.border = Border::default().rounded(size / 2.0);
    style
}

#[cfg(test)]
pub(super) fn reply_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    settings_text_input_style(theme, status)
}
