use super::*;

fn ready_app() -> App {
    #[allow(unused_mut)]
    let mut app = App::new().0;
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.startup_state = crate::app::state::NativeStartupState::Ready;
    }
    app
}

fn update(app: &mut App, message: ViewMessage) {
    let _ = app.update(Message::View(message));
}

#[test]
fn stepping_back_and_forward_restores_pages_and_preserves_the_branch_until_navigation() {
    let mut app = ready_app();
    let back = ViewMessage::NavigateHistory(HistoryDirection::Back, None);
    let forward = ViewMessage::NavigateHistory(HistoryDirection::Forward, None);

    update(&mut app, ViewMessage::OpenApps);
    update(&mut app, ViewMessage::OpenKnowledge);
    assert_eq!(app.navigation_tabs.screen, Screen::Knowledge);
    assert!(app.navigation_tabs.open_tabs.iter().all(|tab| tab.screen != Screen::Apps));

    update(&mut app, back.clone());
    assert_eq!(app.navigation_tabs.screen, Screen::Apps);
    update(&mut app, forward.clone());
    assert_eq!(app.navigation_tabs.screen, Screen::Knowledge);
    update(&mut app, back);
    update(&mut app, ViewMessage::GoHome);
    update(&mut app, forward);
    assert_eq!(app.navigation_tabs.screen, Screen::Home);
}

#[test]
fn settings_entry_is_recorded_once_and_its_tab_can_be_revisited() {
    let mut app = ready_app();
    let back = ViewMessage::NavigateHistory(HistoryDirection::Back, None);
    let forward = ViewMessage::NavigateHistory(HistoryDirection::Forward, None);

    update(&mut app, ViewMessage::OpenSystemSettingsTab(SystemTab::Browser));
    assert!(app.navigation_panels.show_system_settings);
    assert_eq!(app.navigation_panels.system_settings_tab, SystemTab::Browser);

    update(&mut app, back);
    assert!(!app.navigation_panels.show_system_settings);
    assert_eq!(app.navigation_tabs.screen, Screen::Home);

    update(&mut app, forward);
    assert!(app.navigation_panels.show_system_settings);
    assert_eq!(app.navigation_panels.system_settings_tab, SystemTab::Browser);
}

#[test]
fn history_shortcut_is_scoped_to_the_main_window_and_blocked_by_about_dialog() {
    let mut app = ready_app();
    let main = window::Id::unique();
    app.main_window_id = Some(main);
    update(&mut app, ViewMessage::OpenApps);

    update(
        &mut app,
        ViewMessage::NavigateHistory(HistoryDirection::Back, Some(window::Id::unique())),
    );
    assert_eq!(app.navigation_tabs.screen, Screen::Apps);

    app.navigation_panels.show_about_modal = true;
    let back = ViewMessage::NavigateHistory(HistoryDirection::Back, Some(main));
    update(&mut app, back.clone());
    assert_eq!(app.navigation_tabs.screen, Screen::Apps);

    app.navigation_panels.show_about_modal = false;
    update(&mut app, back);
    assert_eq!(app.navigation_tabs.screen, Screen::Home);
}
