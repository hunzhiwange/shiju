//! 验证三列卡片留白与中文配置弹窗在明暗主题中的实际绘制。
use crate::app::{
    App,
    skill_hub::{
        ExpertHubSection, LoadState,
        connectors::{ConnectorForm, ConnectorWork},
    },
};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
use serde_json::json;
fn connector(id: u64) -> vw_gateway_client::ShijuConnector {
    serde_json::from_value(json!({"platform_id": id, "company_platform_id": id, "platform_code": "sample", "platform_name": format!("中文连接器 {id}"), "authorization_available": true, "account_binding_variables": [{"name": "token", "description": "授权密钥", "type": "password", "required": true}], "binding": {"third_party_account_id": "中文账号", "third_party_account_name": "测试用户", "binding_config": {}, "status": "enabled"}})).unwrap()
}
#[tokio::test(flavor = "multi_thread")]
async fn connector_grid_has_three_equal_columns_and_scrollbar_gutter() {
    let (app, _) = App::new();
    let connectors = (1..=5).map(connector).collect::<Vec<_>>();
    let mut element = super::connectors::card_grid(&app, &connectors);
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
#[tokio::test(flavor = "multi_thread")]
async fn connector_chinese_modal_and_independent_waits_draw_in_both_themes() {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Connectors;
    app.expert_hub.connectors.list = LoadState::Loaded((1..=12).map(connector).collect());
    for id in [1, 2] {
        app.expert_hub.connectors.begin(
            id,
            ConnectorWork::Authorizing {
                challenge: vw_gateway_client::ShijuConnectorChallenge {
                    flow_id: id.to_string().repeat(32),
                    authorization_url: "https://provider.example/authorize".into(),
                    poll_interval_seconds: 1,
                    expires_in_seconds: 60,
                },
                deadline: std::time::Instant::now() + std::time::Duration::from_secs(60),
            },
        );
    }
    for mode in 0..3 {
        app.expert_hub.connectors.form = (mode > 0).then(|| {
            let mut connector = connector(3);
            for index in 0..8 {
                connector.account_binding_variables.push(
                    vw_gateway_client::ShijuConnectorVariable {
                        name: format!("field_{index}"),
                        description: format!("中文配置字段 {index}"),
                        kind: "string".into(),
                        required: false,
                    },
                );
            }
            ConnectorForm::new(connector)
        });
        if mode == 2 {
            app.expert_hub.connectors.begin(3, ConnectorWork::Saving);
        }
        for theme in [Theme::Dark, Theme::Light] {
            let mut element = super::view(&app);
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
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(1100, 900), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }
}
