use desktop_chat_input::input_panel::quick_messages::{workspace_results_id, workspace_search_id};
pub use desktop_settings_apps::system_settings_apps::launch::LaunchState;
pub type LaunchMessage = desktop_settings_apps::system_settings_apps::launch::LaunchMessage<
    crate::app::state::AgentAppsContext,
>;
#[cfg(test)]
pub type ReplyScope = desktop_settings_apps::system_settings_apps::launch::ReplyScope<
    crate::app::state::AgentAppsContext,
>;
fn event(value: LaunchMessage) -> crate::app::Message {
    super::message(crate::app::message::settings::AppsCatalogMessage::Launch(value))
}

fn can_browse_project(host: &str, endpoint: &gateway_endpoint::GatewayEndpoint) -> bool {
    host == local_app_auth::HOST_ID && desktop_settings_workspace::environment::can_add(endpoint)
}

pub(crate) fn update(
    app: &mut crate::app::App,
    message: LaunchMessage,
) -> iced::Task<crate::app::Message> {
    match message {
        LaunchMessage::Error(error) => return app.show_error_toast(error),
        LaunchMessage::Success(text) => return app.show_success_toast(text),
        _ => {}
    }
    let Ok(client) = crate::app::config::gateway_client() else {
        app.apps_catalog.launch.invalidate();
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "网关不可用，请重新加载应用",
            vec![],
        );
    };
    let context = crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.navigation_tabs.project_path.clone(),
    };
    #[cfg(not(target_arch = "wasm32"))]
    let (message, directory) = if matches!(message, LaunchMessage::BrowseProject) {
        let catalog = &app.apps_catalog;
        if catalog.context.as_ref() != Some(&context)
            || catalog.launch.request.is_none()
            || !catalog.launch.project_picker_open
            || catalog.launch.busy
            || catalog.launch.accepted
            || !can_browse_project(&catalog.host, client.endpoint())
        {
            return iced::Task::none();
        }
        let Some(directory) = desktop_workspace::basic::pick_folder() else {
            return iced::Task::none();
        };
        (LaunchMessage::ProjectSelected(directory.clone()), Some(directory))
    } else {
        (message, None)
    };
    let opening_picker = matches!(message, LaunchMessage::ProjectPickerOpen(true));
    let searching_projects = matches!(message, LaunchMessage::ProjectSearch(_));
    let state = &mut app.apps_catalog;
    let mut input = desktop_settings_apps::system_settings_apps::launch::Input {
        context: &state.context,
        host: &state.host,
        token: &state.token,
        generation: state.generation,
        loading: state.loading,
        launch: &mut state.launch,
        rows: &state.rows,
    };
    let task = desktop_settings_apps::system_settings_apps::launch::update(
        &mut input,
        &app.navigation_tabs.project_path,
        Ok(client),
        context,
        message,
    )
    .map(event);
    if state.launch.project_picker_open {
        use iced::widget::operation::{RelativeOffset, focus, snap_to};
        if opening_picker {
            return task.chain(focus(workspace_search_id()));
        }
        if searching_projects {
            return task.chain(snap_to(workspace_results_id(), RelativeOffset::START));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(directory) = directory {
        return iced::Task::batch([
            task,
            crate::app::workbench_projects::register_directory(app, directory),
        ]);
    }
    task
}
#[path = "launch_view.rs"]
mod view;
pub(crate) use view::overlay;
