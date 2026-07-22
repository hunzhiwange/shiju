use iced::widget::image::{Handle as ImageHandle, Image};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, svg, text, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use once_cell::sync::Lazy;

use crate::app::assets::{self, Icon};
use crate::app::components::status_animation::spinner_frame;
use crate::app::state::{TaskPetAvatarKind, TaskPetItem, TaskPetStatus};
use crate::app::{App, Message, message};

const PET_CARD_BACKGROUND: Color = Color::from_rgb8(22, 22, 22);
const PET_PANEL_WIDTH: f32 = 360.0;
const PET_PANEL_PADDING: f32 = 10.0;
const PET_CARD_HEIGHT: f32 = 72.0;
const PET_CARD_REPLY_HEIGHT: f32 = 120.0;
const PET_CARD_SPACING: f32 = 8.0;
const PET_VISIBLE_TASKS: f32 = 3.0;
const PET_LIST_HEIGHT: f32 =
    PET_CARD_HEIGHT * PET_VISIBLE_TASKS + PET_CARD_SPACING * (PET_VISIBLE_TASKS - 1.0);
const PET_COLLAPSED_SIZE: f32 = 74.0;
const PET_EXPANDED_SIZE: f32 = PET_COLLAPSED_SIZE;
const PET_CONTROL_SIZE: f32 = 104.0;
const PET_CONTROL_GAP: f32 = 6.0;
const PET_AVATAR_SWITCH_SIZE: f32 = 18.0;
const TASK_STATUS_SLOT_WIDTH: f32 = 20.0;
const TASK_STATUS_SLOT_HEIGHT: f32 = 52.0;
const TASK_STATUS_ICON_SIZE: f32 = 18.0;
const TASK_TITLE_SIZE: u32 = 15;
const TASK_DETAIL_SIZE: u32 = 13;

static PET_WINK_HANDLE: Lazy<svg::Handle> =
    Lazy::new(|| svg::Handle::from_memory(PET_WINK_SVG.as_bytes()));
static PET_HOVER_HANDLE: Lazy<svg::Handle> =
    Lazy::new(|| svg::Handle::from_memory(PET_HOVER_SVG.as_bytes()));
static PET_HOVER_JUMP_HANDLE: Lazy<svg::Handle> =
    Lazy::new(|| svg::Handle::from_memory(PET_HOVER_JUMP_SVG.as_bytes()));
static PET_BEAUTY_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../../assets/task-pet/avatar-beauty.png").as_slice(),
    )
});
static PET_BEAUTY_LEFT_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../../assets/task-pet/avatar-beauty-left.png").as_slice(),
    )
});
static PET_BEAUTY_WORK_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../../assets/task-pet/avatar-beauty-work.png").as_slice(),
    )
});
static PET_HANDSOME_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../../assets/task-pet/avatar-handsome.png").as_slice(),
    )
});
static PET_HANDSOME_LEFT_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../../assets/task-pet/avatar-handsome-left.png").as_slice(),
    )
});
static PET_HANDSOME_WORK_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../../assets/task-pet/avatar-handsome-work.png").as_slice(),
    )
});
static PET_CODE_HANDLES: Lazy<[svg::Handle; 2]> = Lazy::new(|| {
    [
        svg::Handle::from_memory(PET_CODE_SVG_0.as_bytes()),
        svg::Handle::from_memory(PET_CODE_SVG_1.as_bytes()),
    ]
});
static PET_WALK_LEFT_HANDLES: Lazy<[svg::Handle; 2]> = Lazy::new(|| {
    [walk_pet_handle(PET_WALK_RIGHT_BODY_0, true), walk_pet_handle(PET_WALK_RIGHT_BODY_1, true)]
});
static PET_WALK_RIGHT_HANDLES: Lazy<[svg::Handle; 2]> = Lazy::new(|| {
    [walk_pet_handle(PET_WALK_RIGHT_BODY_0, false), walk_pet_handle(PET_WALK_RIGHT_BODY_1, false)]
});

fn walk_pet_handle(body: &str, flipped: bool) -> svg::Handle {
    let content = if flipped {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" shape-rendering="crispEdges"><g transform="translate(128 0) scale(-1 1)">{body}</g></svg>"#
        )
    } else {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" shape-rendering="crispEdges">{body}</svg>"#
        )
    };
    svg::Handle::from_memory(content.into_bytes())
}

const PET_WINK_SVG: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" shape-rendering="crispEdges">
  <rect x="45" y="104" width="13" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
  <rect x="70" y="104" width="13" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
  <rect x="18" y="78" width="24" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
  <rect x="86" y="78" width="24" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
  <rect x="40" y="70" width="48" height="38" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
  <rect x="48" y="74" width="32" height="28" fill="#5c8cff"/>
  <rect x="30" y="24" width="68" height="46" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
  <rect x="42" y="17" width="44" height="14" fill="#7197ff" stroke="#061047" stroke-width="4"/>
  <rect x="25" y="40" width="13" height="16" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
  <rect x="90" y="40" width="13" height="16" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
  <rect x="39" y="42" width="50" height="28" fill="#16275c" stroke="#061047" stroke-width="4"/>
  <rect x="50" y="50" width="4" height="4" fill="#8cf3ff"/>
  <rect x="54" y="54" width="4" height="4" fill="#8cf3ff"/>
  <rect x="50" y="58" width="4" height="4" fill="#8cf3ff"/>
  <rect x="68" y="56" width="14" height="4" fill="#8cf3ff"/>
  <rect x="58" y="86" width="4" height="4" fill="#ffffff"/>
  <rect x="62" y="90" width="4" height="4" fill="#ffffff"/>
  <rect x="66" y="86" width="10" height="4" fill="#ffffff"/>
</svg>
"##;

const PET_HOVER_SVG: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" shape-rendering="crispEdges">
  <rect x="45" y="104" width="13" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
  <rect x="70" y="104" width="13" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
  <rect x="18" y="78" width="24" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
  <rect x="86" y="78" width="24" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
  <rect x="40" y="70" width="48" height="38" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
  <rect x="48" y="74" width="32" height="28" fill="#5c8cff"/>
  <rect x="30" y="24" width="68" height="46" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
  <rect x="42" y="17" width="44" height="14" fill="#7197ff" stroke="#061047" stroke-width="4"/>
  <rect x="25" y="40" width="13" height="16" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
  <rect x="90" y="40" width="13" height="16" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
  <rect x="39" y="42" width="50" height="28" fill="#16275c" stroke="#061047" stroke-width="4"/>
  <rect x="50" y="54" width="4" height="4" fill="#8cf3ff"/>
  <rect x="54" y="50" width="4" height="4" fill="#8cf3ff"/>
  <rect x="58" y="54" width="4" height="4" fill="#8cf3ff"/>
  <rect x="70" y="54" width="4" height="4" fill="#8cf3ff"/>
  <rect x="74" y="50" width="4" height="4" fill="#8cf3ff"/>
  <rect x="78" y="54" width="4" height="4" fill="#8cf3ff"/>
  <rect x="58" y="86" width="4" height="4" fill="#ffffff"/>
  <rect x="62" y="90" width="4" height="4" fill="#ffffff"/>
  <rect x="66" y="86" width="10" height="4" fill="#ffffff"/>
</svg>
"##;

const PET_HOVER_JUMP_SVG: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" shape-rendering="crispEdges">
  <g transform="translate(0 -6)">
    <rect x="43" y="105" width="13" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
    <rect x="72" y="101" width="13" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
    <rect x="14" y="73" width="24" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
    <rect x="90" y="82" width="24" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
    <rect x="40" y="70" width="48" height="38" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
    <rect x="48" y="74" width="32" height="28" fill="#5c8cff"/>
    <rect x="30" y="24" width="68" height="46" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
    <rect x="42" y="17" width="44" height="14" fill="#7197ff" stroke="#061047" stroke-width="4"/>
    <rect x="25" y="40" width="13" height="16" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
    <rect x="90" y="40" width="13" height="16" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
    <rect x="39" y="42" width="50" height="28" fill="#16275c" stroke="#061047" stroke-width="4"/>
    <rect x="50" y="54" width="4" height="4" fill="#8cf3ff"/>
    <rect x="54" y="50" width="4" height="4" fill="#8cf3ff"/>
    <rect x="58" y="54" width="4" height="4" fill="#8cf3ff"/>
    <rect x="70" y="54" width="4" height="4" fill="#8cf3ff"/>
    <rect x="74" y="50" width="4" height="4" fill="#8cf3ff"/>
    <rect x="78" y="54" width="4" height="4" fill="#8cf3ff"/>
    <rect x="58" y="86" width="4" height="4" fill="#ffffff"/>
    <rect x="62" y="90" width="4" height="4" fill="#ffffff"/>
    <rect x="66" y="86" width="10" height="4" fill="#ffffff"/>
  </g>
</svg>
"##;

const PET_CODE_SVG_0: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 42" shape-rendering="crispEdges">
  <rect x="9" y="5" width="54" height="28" rx="2" fill="#101828" stroke="#061047" stroke-width="4"/>
  <rect x="15" y="11" width="42" height="16" fill="#172554"/>
  <rect x="18" y="14" width="10" height="3" fill="#8cf3ff"/>
  <rect x="31" y="14" width="16" height="3" fill="#78a7ff"/>
  <rect x="18" y="21" width="18" height="3" fill="#78a7ff"/>
  <rect x="40" y="21" width="8" height="3" fill="#8cf3ff"/>
  <rect x="4" y="33" width="64" height="7" fill="#2f5df7" stroke="#061047" stroke-width="3"/>
</svg>
"##;

const PET_CODE_SVG_1: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 42" shape-rendering="crispEdges">
  <rect x="9" y="5" width="54" height="28" rx="2" fill="#101828" stroke="#061047" stroke-width="4"/>
  <rect x="15" y="11" width="42" height="16" fill="#172554"/>
  <rect x="18" y="14" width="18" height="3" fill="#78a7ff"/>
  <rect x="39" y="14" width="10" height="3" fill="#8cf3ff"/>
  <rect x="18" y="21" width="8" height="3" fill="#8cf3ff"/>
  <rect x="30" y="21" width="20" height="3" fill="#78a7ff"/>
  <rect x="4" y="33" width="64" height="7" fill="#2f5df7" stroke="#061047" stroke-width="3"/>
</svg>
"##;

const PET_WALK_RIGHT_BODY_0: &str = r##"
  <rect x="48" y="104" width="14" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
  <rect x="70" y="101" width="14" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
  <rect x="26" y="79" width="22" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
  <rect x="79" y="75" width="18" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
  <rect x="42" y="70" width="43" height="38" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
  <rect x="49" y="75" width="29" height="27" fill="#5c8cff"/>
  <rect x="30" y="24" width="66" height="46" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
  <rect x="41" y="17" width="42" height="14" fill="#7197ff" stroke="#061047" stroke-width="4"/>
  <rect x="25" y="41" width="13" height="16" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
  <rect x="88" y="38" width="13" height="20" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
  <rect x="44" y="43" width="45" height="27" fill="#16275c" stroke="#061047" stroke-width="4"/>
  <rect x="71" y="51" width="5" height="12" fill="#8cf3ff"/>
  <rect x="82" y="51" width="5" height="12" fill="#8cf3ff"/>
  <polygon points="73,68 109,68 101,108 66,108" fill="#172554" stroke="#061047" stroke-width="4"/>
  <rect x="82" y="82" width="4" height="4" fill="#8cf3ff"/>
  <rect x="86" y="86" width="4" height="4" fill="#8cf3ff"/>
  <rect x="90" y="82" width="9" height="4" fill="#8cf3ff"/>
"##;

const PET_WALK_RIGHT_BODY_1: &str = r##"
  <rect x="48" y="101" width="14" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
  <rect x="70" y="104" width="14" height="12" fill="#3b66dc" stroke="#061047" stroke-width="3"/>
  <rect x="26" y="75" width="22" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
  <rect x="79" y="79" width="18" height="12" fill="#4f7dff" stroke="#061047" stroke-width="3"/>
  <rect x="42" y="70" width="43" height="38" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
  <rect x="49" y="75" width="29" height="27" fill="#5c8cff"/>
  <rect x="30" y="24" width="66" height="46" fill="#4f7dff" stroke="#061047" stroke-width="4"/>
  <rect x="41" y="17" width="42" height="14" fill="#7197ff" stroke="#061047" stroke-width="4"/>
  <rect x="25" y="38" width="13" height="20" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
  <rect x="88" y="41" width="13" height="16" fill="#5f88ff" stroke="#061047" stroke-width="4"/>
  <rect x="44" y="43" width="45" height="27" fill="#16275c" stroke="#061047" stroke-width="4"/>
  <rect x="71" y="51" width="5" height="12" fill="#8cf3ff"/>
  <rect x="82" y="51" width="5" height="12" fill="#8cf3ff"/>
  <polygon points="73,68 109,68 101,108 66,108" fill="#172554" stroke="#061047" stroke-width="4"/>
  <rect x="82" y="82" width="4" height="4" fill="#8cf3ff"/>
  <rect x="86" y="86" width="4" height="4" fill="#8cf3ff"/>
  <rect x="90" y="82" width="9" height="4" fill="#8cf3ff"/>
"##;

pub fn window(app: &App) -> Element<'_, Message> {
    if app.task_pet_render_collapsed() { collapsed_window(app) } else { expanded_window(app) }
}

fn expanded_window(app: &App) -> Element<'_, Message> {
    let mut cards = column![].spacing(PET_CARD_SPACING).width(Length::Fill);
    for item in &app.task_pet_items {
        cards = cards.push(task_card(app, item));
    }

    let list = scrollable(cards)
        .id(app.task_pet_scroll_id.clone())
        .height(Length::Fixed(PET_LIST_HEIGHT))
        .width(Length::Fixed(PET_PANEL_WIDTH - PET_PANEL_PADDING * 2.0))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));

    let list_layer = container(list)
        .width(Length::Fill)
        .height(Length::Fixed(PET_LIST_HEIGHT))
        .padding(iced::Padding::default().top(0).right(4).bottom(0).left(4));

    let pet = row![Space::new().width(Length::Fill), pet_control(app, PET_EXPANDED_SIZE, false)]
        .align_y(Alignment::End)
        .width(Length::Fill);

    let content =
        column![list_layer, pet].spacing(PET_CONTROL_GAP).width(Length::Fill).height(Length::Fill);
    let content: Element<'_, Message> = if app.task_pet_robot_hovered {
        iced::widget::stack![
            container(content).width(Length::Fill).height(Length::Fill),
            container(avatar_cycle_button())
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding::default().top(0).right(0).bottom(12).left(0)),
        ]
        .into()
    } else {
        content.into()
    };
    let content = mouse_area(content)
        .on_enter(Message::View(message::ViewMessage::TaskPetRobotHover(true)))
        .on_exit(Message::View(message::ViewMessage::TaskPetRobotHover(false)));

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding::default().top(4).right(4).bottom(0).left(8))
        .style(expanded_pet_window_style)
        .into()
}

fn collapsed_window(app: &App) -> Element<'_, Message> {
    let content = pet_control(app, PET_COLLAPSED_SIZE, true);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(transparent_pet_window_style)
        .into()
}

fn pet_control(app: &App, size: f32, collapsed: bool) -> Element<'_, Message> {
    let pet_hit_area = mouse_area(
        container(pet_sprite(app, size))
            .width(Length::Fixed(PET_CONTROL_SIZE))
            .height(Length::Fixed(PET_CONTROL_SIZE))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .on_press(Message::View(message::ViewMessage::TaskPetDragStarted));
    let pet_hit_area = if collapsed {
        pet_hit_area
            .on_enter(Message::View(message::ViewMessage::TaskPetRobotHover(true)))
            .on_exit(Message::View(message::ViewMessage::TaskPetRobotHover(false)))
    } else {
        pet_hit_area
    };

    let mut pet_stack = iced::widget::stack![
        container(pet_hit_area)
            .width(Length::Fixed(PET_CONTROL_SIZE))
            .height(Length::Fixed(PET_CONTROL_SIZE))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
    ];

    if app.task_pet_avatar_kind == TaskPetAvatarKind::Robot
        && app.task_pet_active_count() > 0
        && !app.task_pet_is_walking()
    {
        pet_stack = pet_stack.push(
            container(code_effect(app.status_animation_frame))
                .width(Length::Fixed(PET_CONTROL_SIZE))
                .height(Length::Fixed(PET_CONTROL_SIZE))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding::default().top(0).right(0).bottom(7).left(0)),
        );
    }

    if collapsed {
        let count = app.task_pet_active_count().max(app.task_pet_visible_count());
        if count > 0 {
            pet_stack = pet_stack.push(
                container(badge_button(count))
                    .width(Length::Fixed(PET_CONTROL_SIZE))
                    .height(Length::Fixed(PET_CONTROL_SIZE))
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Top)
                    .padding(iced::Padding::default().top(14).right(12).bottom(0).left(0)),
            );
        }
    } else {
        pet_stack = pet_stack.push(
            container(collapse_button())
                .width(Length::Fixed(PET_CONTROL_SIZE))
                .height(Length::Fixed(PET_CONTROL_SIZE))
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top),
        );
    }

    pet_stack.into()
}

fn pet_sprite(app: &App, size: f32) -> Element<'_, Message> {
    match app.task_pet_avatar_kind {
        TaskPetAvatarKind::Robot => robot_pet_sprite(app, size),
        TaskPetAvatarKind::Beauty => human_pet_sprite(
            human_avatar_handle(
                app,
                PET_BEAUTY_HANDLE.clone(),
                PET_BEAUTY_LEFT_HANDLE.clone(),
                PET_BEAUTY_WORK_HANDLE.clone(),
            ),
            size,
            human_motion_lift(app),
        ),
        TaskPetAvatarKind::Handsome => human_pet_sprite(
            human_avatar_handle(
                app,
                PET_HANDSOME_HANDLE.clone(),
                PET_HANDSOME_LEFT_HANDLE.clone(),
                PET_HANDSOME_WORK_HANDLE.clone(),
            ),
            size,
            human_motion_lift(app),
        ),
    }
}

fn robot_pet_sprite(app: &App, size: f32) -> Element<'_, Message> {
    let handle = if app.task_pet_is_walking() {
        let frame = app.status_animation_frame % 2;
        if app.task_pet_facing_left() {
            PET_WALK_LEFT_HANDLES[frame].clone()
        } else {
            PET_WALK_RIGHT_HANDLES[frame].clone()
        }
    } else if app.task_pet_robot_hovered {
        if one_second_pulse(app.status_animation_frame) {
            PET_HOVER_JUMP_HANDLE.clone()
        } else {
            PET_HOVER_HANDLE.clone()
        }
    } else {
        PET_WINK_HANDLE.clone()
    };
    svg::Svg::new(handle).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

fn human_avatar_handle(
    app: &App,
    normal: ImageHandle,
    left: ImageHandle,
    work: ImageHandle,
) -> ImageHandle {
    if app.task_pet_is_walking() {
        return if app.task_pet_facing_left() { left } else { normal };
    }
    if app.task_pet_robot_hovered {
        return normal;
    }
    if app.task_pet_active_count() > 0 && one_second_pulse(app.status_animation_frame) {
        return work;
    }
    left
}

fn human_motion_lift(app: &App) -> f32 {
    if app.task_pet_is_walking() || app.task_pet_robot_hovered || app.task_pet_active_count() > 0 {
        match app.status_animation_frame % 11 {
            0 => 5.0,
            1 => 2.0,
            _ => 0.0,
        }
    } else {
        0.0
    }
}

fn one_second_pulse(frame: usize) -> bool {
    frame % 11 <= 1
}

fn double_speed_frame(frame: usize, divisor: usize) -> usize {
    frame.saturating_mul(2) / divisor
}

fn human_pet_sprite(handle: ImageHandle, size: f32, lift: f32) -> Element<'static, Message> {
    column![
        Image::new(handle).width(Length::Fixed(size)).height(Length::Fixed(size)),
        Space::new().height(Length::Fixed(lift)),
    ]
    .width(Length::Fixed(size))
    .height(Length::Shrink)
    .align_x(Alignment::Center)
    .into()
}

fn code_effect(frame: usize) -> Element<'static, Message> {
    svg::Svg::new(PET_CODE_HANDLES[double_speed_frame(frame, 5) % PET_CODE_HANDLES.len()].clone())
        .width(Length::Fixed(48.0))
        .height(Length::Fixed(28.0))
        .into()
}

fn avatar_cycle_button() -> Element<'static, Message> {
    button(
        svg::Svg::new(assets::get_icon(Icon::ArrowRepeat))
            .width(Length::Fixed(9.0))
            .height(Length::Fixed(9.0))
            .style(|_theme: &Theme, _status| svg::Style {
                color: Some(Color::from_rgb8(196, 206, 220)),
            }),
    )
    .width(Length::Fixed(PET_AVATAR_SWITCH_SIZE))
    .height(Length::Fixed(PET_AVATAR_SWITCH_SIZE))
    .padding(0)
    .style(borderless_icon_button_style)
    .on_press(Message::View(message::ViewMessage::TaskPetAvatarCycle))
    .into()
}

/// 收起态计数圆形徽标尺寸：较常规按钮缩小约 1/3，避免在小窗里喧宾夺主。
const BADGE_BUTTON_SIZE: f32 = 16.0;

fn badge_button(count: usize) -> Element<'static, Message> {
    let label = container(
        text(count.to_string()).size(8).line_height(1.0).color(Color::from_rgb8(2, 36, 24)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    button(label)
        .width(Length::Fixed(BADGE_BUTTON_SIZE))
        .height(Length::Fixed(BADGE_BUTTON_SIZE))
        .padding(0)
        .style(green_badge_button_style)
        .on_press(Message::View(message::ViewMessage::TaskPetToggleCollapsed))
        .into()
}

fn collapse_button() -> Element<'static, Message> {
    button(
        svg::Svg::new(assets::get_icon(Icon::ChevronDown))
            .width(Length::Fixed(13.0))
            .height(Length::Fixed(13.0))
            .style(|_theme: &Theme, _status| svg::Style {
                color: Some(Color::from_rgb8(190, 190, 190)),
            }),
    )
    .width(Length::Fixed(24.0))
    .height(Length::Fixed(24.0))
    .padding(0)
    .style(borderless_icon_button_style)
    .on_press(Message::View(message::ViewMessage::TaskPetToggleCollapsed))
    .into()
}

fn task_card<'a>(app: &'a App, item: &'a TaskPetItem) -> Element<'a, Message> {
    let status_icon = match item.status {
        TaskPetStatus::Running => running_status_icon(app.status_animation_frame),
        TaskPetStatus::Completed => completed_status_icon(),
    };
    let request_id = item.request_id;
    let hovered = app.task_pet_hovered_request_id == Some(request_id);
    let replying = app.task_pet_reply_request_id == Some(request_id);
    let show_actions = hovered || replying;

    let remove = button(
        svg::Svg::new(assets::get_icon(Icon::X))
            .width(Length::Fixed(7.0))
            .height(Length::Fixed(7.0))
            .style(|_theme: &Theme, _status| svg::Style {
                color: Some(Color::from_rgb8(214, 214, 214)),
            }),
    )
    .width(Length::Fixed(15.0))
    .height(Length::Fixed(15.0))
    .padding(0)
    .style(tiny_action_button_style)
    .on_press(Message::View(message::ViewMessage::TaskPetRemove(request_id)));

    let body = row![
        column![
            text(truncate_for_card(&item.title, 18))
                .size(TASK_TITLE_SIZE)
                .line_height(1.06)
                .color(Color::WHITE)
                .width(Length::Fill),
            task_detail_line(&item.detail),
        ]
        .spacing(2)
        .width(Length::Fill),
        task_status_slot(status_icon),
    ]
    .spacing(6)
    .height(Length::Fill)
    .align_y(Alignment::Center);

    let card = iced::widget::stack![
        container(body)
            .width(Length::Fill)
            .height(Length::Fixed(PET_CARD_HEIGHT))
            .padding(iced::Padding::default().top(10).right(12).bottom(10).left(22))
            .style(task_card_style),
    ];
    let card = if show_actions {
        card.push(
            container(remove)
                .width(Length::Fill)
                .height(Length::Fixed(PET_CARD_HEIGHT))
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding::default().top(0).right(0).bottom(10).left(12)),
        )
        .push(
            container(reply_button(request_id))
                .width(Length::Fill)
                .height(Length::Fixed(PET_CARD_HEIGHT))
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding::default().top(0).right(8).bottom(6).left(0)),
        )
    } else {
        card
    };

    let content: Element<'_, Message> = if replying {
        column![
            container(card).height(Length::Fixed(PET_CARD_HEIGHT)),
            reply_input(&app.task_pet_reply_input),
        ]
        .spacing(6)
        .into()
    } else {
        container(card).height(Length::Fixed(PET_CARD_HEIGHT)).into()
    };
    let height = if replying { PET_CARD_REPLY_HEIGHT } else { PET_CARD_HEIGHT };

    mouse_area(container(content).width(Length::Fill).height(Length::Fixed(height)))
        .on_enter(Message::View(message::ViewMessage::TaskPetHover(Some(request_id))))
        .on_exit(Message::View(message::ViewMessage::TaskPetHover(None)))
        .on_press(Message::View(message::ViewMessage::TaskPetItemClicked(request_id)))
        .into()
}

fn reply_button(request_id: u64) -> Element<'static, Message> {
    button(centered_button_label("回复", 12))
        .width(Length::Fixed(42.0))
        .height(Length::Fixed(22.0))
        .padding(0)
        .style(reply_button_style)
        .on_press(Message::View(message::ViewMessage::TaskPetReplyPressed(request_id)))
        .into()
}

fn reply_input(value: &str) -> Element<'_, Message> {
    row![
        text_input("继续说点什么...", value)
            .on_input(|next| Message::View(message::ViewMessage::TaskPetReplyInputChanged(next)))
            .on_submit(Message::View(message::ViewMessage::TaskPetReplySubmit))
            .padding([8, 10])
            .size(13)
            .style(reply_input_style)
            .width(Length::Fill),
        button(centered_button_label("发送", 12))
            .width(Length::Fixed(42.0))
            .height(Length::Fixed(30.0))
            .padding(0)
            .style(reply_button_style)
            .on_press(Message::View(message::ViewMessage::TaskPetReplySubmit)),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn centered_button_label(label: &'static str, size: u32) -> Element<'static, Message> {
    container(text(label).size(size).color(Color::from_rgb8(238, 238, 238)))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn task_detail_line(detail: &str) -> Element<'_, Message> {
    text(truncate_for_card(detail, 46))
        .size(TASK_DETAIL_SIZE)
        .line_height(1.12)
        .color(Color::from_rgb8(225, 225, 225))
        .width(Length::Fill)
        .into()
}

fn truncate_for_card(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let collected = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{collected}...") } else { collected }
}

fn completed_status_icon() -> Element<'static, Message> {
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

fn running_status_icon(animation_frame: usize) -> Element<'static, Message> {
    container(text(spinner_frame(animation_frame)).size(14).style(|_theme: &Theme| {
        iced::widget::text::Style { color: Some(Color::from_rgb8(225, 225, 225)) }
    }))
    .width(Length::Fixed(TASK_STATUS_ICON_SIZE))
    .height(Length::Fixed(TASK_STATUS_ICON_SIZE))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn task_status_slot(status_icon: Element<'static, Message>) -> Element<'static, Message> {
    container(status_icon)
        .width(Length::Fixed(TASK_STATUS_SLOT_WIDTH))
        .height(Length::Fixed(TASK_STATUS_SLOT_HEIGHT))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}

fn expanded_pet_window_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

fn transparent_pet_window_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

fn task_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(PET_CARD_BACKGROUND)),
        text_color: Some(Color::WHITE),
        border: Border { radius: 24.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

fn tiny_action_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Color::from_rgb8(34, 34, 34)
    } else {
        Color::from_rgb8(18, 18, 18)
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border { radius: 7.5.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

fn reply_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Color::from_rgb8(38, 38, 38)
    } else {
        Color::from_rgb8(28, 28, 28)
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border { radius: 12.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

fn reply_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(Color::from_rgb8(28, 28, 28)),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: if focused {
                Color::from_rgb8(96, 140, 255)
            } else {
                Color::from_rgb8(62, 62, 62)
            },
        },
        icon: Color::from_rgb8(210, 210, 210),
        placeholder: Color::from_rgb8(134, 134, 134),
        value: Color::WHITE,
        selection: Color::from_rgba8(96, 140, 255, 0.28),
    }
}

fn borderless_icon_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Color::from_rgb8(28, 28, 28)
    } else {
        Color::from_rgb8(18, 18, 18)
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border { radius: 12.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

fn green_badge_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Color::from_rgb8(80, 220, 130)
    } else {
        Color::from_rgb8(70, 208, 120)
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::from_rgb8(2, 36, 24),
        border: Border {
            radius: (BADGE_BUTTON_SIZE / 2.0).into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "task_pet_tests.rs"]
mod task_pet_tests;
