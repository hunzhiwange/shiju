use super::*;

#[test]
fn managed_versions_recognize_old_installs_and_reject_custom_definitions() {
    let root = PathBuf::from("gateway-bin").join("rustcodegraph");
    let old_binary = rustcodegraph::binary_at(&root, "v1.2.10");
    let mut old = json!({"command": old_binary, "args": rustcodegraph::ARGS, "enabled": false});
    assert_eq!(
        managed_version(&old, McpExtension::RustCodeGraph, &root),
        Version::parse("1.2.10").ok()
    );
    old["command"] = json!(rustcodegraph::binary_at(FilePath::new("custom"), "v1.2.10"));
    assert_eq!(managed_version(&old, McpExtension::RustCodeGraph, &root), None);

    let npx = FilePath::new(npx_name());
    let mut args = chrome_args();
    args[1] = "chrome-devtools-mcp@1.13.0".into();
    let mut old = json!({"type": "stdio", "command": npx_name(), "args": args});
    assert_eq!(
        managed_version(&old, McpExtension::ChromeDevTools, npx),
        Version::parse("1.13.0").ok()
    );
    old["args"][2] = json!("--autoConnect");
    assert_eq!(managed_version(&old, McpExtension::ChromeDevTools, npx), None);
}

#[test]
fn install_status_distinguishes_fresh_install_upgrade_and_installed_versions() {
    let old = Version::new(1, 9, 0);
    let target = Version::new(1, 10, 0);
    let fresh = install_status(None, false, Ok(target.clone()));
    assert!(fresh.installed_version.is_none());
    assert!(
        matches!(fresh.action, Some(McpInstallAction::Install { version }) if version == "1.10.0")
    );

    let upgrade = install_status(Some(old.clone()), true, Ok(target.clone()));
    assert_eq!(upgrade.installed_version.as_deref(), Some("1.9.0"));
    assert!(
        matches!(upgrade.action, Some(McpInstallAction::Upgrade { version }) if version == "1.10.0")
    );

    let repair = install_status(Some(old), false, Ok(target.clone()));
    assert!(repair.installed_version.is_none());
    assert!(matches!(repair.action, Some(McpInstallAction::Install { .. })));

    let installed = install_status(Some(target.clone()), true, Ok(target.clone()));
    assert_eq!(installed.installed_version.as_deref(), Some("1.10.0"));
    assert!(installed.action.is_none());

    let newer = install_status(Some(Version::new(2, 0, 0)), true, Ok(target.clone()));
    assert_eq!(newer.installed_version.as_deref(), Some("2.0.0"));
    assert!(newer.action.is_none());

    let offline = install_status(Some(target), true, Err("offline".into()));
    assert_eq!(offline.installed_version.as_deref(), Some("1.10.0"));
    assert!(offline.action.is_none());
    assert_eq!(offline.warning.as_deref(), Some("offline"));
}

#[test]
fn install_access_rejects_nonlocal_proxy_and_browser_requests() {
    let local = "127.0.0.1:12345".parse().unwrap();
    let remote = "192.0.2.1:12345".parse().unwrap();
    let mut headers = HeaderMap::new();
    assert!(admit(local, &headers, "chrome-devtools").is_ok());
    assert_eq!(admit(local, &headers, "browse").ok(), Some(McpExtension::OpenCodeBrowse));
    assert!(admit(local, &headers, "unlisted").is_err());
    assert!(admit(remote, &headers, "chrome-devtools").is_err());
    headers.insert("x-forwarded-for", "192.0.2.1".parse().unwrap());
    assert!(admit(local, &headers, "chrome-devtools").is_err());
    headers.clear();
    headers.insert("origin", "https://example.com".parse().unwrap());
    assert!(admit(local, &headers, "chrome-devtools").is_err());
}

#[test]
fn release_metadata_requires_the_official_stable_tag_and_exact_asset() {
    let tag = |url| {
        let url = reqwest::Url::parse(url).unwrap();
        rustcodegraph::stable_release(&url).map(|(tag, _)| tag.to_owned())
    };
    assert_eq!(
        tag("https://github.com/hunzhiwange/rustcodegraph/releases/tag/v2.0.1").as_deref(),
        Some("v2.0.1"),
    );
    assert!(tag("https://github.com/hunzhiwange/rustcodegraph/releases/tag/v2.1.0-rc.1").is_none());
    assert!(tag("https://example.com/hunzhiwange/rustcodegraph/releases/tag/v2.0.1").is_none());
    assert!(tag("https://github.com/someone/rustcodegraph/releases/tag/v2.0.1").is_none());

    let asset = "rustcodegraph-aarch64-apple-darwin.tar.xz";
    let hash = "AB".repeat(32);
    let matching = format!("{hash} *{asset}\n");
    assert_eq!(
        rustcodegraph::package_checksum(matching.as_bytes(), asset),
        Some(hash.to_lowercase())
    );
    assert!(rustcodegraph::package_checksum(matching.as_bytes(), "other.tar.xz").is_none());
    assert!(
        rustcodegraph::package_checksum(b"invalid-digest *other.tar.xz", "other.tar.xz").is_none()
    );
}
