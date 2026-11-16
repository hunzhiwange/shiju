use crate::panel::{chat_message_column, chat_virtual_content, conversation_reading_column};
use crate::virtualizer::ChatVirtualizer;
type Message = ();
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
#[cfg(not(target_arch = "wasm32"))]
use iced::{Font, Pixels, Size};
#[test]
fn chat_message_column_fills_available_width() {
    let element: iced::Element<'_, Message> = chat_message_column().into();

    assert_eq!(element.as_widget().size().width, iced::Length::Fill);
}
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn conversation_shell_uses_shared_reading_column_width() {
    const EXPECTED_MAX_WIDTH: f32 = 880.0;
    const HORIZONTAL_PADDING: f32 = 18.0;

    async fn message_column_bounds(viewport_width: f32) -> iced::Rectangle {
        let renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("tiny-skia headless renderer should be available");
        let mut element: iced::Element<'_, Message> =
            iced::widget::container(chat_message_column())
                .width(iced::Length::Fill)
                .center_x(iced::Length::Fill)
                .padding(iced::Padding {
                    top: 0.0,
                    right: HORIZONTAL_PADDING,
                    bottom: 0.0,
                    left: HORIZONTAL_PADDING,
                })
                .into();
        let limits = layout::Limits::new(Size::ZERO, Size::new(viewport_width, 600.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);

        Layout::new(&node)
            .children()
            .next()
            .expect("conversation shell should contain the message column")
            .bounds()
    }

    let wide = message_column_bounds(1_200.0).await;
    assert_eq!(wide.width, EXPECTED_MAX_WIDTH);
    assert_eq!(wide.x, (1_200.0 - EXPECTED_MAX_WIDTH) / 2.0);

    let narrow = message_column_bounds(540.0).await;
    assert_eq!(narrow.width, 540.0 - HORIZONTAL_PADDING * 2.0);
    assert_eq!(narrow.x, HORIZONTAL_PADDING);
}
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn virtual_content_layout_matches_height_index_at_all_scroll_positions() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0; 80]);
    virtualizer.set_viewport_height(300.0);
    for offset in [0.0, 3000.0, 9000.0, 0.0] {
        virtualizer.set_scroll_top_px(offset);
        let window = virtualizer.window();
        let mut messages = chat_message_column();
        for _ in window.render_start_idx..window.render_end_idx {
            messages = messages.push(iced::widget::Space::new().height(100.0));
        }
        let mut element: iced::Element<'_, Message> = conversation_reading_column(
            chat_virtual_content(messages, window.top_spacer_h, window.bottom_spacer_h),
            iced::Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
        )
        .into();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(800.0, f32::INFINITY)),
        );
        assert!((node.size().height - virtualizer.total_height_px()).abs() < 0.01);
    }
}
