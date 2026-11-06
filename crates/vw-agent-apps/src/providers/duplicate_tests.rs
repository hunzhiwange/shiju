use super::duplicate::duplicate;
use vw_api_types::{agent_apps::AppId, app_providers::duplicate::AppProviderDuplicate};
fn source() -> serde_json::Value {
    serde_json::json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-secret","baseUrl":"https://fixture.example.test","model":"fixture-model","websiteUrl":"https://fixture.example.test","category":"custom","icon":"fixture-icon","iconColor":"#ffffff","notes":"description","meta":{"notes":"nested description","authBinding":{"accountId":"fixture-account"},"githubAccountId":"fixture-account","liveConfigManaged":true,"unknownFutureField":"deny"},"createdAt":123,"sortIndex":4,"inFailoverQueue":true,"current":true,"login":{"token":"fixture-login"},"unknown":"deny"})
}
fn request() -> AppProviderDuplicate {
    AppProviderDuplicate { new_id: "copy".into(), name: "fixture copy".into() }
}
#[test]
fn cc04_duplicate_copies_editable_fields_only() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("source.json");
    let target = root.path().join("copy.json");
    let before = serde_json::to_vec(&source()).unwrap();
    std::fs::write(&original, &before).unwrap();
    let summary = duplicate(AppId::Claude, "source", &source(), &request(), |copy| {
        std::fs::write(&target, serde_json::to_vec(copy).unwrap()).unwrap();
        Ok(true)
    })
    .unwrap();
    let copied: serde_json::Value =
        serde_json::from_slice(&std::fs::read(target).unwrap()).unwrap();
    assert_eq!(summary.id, "copy");
    let expected = serde_json::json!({"name":"fixture copy","protocol":"anthropic","apiKey":"fixture-secret","baseUrl":"https://fixture.example.test","model":"fixture-model","websiteUrl":"https://fixture.example.test","category":"custom","icon":"fixture-icon","iconColor":"#ffffff","notes":"description","meta":{"notes":"nested description"}});
    assert_eq!(copied, expected, "only allowlisted editable fields may be copied");
    assert_eq!(std::fs::read(original).unwrap(), before);
}
#[test]
fn cc04_duplicate_rejects_collision() {
    use std::io::Write;
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("copy.json");
    std::fs::write(&target, b"existing").unwrap();
    let result = duplicate(AppId::Claude, "source", &source(), &request(), |copy| {
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&target) {
            Ok(mut f) => {
                f.write_all(&serde_json::to_vec(copy).unwrap()).unwrap();
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(_) => Err("fixture persistence failed".into()),
        }
    });
    assert_eq!(result.unwrap_err(), "application provider already exists");
    assert_eq!(std::fs::read(target).unwrap(), b"existing");
    for id in ["source", "../bad", ""] {
        let input = AppProviderDuplicate { new_id: id.into(), name: "copy".into() };
        assert!(
            duplicate(AppId::Claude, "source", &source(), &input, |_| panic!(
                "must reject before insertion"
            ))
            .is_err()
        );
    }
}

#[test]
fn cc04_duplicate_rejects_invalid_profiles_and_drops_opaque_metadata() {
    for name in ["", "bad\nname"] {
        let input = AppProviderDuplicate { name: name.into(), ..request() };
        assert!(
            duplicate(AppId::Claude, "source", &source(), &input, |_| panic!(
                "invalid input must not persist"
            ))
            .is_err()
        );
    }
    let mut input = source();
    input["apiKey"] = serde_json::Value::Null;
    assert!(
        duplicate(AppId::Claude, "source", &input, &request(), |_| panic!(
            "login-only record must not persist"
        ))
        .is_err()
    );
    let mut input = source();
    input["notes"] = serde_json::json!({"token":"fixture-login"});
    input["meta"]["notes"] = serde_json::json!({"token":"fixture-login"});
    duplicate(AppId::Claude, "source", &input, &request(), |copy| {
        assert!(copy.get("notes").is_none());
        assert!(copy.get("meta").is_none());
        Ok(true)
    })
    .unwrap();
}
