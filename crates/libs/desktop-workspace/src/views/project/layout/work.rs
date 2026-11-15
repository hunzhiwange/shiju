use iced::{Element,Length};
use iced::widget::{Space,container,mouse_area,row,stack};
use super::super::handles::HResizeHandle;
use super::chrome::right_column_chrome;
#[derive(Clone, Copy)]
pub struct WorkLayout {
pub work_resources_visible: bool,
pub work_chat_fullscreen: bool,
pub work_chat_half_fullscreen: bool,
pub work_resource_fullscreen: bool,
pub work_resource_half_fullscreen: bool,
pub work_split_ratio: f32,
}
pub fn build_work_layout<'a, Message: Clone + 'a>(state: WorkLayout, left_sidebar: Element<'a, Message>, chat: Element<'a, Message>, resources: Element<'a, Message>, overlay: Element<'a, Message>, bottom: Option<Element<'a, Message>>, split_drag: Message, sidebar_drag: Message, corner_radius:f32, spacing:f32) -> Element<'a, Message> {
    let chat = container(chat)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding::default().right(3.0));
    let resources = container(resources)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding::default().left(3.0));

    let content: Element<'_, Message> =
        if !state.work_resources_visible || state.work_chat_fullscreen || state.work_chat_half_fullscreen
        {
            chat.into()
        } else if state.work_resource_fullscreen || state.work_resource_half_fullscreen {
            resources.into()
        } else {
            let left_portion = (state.work_split_ratio.clamp(0.25, 0.75) * 1_000.0) as u16;
            let right_portion = 1_000u16.saturating_sub(left_portion).max(1);
            let divider = mouse_area(HResizeHandle)
                .on_press(split_drag);
            row![
                chat.width(Length::FillPortion(left_portion.max(1))),
                divider,
                resources.width(Length::FillPortion(right_portion)),
            ]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

    let right_column_base: Element<'_, Message> = content;
    let right_overlay = overlay;
    let right_column =
        stack![right_column_base, right_overlay].width(Length::Fill).height(Length::Fill);
    let right_column = right_column_chrome(right_column.into(), corner_radius, true);

    if state.work_chat_fullscreen || state.work_resource_fullscreen {
        return right_column;
    }
    let sidebar_divider =
        mouse_area(HResizeHandle).on_press(sidebar_drag);
    let workspace = row![left_sidebar, sidebar_divider, right_column].spacing(spacing);

    let hide_bottom = bottom.is_none();
    super::build_base_layout(workspace.into(), bottom.unwrap_or_else(|| Space::new().into()), hide_bottom)
}
