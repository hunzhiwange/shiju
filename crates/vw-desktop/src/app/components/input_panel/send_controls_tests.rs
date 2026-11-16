#![allow(unused_imports, dead_code)]

//! 验证发送控件在运行、排队、停止与禁用状态下的消息和布局。
//! 控件测试分别构造会话运行态和发送策略，确保主操作不会映射到错误消息。

use super::send_controls::{
    bottom_bar, cancel_button, full_access_button, is_dark_theme, permission_access_button_style,
    pool_button, prominent_action_background, prominent_action_foreground, prominent_action_style,
    send_behavior_icon, send_behavior_popover, send_button, workflow_mode_button,
};
use super::styles::square_icon_button_style;
use crate::app::Message;
use crate::app::assets::Icon;
use crate::app::state::ChatSendBehavior;
use crate::app::task::TaskSplitMode;
use iced::widget::{Space, button, text};
use iced::{Background, Color, Element, Length, Theme};

fn spacer() -> Element<'static, Message> {
    Space::new().into()
}

fn button_background(style: &iced::widget::button::Style) -> Color {
    match style.background.as_ref() {
        Some(Background::Color(color)) => *color,
        other => panic!("expected solid button background, got {other:?}"),
    }
}

// 宿主保留：宿主发送策略消息路由与语音状态装配契约。
#[test]
fn bottom_bar_places_workflow_after_full_access_and_before_session_tabs() {
    let controls = include_str!("../../../../../libs/desktop-chat-input/src/input_panel/send_controls.rs");
    let workflow_push = controls.find("bar = bar.push(workflow_btn)").expect("workflow push");
    let permission_push = controls.find("bar = bar.push(permission_btn)").expect("permission push");
    let agent_push = controls.find("bar = bar.push(agent_tab_btn)").expect("agent tab push");
    assert!(permission_push < workflow_push && workflow_push < agent_push);
    assert!(!controls.contains("bar = bar.push(goal_btn)"));
    assert!(!controls.contains("bar = bar.push(subagent_btn)"));
    let mod_source = include_str!("mod.rs");
    assert!(!mod_source.contains("goal_btn"));
    for tab in ["Agent", "Tools", "Skills"] {
        assert!(
            mod_source.contains(&format!("SessionToolSelectorTab::{tab}")),
            "bottom bar should expose session tab entry for {tab}"
        );
        assert!(
            mod_source.contains("OpenSessionToolSelectorTab"),
            "bottom bar session tabs should open via OpenSessionToolSelectorTab"
        );
    }
    assert!(
        mod_source.contains("SessionToolSelectorTab::Agent => Icon::Hexagon"),
        "agent tab should use hexagon identity icon, not a chat bubble"
    );
    assert!(
        mod_source.contains("SessionToolSelectorTab::Tools => Icon::Terminal"),
        "tools tab should use terminal icon aligned with slash palette"
    );
    assert!(
        mod_source.contains("SessionToolSelectorTab::Skills => Icon::Star"),
        "skills tab should use star icon aligned with slash palette"
    );
    assert!(
        !mod_source.contains("\"会话控制\""),
        "bottom bar should no longer use a single 会话控制 entry button"
    );

    let normal = bottom_bar(
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        false,
    );
    let children = normal.as_widget().children();
    // permission + workflow + agent + tools + skills + acp + reasoning + model + fill + utility + actions
    assert_eq!(children.len(), 11);
    assert_eq!(children[9].children.len(), 2);
    assert_eq!(children[10].children.len(), 3);
}

pub(super) async fn click_control(mut element: Element<'_, Message>) -> Vec<Message> {
    use iced::advanced::{Layout, Shell, layout, renderer::Headless, widget::Tree};
    let renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("headless renderer");
    let mut tree = Tree::new(element.as_widget());
    let size = iced::Size::new(400.0, 80.0);
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(iced::Size::ZERO, size),
    );
    let mut messages = Vec::new();
    for event in [
        iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left),
        iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left),
    ] {
        element.as_widget_mut().update(
            &mut tree,
            &iced::Event::Mouse(event),
            Layout::new(&node),
            iced::mouse::Cursor::Available(iced::Point::new(12.0, 12.0)),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
            &iced::Rectangle::with_size(size),
        );
    }
    messages
}

// 宿主保留：宿主发送策略消息路由与语音状态装配契约。
#[tokio::test]
async fn voice_ui_primary_empty_opens_chat() {
    let app = crate::app::App::new().0;
    // 同一编辑器依次输入空白、中文，再删除到空；与 view 一样读取 Content。
    for value in ["", "  \n", "中文", ""] {
        let editor: iced::widget::text_editor::Content =
            iced::widget::text_editor::Content::with_text(value);
        let enabled = !editor.text().trim().is_empty();
        let messages = click_control(send_button(&app, enabled, enabled, false)).await;
        if enabled {
            assert!(messages.iter().any(|m| matches!(
                m,
                Message::Chat(crate::app::message::ChatMessage::SendPressed)
            )));
        } else {
            assert!(messages.iter().any(|m| matches!(
                m,
                Message::Voice(crate::app::message::voice::Event::ChatStart)
            )));
        }
    }
}
// 宿主保留：宿主发送策略消息路由与语音状态装配契约。
#[tokio::test]
async fn voice_ui_primary_text_sends() {
    let app = crate::app::App::new().0;
    let messages = click_control(send_button(&app, true, true, false)).await;
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::Chat(crate::app::message::ChatMessage::SendPressed)))
    );
}
// 宿主保留：宿主发送策略消息路由与语音状态装配契约。
#[tokio::test]
async fn voice_ui_primary_attachment_sends() {
    let mut app = crate::app::App::new().0;
    app.files.push(String::from("fixture.txt"));
    let enabled = !app.files.is_empty();
    let messages = click_control(send_button(&app, enabled, enabled, false)).await;
    assert!(matches!(
        messages.as_slice(),
        [Message::Chat(crate::app::message::ChatMessage::SendPressed)]
    ));
}
// 宿主保留：宿主发送策略消息路由与语音状态装配契约。
#[tokio::test]
async fn voice_ui_primary_busy_preserves_send_behavior() {
    for behavior in
        [ChatSendBehavior::Queue, ChatSendBehavior::StopAndSend, ChatSendBehavior::Guide]
    {
        let mut app = crate::app::App::new().0;
        app.chat_send_behavior = behavior;
        let messages = click_control(send_button(&app, true, true, true)).await;
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, Message::Chat(crate::app::message::ChatMessage::SendPressed)))
        );
        assert!(click_control(send_button(&app, false, false, true)).await.is_empty());
    }
}
