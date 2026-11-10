//! 系统设置页面复用的通用控件、样式与辅助能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::{settings_field_label, settings_hover_help_label};
use crate::app::Message;
use crate::app::components::system_settings_common::styles::{
    settings_modal_backdrop_style, settings_modal_card_style, settings_muted_text_style,
    settings_panel_style,
};
use crate::app::components::system_settings_common::theme::is_dark_theme;
use crate::app::i18n::I18n;
use iced::advanced::{Layout, Widget, layout, renderer, widget::Tree};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, column, container, mouse_area, opaque, responsive, row, scrollable, stack, text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Rectangle, Renderer, Size,
    Theme, mouse,
};

use std::borrow::Cow;

pub(super) const SETTINGS_SECTION_CONTENT_PADDING: Padding = Padding::ZERO;

/// 构建或处理 `settings_panel` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_panel<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(content.into())
        .padding([18, 20])
        .width(Length::Fill)
        .style(settings_content_panel_style)
}

pub(super) fn settings_content_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = 9.0.into();
    style
}

/// 构建系统设置一级分区的无框内容容器。
pub fn settings_section_content<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(content.into())
        .padding(SETTINGS_SECTION_CONTENT_PADDING)
        .width(Length::Fill)
        .style(settings_section_content_style)
}

pub(super) fn settings_section_content_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style::default()
}

/// 常规设置样式的页面标题与说明。
pub fn settings_page_heading<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    column![text(title).size(28), text(description).size(13).style(settings_muted_text_style)]
        .spacing(8)
        .into()
}

/// 圆角设置分组上方的标题与说明。
pub fn settings_group_heading<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    column![text(title).size(17), text(description).size(13).style(settings_muted_text_style)]
        .spacing(6)
        .into()
}

/// 页面说明在问号卡片内显示，保留页面标题的字号。
pub(crate) fn settings_page_heading_with_help<'a>(
    i18n: I18n,
    theme: &Theme,
    title: &'a str,
    description: impl Into<Cow<'a, str>>,
) -> Element<'a, Message> {
    settings_hover_help_label(i18n, theme, title, text(title).size(28), description).wrap().into()
}

/// 分组说明在问号卡片内显示，保留分组标题的字号。
pub(crate) fn settings_group_heading_with_help<'a, 'theme>(
    i18n: I18n,
    theme: impl Into<Option<&'theme Theme>>,
    title: &'a str,
    description: impl Into<Cow<'a, str>>,
) -> Element<'a, Message> {
    settings_hover_help_label(i18n, theme, title, text(title).size(17), description).wrap().into()
}

/// 承载带分隔线的设置行，横向留白与行内间距独立。
pub fn settings_group_card<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(content.into()).padding([0, 22]).width(Length::Fill).style(settings_group_card_style)
}

fn settings_group_card_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.base.color.into()),
        border: Border {
            color: palette.background.strong.color.scale_alpha(0.55),
            width: 1.0,
            radius: 18.0.into(),
        },
        ..Default::default()
    }
}

/// 将说明放在标签下方，并使控件与文字区垂直居中。
pub fn settings_row<'a>(
    label: &'a str,
    description: &'a str,
    control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_layout(label, description, container(control.into()).align_right(control_width))
        .into()
}

/// 在窄内容区为说明保留一半可用宽度，宽屏仍限制控件的最大宽度。
pub(crate) fn settings_row_with_flexible_control<'a>(
    label: &'a str,
    description: &'a str,
    max_control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_flexible_row(label, description, max_control_width, control).into()
}

pub(crate) fn settings_flexible_row<'a>(
    label: &'a str,
    description: &'a str,
    max_control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    settings_flexible_row_with_label(
        settings_row_labels(label, description).width(Length::Fill),
        max_control_width,
        control,
    )
}

/// 字段说明使用问号，控件可继续显示错误和其它常驻提示。
pub(crate) fn settings_flexible_row_with_help<'a>(
    i18n: I18n,
    theme: &Theme,
    label: &'a str,
    description: impl Into<Cow<'a, str>>,
    max_control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let label =
        settings_hover_help_label(i18n, theme, label, settings_field_label(label), description)
            .width(Length::Fill)
            .wrap();
    settings_flexible_row_with_label(label, max_control_width, control)
}

fn settings_flexible_row_with_label<'a>(
    label: impl Into<Element<'a, Message>>,
    max_control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let control = container(control.into()).align_right(Length::Fill).max_width(max_control_width);
    settings_row_with_label(label, container(control).align_right(Length::Fill))
}

fn settings_row_layout<'a>(
    label: &'a str,
    description: &'a str,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    settings_row_with_label(settings_row_labels(label, description).width(Length::Fill), control)
}

fn settings_row_with_label<'a>(
    label: impl Into<Element<'a, Message>>,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(row![label.into(), control.into()].spacing(24).align_y(Alignment::Center))
        .padding([20, 0])
        .width(Length::Fill)
}

fn settings_row_labels<'a>(
    label: &'a str,
    description: &'a str,
) -> iced::widget::Column<'a, Message> {
    let labels = column![settings_field_label(label)].spacing(6);
    if description.is_empty() {
        labels
    } else {
        labels.push(text(description).size(13).style(settings_muted_text_style))
    }
}

/// 窄表单中为单行控件保留全宽，空间足够时采用常规设置的左右布局。
pub(crate) fn settings_responsive_row<'a, Control>(
    label: &'a str,
    description: &'a str,
    control: impl Fn() -> Control + 'a,
) -> Element<'a, Message>
where
    Control: Into<Element<'a, Message>>,
{
    responsive(move |size| {
        if size.width >= 600.0 {
            settings_flexible_row(label, description, 460.0, control()).padding([6, 0]).into()
        } else {
            column![settings_row_labels(label, description), control().into()]
                .spacing(6)
                .width(Length::Fill)
                .into()
        }
    })
    .height(Length::Shrink)
    .into()
}

/// 弹窗标题占用可用宽度，操作区和关闭按钮固定在右侧。
pub fn settings_modal_header<'a>(
    title: impl Into<Element<'a, Message>>,
    actions: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![container(title.into()).width(Length::Fill), actions.into()]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

/// 只滚动弹窗内容，将标题区和可选操作区固定在卡片内。
pub fn settings_modal_layout<'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    settings_modal_layout_with_scroll_id(header, body, footer, None)
}

/// 保留可定位的内容区，供需要恢复或检查滚动位置的调用者使用。
pub(crate) fn settings_modal_layout_with_scroll_id<'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
    scroll_id: Option<&'static str>,
) -> iced::widget::Container<'a, Message> {
    let body = scrollable(container(body.into()).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .width(Length::Fill)
        .height(Length::Fill);
    let body = match scroll_id {
        Some(id) => body.id(id),
        None => body,
    };
    let mut content = column![header.into(), body].spacing(16).width(Length::Fill);
    if let Some(footer) = footer {
        content = content.push(footer);
    }
    settings_modal_card(content.height(Length::Fill))
}

/// 构建或处理 `settings_modal_card` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_modal_card<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(content.into()).padding([22, 24]).style(settings_modal_card_style)
}

/// 构建或处理 `settings_modal_overlay` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_modal_overlay<'a>(
    base: Option<Element<'a, Message>>,
    close_message: Message,
    card: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    modal_overlay(base, close_message, card.into(), false)
}

/// Keep the background at the same tree position as the modal opens and closes.
pub(crate) fn settings_optional_modal_overlay<'a>(
    base: Element<'a, Message>,
    modal: Option<(Message, Element<'a, Message>)>,
) -> Element<'a, Message> {
    if let Some((close, card)) = modal {
        settings_modal_overlay(Some(base), close, card)
    } else {
        stack![base].into()
    }
}

/// 让当前弹窗通过 Escape 发送和关闭按钮相同的取消消息。
pub(crate) fn settings_modal_overlay_with_escape<'a>(
    base: Element<'a, Message>,
    close_message: Message,
    card: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    stack![
        settings_modal_overlay(Some(base), close_message.clone(), card),
        Element::new(ModalEscape(close_message)),
    ]
    .into()
}

struct ModalEscape(Message);

impl Widget<Message, Theme, Renderer> for ModalEscape {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&mut self, _: &mut Tree, _: &Renderer, _: &layout::Limits) -> layout::Node {
        layout::Node::new(Size::ZERO)
    }

    fn draw(
        &self,
        _: &Tree,
        _: &mut Renderer,
        _: &Theme,
        _: &renderer::Style,
        _: Layout<'_>,
        _: mouse::Cursor,
        _: &Rectangle,
    ) {
    }

    fn update(
        &mut self,
        _: &mut Tree,
        event: &iced::Event,
        _: Layout<'_>,
        _: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _: &Rectangle,
    ) {
        use iced::keyboard::{Event, Key, key::Named};
        if let iced::Event::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
            repeat,
            ..
        }) = event
        {
            shell.capture_event();
            if !repeat {
                shell.publish(self.0.clone());
            }
        }
    }
}

/// 将遮罩点击映射为取消，同时隔离底层控件的事件与焦点。
pub(crate) fn settings_modal_overlay_with_backdrop_dismiss<'a>(
    base: Element<'a, Message>,
    close_message: Message,
    card: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    modal_overlay(Some(base), close_message, card.into(), true)
}

fn modal_overlay<'a>(
    base: Option<Element<'a, Message>>,
    close_message: Message,
    card: Element<'a, Message>,
    dismiss_on_backdrop: bool,
) -> Element<'a, Message> {
    let overlay = opaque(
        mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .style(settings_modal_backdrop_style),
        )
        .on_press(close_message.clone()),
    );

    let card = if dismiss_on_backdrop { opaque(card) } else { card };
    let centered = container(card).center_x(Length::Fill).center_y(Length::Fill);
    let modal_layer: Element<'_, Message> =
        if dismiss_on_backdrop { centered.padding(12).into() } else { opaque(centered) };

    let modal_stack: Element<'a, Message> = stack![overlay, modal_layer].into();

    if let Some(base) = base {
        stack![Element::new(ModalBackground(base)), modal_stack].into()
    } else {
        modal_stack
    }
}

// Paint the underlying view without exposing its overlays, focus targets, or events.
struct ModalBackground<'a>(Element<'a, Message>);

impl Widget<Message, Theme, Renderer> for ModalBackground<'_> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.0.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.0.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.0.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.0.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.0.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.0.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.0.as_widget_mut().layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.0.as_widget().draw(
            tree,
            renderer,
            theme,
            style,
            layout,
            mouse::Cursor::Unavailable,
            viewport,
        );
    }
}

/// 构建或处理 `settings_section_card` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_section_card<'a>(
    title: &'a str,
    description: &'a str,
) -> iced::widget::Container<'a, Message> {
    container(
        row![text(title).size(14), text(description).size(12).style(settings_muted_text_style),]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .padding([14, 16])
    .width(Length::Fill)
    .style(settings_section_card_style)
}

/// 构建或处理 `settings_page_intro` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_page_intro<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    row![text(title).size(18), text(description).size(12).style(settings_muted_text_style),]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
}

/// 构建或处理 `settings_divider` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_divider() -> Element<'static, Message> {
    container(Space::new()).width(Length::Fill).height(1.0).style(settings_divider_style).into()
}

/// 构建或处理 `settings_value_badge` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_value_badge(value: impl ToString) -> Element<'static, Message> {
    container(text(value.to_string()).size(13))
        .padding([7, 12])
        .style(settings_value_badge_style)
        .into()
}

/// 将字段提示紧贴在对应控件下方，避免提示脱离字段落到面板底部。
pub fn settings_control_with_hint<'a>(
    control: impl Into<Element<'a, Message>>,
    hint: &'a str,
) -> Element<'a, Message> {
    column![control.into(), text(hint).size(12).style(settings_muted_text_style)]
        .spacing(6)
        .width(Length::Fill)
        .into()
}

/// 构建或处理 `settings_error_banner` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_error_banner<'a>(
    message: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, Message> {
    container(text(message).size(13))
        .padding([10, 12])
        .width(Length::Fill)
        .style(settings_error_banner_style)
        .into()
}

/// 构建或处理 `settings_success_banner` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_success_banner<'a>(
    message: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, Message> {
    container(text(message).size(13))
        .padding([10, 12])
        .width(Length::Fill)
        .style(settings_success_banner_style)
        .into()
}

pub(super) fn settings_section_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.weak.color.scale_alpha(0.20)
        } else {
            Color::from_rgba8(246, 248, 252, 0.96)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.8)
            } else {
                Color::from_rgba8(15, 23, 42, 0.06)
            },
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn settings_divider_style(theme: &Theme) -> iced::widget::container::Style {
    let color = if is_dark_theme(theme) {
        theme.extended_palette().background.strong.color.scale_alpha(0.72)
    } else {
        Color::from_rgba8(15, 23, 42, 0.06)
    };
    iced::widget::container::Style {
        background: Some(Background::Color(color)),
        ..Default::default()
    }
}

pub(super) fn settings_value_badge_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(if is_dark {
            palette.background.weak.color.scale_alpha(0.36)
        } else {
            Color::from_rgba8(246, 248, 252, 0.96)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.84)
            } else {
                Color::from_rgba8(15, 23, 42, 0.08)
            },
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn settings_error_banner_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        text_color: Some(if is_dark_theme(theme) {
            palette.background.base.text
        } else {
            palette.danger.base.color
        }),
        background: Some(Background::Color(palette.danger.weak.color.scale_alpha(0.18))),
        border: Border {
            width: 1.0,
            color: palette.danger.base.color.scale_alpha(0.45),
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn settings_success_banner_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        text_color: Some(if is_dark_theme(theme) {
            theme.palette().text
        } else {
            palette.success.base.color
        }),
        background: Some(Background::Color(palette.success.weak.color.scale_alpha(0.18))),
        border: Border {
            width: 1.0,
            color: palette.success.base.color.scale_alpha(0.45),
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}
