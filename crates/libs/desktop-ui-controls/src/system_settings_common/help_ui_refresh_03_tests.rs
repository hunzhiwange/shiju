use super::panels::{settings_modal_card, settings_modal_overlay};
use super::select::settings_select;
#[derive(Clone)]
enum Message { None, GatewayHealthTick }
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, widget::Tree};
use iced::widget::{container, text};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Vector, mouse};

fn update(
    element: &mut Element<'_, Message>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &Renderer,
    viewport: &Rectangle,
    event: Event,
    cursor: mouse::Cursor,
) -> Vec<Message> {
    let mut messages = Vec::new();
    element.as_widget_mut().update(
        tree,
        &event,
        layout,
        cursor,
        renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut messages),
        viewport,
    );
    messages
}

#[tokio::test]
async fn ui_refresh_03_layout_and_interaction_hides_background_menu() {
    let renderer = Renderer::new(Font::DEFAULT, 16.0.into(), Some("tiny-skia")).await.unwrap();
    let viewport = Rectangle::with_size(Size::new(420.0, 440.0));
    let limits = layout::Limits::new(Size::ZERO, viewport.size());
    let mut modal = settings_modal_overlay(
        Some(
            container(settings_select(["原值", "第二项"], Some("原值"), |_| {
                Message::GatewayHealthTick
            }))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into(),
        ),
        Message::None,
        settings_modal_card(text("設定の確認")).width(240).height(180),
    );
    let mut tree = Tree::new(modal.as_widget());
    let node = modal.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let root = Layout::new(&node);
    let base = root.children().next().unwrap();
    let select = base.children().next().unwrap();
    let cursor = mouse::Cursor::Available(Point::new(select.bounds().center_x(), 16.0));

    // Recreate the state of a selector that was already open when the modal appeared.
    let mut underlying: Element<'_, Message> =
        settings_select(["原值", "第二项"], Some("原值"), |_| Message::GatewayHealthTick).into();
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        assert!(
            update(
                &mut underlying,
                &mut tree.children[0],
                select,
                &renderer,
                &viewport,
                Event::Mouse(event),
                cursor,
            )
            .is_empty()
        );
    }
    assert!(
        underlying
            .as_widget_mut()
            .overlay(&mut tree.children[0], select, &renderer, &viewport, Vector::ZERO,)
            .is_some(),
        "the background select must really be open"
    );
    assert!(
        modal
            .as_widget_mut()
            .overlay(&mut tree, root, &renderer, &viewport, Vector::ZERO)
            .is_none(),
        "an open background select must stay behind the modal"
    );
}
