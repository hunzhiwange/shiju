use crate::app::Message;
use iced::advanced::widget::Tree;
use iced::widget::{Space, mouse_area};
use iced::{Element, Length};

use super::drag::{drag_spacer, traffic_light_spacer};

fn element_tree(element: Element<'static, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn mouse_area_tree() -> Tree {
    let element: Element<'static, Message> = mouse_area(Space::new()).into();
    element_tree(element)
}

#[cfg(target_os = "macos")]
#[test]
fn traffic_light_spacer_reserves_macos_window_controls_width() {
    let element = traffic_light_spacer();
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Fixed(75.0));
    assert_eq!(size.height, Length::Fill);
}

#[cfg(target_os = "macos")]
#[test]
fn traffic_light_spacer_wraps_reserved_space_in_drag_target() {
    let tree = element_tree(traffic_light_spacer());

    assert_eq!(tree.tag, mouse_area_tree().tag);
    assert_eq!(tree.children.len(), 1);
    assert!(tree.children[0].children.is_empty());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn traffic_light_spacer_collapses_without_macos_window_controls() {
    let element = traffic_light_spacer();
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Fixed(0.0));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn traffic_light_spacer_stays_non_interactive_without_window_controls() {
    let tree = element_tree(traffic_light_spacer());

    assert!(tree.children.is_empty());
}

#[test]
fn drag_spacer_fills_available_top_bar_space() {
    let element = drag_spacer();
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Fill);
    assert_eq!(size.height, Length::Fill);
}

#[test]
fn drag_spacer_wraps_content_in_drag_target() {
    let tree = element_tree(drag_spacer());

    assert_eq!(tree.tag, mouse_area_tree().tag);
    assert_eq!(tree.children.len(), 1);
    assert!(tree.children[0].children.is_empty());
}
