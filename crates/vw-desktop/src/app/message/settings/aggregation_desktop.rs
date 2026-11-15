use crate::app::{App, Message};
use crate::app::message::app_providers::list::{self, ListMessage};
use iced::Task;
pub use desktop_settings_apps::aggregation::desktop::*;
fn current(app: &App) -> Result<(GatewayTransport, Binding), String> {
    let (client, scope) = list::current(app, AppId::ClaudeDesktop)?;
    if app.app_providers.scope.as_ref() != Some(&scope)
        || scope.host.is_empty()
        || app.apps_catalog.token.is_empty()
    {
        return Err("请先连接已授权执行主机".into());
    }
    let project = app.project_id.as_ref().filter(|p| !p.is_empty()).ok_or("请先打开已注册项目")?;
    let endpoint = client
        .endpoint()
        .clone()
        .with_auth(GatewayAuth { skey: Some(app.apps_catalog.token.clone()) });
    Ok((
        GatewayTransport::new(endpoint)?,
        Binding { scope, project: project.as_str().into(), access: app.apps_catalog.generation },
    ))
}
use gateway_http::GatewayTransport;
use gateway_endpoint::GatewayAuth;
use vw_api_app_identity::AppId;
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::DesktopAggregation(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = current(app);
    let mut notice = None;
    let task = desktop_settings_apps::aggregation::desktop::update(&mut app.app_providers.desktop, &mut app.app_providers.generation, app.app_providers.loading, current, &mut notice, event).map(message);
    let feedback = match notice {
        Some(desktop_settings_apps::aggregation::Notice::Error(s)) => app.show_error_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Info(s)) => app.show_info_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Success(s)) => app.show_success_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Warning(s)) => app.show_warning_toast(s),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
pub(crate) fn binding_is_current(app: &App) -> bool {
    current(app).ok().is_some_and(|(_, binding)| {
        app.app_providers
            .desktop
            .as_ref()
            .and_then(|f| f.aggregation.as_ref())
            .is_some_and(|s| s.binding == binding)
    })
}
