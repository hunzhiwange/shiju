use crate::components::top_bar::types::ExternalOpenApp;
use crate::panel::external::State;
#[test]
fn can_open_external_requires_cached_true_value() {
    let mut app = State {
        open_external_app: ExternalOpenApp::VSCode,
        open_external_exists: Default::default(),
    };
    app.open_external_exists.clear();

    assert!(!app.available(ExternalOpenApp::VSCode));

    app.open_external_exists.insert(ExternalOpenApp::VSCode, false);
    assert!(!app.available(ExternalOpenApp::VSCode));

    app.open_external_exists.insert(ExternalOpenApp::VSCode, true);
    assert!(app.available(ExternalOpenApp::VSCode));
}
