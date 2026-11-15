use super::{session_loading_placeholder, empty_session_placeholder};
use crate::app::App;
#[test]
fn session_loading_placeholder_produces_element_with_default_app() {
    let app = App::new().0;
    let _element = session_loading_placeholder(&app);
}

#[test]
fn session_loading_placeholder_produces_element_with_context() {
    let mut app = App::new().0;
    app.active_session_id = Some("test-id".into());
    app.project_path = Some("/project".into());
    app.selected_branch = Some("main".into());
    app.project_updated_at_ms = Some(1_000_000);
    let _element = session_loading_placeholder(&app);
}

#[test]
fn empty_session_placeholder_produces_element_with_default_app() {
    let app = App::new().0;
    let _element = empty_session_placeholder(&app);
}

#[test]
fn empty_session_placeholder_produces_element_with_context() {
    let mut app = App::new().0;
    app.project_path = Some("/project".into());
    app.project_is_git_repo = true;
    app.selected_branch = Some("develop".into());
    app.project_updated_at_ms = Some(2_000_000);
    let _element = empty_session_placeholder(&app);
}
