use iced::widget::image::{Handle as ImageHandle, Image};
use iced::widget::{Space, button, column, container, mouse_area, row, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Theme};
use once_cell::sync::Lazy;

mod cards;
mod controls;

#[cfg(test)]
use cards::*;

use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::widgets::RightClickArea;
use crate::app::state::TaskPetAvatarKind;
use crate::app::{App, Message, message};

const PET_COLLAPSED_SIZE: f32 = 74.0;
const PET_EXPANDED_SIZE: f32 = PET_COLLAPSED_SIZE;
const PET_CONTROL_SIZE: f32 = 104.0;
const PET_CONTROL_GAP: f32 = 6.0;
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
    let content = column![pet_control(app, PET_EXPANDED_SIZE, false), cards::expanded_list(app)]
        .spacing(PET_CONTROL_GAP)
        .align_x(Alignment::Center);
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .style(transparent_pet_window_style)
        .into()
}

fn collapsed_window(app: &App) -> Element<'_, Message> {
    container(
        column![pet_control(app, PET_COLLAPSED_SIZE, true), cards::collapsed_stack(app)]
            .spacing(PET_CONTROL_GAP)
            .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .height(Length::Fill)
    .padding(8)
    .style(transparent_pet_window_style)
    .into()
}

fn pet_control(app: &App, size: f32, _collapsed: bool) -> Element<'_, Message> {
    let avatar: Element<'_, Message> = if app.task_pet_avatar_kind == TaskPetAvatarKind::None {
        Space::new().height(PET_CONTROL_SIZE).into()
    } else {
        let mut sprite = iced::widget::stack![pet_sprite(app, size * app.task_pet_scale)];
        if app.task_pet_avatar_kind == TaskPetAvatarKind::Robot
            && app.task_pet_active_count() > 0
            && !app.task_pet_is_walking()
        {
            sprite = sprite.push(
                container(code_effect(app.status_animation_frame))
                    .center_x(Length::Fill)
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Bottom),
            );
        }
        let avatar = mouse_area(
            container(sprite)
                .center_x(Length::Fixed(PET_CONTROL_SIZE * app.task_pet_scale))
                .center_y(Length::Fixed(PET_CONTROL_SIZE * app.task_pet_scale)),
        )
        .on_press(Message::View(message::ViewMessage::TaskPetDragStarted))
        .on_enter(Message::View(message::ViewMessage::TaskPetRobotHover(true)))
        .on_exit(Message::View(message::ViewMessage::TaskPetRobotHover(false)));
        avatar.into()
    };
    let content =
        column![avatar, controls::toolbar(app)].spacing(PET_CONTROL_GAP).align_x(Alignment::Center);
    let content = Element::new(
        RightClickArea::new(
            content.into(),
            Box::new(|position| {
                Message::View(message::ViewMessage::TaskPetContextMenuOpened(
                    position.x, position.y,
                ))
            }),
        )
        .preserve_on_right_click(),
    );
    if let Some((x, y)) = app.task_pet_context_menu_pos {
        PointBelowOverlay::new(
            content,
            task_pet_context_menu(crate::app::i18n::I18n::new(app.desktop_locale)),
        )
        .show(true)
        .anchor(Point::new(x, y))
        .gap(2.0)
        .on_close(Message::View(message::ViewMessage::TaskPetContextMenuClosed))
        .into()
    } else {
        content
    }
}

fn task_pet_context_menu(i18n: crate::app::i18n::I18n) -> Element<'static, Message> {
    use crate::app::components::system_settings_common::rounded_action_btn_style;
    let action = |label, intent| {
        button(text(i18n.__(label)).size(13))
            .width(Length::Fill)
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(Message::View(intent))
    };
    mouse_area(
        column![
            action("选择智能伙伴", message::ViewMessage::TaskPetOpenSettings),
            action("隐藏伙伴", message::ViewMessage::TaskPetHideWindow),
        ]
        .spacing(4)
        .width(180),
    )
    .on_exit(Message::View(message::ViewMessage::TaskPetContextMenuClosed))
    .into()
}

/// 设置预览与 Mini 共用素材；不绑定窗口、拖动或悬停事件。
pub(super) fn avatar_preview(avatar: TaskPetAvatarKind, size: f32) -> Element<'static, Message> {
    match avatar {
        TaskPetAvatarKind::None => Space::new().width(size).height(size).into(),
        TaskPetAvatarKind::Robot => {
            svg::Svg::new(PET_WINK_HANDLE.clone()).width(size).height(size).into()
        }
        TaskPetAvatarKind::Beauty => human_pet_sprite(PET_BEAUTY_LEFT_HANDLE.clone(), size, 0.0),
        TaskPetAvatarKind::Handsome => {
            human_pet_sprite(PET_HANDSOME_LEFT_HANDLE.clone(), size, 0.0)
        }
    }
}

fn pet_sprite(app: &App, size: f32) -> Element<'_, Message> {
    if !app.task_pet_is_walking() && !app.task_pet_robot_hovered && app.task_pet_active_count() == 0
    {
        return avatar_preview(app.task_pet_avatar_kind, size);
    }
    match app.task_pet_avatar_kind {
        TaskPetAvatarKind::None => Space::new().into(),
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

fn transparent_pet_window_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "task_pet_tests.rs"]
mod task_pet_tests;
