use super::*;
use super::super::repo_modal::{Event, update};
use crate::app::App;

#[test]
fn cp12_deeplink_success() {
    let request = parse("ccswitch://v1/import?resource=skill&repo=owner/name&branch=feature%2Fone&enabled=false",
        "/project".into(), "host".into()).unwrap();
    assert_eq!(request.repo.branch, "feature/one");
    assert!(!request.repo.enabled);
    assert_eq!(request.directory, "/project");
    let mut app = App::new().0;
    app.project_path = Some("/project".into());
    let _ = update(&mut app, Event::Open);
    app.skills_settings.repositories.busy = false;
    let _ = update(&mut app, Event::SourceLink("ccswitch://v1/import?resource=skill&repo=owner/name".into()));
    let _ = update(&mut app, Event::PreviewSource);
    assert!(app.skills_settings.repositories.source_preview.is_some());
    assert!(!app.skills_settings.repositories.busy);
    assert!(app.skills_settings.repositories.snapshot.is_none());
    let _ = update(&mut app, Event::CancelSource);
    assert!(app.skills_settings.repositories.source_preview.is_none());
}

#[test]
fn cp12_deeplink_rejects_without_side_effects() {
    for link in [
        "https://example.org/?resource=skill&repo=a/b",
        "ccswitch://v1/import?resource=provider&repo=a/b",
        "ccswitch://v1/import?resource=skill&repo=a/b&branch=..%2Fevil",
        "ccswitch://v1/import?resource=skill&repo=a/b&repo=c/d",
        "ccswitch://v1/import?resource=skill&repo=a/b&directory=/tmp",
        "ccswitch://v1/import?resource=skill&repo=a/..",
    ] {
        assert!(parse(link, "/project".into(), "host".into()).is_err(), "{link}");
    }
    let mut app = App::new().0;
    let _ = update(&mut app, Event::ConfirmSource);
    assert!(!app.skills_settings.repositories.busy);
    assert!(app.skills_settings.repositories.snapshot.is_none());
}
