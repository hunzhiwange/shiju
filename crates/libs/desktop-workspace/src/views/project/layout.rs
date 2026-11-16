use iced::{Element, Length};
use iced::widget::column;
pub mod bottom;
pub const WORKSPACE_BOTTOM_GAP: f32 = 6.0;
pub fn build_base_layout<'a, Message: 'a>(workspace: Element<'a, Message>, bottom: Element<'a, Message>, hide_bottom: bool) -> Element<'a, Message> {
    if hide_bottom {
        workspace
    } else {
        column![
            iced::widget::container(workspace)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding::default().bottom(WORKSPACE_BOTTOM_GAP)),
            bottom
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

pub mod styles;
pub mod chat;
pub mod chrome;
pub mod drag;
pub mod main;
pub mod sessions_panel;
pub mod sidebar;
pub mod work;
pub mod work_resources;

pub mod work_sidebar;

/// Compose the code workspace from domain surfaces and host-owned cross-domain content.
pub fn code_workspace<'a, M: Clone + 'a>(
    left: Element<'a, M>,
    resize: Option<Element<'a, M>>,
    main: Element<'a, M>,
    overlay: Element<'a, M>,
    hover: Option<Element<'a, M>>,
    close_hover: M,
    bottom: Option<Element<'a, M>>,
    width: f32,
    rail: f32,
    scale: f32,
    radius: f32,
    spacing: f32,
) -> Element<'a, M> {
    let main = iced::widget::container(main).width(Length::Fill).height(Length::Fill);
    let base: Element<'a, M> = column![main].height(Length::Fill).into();
    let right = iced::widget::stack![base, overlay].width(Length::Fill).height(Length::Fill);
    let right = chrome::right_column_chrome(right.into(), radius, false);
    let workspace = if let Some(resize) = resize {
        iced::widget::row![left, resize, right].spacing(spacing).into()
    } else {
        sidebar::hover_overlay_layout(left, right, hover, close_hover, width, rail, scale, spacing)
    };
    let hide_bottom = bottom.is_none();
    build_base_layout(
        workspace,
        bottom.unwrap_or_else(|| iced::widget::Space::new().into()),
        hide_bottom,
    )
}
