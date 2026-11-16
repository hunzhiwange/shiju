//! Application page layout over host-provided tile content.
use desktop_ui_controls::system_settings_common::{settings_panel_style, settings_modal_overlay};
use iced::{Element, Length};
use iced::widget::{container, column, scrollable};
pub fn view<'a, M: Clone + 'static>(header: Element<'a, M>, grid: Element<'a, M>, active_modal: Option<(Element<'a, M>, M)>) -> Element<'a, M> {
    // 构建基础内容容器
    // 包含头部和磁贴网格的垂直布局
    let header_panel: Element<'a, M> =
        container(header).padding([18, 20]).width(Length::Fill).style(settings_panel_style).into();

    let grid_panel: Element<'a, M> = grid;

    let base_content: Element<'a, M> =
        container(column![header_panel, grid_panel].spacing(14).width(Length::Fill))
            .width(Length::Fill)
            .padding([18, 24])
            .into();

    // 将基础内容包装在可滚动容器中
    // 确保内容超出视口时可以滚动查看
    let base_content: Element<'a, M> =
        scrollable(base_content).width(Length::Fill).height(Length::Fill).into();

    if let Some((modal, close_msg)) = active_modal {
        settings_modal_overlay(Some(base_content), close_msg, modal)
    } else {
        // 没有模态框时，直接返回基础内容
        base_content
    }
}
