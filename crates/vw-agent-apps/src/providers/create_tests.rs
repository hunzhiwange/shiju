use vw_api_types::{
    agent_apps::AppId,
    app_providers::{ProviderProtocol, create::*},
};
fn request() -> AppProviderCreate {
    AppProviderCreate {
        name: "fixture".into(),
        protocol: ProviderProtocol::Anthropic,
        api_key: ProviderSecretWrite::Replace { value: "fixture-private-key".into() },
        base_url: "https://fixture.example.test".into(),
        model: "fixture-model".into(),
        add_to_live: false,
    }
}
use super::create::create;
// Real isolated file persistence exercises the callback contract; SQLite is tested in agent.
fn persist(
    path: &std::path::Path,
    profile: &vw_api_types::app_providers::AppProviderWrite,
) -> Result<bool, String> {
    use std::io::Write;
    match std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(&serde_json::to_vec(profile).unwrap()).unwrap();
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(_) => Err("application provider storage unavailable".into()),
    }
}
#[test]
fn cc04_create_does_not_write_live() {
    let root = tempfile::tempdir().unwrap();
    let live = root.path().join("settings.json");
    std::fs::write(&live, b"untouched native configuration").unwrap();
    let record = root.path().join("record.json");
    let summary = create(AppId::Claude, "new", &request(), |p| persist(&record, p)).unwrap();
    assert_eq!(summary.id, "new");
    assert_eq!(std::fs::read(&live).unwrap(), b"untouched native configuration");
    assert!(record.exists());
    let output = serde_json::to_string(&summary).unwrap();
    assert!(!output.contains("fixture-private-key"));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
}
#[test]
fn cc04_create_duplicate_is_atomic() {
    let root = tempfile::tempdir().unwrap();
    let record = root.path().join("record.json");
    create(AppId::Claude, "new", &request(), |p| persist(&record, p)).unwrap();
    let before = std::fs::read(&record).unwrap();
    let mut changed = request();
    changed.name = "replacement".into();
    assert!(create(AppId::Claude, "new", &changed, |p| persist(&record, p)).is_err());
    assert_eq!(std::fs::read(record).unwrap(), before);
}
#[test]
fn cc04_create_rejects_implicit_secrets_live_and_invalid_input_before_storage() {
    for secret in [ProviderSecretWrite::Keep, ProviderSecretWrite::Clear] {
        let mut input = request();
        input.api_key = secret;
        assert!(
            create(AppId::Claude, "new", &input, |_| panic!("invalid input reached storage"))
                .is_err()
        );
    }
    let mut input = request();
    input.add_to_live = true;
    assert!(
        create(AppId::Claude, "new", &input, |_| panic!("activation reached storage")).is_err()
    );
    assert!(
        create(AppId::Claude, "../bad", &request(), |_| panic!("invalid id reached storage"))
            .is_err()
    );
    assert!(
        create(AppId::Claude, "new", &request(), |_| Err(
            "application provider storage unavailable".into()
        ))
        .is_err()
    );
}
