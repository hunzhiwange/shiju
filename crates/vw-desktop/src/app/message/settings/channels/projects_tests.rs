//! 通道候选直接使用项目管理列表，保留名称与顺序。
use super::project_choices;
use crate::app::App;

#[test]
fn project_choices_uses_managed_projects_names_and_order() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/data/two".into(), "/data/one".into()];
    app.recent_projects_edits = vec!["项目二".into(), "项目一".into()];
    assert_eq!(
        project_choices(&app),
        vec![("/data/two".into(), "项目二".into()), ("/data/one".into(), "项目一".into()),]
    );
}

#[test]
fn project_choices_uses_directory_name_when_managed_name_is_blank() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/data/one".into()];
    app.recent_projects_edits = vec![" ".into()];
    app.recent_projects_meta.clear();
    assert_eq!(project_choices(&app), vec![("/data/one".into(), "one".into())]);
}
