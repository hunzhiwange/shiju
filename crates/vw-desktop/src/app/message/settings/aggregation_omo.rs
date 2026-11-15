use crate::app::{App, Message};
use crate::app::message::app_providers::list::{self, ListMessage};
use iced::Task;
pub use desktop_settings_apps::aggregation::omo::*;
fn current(app: &App) -> Result<(GatewayAggregationOmoClient, Context), String> {
    let (client, scope) = list::current(app, AppId::OpenCode)?;
    let project =
        app.project_id.as_ref().filter(|id| !id.is_empty()).ok_or("请先打开已注册项目")?;
    if app.app_providers.scope.as_ref() != Some(&scope)
        || scope.host.is_empty()
        || app.apps_catalog.token.is_empty()
    {
        return Err("请先连接应用执行主机".into());
    }
    let endpoint = client
        .endpoint()
        .clone()
        .with_auth(GatewayAuth { skey: Some(app.apps_catalog.token.clone()) });
    Ok((
        GatewayAggregationOmoClient::new(gateway_http::GatewayTransport::new(endpoint)?),
        Context { scope, project: project.as_str().into(), access: app.apps_catalog.generation },
    ))
}
use gateway_aggregation_omo::GatewayAggregationOmoClient;
use gateway_endpoint::GatewayAuth;
use vw_api_app_identity::AppId;
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::Omo(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = current(app);
    let saved = app.app_providers.omo.as_ref().is_some_and(|form| app.app_providers.snapshot.as_ref().is_some_and(|s| s.catalog.rows.iter().any(|r| r.provider.id == form.provider)));
    let mut notice = None;
    let task = desktop_settings_apps::aggregation::omo::update(&mut app.app_providers.omo, &mut app.app_providers.generation, app.app_providers.loading, saved, current, &mut notice, event).map(message);
    let feedback = match notice {
        Some(desktop_settings_apps::aggregation::Notice::Error(s)) => app.show_error_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Info(s)) => app.show_info_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Success(s)) => app.show_success_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Warning(s)) => app.show_warning_toast(s),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
#[cfg(test)]
#[path = "aggregation_omo_tests.rs"]
mod tests;
