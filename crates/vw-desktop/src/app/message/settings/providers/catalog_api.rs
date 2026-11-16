use crate::app::{App, Message, message::app_providers::list::{self, ListMessage}, state::app_providers::{list::Scope, model_catalog::CatalogContext}};
use gateway_model_catalog::GatewayModelCatalogClient;
use vw_api_model_catalog::ModelCatalogProject;
use iced::Task;
pub use desktop_settings_providers::catalog_api::Event;
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::ModelCatalog(event)) }
fn current(app: &App, scope: &Scope) -> Result<(GatewayModelCatalogClient, CatalogContext), String> {
    let (client, actual_scope) = list::current(app, scope.app)?;
    if &actual_scope != scope || app.apps_catalog.token.is_empty() {
        return Err("应用执行主机已切换，请重新打开模型目录".into());
    }
    let project = match (&app.workspace_state.project_id, &scope.context.project) {
        (Some(id), Some(directory)) if !id.is_empty() => {
            ModelCatalogProject { project_id: id.as_str().into(), directory: directory.clone() }
        }
        _ => return Err("请先打开已注册的项目，再使用模型目录".into()),
    };
    Ok((
        GatewayModelCatalogClient::new(client.transport().clone()),
        CatalogContext {
            scope: actual_scope,
            project,
            access_generation: app.apps_catalog.generation,
        },
    ))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Feedback { success, text } = event {
        return if success { app.show_success_toast(text) } else { app.show_error_toast(text) };
    }
    let context = app.app_providers.scope.as_ref().map(|scope| current(app, scope))
        .unwrap_or_else(|| Err("应用执行主机已切换，请重新打开模型目录".into()));
    desktop_settings_providers::catalog_api::update(
        &mut app.app_providers.model_catalog, &mut app.app_providers.generation,
        app.app_providers.scope.as_ref(), app.app_providers.snapshot.as_ref(),
        context, &app.apps_catalog.token, event,
    ).map(message)
}
