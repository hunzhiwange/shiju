//! 验证网关客户端服务器、认证字段、健康状态与删除帮助覆盖层。
//! 健康状态按服务器键隔离，删除目标缺失时覆盖层需安全退回基础视图。

use super::*;
use crate::app::App;
use crate::app::message::gateway_health::server_health_key;
use crate::app::state::GatewayClientServerDraft;
use iced::{Element, Length};

// 客户端页面 fixture 保持服务端列表为空，各测试再按目标场景添加草稿。
fn test_app() -> App {
    App::new().0
}

fn server(id: &str, host: &str, port: u16) -> GatewayClientServerDraft {
    GatewayClientServerDraft {
        id: id.to_string(),
        name: format!("Gateway {id}"),
        host: host.to_string(),
        port,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        skey: String::new(),
    }
}

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn system_settings_gateway_client_tests_are_wired() {
    assert!(module_path!().contains("system_settings_gateway_client_tests"));
}

#[test]
fn field_helpers_fill_width_for_text_secure_and_number_controls() {
    let field = field_row("Label", "Description", iced::widget::text("control"));
    let text =
        text_row("Text", "Description", "placeholder", "value", |_| Message::GatewayHealthTick);
    let secure = secure_text_row("Secret", "Description", "placeholder", "value", |_| {
        Message::GatewayHealthTick
    });
    let number_without_suffix =
        number_row("Port", "Description", 42617, 1, u16::MAX as u32, "", |_| {
            Message::GatewayHealthTick
        });
    let number_with_suffix =
        number_row("Timeout", "Description", 30, 1, 60, "s", |_| Message::GatewayHealthTick);

    for element in [field, text, secure, number_without_suffix, number_with_suffix] {
        assert_size(&element, Length::Fill, Length::Shrink);
    }
}

#[test]
fn server_health_defaults_false_and_uses_only_the_matching_key() {
    let mut app = test_app();
    let local = server("local", "127.0.0.1", 42617);
    let remote = server("remote", "gateway.example.test", 8080);

    assert!(!server_healthy(&app, &local));

    let local_key = server_health_key(&local).expect("local server should produce a health key");
    let remote_key = server_health_key(&remote).expect("remote server should produce a health key");
    app.gateway_client_settings.health.insert(local_key, false);
    app.gateway_client_settings.health.insert(remote_key, true);

    assert!(!server_healthy(&app, &local));
    assert!(server_healthy(&app, &remote));

    let malformed = server("malformed", "http://[", 8080);
    assert!(!server_healthy(&app, &malformed));
}

#[test]
fn button_builders_keep_server_width_and_action_height_across_states() {
    let selected = server_button(
        "Local".to_string(),
        "127.0.0.1:42617".to_string(),
        true,
        true,
        Message::GatewayHealthTick,
    );
    let unselected = server_button(
        "Remote".to_string(),
        "gateway.example.test:8080".to_string(),
        false,
        false,
        Message::GatewayHealthTick,
    );
    let enabled_label: Element<'_, Message> = iced::widget::text("enabled").into();
    let disabled_label: Element<'_, Message> = iced::widget::text("disabled").into();
    let enabled = rounded_action_button(enabled_label, true, Some(Message::GatewayHealthTick));
    let disabled = rounded_action_button(disabled_label, false, None);

    assert_size(&selected, Length::Fill, Length::Shrink);
    assert_size(&unselected, Length::Fill, Length::Shrink);
    assert_size(&enabled, Length::Shrink, Length::Fixed(36.0));
    assert_size(&disabled, Length::Shrink, Length::Fixed(36.0));
}

#[test]
fn view_builds_empty_single_and_multi_server_states() {
    let mut empty = test_app();
    empty.gateway_client_settings.servers.clear();
    empty.gateway_client_settings.save_error = Some("client save failed".to_string());
    let empty_view = view(&empty);
    assert_size(&empty_view, Length::Fill, Length::Shrink);

    let app = test_app();
    let single_view = view(&app);
    assert_size(&single_view, Length::Fill, Length::Shrink);

    let mut multi = test_app();
    let extra = GatewayClientServerDraft {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        host: "https://gateway.example.com".to_string(),
        port: 443,
        platform_id: "20000".to_string(),
        company_id: "200000001".to_string(),
        skey: "skey".to_string(),
    };
    if let Some(key) = server_health_key(&extra) {
        multi.gateway_client_settings.health.insert(key, true);
    }
    multi.gateway_client_settings.selected_server_id = extra.id.clone();
    multi.gateway_client_settings.name_input = extra.name.clone();
    multi.gateway_client_settings.host_input = extra.host.clone();
    multi.gateway_client_settings.port = extra.port;
    multi.gateway_client_settings.platform_id_input = extra.platform_id.clone();
    multi.gateway_client_settings.company_id_input = extra.company_id.clone();
    multi.gateway_client_settings.skey_input = extra.skey.clone();
    multi.gateway_client_settings.servers.push(extra);
    let multi_view = view(&multi);
    assert_size(&multi_view, Length::Fill, Length::Shrink);
}

#[test]
fn view_keeps_intro_service_and_authentication_sections_across_server_counts() {
    let mut app = test_app();

    app.gateway_client_settings.servers.clear();
    let empty_view = view(&app);
    assert_eq!(empty_view.as_widget().children().len(), 5);
    drop(empty_view);

    app.gateway_client_settings.servers.push(server("local", "127.0.0.1", 42617));
    app.gateway_client_settings.selected_server_id = "local".to_string();
    let single_view = view(&app);
    let single_children = single_view.as_widget().children();
    assert_eq!(single_children.len(), 5);
    assert_eq!(single_children[2].children.len(), 3);
    assert_eq!(single_children[4].children.len(), 1);
    drop(single_view);

    app.gateway_client_settings.servers.push(server("remote", "gateway.example.test", 8080));
    let multi_view = view(&app);
    assert_eq!(multi_view.as_widget().children().len(), 5);
}

#[test]
fn tenant_controls_and_help_example_are_declared() {
    let source = include_str!("system_settings_gateway_client.rs");

    assert!(source.contains("平台 ID"));
    assert!(source.contains("公司 ID"));
    assert!(source.contains("PlatformIdChanged"));
    assert!(source.contains("CompanyIdChanged"));
    assert!(source.contains(r#""platform_id": "10000""#));
    assert!(source.contains(r#""company_id": "100000000""#));
}

#[test]
fn overlays_cover_remove_confirmation_missing_server_and_help_modal() {
    let mut app = test_app();
    app.gateway_client_settings.pending_remove_server_id =
        Some(app.gateway_client_settings.selected_server_id.clone());
    let base = iced::widget::container(iced::widget::text("base")).into();
    let confirmation = view_overlays(&app, base);
    assert_eq!(confirmation.as_widget().children().len(), 2);
    drop(confirmation);

    app.gateway_client_settings.pending_remove_server_id = Some("missing".to_string());
    let base = iced::widget::container(iced::widget::text("base")).into();
    let missing = view_overlays(&app, base);
    assert!(missing.as_widget().children().is_empty());
    drop(missing);

    app.gateway_client_settings.pending_remove_server_id = None;
    app.gateway_client_settings.show_help_modal = true;
    let base = iced::widget::container(iced::widget::text("base")).into();
    let help = view_overlays(&app, base);
    assert_eq!(help.as_widget().children().len(), 2);
}

#[test]
fn overlays_return_base_without_pending_remove_or_help() {
    let app = test_app();
    let base = iced::widget::container(iced::widget::text("base")).into();

    let unchanged = view_overlays(&app, base);

    assert!(unchanged.as_widget().children().is_empty());
}

#[test]
fn overlays_stack_help_over_remove_confirmation() {
    let mut app = test_app();
    app.gateway_client_settings.pending_remove_server_id =
        Some(app.gateway_client_settings.selected_server_id.clone());
    app.gateway_client_settings.show_help_modal = true;
    let base = iced::widget::container(iced::widget::text("base")).into();

    let combined = view_overlays(&app, base);
    let layers = combined.as_widget().children();

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].children.len(), 2);
}

#[test]
fn overlays_show_help_when_pending_remove_server_is_missing() {
    let mut app = test_app();
    app.gateway_client_settings.pending_remove_server_id = Some("missing".to_string());
    app.gateway_client_settings.show_help_modal = true;
    let base = iced::widget::container(iced::widget::text("base")).into();

    let combined = view_overlays(&app, base);
    let layers = combined.as_widget().children();

    assert_eq!(layers.len(), 2);
    assert!(layers[0].children.is_empty());
}
