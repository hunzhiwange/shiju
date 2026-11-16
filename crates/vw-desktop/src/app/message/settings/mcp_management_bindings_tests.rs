use vw_api_app_identity::{
    AppCatalogEntry,
    AppId,
    AppInstallation,
    AppInstallationEnvironment,
    AppInstallationId,
    AppInstallationRuntimeStatus,
    AppInstallationSource,
    AppReadResponse,
    AppRuntimeKind,
};
use super::catalog::rows;
use super::*;

fn record(id: &str) -> McpCatalogRecord {
    McpCatalogRecord {
        record: McpRecordRef { id: McpRecordId(id.into()), revision: 4 },
        scope: McpCatalogScope::Tenant {
            scope: vw_api_transport::TenantScope::new("10000", "100000001").unwrap(),
        },
        name: "中文连接 😀".into(),
        description: None,
        homepage: None,
        documentation: None,
        tags: vec![],
        transport: McpTransport::Stdio,
        connection_revision: 2,
    }
}
fn target() -> McpAppTarget {
    McpAppTarget {
        app_id: AppId::Codex,
        installation_id: vw_api_app_identity::AppInstallationId(
            "trusted-instance".into(),
        ),
        target_host_id: "host".into(),
    }
}
fn result() -> McpDistributionResult {
    McpDistributionResult {
        outcome: McpBatchOutcome::PartiallySucceeded,
        items: vec![
            McpDistributionItem {
                record: record("a").record,
                target: target(),
                enabled: false,
                desired_committed: true,
                native_outcome: McpDistributionOutcome::Applied,
            },
            McpDistributionItem {
                record: record("b").record,
                target: target(),
                enabled: false,
                desired_committed: true,
                native_outcome: McpDistributionOutcome::Failed { code: McpFailureCode::Conflict },
            },
        ],
    }
}
#[test]
fn cc_mcp_t46_matrix_draws_chinese_results_and_preserves_runtime_and_acp_selections() {
    use crate::app::components::system_settings_apps::catalog::CatalogRow;
    use iced::{
        Font, Renderer, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    use vw_api_types::agent_apps::*;
    crate::app::config::with_settings_gateway(|_, _| {
        let mut app = App::new().0;
        app.apps_catalog.host = "host".into();
        app.apps_catalog.token = "fixture".into();
        let context = catalog::context(&app).unwrap();
        app.apps_catalog.context = Some(context.endpoint.clone());
        let before_runtime = app.mcp_settings.runtime_overrides.clone();
        let before_acp = app.acp_settings.enabled.clone();
        let record = record("a");
        let catalog_response = McpCatalogResponse {
            records: vec![record.clone()],
            bindings: vec![],
            capabilities: vec![McpAppCapability {
                app_id: AppId::Codex,
                stdio: McpSupport::Supported,
                streamable_http: McpSupport::Supported,
                sse: McpSupport::Supported,
            }],
        };
        drop(catalog::update(&mut app, catalog::CatalogMessage::Refresh));
        drop(catalog::update(
            &mut app,
            catalog::CatalogMessage::Loaded {
                context: context.clone(),
                generation: 1,
                result: Ok(catalog_response),
            },
        ));
        app.apps_catalog.rows.push(CatalogRow {
            entry: AppCatalogEntry {
                id: AppId::Codex,
                name: "Codex".into(),
                cli_command: Some("codex".into()),
                runtime_kinds: vec![AppRuntimeKind::NativeCli],
            },
            observation: Ok(AppReadResponse {
                desktop: None,
                installations: vec![AppInstallation {
                    id: target().installation_id,
                    app_id: AppId::Codex,
                    target_host_id: "host".into(),
                    environment: AppInstallationEnvironment::Macos,
                    path: "/fixture/codex".into(),
                    real_path: None,
                    source: AppInstallationSource::Native,
                    is_path_default: true,
                    version: Some("1".into()),
                    runtime_status: AppInstallationRuntimeStatus::Runnable,
                }],
            }),
        });
        let mut obsolete = record.clone();
        obsolete.record.revision = 3;
        app.mcp_settings.bindings.selected.push(obsolete);
        drop(update(&mut app, BindingsMessage::Select(record.clone(), true)));
        assert_eq!(app.mcp_settings.bindings.selected, vec![record]);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let mut renderer = runtime
            .block_on(Renderer::new(
                Font::with_name("Noto Sans CJK SC"),
                14.0.into(),
                Some("tiny-skia"),
            ))
            .unwrap();
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in crate::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        for response in [Some(Ok(result())), Some(Err("中文目标无法写入 😀".into())), None]
        {
            app.mcp_settings.bindings.result = response;
            for theme in [Theme::Light, Theme::Dark] {
                let element = crate::app::components::system_settings_mcp::view(&app);
                let mut ui = UserInterface::build(
                    element,
                    Size::new(1200., 3000.),
                    Cache::new(),
                    &mut renderer,
                );
                ui.draw(
                    &mut renderer,
                    &theme,
                    &Style { text_color: theme.palette().text },
                    mouse::Cursor::Unavailable,
                );
            }
        }
        let mut invalid_target = target();
        invalid_target.installation_id.0 = "untrusted".into();
        drop(update(&mut app, BindingsMessage::Apply(invalid_target, true)));
        assert!(!app.mcp_settings.bindings.running);
        drop(update(&mut app, BindingsMessage::Apply(target(), true)));
        assert!(app.mcp_settings.bindings.running);
        let generation = app.mcp_settings.bindings.generation;
        drop(update(
            &mut app,
            BindingsMessage::Finished { context, generation, result: Ok(result()) },
        ));
        assert!(!app.mcp_settings.bindings.running);
        assert_eq!(app.mcp_settings.bindings.result, Some(Ok(result())));
        assert_eq!(app.mcp_settings.runtime_overrides, before_runtime);
        assert_eq!(app.acp_settings.enabled, before_acp);
    });
}
