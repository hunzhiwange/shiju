//! 当前文件的互斥工具浮层；宿主只占工具图标的尺寸。
use super::{
    App, Icon, Message, overlay_icon_button_style, preview_control_tooltip, small_icon_svg,
};
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::system_settings_common::rounded_action_btn_style;
use crate::app::message::editor::{EditorMessage, EditorPopover};
use iced::widget::scrollable;
use iced::widget::{button, column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Theme};

pub(super) fn tools(app: &App) -> Element<'_, Message> {
    let mut tools = row![
        button(small_icon_svg(Icon::Search))
            .padding(4)
            .style(overlay_icon_button_style)
            .on_press(Message::Editor(EditorMessage::OpenSearch))
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);
    for (kind, label) in [
        (EditorPopover::Actions, "操作"),
        (EditorPopover::Settings, "设置"),
        (EditorPopover::Help, "帮助"),
    ] {
        let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
        let icon = match kind {
            EditorPopover::Actions => Icon::ListUl,
            EditorPopover::Settings => Icon::Gear,
            EditorPopover::Help => Icon::QuestionCircle,
        };
        let trigger = button(small_icon_svg(icon))
            .padding(4)
            .style(overlay_icon_button_style)
            .on_press(Message::Editor(EditorMessage::TogglePopover(kind)));
        let show = app.editor_popover == Some(kind)
            && !app.show_preview_context_menu
            && !app.show_preview_settings
            && app.preview_nav_popup.is_none();
        // BelowOverlay 没有键盘捕获接口；PointBelowOverlay 复用现有 Escape
        // 与外部点击拦截，锚点是当前按钮底部，不扩大正文布局。
        tools = tools.push(
            PointBelowOverlay::new(
                preview_control_tooltip(trigger.into(), i18n.__(label)),
                panel(app, kind),
            )
            .anchor(iced::Point::new(0.0, 26.0))
            .gap(4.0)
            .show(show)
            .capture_keyboard()
            .capture_outside_click(true)
            .close_on_target_click(true)
            .on_close(Message::Editor(EditorMessage::ClosePopover)),
        );
    }
    tools.into()
}

fn action<'a>(app: &'a App, label: &'static str, message: EditorMessage) -> Element<'a, Message> {
    button(text(crate::app::i18n::I18n::new(app.desktop_locale).__(label)).size(12))
        .padding([6, 10])
        .width(Length::Fill)
        .style(rounded_action_btn_style)
        .on_press(Message::Editor(message))
        .into()
}

fn panel(app: &App, kind: EditorPopover) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let content: Element<'_, Message> = match kind {
        EditorPopover::Actions => {
            let mut items = column![
                action(app, "搜索", EditorMessage::OpenSearch),
                action(app, "复制", EditorMessage::Copy)
            ]
            .spacing(4);
            let read_only = app.active_preview_path.as_ref().is_some_and(|path| {
                app.preview_tabs.iter().any(|tab| tab.path == *path && tab.read_only)
            });
            if !read_only {
                items = items.push(action(app, "撤销", EditorMessage::Undo)).push(action(
                    app,
                    "重做",
                    EditorMessage::Redo,
                ));
            }
            items.into()
        }
        EditorPopover::Settings => {
            column![action(app, "编辑器设置", EditorMessage::ToggleSettings)].into()
        }
        EditorPopover::Help => column![
            text(i18n.__("帮助")).size(14),
            text(i18n.__("搜索")).size(12),
            text("Ctrl+F / Cmd+F").size(12),
            text(i18n.__("撤销")).size(12),
            text("Ctrl+Z / Cmd+Z").size(12),
        ]
        .spacing(8)
        .into(),
    };
    let scroll = scrollable(container(content).padding([7, 7]).width(Length::Fill))
        .height(Length::Shrink)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ));
    container(scroll)
        .width(240)
        .max_height(360)
        .padding(8)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(Background::Color(palette.background.base.color)),
                text_color: Some(palette.background.base.text),
                border: Border {
                    color: palette.background.strong.color,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(0.2),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                ..Default::default()
            }
        })
        .into()
}
