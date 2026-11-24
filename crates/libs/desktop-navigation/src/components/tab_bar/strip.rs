use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::widget::{Row, Space, button, column, container, row, scrollable, svg};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};

use super::super::top_bar::{Menu, widgets};
use super::{Message, State, add_button_style, tab_btn};

const TAB_SPACING: f32 = 3.0;
const ADD_WIDTH: f32 = 22.0;
const OVERFLOW_WIDTH: f32 = 30.0;

struct Tab {
    id: String,
    title: String,
    active: bool,
    hovered: bool,
}

pub(super) struct TabStrip {
    tabs: Vec<Tab>,
    show_add: bool,
    overflow_open: bool,
    content: Element<'static, Message>,
}

impl TabStrip {
    pub(super) fn new(app: &State<'_>) -> Self {
        let i18n = desktop_i18n::I18n::new(app.desktop_locale);
        let tabs = app
            .open_tabs
            .iter()
            .filter(|tab| !tab.id.starts_with("find:"))
            .map(|tab| Tab {
                title: desktop_i18n::other::tab_label(i18n, &tab.id, &tab.title),
                active: app.active_tab_id.as_deref() == Some(tab.id.as_str()),
                hovered: app.hovered_tab_id.as_deref() == Some(tab.id.as_str()),
                id: tab.id.clone(),
            })
            .collect();
        Self {
            tabs,
            show_add: !app.open_tabs.iter().any(|tab| tab.id == "apps"),
            overflow_open: app.overflow_open,
            content: Space::new().into(),
        }
    }
}

pub(super) fn visible_tab_count(
    widths: impl IntoIterator<Item = f32>,
    available: f32,
    show_add: bool,
) -> usize {
    let mut occupied = if show_add { ADD_WIDTH } else { 0.0 };
    let mut visible = 0;
    let mut visible_with_overflow = 0;
    for width in widths {
        if show_add || visible > 0 {
            occupied += TAB_SPACING;
        }
        occupied += width;
        if occupied > available {
            return visible_with_overflow;
        }
        visible += 1;
        if occupied + TAB_SPACING + OVERFLOW_WIDTH <= available {
            visible_with_overflow = visible;
        }
    }
    visible
}

impl Widget<Message, Theme, Renderer> for TabStrip {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn diff(&self, _tree: &mut widget::Tree) {
        // The row is reconciled after its visible prefix is known during layout.
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let available = limits.max();
        let measurement_limits =
            layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, available.height));
        let mut buttons = Vec::new();
        let widths = self.tabs.iter().map(|tab| {
            let mut button = tab_btn(
                tab.id.clone(),
                tab.title.clone(),
                tab.active,
                tab.hovered,
                Message::TabSelected(tab.id.clone()),
            );
            // Measure before the row can compress a label to fit the remaining space.
            let mut state = widget::Tree::new(button.as_widget());
            let width = button
                .as_widget_mut()
                .layout(&mut state, renderer, &measurement_limits)
                .size()
                .width;
            buttons.push(button);
            width
        });
        let visible = visible_tab_count(widths, available.width, self.show_add);
        buttons.truncate(visible);
        let hidden = &self.tabs[visible..];
        if self.show_add
            && (hidden.is_empty() || available.width >= ADD_WIDTH + TAB_SPACING + OVERFLOW_WIDTH)
        {
            buttons.push(icon_button(Icon::Plus, ADD_WIDTH, Message::OpenApps, false).into());
        }
        if !hidden.is_empty() {
            buttons.push(overflow_button(hidden, self.overflow_open));
        }
        self.content = Row::with_children(buttons)
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(TAB_SPACING)
            .align_y(iced::Alignment::Center)
            .into();
        tree.diff_children(std::slice::from_ref(&self.content));
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if self.overflow_open
            && matches!(
                event,
                Event::Window(iced::window::Event::Resized(_) | iced::window::Event::Unfocused)
            )
        {
            shell.publish(Message::ToggleMenu(None));
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn icon_button(
    icon: Icon,
    width: f32,
    action: Message,
    highlighted: bool,
) -> button::Button<'static, Message> {
    button(
        container(
            svg(assets::get_icon(icon))
                .width(14)
                .height(14)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) }),
        )
        .center(Length::Fill),
    )
    .on_press(action)
    .style(move |theme, status| {
        let mut style = add_button_style(theme, status);
        if highlighted && style.background.is_none() {
            style.background = Some(theme.palette().text.scale_alpha(0.12).into());
        }
        style
    })
    .padding(0)
    .width(width)
    .height(22)
}

fn overflow_button(hidden: &[Tab], open: bool) -> Element<'static, Message> {
    let trigger = icon_button(
        Icon::ChevronDown,
        OVERFLOW_WIDTH,
        Message::ToggleMenu(Some(Menu::TabOverflow)),
        open || hidden.iter().any(|tab| tab.active),
    );
    if !open {
        return trigger.into();
    }
    let items = column(hidden.iter().map(overflow_item)).spacing(2);
    let menu = container(
        scrollable(container(items).padding([0, 7]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Shrink),
    )
    .padding([4, 0])
    .width(Length::Fixed(260.0))
    .max_height(320)
    .style(widgets::menu_container_style);
    BelowOverlay::new(trigger, menu)
        .show(true)
        .gap(4.0)
        .align_right(true)
        .on_close(Message::ToggleMenu(None))
        .dismiss_on_escape()
        .into()
}

fn overflow_item(tab: &Tab) -> Element<'static, Message> {
    let mark: Element<'static, Message> = if tab.active {
        widgets::icon_svg(Icon::Check).style(|theme, _| widgets::icon_svg_style(theme, true)).into()
    } else {
        Space::new().width(14).into()
    };
    let select = button(
        row![mark, SingleLineLabel::new(tab.title.clone(), 13.0)]
            .spacing(6)
            .align_y(iced::Alignment::Center),
    )
    .on_press(Message::MenuAction(Box::new(Message::TabSelected(tab.id.clone()))))
    .style(|theme, status| widgets::menu_item_button_style(theme, status, true))
    .width(Length::Fill)
    .padding([5, 6]);
    let mut item = row![select].spacing(2).align_y(iced::Alignment::Center);
    if tab.id != "home" {
        item = item.push(icon_button(
            Icon::X,
            22.0,
            Message::MenuAction(Box::new(Message::TabClosed(tab.id.clone()))),
            false,
        ));
    }
    item.into()
}
