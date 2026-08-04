use super::{PackageVersionCacheEntry, load_package_version_cache, write_package_version_cache};
use std::fs;

#[test]
fn package_version_cache_rejects_sensitive_or_invalid_payload_and_round_trips_version() {
    let temp = tempfile::tempdir().expect("cache fixture");
    let cache_path = temp.path().join("shiju-package-version.json");
    let entry =
        PackageVersionCacheEntry::new("3.1.0".to_string(), 1_785_830_400).expect("valid cache");

    assert_eq!(
        load_package_version_cache(&cache_path).expect("missing cache is a cold start"),
        None
    );
    write_package_version_cache(&cache_path, &entry).expect("write safe cache");

    assert_eq!(load_package_version_cache(&cache_path).expect("load safe cache"), Some(entry));
    let persisted = fs::read_to_string(&cache_path).expect("read cache fixture");
    assert!(!persisted.contains("https://"));
    assert!(!persisted.contains("signature"));
    assert!(!persisted.contains("credential"));

    fs::write(
        &cache_path,
        r#"{
            "server_version": "3.1.0",
            "checked_at_unix": 1785830400,
            "download_url": "https://skill.shiju.cloud/releases/3.1.0/shiju.zip?signature=secret"
        }"#,
    )
    .expect("write sensitive fixture");
    assert!(load_package_version_cache(&cache_path).is_err());

    fs::write(
        &cache_path,
        r#"{"server_version":"3.1.0?credential=secret","checked_at_unix":1785830400}"#,
    )
    .expect("write invalid fixture");
    assert!(load_package_version_cache(&cache_path).is_err());

    fs::write(&cache_path, b"{").expect("write damaged fixture");
    assert!(load_package_version_cache(&cache_path).is_err());
}
