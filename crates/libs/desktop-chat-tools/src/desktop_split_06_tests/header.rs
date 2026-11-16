use crate::card_input::Message;
use crate::tools::tool_meta::{tool_activity_header, tool_visual_state};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, layout};
use iced::widget::Space;
use iced::{Font, Length, Pixels, Size};
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn command_status_icon_stays_next_to_command() {
    async fn status_x(viewport_width: f32) -> f32 {
        let renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("tiny-skia headless renderer should be available");
        let action = Space::new().width(Length::Fixed(22.0)).into();
        let mut element: iced::Element<'_, Message> = tool_activity_header(
            "bash",
            "运行",
            "cargo clippy --all-targets --all-features",
            tool_visual_state("completed"),
            action,
        )
        .into();
        let limits = layout::Limits::new(Size::ZERO, Size::new(viewport_width, 100.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);

        Layout::new(&node).children().nth(2).expect("command status icon").bounds().x
    }

    let narrow = status_x(480.0).await;
    let wide = status_x(960.0).await;

    assert!((wide - narrow).abs() < 1.0, "narrow={narrow}, wide={wide}");
}
