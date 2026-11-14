use super::*;

#[test]
fn cp12_install_view_rejects_without_side_effects() {
    let mut app = App::new().0;
    // A late response cannot reopen a closed panel or populate its inventory.
    let _ = update(&mut app, Event::Loaded(0, Ok(SkillInventoryResponse {
        scope: SkillReposQuery { project_id: "project".into(), target_host_id: "host".into() },
        version: 7, skills: vec![],
    })));
    assert!(!app.skills_settings.install.open);
    assert!(app.skills_settings.install.inventory.is_none());
}

#[test]
fn cp12_install_view_success() {
    let mut app = App::new().0;
    app.skills_settings.install.open = true;
    app.skills_settings.install.generation = 4;
    let _ = update(&mut app, Event::Close);
    assert!(!app.skills_settings.install.open);
    assert_eq!(app.skills_settings.install.generation, 5);
}
