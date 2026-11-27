use crate::input_panel::send_controls::bottom_bar;
use crate::input_panel::send_message::Message;
use iced::{
    Element,
    widget::{Space, button, text},
};
fn spacer() -> Element<'static, Message> {
    Space::new().into()
}
#[test]
fn bottom_bar_groups_session_tabs_apart_from_actions() {
    let source = include_str!("../input_panel/panel.rs");
    for identity in [
        "SessionToolSelectorTab::Agent => ComposerIcon::Agent",
        "SessionToolSelectorTab::Tools => ComposerIcon::Tools",
        "SessionToolSelectorTab::Skills => ComposerIcon::Skill",
    ] {
        assert!(source.contains(identity));
    }
    let normal = bottom_bar(
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        None,
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        false,
    );
    let children = normal.as_widget().children();
    assert_eq!(children.len(), 3);
    assert_eq!(children[1].children.len(), 5);
    assert_eq!(children[2].children.len(), 5);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn bottom_bar_keeps_requesting_actions_on_one_line() {
    use crate::input_panel::send_controls::{cancel_button, send_button};
    use desktop_session::message::chat::request::ChatSendBehavior;
    use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
    use iced::{Font, Pixels, Size};

    let i18n = desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese);
    let control = |width: u32| -> Element<'static, Message> {
        Space::new().width(width).height(34).into()
    };
    let mut bar = bottom_bar(
        Some(control(80)),
        Some(control(34)),
        Some(control(34)),
        None,
        control(34),
        control(216),
        control(34),
        control(34),
        Some(control(34)),
        None,
        Some(cancel_button(i18n, 0)),
        Some(send_button(i18n, ChatSendBehavior::Queue, false, true, false, false, true)),
        false,
    );
    let renderer = <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("headless renderer");
    let mut tree = Tree::new(bar.as_widget());
    let node = bar.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(880.0, 120.0)),
    );
    let actions = Layout::new(&node).children().nth(2).expect("right-side actions");
    let children: Vec<_> = actions.children().map(|child| child.bounds()).collect();
    assert_eq!(children.len(), 5);
    assert!(children.iter().all(|bounds| bounds.y == children[0].y));
    let send = children.last().expect("send controls");
    assert_eq!(send.width, 72.0);
    assert_eq!(send.x + send.width, 880.0 - 13.0);
}
