use iced::widget::image::{Handle as ImageHandle, Image};
use iced::widget::{Space, button, column, container, mouse_area, row, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Theme};
use once_cell::sync::Lazy;

mod cards;
mod controls;

#[cfg(test)]
use cards::{
    centered_button_label, completed_status_icon, reply_button, reply_button_style, reply_input,
    reply_input_style, running_status_icon, task_card, task_card_style, task_detail_line,
    task_status_slot, tiny_action_button_style, truncate_for_card,
};

use crate::pet_state::TaskPetAvatarKind;
use crate::pet_view::{
    ACTIONS_WIDTH, DOCK_HEIGHT, PetMessage as Message, PetView, REMINDER_WIDTH, WINDOW_PADDING,
};
use desktop_ui_controls::overlays::PointBelowOverlay;
use desktop_ui_controls::right_click_area::RightClickArea;

const PET_COLLAPSED_SIZE: f32 = 74.0;
const PET_CONTROL_GAP: f32 = 4.0;
static PET_WINK_HANDLE: Lazy<svg::Handle> =
    Lazy::new(|| svg::Handle::from_memory(PET_WINK_SVG.as_bytes()));
static PET_HOVER_HANDLE: Lazy<svg::Handle> =
    Lazy::new(|| svg::Handle::from_memory(PET_HOVER_SVG.as_bytes()));
static PET_HOVER_JUMP_HANDLE: Lazy<svg::Handle> =
    Lazy::new(|| svg::Handle::from_memory(PET_HOVER_JUMP_SVG.as_bytes()));
static PET_BEAUTY_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../assets/task-pet/avatar-beauty.png").as_slice(),
    )
});
static PET_BEAUTY_LEFT_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../assets/task-pet/avatar-beauty-left.png").as_slice(),
    )
});
static PET_BEAUTY_WORK_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../assets/task-pet/avatar-beauty-work.png").as_slice(),
    )
});
static PET_HANDSOME_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../assets/task-pet/avatar-handsome.png").as_slice(),
    )
});
static PET_HANDSOME_LEFT_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../assets/task-pet/avatar-handsome-left.png").as_slice(),
    )
});
static PET_HANDSOME_WORK_HANDLE: Lazy<ImageHandle> = Lazy::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../../assets/task-pet/avatar-handsome-work.png").as_slice(),
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

pub use cards::{reminder_height, reply_id};
pub use controls::{composer_id, project_search_id};

pub fn window<'a>(app: PetView<'a>) -> Element<'a, Message> {
    let mut content = column![].align_x(Alignment::Center);
    if app.dock.reminder_height > 0.0 && !app.task_pet_items.is_empty() {
        let cards = if app.render_collapsed {
            cards::collapsed_stack(app)
        } else {
            cards::expanded_list(app, app.dock.reminder_height)
        };
        content = content.push(
            container(cards)
                .width(REMINDER_WIDTH)
                .height(app.dock.reminder_height)
                .align_y(iced::alignment::Vertical::Bottom)
                .clip(true),
        );
    }
    content = content.push(pet_control(app));
    let tooltip_space = app.dock.mode.project_tooltip_space(app.dock.project_path.is_some());
    let content = container(content)
        .center_x(Length::Fill)
        .height(Length::Fill)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(iced::Padding::new(WINDOW_PADDING).bottom(WINDOW_PADDING + tooltip_space))
        .style(transparent_pet_window_style);
    let content: Element<'a, Message> = if app.dock.mode == crate::pet_view::DockMode::Composer {
        mouse_area(content).on_press(Message::TaskPetComposerClose).into()
    } else {
        content.into()
    };
    let content = Element::new(
        RightClickArea::new(
            content,
            Box::new(|position| Message::TaskPetContextMenuOpened(position.x, position.y)),
        )
        .preserve_on_right_click(),
    );
    if let Some((x, y)) = app.task_pet_context_menu_pos {
        PointBelowOverlay::new(
            content,
            task_pet_context_menu(desktop_i18n::I18n::new(app.desktop_locale), app.can_restore_reminder),
        )
        .show(true)
        .anchor(Point::new(x, y))
        .gap(2.0)
        .on_close(Message::TaskPetContextMenuClosed)
        .close_on_target_click(true)
        .capture_keyboard()
        .into()
    } else {
        content
    }
}

fn pet_control<'a>(app: PetView<'a>) -> Element<'a, Message> {
    let mut content = column![].spacing(PET_CONTROL_GAP).align_x(Alignment::Center);
    if app.task_pet_avatar_kind != TaskPetAvatarKind::None {
        let mut sprite =
            iced::widget::stack![pet_sprite(app, PET_COLLAPSED_SIZE * app.preview_scale)];
        if app.task_pet_avatar_kind == TaskPetAvatarKind::Robot
            && app.active_count > 0
            && !app.is_walking
        {
            sprite = sprite.push(
                container(code_effect(app.status_animation_frame))
                    .center_x(Length::Fill)
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Bottom),
            );
        }
        content = content.push(
            mouse_area(container(sprite).center_x(app.avatar_bounds).center_y(app.avatar_bounds))
                .on_press(Message::TaskPetDragStarted)
                .on_enter(Message::TaskPetRobotHover(true))
                .on_exit(Message::TaskPetRobotHover(false)),
        );
    }
    // Keep the approach region stable while the dock morphs. The cards above
    // are outside this region and keep the dock pinned while open.
    let hover_width = app.avatar_bounds.max(ACTIONS_WIDTH).max(app.dock.width);
    let controls = container(controls::toolbar(app)).center_x(hover_width).height(DOCK_HEIGHT);
    mouse_area(content.push(controls))
        .on_enter(Message::TaskPetDockHover(true))
        .on_exit(Message::TaskPetDockHover(false))
        .into()
}

fn task_pet_context_menu(i18n: desktop_i18n::I18n, can_restore: bool) -> Element<'static, Message> {
    use desktop_assets::assets::Icon;
    use desktop_ui_controls::system_settings_common::{icon_svg, rounded_action_btn_style};

    let action = |icon, label, intent| {
        let icon = icon_svg(icon, 16.0)
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) });
        button(row![icon, text(i18n.__(label)).size(13)].spacing(8).align_y(Alignment::Center))
            .width(Length::Fill)
            .padding([6, 8])
            .style(|theme: &Theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                style.border = Border::default().rounded(8);
                style.background = match status {
                    button::Status::Hovered => {
                        Some(theme.palette().primary.scale_alpha(0.24).into())
                    }
                    button::Status::Pressed => {
                        Some(theme.palette().primary.scale_alpha(0.36).into())
                    }
                    _ => None,
                };
                style
            })
            .on_press(intent)
    };

    let mut actions = column![
        action(Icon::Gear, "选择智能伙伴", Message::TaskPetOpenSettings),
        action(Icon::Sliders, "自定义伙伴大小", Message::TaskPetSizeOpen),
    ];
    if can_restore {
        actions = actions.push(action(Icon::ArrowLeft, "恢复提醒", Message::TaskPetRestoreReminder));
    }
    container(
        actions
            .push(action(Icon::EyeSlash, "隐藏伙伴", Message::TaskPetHideWindow))
            .spacing(2)
            .width(142),
    )
    .padding(5)
    .style(|theme| {
        let mut style = controls::surface(theme, 14.0, false);
        let opacity = if theme.extended_palette().is_dark { 0.72 } else { 0.82 };
        style.background = style.background.map(|background| background.scale_alpha(opacity));
        style
    })
    .into()
}

/// 设置预览与 Mini 共用素材；不绑定窗口、拖动或悬停事件。
pub fn avatar_preview(avatar: TaskPetAvatarKind, size: f32) -> Element<'static, Message> {
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

fn pet_sprite<'a>(app: PetView<'a>, size: f32) -> Element<'a, Message> {
    if !app.is_walking && !app.task_pet_robot_hovered && app.active_count == 0 {
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

fn robot_pet_sprite<'a>(app: PetView<'a>, size: f32) -> Element<'a, Message> {
    let handle = if app.is_walking {
        let frame = app.status_animation_frame % 2;
        if app.facing_left {
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
    app: PetView<'_>,
    normal: ImageHandle,
    left: ImageHandle,
    work: ImageHandle,
) -> ImageHandle {
    if app.is_walking {
        return if app.facing_left { left } else { normal };
    }
    if app.task_pet_robot_hovered {
        return normal;
    }
    if app.active_count > 0 && one_second_pulse(app.status_animation_frame) {
        return work;
    }
    left
}

fn human_motion_lift<'a>(app: PetView<'a>) -> f32 {
    if app.is_walking || app.task_pet_robot_hovered || app.active_count > 0 {
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

pub mod window;
