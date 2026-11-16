use crate::components::top_bar::external::{
    OPEN_EXTERNAL_TARGETS, State, open_external_module, supported_external_apps,
};
use crate::components::top_bar::{
    Menu as MenuType,
    types::{ExternalOpenApp, RuntimePlatform},
};
use crate::ui_types::Screen;

struct Fixture {
    open_external_exists: std::collections::HashMap<ExternalOpenApp, bool>,
    screen: Screen,
    active_menu: Option<MenuType>,
    has_project: bool,
    open_external_app: ExternalOpenApp,
    open_external_platform: Option<RuntimePlatform>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            open_external_exists: Default::default(),
            screen: Screen::Home,
            active_menu: None,
            has_project: false,
            open_external_app: ExternalOpenApp::VSCode,
            open_external_platform: None,
        }
    }
}
impl Fixture {
    fn state(&self) -> State<'_> {
        State {
            screen: self.screen,
            desktop_locale: Default::default(),
            open_external_exists: &self.open_external_exists,
            active_menu: self.active_menu,
            has_project: self.has_project,
            open_external_app: self.open_external_app,
            open_external_platform: self.open_external_platform,
        }
    }
}
#[test]
fn supported_external_apps_keeps_menu_order_and_includes_cached_false_entries() {
    let mut app = Fixture::default();
    app.open_external_exists.clear();
    app.open_external_exists.insert(ExternalOpenApp::VSCode, false);
    app.open_external_exists.insert(ExternalOpenApp::Finder, true);
    app.open_external_exists.insert(ExternalOpenApp::Terminal, true);

    assert_eq!(
        supported_external_apps(&app.state()),
        [ExternalOpenApp::Finder, ExternalOpenApp::VSCode, ExternalOpenApp::Terminal]
    );
}

#[test]
fn supported_external_apps_is_empty_without_cached_targets() {
    let mut app = Fixture::default();
    app.open_external_exists.clear();

    assert!(supported_external_apps(&app.state()).is_empty());
}

#[test]
fn open_external_module_builds_empty_element_outside_project_screen() {
    let mut app = Fixture::default();
    app.screen = Screen::Home;

    let element = open_external_module(&app.state());

    let _ = std::hint::black_box(element);
}

#[test]
fn open_external_module_builds_open_menu_without_project_path_or_targets() {
    let mut app = Fixture::default();
    app.screen = Screen::Project;
    app.has_project = false;
    app.active_menu = Some(MenuType::OpenExternal);
    app.open_external_exists.clear();

    let element = open_external_module(&app.state());

    let _ = std::hint::black_box(element);
}

#[test]
fn open_external_module_builds_disabled_preferred_target_with_copy_available() {
    let mut app = Fixture::default();
    app.screen = Screen::Project;
    app.has_project = true;
    app.open_external_app = ExternalOpenApp::VSCode;
    app.open_external_exists.clear();
    app.open_external_exists.insert(ExternalOpenApp::VSCode, false);

    let element = open_external_module(&app.state());

    let _ = std::hint::black_box(element);
}

#[test]
fn open_external_module_builds_project_controls_for_available_targets() {
    let mut app = Fixture::default();
    app.screen = Screen::Project;
    app.has_project = true;
    app.open_external_app = ExternalOpenApp::VSCode;
    app.open_external_platform = Some(RuntimePlatform::MacOs);
    app.open_external_exists.clear();
    for target in OPEN_EXTERNAL_TARGETS {
        app.open_external_exists.insert(target, true);
    }

    let element = open_external_module(&app.state());

    let _ = std::hint::black_box(element);
}
