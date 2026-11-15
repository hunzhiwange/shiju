use iced::{Element, Length, Theme};
use iced::widget::{column, container, row, rule, scrollable};
use iced::widget::scrollable::{Direction, Scrollbar};
use crate::{SETTINGS_SCROLL_GUTTER, SETTINGS_CONTENT_SCROLL};
pub const SETTINGS_CONTENT_MAX_WIDTH: f32 = 860.0;
pub const SELECT_FOCUS_SCOPE: &str = "system-settings-select-focus";
pub fn content_page<'a, Message: 'a>(content: iced::widget::Column<'a, Message>, full_width: bool, owns_scroll: bool) -> Element<'a, Message> {
    let content =
        if owns_scroll { content.height(Length::Fill) } else { content };
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

pub fn shell<'a, Message: 'a>(width: f32, actions: Element<'a, Message>, content: Element<'a, Message>, navigation: impl FnOnce(f32) -> Element<'a, Message>) -> Element<'a, Message> {
    let sidebar_width = (width * 0.24).clamp(112.0, 300.0);
    let divider = rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: theme.palette().text.scale_alpha(0.12), ..rule::default(theme)
    });
    row![navigation(sidebar_width), divider, column![actions, content].width(Length::Fill).height(Length::Fill)].width(Length::Fill).height(Length::Fill).into()
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
