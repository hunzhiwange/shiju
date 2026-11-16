use vw_api_mcp_management::*;
use {
    vw_api_app_identity::AppId, vw_api_app_identity::AppInstallationId,
    vw_api_transport::TenantScope,
};
fn fixture() -> McpCatalogResponse {
    McpCatalogResponse {
        records: vec![McpCatalogRecord {
            record: McpRecordRef { id: McpRecordId("demo".into()), revision: 4 },
            scope: McpCatalogScope::Tenant {
                scope: TenantScope::new("10000", "100000001").unwrap(),
            },
            name: "中文目录".into(),
            description: None,
            homepage: None,
            documentation: None,
            tags: vec![],
            transport: McpTransport::Stdio,
            connection_revision: 2,
        }],
        bindings: vec![McpCliBinding {
            record: McpRecordRef { id: McpRecordId("demo".into()), revision: 3 },
            target: McpAppTarget {
                app_id: AppId::Codex,
                installation_id: AppInstallationId("fixture".into()),
                target_host_id: "host".into(),
            },
            enabled: true,
            native_status: McpNativeStatus::Missing,
        }],
        capabilities: vec![],
    }
}

#[tokio::test]
async fn cc_mcp_t44_chinese_catalog_shapes_and_draws_in_both_themes() {
    use iced::{
        Font, Renderer, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("native renderer");
    let empty = Ok(McpCatalogResponse { records: vec![], bindings: vec![], capabilities: vec![] });
    let populated = Ok(fixture());
    let error = Err("目标主机不支持目录 😀".into());
    for theme in [Theme::Light, Theme::Dark] {
        for (result, loading) in
            [(None, true), (Some(&empty), false), (Some(&error), false), (Some(&populated), false)]
        {
            let element = crate::catalog::content(result, loading);
            let mut ui =
                UserInterface::build(element, Size::new(800., 700.), Cache::new(), &mut renderer);
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
        }
    }
}
