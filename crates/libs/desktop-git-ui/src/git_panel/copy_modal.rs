//! Git 复制与编辑弹窗。彩色编辑器由宿主注入。
use iced::{Element, Background, Border, Color, Length};
use iced::widget::{Space, container, column, button, text, text_editor, stack};
use desktop_ui_controls::system_settings_common::settings_text_editor_style;
#[derive(Debug, Clone)]
pub enum CopyMessage<E> { External(E), ToggleCopyModalColored(bool), InsertCopyModalToChatCurrent, CopyModalCopyCurrent, CloseCopyModal, CopyModalEditorAction(text_editor::Action) }
pub fn copy_overlay<'a, E: Clone + 'static>(locale: config_ui::DesktopLocale, use_color: bool, editor_content: &'a text_editor::Content, colored_editor: Option<Element<'a, CopyMessage<E>>>, base: Element<'a, CopyMessage<E>>) -> Element<'a, CopyMessage<E>> {
    // 半透明遮罩层
    let overlay =
        container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| {
            iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
                ..Default::default()
            }
        });

    // 彩色/纯文本切换开关
    let colored_toggle = iced::widget::toggler(use_color)
        .label(desktop_i18n::I18n::new(locale).__("彩色"))
        .on_toggle(|v| CopyMessage::ToggleCopyModalColored(v));

    // 模态框头部：标题、开关、操作按钮
    let modal_header = iced::widget::row![
        text(desktop_i18n::I18n::new(locale).__("复制 / 编辑")).size(16),
        colored_toggle,
        Space::new().width(Length::Fill),
        button(text(desktop_i18n::I18n::new(locale).__("插入到Chat")))
            .on_press(CopyMessage::InsertCopyModalToChatCurrent)
            .padding([6, 10]),
        button(text(desktop_i18n::I18n::new(locale).__("复制")))
            .on_press(CopyMessage::CopyModalCopyCurrent)
            .padding([6, 10]),
        button(text(desktop_i18n::I18n::new(locale).__("关闭")))
            .on_press(CopyMessage::CloseCopyModal)
            .padding([6, 10]),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    // 根据是否使用彩色模式，渲染不同的编辑器
    let editor: Element<'_, CopyMessage<E>> = if use_color {
        // 彩色模式：使用代码高亮编辑器
        let e = colored_editor.expect("colored copy editor");
        container(e)
            .width(Length::Fill)
            .height(Length::Fixed(420.0))
            .style(container::rounded_box)
            .padding(6)
            .into()
    } else {
        // 纯文本模式：使用普通文本编辑器
        text_editor(editor_content)
            .placeholder(
                desktop_i18n::I18n::new(locale)
                    .__("可编辑文本；支持鼠标选择后 ⌘C 直接复制"),
            )
            .on_action(|a| CopyMessage::CopyModalEditorAction(a))
            .height(Length::Fixed(420.0))
            .padding(12)
            .font(iced::Font::with_name("Noto Sans CJK SC"))
            .style(settings_text_editor_style)
            .into()
    };

    // 组合模态框内容
    let modal = container(column![modal_header, editor].spacing(12))
        .padding(16)
        .width(Length::Fixed(860.0))
        .style(|theme: &iced::Theme| iced::widget::container::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            border: Border {
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
                radius: 12.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.25),
                offset: iced::Vector::new(0.0, 8.0),
                blur_radius: 18.0,
            },
            ..Default::default()
        });

    // 返回带模态框的堆叠视图
    stack![
        base,
        overlay,
        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
    ]
    .into()
}
