use super::endpoints::{read, update};
use vw_api_types::app_providers::endpoints::{
    AppProviderEndpointChange as Change, AppProviderEndpointKind as Kind,
};
#[test]
fn cc04_endpoints_add_remove_and_last_used() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("profile.json");
    let original = serde_json::json!({"baseUrl":"https://active.example.test", "apiKey":"fixture-secret", "meta":{"binding":"fixture"}, "current":true});
    let add = Change::Add { url: " https://relay.example.test/v1/ ".into(), kind: Kind::BaseUrl };
    let value = update(&original, &add, 10).unwrap();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let reload = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows = read(&reload).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].url, "https://relay.example.test/v1");
    assert_eq!(rows[0].added_at, 10);
    assert_eq!(rows[0].last_used, None);
    let duplicate = update(&reload, &add, 15).unwrap();
    assert_eq!(duplicate, reload);
    let full =
        Change::Add { url: "https://relay.example.test/v1/messages/".into(), kind: Kind::FullUrl };
    let value = update(&duplicate, &full, 20).unwrap();
    let rows = read(&value).unwrap();
    assert_eq!(rows[0].url, "https://relay.example.test/v1/messages/");
    assert_eq!(rows[0].kind, Kind::FullUrl);
    let value =
        update(&value, &Change::MarkUsed { url: rows[0].url.clone(), kind: Kind::FullUrl }, 30)
            .unwrap();
    assert_eq!(read(&value).unwrap()[0].last_used, Some(30));
    let value =
        update(&value, &Change::Remove { url: rows[0].url.clone(), kind: Kind::FullUrl }, 40)
            .unwrap();
    assert_eq!(read(&value).unwrap().len(), 1);
    for key in ["baseUrl", "apiKey", "meta", "current"] {
        assert_eq!(value[key], original[key]);
    }
}
#[test]
fn cc04_endpoints_invalid_url_does_not_mutate() {
    let stored = serde_json::json!({"baseUrl":"https://active.example.test"});
    let before = stored.clone();
    for (url, kind) in [
        ("", Kind::BaseUrl),
        ("relative/v1", Kind::BaseUrl),
        ("ftp://relay.test/v1", Kind::BaseUrl),
        ("http://relay.test", Kind::BaseUrl),
        ("https://user:secret@relay.test/v1", Kind::FullUrl),
        ("https://relay.test/v1?token=secret", Kind::FullUrl),
        ("https://relay.test/#secret", Kind::BaseUrl),
        ("https://relay.test", Kind::FullUrl),
        ("https://relay.test/\npath", Kind::FullUrl),
    ] {
        assert!(
            update(&stored, &Change::Add { url: url.into(), kind }, 10).is_err(),
            "accepted {url}"
        );
        assert_eq!(stored, before);
    }
}
#[test]
fn cc04_endpoints_missing_usage_corrupt_storage_and_bound_are_rejected() {
    let stored = serde_json::json!({});
    assert!(
        update(
            &stored,
            &Change::MarkUsed { url: "https://missing.test/v1".into(), kind: Kind::BaseUrl },
            10
        )
        .is_err()
    );
    assert_eq!(
        update(
            &stored,
            &Change::Remove { url: "https://missing.test/v1".into(), kind: Kind::BaseUrl },
            10
        )
        .unwrap(),
        stored
    );
    assert!(read(&serde_json::json!({"customEndpoints":"broken"})).is_err());
    let mut value = stored;
    for i in 0..64 {
        value = update(
            &value,
            &Change::Add { url: format!("http://127.0.0.1:8080/v{i}"), kind: Kind::BaseUrl },
            i,
        )
        .unwrap();
    }
    assert!(
        update(
            &value,
            &Change::Add { url: "https://overflow.test/v1".into(), kind: Kind::BaseUrl },
            70
        )
        .is_err()
    );
    assert_eq!(read(&value).unwrap().len(), 64);
    let used = update(
        &value,
        &Change::MarkUsed { url: "http://127.0.0.1:8080/v63".into(), kind: Kind::BaseUrl },
        50,
    )
    .unwrap();
    assert_eq!(read(&used).unwrap()[0].last_used, Some(63));
    let again = update(
        &used,
        &Change::MarkUsed { url: "http://127.0.0.1:8080/v63".into(), kind: Kind::BaseUrl },
        40,
    )
    .unwrap();
    assert_eq!(read(&again).unwrap()[0].last_used, Some(63));
}
