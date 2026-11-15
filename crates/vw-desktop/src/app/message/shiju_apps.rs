//! Desktop tab, notification and native launch effects.
use crate::app::{App, AppTab, Message, Screen};
use crate::app::shiju_apps::ShijuAppLaunchState;
use iced::Task;
pub use desktop_apps_ui::shiju_apps::{ShijuAppsMessage, SHIJU_APPS_TAB_ID, SHIJU_APPS_TITLE};
pub(crate) fn update(app: &mut App, message: ShijuAppsMessage) -> Task<Message> {
    let mut input = desktop_apps_ui::shiju_apps::Input {
        shiju_apps: &mut app.shiju_apps, account: app.account.active_account(),
        client: crate::app::config::gateway_client(), effects: Vec::new(),
    };
    let task = desktop_apps_ui::shiju_apps::update(&mut input, message).map(Message::ShijuApps);
    let effects = input.effects;
    let mut tasks = vec![task];
    for effect in effects {
        use desktop_apps_ui::shiju_apps::Effect;
        tasks.push(match effect {
            Effect::Open => { app.expert_hub.install_guide.close(); open_shiju_apps_tab(app); Task::none() }
            Effect::Toast(text, args) => app.show_localized_toast(crate::app::state::ToastKind::Error, &text, args),
            Effect::Launch { target, generation, installation_id } => launch(app, target, generation, &installation_id),
        });
    }
    Task::batch(tasks)
}
fn open_shiju_apps_tab(app: &mut App) {
    if !app.open_tabs.iter().any(|tab| tab.id == SHIJU_APPS_TAB_ID) {
        app.open_tabs.push(AppTab {
            id: SHIJU_APPS_TAB_ID.to_string(),
            title: crate::app::i18n::I18n::new(app.desktop_locale).__(SHIJU_APPS_TITLE).to_string(),
            screen: Screen::ShijuApps,
            project_path: None,
        });
    }
    app.active_tab_id = Some(SHIJU_APPS_TAB_ID.to_string());
    app.screen = Screen::ShijuApps;
}

fn launch(app: &mut App, target: gateway_shiju_app::ShijuAppLaunchDto, generation: u64, installation_id: &str) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    match crate::app::shiju_apps::launch_shiju_app(&target, installation_id) {
        Ok(child) => {
            app.independent_webview_children.push(child);
            app.shiju_apps.launch = ShijuAppLaunchState::Idle;
            app.show_localized_toast(
                crate::app::state::ToastKind::Success,
                "识矩应用已安全打开",
                vec![],
            )
        }
        Err(error) => {
            let original = error.to_string();
            set_launch_error(app, generation, installation_id, original.clone());
            app.shiju_apps.launch_feedback =
                Some((generation, original.clone(), error.clone()));
            let task = app.show_error_toast(original);
            if let Some(toast) = &mut app.active_toast {
                toast.localized_message = Some(error.message());
            }
            task
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = target;
        set_launch_error(
            app,
            generation,
            installation_id,
            "当前平台不支持识矩应用 WebView".to_string(),
        );
        app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "当前平台不支持识矩应用 WebView",
            vec![],
        )
    }
}
fn set_launch_error(app: &mut App, generation: u64, installation_id: &str, message: String) {
    app.shiju_apps.launch_feedback = None;
    app.shiju_apps.launch = ShijuAppLaunchState::Error {
        generation,
        installation_id: installation_id.to_string(),
        message,
    };
}


#[cfg(test)]
#[path = "shiju_apps_tests.rs"]
mod shiju_apps_tests;
