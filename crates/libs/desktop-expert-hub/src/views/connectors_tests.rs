use iced::advanced::renderer::Headless;
use iced::advanced::{layout, widget::Tree};
use iced::{Font, Pixels, Size, Theme};
use serde_json::json;
fn connector(id: u64) -> gateway_shiju_connector::ShijuConnector {
    serde_json::from_value(json!({"platform_id": id, "company_platform_id": id, "platform_code": "sample", "platform_name": format!("中文连接器 {id}"), "authorization_available": true, "account_binding_variables": [{"name": "token", "description": "授权密钥", "type": "password", "required": true}], "binding": {"third_party_account_id": "中文账号", "third_party_account_name": "测试用户", "binding_config": {}, "status": "enabled"}})).unwrap()
}
#[tokio::test(flavor = "multi_thread")]
async fn connector_grid_has_three_equal_columns_and_scrollbar_gutter() {
    let state = desktop_skill_hub::skill_hub::ExpertHubState::default();
    let app = super::ViewInput { expert_hub: &state, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese, app_theme: &Theme::Dark };
    let connectors = (1..=5).map(connector).collect::<Vec<_>>();
    let mut element = super::connectors::card_grid(&app, &connectors, super::Message::Connectors);
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(960.0, 900.0)),
    );
    let rows = node.children()[0].children();
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row.children().len(), 3);
        let widths = row.children().iter().map(|card| card.bounds().width).collect::<Vec<_>>();
        assert!((widths[0] - widths[1]).abs() < 1.0);
        assert!((widths[1] - widths[2]).abs() < 1.0);
    }
    let grid = &node.children()[0];
    assert!(grid.bounds().x >= 8.0);
    assert!(grid.bounds().x + grid.bounds().width <= 952.0);
}
