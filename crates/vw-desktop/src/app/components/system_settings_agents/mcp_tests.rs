use super::{defaults, mcp};
use crate::app::{
    App, Message,
    message::settings::{AgentsMessage, SettingsMessage},
};
use vw_config_types::agent::AgentDefault;

#[test]
fn agent_config_cleanup_mcp_tab_uses_stable_server_ids() {
    assert_eq!(crate::app::state::AGENT_DETAIL_MCP_TAB, "mcp");
    let mut defaults = vw_config_types::agent::AgentDefaults::default();
    defaults.mcp = AgentDefault::Fixed(vec![]);
    let message = mcp::toggle_message(&defaults, "server-id", true, true);
    let Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileDefaultsChanged(next))) =
        message
    else {
        panic!("typed defaults event")
    };
    assert_eq!(next.mcp, AgentDefault::Fixed(vec!["server-id".into()]));
    assert_eq!(super::defaults::patch(&next)["mcp"]["value"], serde_json::json!(["server-id"]));
}

#[test]
fn agent_config_cleanup_mcp_inherit_and_fixed_empty_roundtrip() {
    let mut defaults = vw_config_types::agent::AgentDefaults::default();
    for selection in [AgentDefault::Inherit, AgentDefault::Fixed(vec![])] {
        defaults.mcp = selection.clone();
        let restored: vw_config_types::agent::AgentDefaults =
            serde_json::from_value(defaults::patch(&defaults)).unwrap();
        assert_eq!(restored.mcp, selection);
    }
    assert!(
        defaults::patch(&vw_config_types::agent::AgentDefaults::default())["mcp"]["value"]
            .is_null()
    );
}

#[test]
fn agent_config_cleanup_mcp_unavailable_selection_is_explicit() {
    let mut app = App::new().0;
    let mut draft = app.agents_settings.entries[0].clone();
    draft.defaults.mcp = AgentDefault::Fixed(vec!["unknown".into()]);
    assert!(defaults::validation_error(&app, &draft).is_some());
    let message = mcp::toggle_message(&draft.defaults, "unknown", false, false);
    let Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileDefaultsChanged(next))) =
        message
    else {
        panic!("typed defaults event")
    };
    assert_eq!(next.mcp, AgentDefault::Fixed(vec![]));
    app.mcp_settings.inspect_error = Some("fixture failure".into());
    let before = draft.defaults.clone();
    let _ = mcp::view(&app, &draft);
    assert_eq!(draft.defaults, before);
    app.mcp_settings.inspect = Some(serde_json::from_value(serde_json::json!({"servers":[
        {"name":"stable-server", "transport":"stdio", "enabled":true, "required":false, "connection_status":"connected"},
        {"name":"disabled-server", "transport":"stdio", "enabled":false, "required":false, "connection_status":"disabled"}
    ]})).unwrap());
    draft.defaults.mcp = AgentDefault::Fixed(vec!["stable-server".into()]);
    assert!(defaults::validation_error(&app, &draft).is_some());
    app.mcp_settings.inspect_error = None;
    assert!(defaults::validation_error(&app, &draft).is_none());
    draft.defaults.mcp = AgentDefault::Fixed(vec!["disabled-server".into()]);
    assert!(defaults::validation_error(&app, &draft).is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_mcp_renders_chinese_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = App::new().0;
        app.agents_settings.config_ready = true;
        let _ = crate::app::message::settings::update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::ProfileOpen("main".into())),
        );
        app.agents_settings.active_detail_tab = crate::app::state::AGENT_DETAIL_MCP_TAB.into();
        app.mcp_settings.inspect_loading = false;
        app.mcp_settings.inspect_error = Some("隔离目录加载失败".into());
        app.agents_settings.profile_modal.as_mut().unwrap().defaults.mcp =
            AgentDefault::Fixed(vec!["中文服务".into()]);
        let mut element = super::profile_modal::overlay(
            &app,
            iced::widget::container(iced::widget::text("背景"))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
        );
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .unwrap();
        let bounds = Size::new(1100.0, 900.0);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        assert_eq!(node.bounds().size(), bounds);
        #[derive(Default)]
        struct Labels(Vec<String>);
        impl iced::advanced::widget::Operation for Labels {
            fn traverse(
                &mut self,
                operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
            ) {
                operate(self);
            }
            fn text(
                &mut self,
                _id: Option<&iced::advanced::widget::Id>,
                _bounds: Rectangle,
                text: &str,
            ) {
                self.0.push(text.to_owned());
            }
        }
        let mut labels = Labels::default();
        element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut labels);
        assert_eq!(labels.0.iter().filter(|text| text.as_str() == "默认 MCP").count(), 1);
        for label in ["基础参数", "身份", "工具", "技能", "MCP"] {
            assert_eq!(labels.0.iter().filter(|text| text.as_str() == label).count(), 1);
        }
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(bounds),
        );
        let pixels = renderer.screenshot(Size::new(1100, 900), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        image::save_buffer(
            format!("/tmp/shiju-task5-mcp-{name}.png"),
            &pixels,
            1100,
            900,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
