//! Startup/health messages assemble domain effects and preserve the update finalization boundary.
use crate::app::state::RuntimePlatform;
use crate::app::{App, Message, message};
use desktop_settings_agents::acp_update::sort_acp_agent_names;
#[cfg(target_arch = "wasm32")]
use desktop_settings_network::bootstrap::selected_gateway_became_healthy;
#[cfg(not(target_arch = "wasm32"))]
use desktop_settings_network::bootstrap::selected_gateway_health_key;
#[cfg(not(target_arch = "wasm32"))]
use desktop_settings_network::startup::startup_retry_interval;
use iced::Task;

pub(in crate::app) enum Update {
    Continue(Task<Message>),
    Return(Task<Message>),
}
pub(in crate::app) fn update(app: &mut App, message: Message) -> Update {
    match message {
        #[cfg(not(target_arch = "wasm32"))]
        Message::StartupCliServiceBootstrapped(result) => Update::Continue({
            if app.startup_state == crate::app::state::NativeStartupState::Ready {
                return Update::Return(Task::none());
            }
            match result {
                Ok(()) => {
                    app.startup_state = crate::app::state::NativeStartupState::Ready;
                    tracing::info!(
                        target: "vw_desktop",
                        "desktop gateway is ready"
                    );
                }
                Err(error) => {
                    let retry_after = startup_retry_interval(app.startup_started_at.elapsed());
                    tracing::debug!(
                        target: "vw_desktop",
                        error = %error,
                        retry_after_seconds = retry_after.as_secs(),
                        "desktop CLI service bootstrap will retry"
                    );
                    return Update::Return(Task::perform(
                        async move {
                            tokio::time::sleep(retry_after).await;
                            crate::app::config::bootstrap_cli_service_async().await
                        },
                        Message::StartupCliServiceBootstrapped,
                    ));
                }
            }
            let mut tasks = vec![
                crate::app::message::account::load_repository(),
                app.startup_configuration.load(crate::app::config::gateway_client_endpoint()).map(
                    |(generation, snapshot)| {
                        Message::StartupGatewaySnapshotLoaded(generation, snapshot)
                    },
                ),
                Task::done(Message::GatewayHealthTick),
                Task::done(Message::Settings(message::SettingsMessage::ModelsRefresh)),
            ];
            tasks.push(app.load_startup_sessions());
            Task::batch(tasks)
        }),
        #[cfg(not(target_arch = "wasm32"))]
        Message::StartupGatewaySnapshotLoaded(generation, snapshot) => {
            Update::Continue(app.apply_startup_gateway_snapshot(generation, *snapshot))
        }
        #[cfg(not(target_arch = "wasm32"))]
        Message::StartupSessionsLoaded(generation, message) => Update::Continue({
            let succeeded = matches!(
                &*message,
                message::ProjectMessage::SessionBootstrapLoaded { result: Ok(_), .. }
            );
            if app.startup_configuration.accept_sessions(
                generation,
                succeeded,
                &crate::app::config::gateway_client_endpoint(),
            ) && app.session_state.active_session_id.is_none()
            {
                return Update::Return(app.update(Message::Project(*message)));
            }
            Task::none()
        }),
        Message::StartupAppConfigLoaded(result) | Message::BootstrapAppConfig(result) => {
            Update::Continue({
                if let Ok(cfg) = result {
                    app.apply_bootstrap_app_config(cfg);
                }
                #[cfg(not(target_arch = "wasm32"))]
                if !app.desktop_update.initial_check_started {
                    app.desktop_update.initial_check_started = true;
                    return Update::Return(Task::batch([
                        message::view::sync_task_pet_window(app),
                        Task::done(Message::View(message::ViewMessage::AutoCheckAppUpdate)),
                    ]));
                }
                message::view::sync_task_pet_window(app)
            })
        }
        Message::StartupSystemSettingsLoaded(result) | Message::BootstrapSystemSettings(result) => {
            Update::Continue({
                if let Ok(system) = result {
                    app.apply_bootstrap_system_settings(system);
                    let health = Task::done(Message::GatewayHealthTick);
                    // 输入栏依赖模型能力数据，桌面端也必须在启动时加载。
                    return Update::Return(Task::batch(vec![
                        health,
                        Task::done(Message::Settings(
                            crate::app::message::SettingsMessage::ModelsRefresh,
                        )),
                    ]));
                }
                Task::none()
            })
        }
        Message::StartupAgentConfigLoaded(result) => Update::Continue({
            match result {
                Ok(config) => app.apply_bootstrap_agent_config(&config),
                Err(error) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        error = %error,
                        "failed to reload agent config after gateway bootstrap"
                    );
                }
            }
            Task::none()
        }),
        Message::StartupBrowserConfigLoaded(result) | Message::BootstrapBrowserConfig(result) => {
            Update::Continue({
                if let Ok(browser) = result {
                    desktop_settings_network::bootstrap::apply_bootstrap_browser_settings(
                        &mut app.browser_settings,
                        browser,
                    );
                }
                Task::none()
            })
        }
        Message::BootstrapAcpAgentsLoaded(result) => Update::Continue({
            if let Ok(acp_agents) = result {
                app.session_state.acp_agents = sort_acp_agent_names(acp_agents);
            }
            Task::none()
        }),
        Message::BootstrapArchivedSessions(result) => Update::Continue({
            if let Ok(archived_session_ids) = result {
                app.session_state.archived_session_ids = archived_session_ids;
            }
            Task::none()
        }),
        Message::ExternalAppsLoaded(result) => Update::Continue({
            if let Ok(state) = result {
                let platform =
                    state.platform.as_deref().and_then(RuntimePlatform::from_gateway_str);
                app.apply_external_apps_state(platform, state.apps);
            }
            Task::none()
        }),
        Message::GatewayHealthTick => Update::Continue({
            let servers = app.gateway_client_settings.servers.clone();
            if servers.is_empty() {
                Task::none()
            } else {
                Task::perform(
                    message::gateway_health::check_servers(servers),
                    Message::GatewayHealthChecked,
                )
            }
        }),
        Message::GatewayHealthChecked(results) => Update::Continue({
            #[cfg(not(target_arch = "wasm32"))]
            let selected_gateway_is_healthy =
                selected_gateway_health_key(&app.gateway_client_settings).is_some_and(|key| {
                    results.iter().any(|(candidate, healthy)| *healthy && candidate == &key)
                });
            #[cfg(target_arch = "wasm32")]
            let selected_gateway_became_healthy =
                selected_gateway_became_healthy(&app.gateway_client_settings, &results);
            desktop_settings_network::bootstrap::apply_health_results(
                &mut app.gateway_client_settings,
                results,
            );
            #[cfg(not(target_arch = "wasm32"))]
            if app.startup_state == crate::app::state::NativeStartupState::Ready
                && selected_gateway_is_healthy
            {
                return Update::Return(Task::batch([
                    app.startup_configuration
                        .load(crate::app::config::gateway_client_endpoint())
                        .map(|(generation, snapshot)| {
                            Message::StartupGatewaySnapshotLoaded(generation, snapshot)
                        }),
                    app.load_startup_sessions(),
                ]));
            }
            #[cfg(target_arch = "wasm32")]
            if selected_gateway_became_healthy {
                return Update::Return(Task::batch(vec![
                    Task::perform(crate::app::config::load_full_agent_config_async(), |result| {
                        Message::StartupAgentConfigLoaded(result.map(Box::new))
                    }),
                    crate::app::message::settings::reload_agents_config_task(
                        app.agents_settings.project_path.clone(),
                    ),
                ]));
            }
            Task::none()
        }),
        _ => unreachable!("non-startup message routed to startup adapter"),
    }
}
