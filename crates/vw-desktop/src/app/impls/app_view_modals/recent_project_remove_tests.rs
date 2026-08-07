use super::with_recent_project_remove_confirm;
use crate::app::{App, Message};
use iced::Element;
use iced::widget::text;

fn keep_element(element: Element<'_, Message>) {
    let _ = element.as_widget().size();
}

#[test]
fn without_pending_path_returns_root_content_unchanged() {
    let app = App::new().0;
    let root = text("root").into();
    keep_element(with_recent_project_remove_confirm(&app, root));
}

#[test]
fn with_pending_path_builds_confirm_overlay() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/tmp/demo-project".to_string()];
    app.recent_projects_edits = vec!["Demo".to_string()];
    app.recent_project_remove_confirm_path = Some("/tmp/demo-project".to_string());
    let root = text("root").into();
    keep_element(with_recent_project_remove_confirm(&app, root));
}
