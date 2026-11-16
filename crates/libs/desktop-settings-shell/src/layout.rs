use crate::{SETTINGS_CONTENT_SCROLL, SETTINGS_SCROLL_GUTTER};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{column, container, row, rule, scrollable};
use iced::{Element, Length, Theme};
pub const SETTINGS_CONTENT_MAX_WIDTH: f32 = 860.0;
pub const SELECT_FOCUS_SCOPE: &str = "system-settings-select-focus";
pub fn content_page<'a, Message: 'a>(
    content: iced::widget::Column<'a, Message>,
    full_width: bool,
    owns_scroll: bool,
) -> Element<'a, Message> {
    let content = if owns_scroll { content.height(Length::Fill) } else { content };
    let content = if full_width {
        container(content).padding([0, SETTINGS_SCROLL_GUTTER]).width(Length::Fill)
    } else {
        centered_content(content)
    };
    let page: Element<'_, Message> = if owns_scroll {
        content.height(Length::Fill).into()
    } else {
        scrollable(content)
            .id(SETTINGS_CONTENT_SCROLL)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill)
            .into()
    };

    container(page).width(Length::Fill).height(Length::Fill).clip(true).into()
}

pub fn centered_content<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(container(content).width(Length::Fill).max_width(SETTINGS_CONTENT_MAX_WIDTH))
        .padding([0, SETTINGS_SCROLL_GUTTER])
        .center_x(Length::Fill)
}

pub fn shell<'a, Message: 'a>(
    width: f32,
    actions: Element<'a, Message>,
    content: Element<'a, Message>,
    navigation: impl FnOnce(f32) -> Element<'a, Message>,
) -> Element<'a, Message> {
    let sidebar_width = (width * 0.24).clamp(112.0, 300.0);
    let divider = rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: theme.palette().text.scale_alpha(0.12),
        ..rule::default(theme)
    });
    row![
        navigation(sidebar_width),
        divider,
        column![actions, content].width(Length::Fill).height(Length::Fill)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
pub fn traverse_select_focus<Message: Send + 'static>(reverse: bool) -> iced::Task<Message> {
    use iced::advanced::widget::operation::{focusable, scope};
    use iced::advanced::widget::{Id, operate};

    let scope_id = Id::from(SELECT_FOCUS_SCOPE);
    if reverse {
        operate(scope(scope_id, focusable::focus_previous::<()>())).discard()
    } else {
        operate(scope(scope_id, focusable::focus_next::<()>())).discard()
    }
}

use crate::SystemTab;
pub fn tabbed_page_owns_save_error(active_tab: SystemTab) -> bool {
    matches!(
        active_tab,
        SystemTab::Models
            | SystemTab::Cron
            | SystemTab::Agents
            | SystemTab::Channels
            | SystemTab::Memory
            | SystemTab::Autonomy
            | SystemTab::Security
            | SystemTab::Gateway
            | SystemTab::WebSearch
            | SystemTab::HttpRequest
            | SystemTab::Browser
            | SystemTab::Proxy
            | SystemTab::Tunnel
    )
}

/// Mobile settings chrome; effects are chosen by the host.
pub fn mobile_shell<'a, Message: Clone + 'a>(
    content: Element<'a, Message>,
    help: Option<Element<'a, Message>>,
    back: Message,
    retry: Option<Message>,
) -> Element<'a, Message> {
    use desktop_ui_controls::system_settings_common::{
        rounded_action_btn_style, settings_muted_text_style,
    };
    use iced::widget::{button, text};
    let back =
        button(text("返回分类")).padding([10, 12]).style(rounded_action_btn_style).on_press(back);
    let mut header = row![back].spacing(8).align_y(iced::Alignment::Center);
    if let Some(help) = help {
        header = header.push(help);
    }
    let mut page = column![header, content].spacing(8).width(Length::Fill).height(Length::Fill);
    if let Some(retry) = retry {
        page = page.push(
            column![
                text(
                    "有效输入自动保存；保存不代表已连接。连接失败时检查地址、认证及浏览器跨域配置。"
                )
                .size(12)
                .style(settings_muted_text_style),
                button(text("刷新连接 / 重试"))
                    .padding([10, 12])
                    .style(rounded_action_btn_style)
                    .on_press(retry),
            ]
            .spacing(6),
        );
    }
    page.into()
}
