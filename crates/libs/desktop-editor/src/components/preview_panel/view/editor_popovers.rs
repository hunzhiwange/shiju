//! 当前文件的互斥工具浮层；宿主只占工具图标的尺寸。
use super::Message;
use super::controls::{overlay_icon_button_style, preview_control_tooltip};
use super::super::styles::small_icon_svg;
use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::PointBelowOverlay;
use super::options::EditorPopover;
use iced::widget::scrollable;
use iced::widget::{button, container, row};
use iced::{Background, Border, Color, Element, Length, Theme};

pub fn tools<'a>(editor: Option<&'a iced_code_editor::CodeEditor>, read_only: bool, i18n: desktop_i18n::I18n, popover: Option<EditorPopover>) -> Element<'a, Message> {
    let search = button(small_icon_svg(Icon::Search))
        .padding(4)
        .style(overlay_icon_button_style)
        .on_press(Message::OpenSearch);
    let mut tools = row![preview_control_tooltip(search.into(), i18n.__("搜索"))]
        .spacing(4)
        .align_y(iced::Alignment::Center);
    if editor.is_some() && !read_only
    {
        let replace = button(small_icon_svg(Icon::ArrowRepeat))
            .padding(4)
            .style(overlay_icon_button_style)
            .on_press(Message::OpenReplace);
        tools = tools.push(preview_control_tooltip(replace.into(), i18n.__("替换")));
    }
    for (kind, label) in [
        (EditorPopover::Actions, "操作"),
        (EditorPopover::Settings, "设置"),
        (EditorPopover::Help, "编辑器使用帮助"),
    ] {
            let icon = match kind {
            EditorPopover::Actions => Icon::ListUl,
            EditorPopover::Settings => Icon::Gear,
            EditorPopover::Help => Icon::QuestionCircle,
        };
        let trigger = button(small_icon_svg(icon))
            .padding(4)
            .style(overlay_icon_button_style)
            .on_press(Message::TogglePopover(kind));
        let show = popover == Some(kind);
        // BelowOverlay 没有键盘捕获接口；PointBelowOverlay 复用现有 Escape
        // 与外部点击拦截，锚点是当前按钮底部，不扩大正文布局。
        tools = tools.push(
            PointBelowOverlay::new(
                preview_control_tooltip(trigger.into(), i18n.__(label)),
                panel(editor, read_only, i18n, kind),
            )
            .anchor(iced::Point::new(0.0, 26.0))
            .gap(4.0)
            .show(show)
            .capture_keyboard()
            .capture_outside_click(true)
            .close_on_target_click(true)
            .on_close(Message::ClosePopover),
        );
    }
    tools.into()
}

fn panel(editor: Option<&iced_code_editor::CodeEditor>, read_only: bool, i18n: desktop_i18n::I18n, kind: EditorPopover) -> Element<'_, Message> {
    let content: Element<'_, Message> = match kind {
        EditorPopover::Actions => super::editor_actions::view(editor, i18n, read_only),
        EditorPopover::Settings => super::editor_options::view(editor, i18n),
        EditorPopover::Help => super::editor_help::view(editor, i18n),
    };
    let scroll: Element<'_, Message> = if kind == EditorPopover::Help {
        content
    } else {
        scrollable(container(content).padding([7, 7]).width(Length::Fill))
            .height(Length::Shrink)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .into()
    };
    container(scroll)
        .width(if kind == EditorPopover::Settings { 240 } else { 340 })
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
