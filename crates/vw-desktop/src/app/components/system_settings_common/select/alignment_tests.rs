use super::*;
use iced::advanced::renderer::Headless;
use iced::{Font, Pixels};

fn assert_row_centered(button: Layout<'_>) {
    let row = button.children().next().expect("button content");
    let expected = button.bounds().center_y();
    for item in row.children() {
        assert!(
            (item.bounds().center_y() - expected).abs() < 0.5,
            "text/icon center {} differs from button center {expected}",
            item.bounds().center_y()
        );
    }
}

#[tokio::test]
async fn select_text_and_icons_are_centered_in_trigger_and_menu_rows() {
    let renderer =
        <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    let limits = layout::Limits::new(Size::ZERO, Size::new(280.0, 420.0));
    let labels = ["简体中文", "繁體中文", "English"].map(str::to_owned);
    for label in &labels {
        let mut select = settings_select(labels.clone(), Some(label.clone()), |_| unreachable!());
        let mut tree = widget::Tree::new(&select as &dyn Widget<Message, Theme, Renderer>);
        let node = select.layout(&mut tree, &renderer, &limits);
        assert_row_centered(Layout::new(&node));
    }
    let mut content = menu::content(&labels, Some(0), None);
    let mut tree = widget::Tree::new(content.as_widget());
    let node = content.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let scroll = Layout::new(&node).children().next().expect("scrollable");
    let padding = scroll.children().next().expect("scroll content");
    let rows = padding.children().next().expect("options");
    assert_eq!(rows.children().count(), labels.len());
    for row in rows.children() {
        assert_row_centered(row);
    }
}
