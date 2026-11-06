use super::patch_yaml::{ProviderSection, patch_provider};
use serde_json::json;

#[test]
fn cc04_patch_yaml_preserves_other_nodes() {
    for (section, name) in [
        (ProviderSection::Providers, "providers"),
        (ProviderSection::CustomProvider, "custom_provider"),
    ] {
        let prefix = format!(
            "# 中文 🙂\nother: 'quoted' # keep\n{name}: # catalog\n    first:\n        name: Search & Fetch * x\n\n    # target\n"
        );
        let suffix = "\n    # last\n    last:\n        headers: {X-Test: 'verbatim'}\n\n# after\nother2: [1, 2]\n";
        let before = format!("{prefix}    'target:key':\n        apiKey: old # replaced\n{suffix}");
        let node = json!({"apiKey":"fake-new", "models":[{"id":"m"}]});
        let after =
            patch_provider(Some(before.as_bytes()), section, "target:key", Some(&node)).unwrap();
        assert_ne!(after, before.as_bytes());
        let text = std::str::from_utf8(&after).unwrap();
        assert!(text.starts_with(&prefix) && text.ends_with(suffix));
        assert!(text.contains("fake-new") && !text.contains("old # replaced"));
        assert_eq!(
            patch_provider(Some(&after), section, "target:key", Some(&node)).unwrap(),
            after
        );
        let removed = patch_provider(Some(&after), section, "target:key", None).unwrap();
        assert_eq!(removed, format!("{prefix}{suffix}").as_bytes());
    }
}

#[test]
fn cc04_patch_yaml_rejects_unsafe_structure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models.yml");
    for text in [
        "providers: {p: {apiKey: key}}\n",
        "providers:\n  p: &anchor\n    apiKey: key\n",
        "base: &anchor {}\nproviders:\n  p: *anchor\n",
        "providers:\n  p: !custom {apiKey: key}\n",
        "providers:\n  <<: {p: {apiKey: key}}\n",
        "providers:\n  p: {apiKey: a}\n  p: {apiKey: b}\n",
        "providers:\n  p:\n    apiKey: a\n    apiKey: b\n",
        "providers:\n\tp: {}\n",
        "[]\n",
        "providers: 42\n",
        "---\nproviders:\n...\n",
        "providers:\n  p:\n    headers: [*anchor]\n",
        "providers:\n  p:\n    compat: {a: !!str b}\n",
        "providers:\n  p:\n    headers: {'a'' \"x:\"': &a hello}\n",
        "providers:\n  p:\n    headers: {X: -'abc, Y: &a b}\n",
        "providers:\n  p:\n    models:\n      - - *anchor\n",
        "providers:\n  p: [broken\n",
    ] {
        std::fs::write(&path, text).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(
            patch_provider(
                Some(&before),
                ProviderSection::Providers,
                "p",
                Some(&json!({"apiKey":"fake"}))
            )
            .is_err(),
            "unsafe input accepted"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn cc04_patch_yaml_creates_and_removes_nodes_without_losing_surroundings() {
    for (section, name) in [
        (ProviderSection::Providers, "providers"),
        (ProviderSection::CustomProvider, "custom_provider"),
    ] {
        for before in [
            String::new(),
            "# keep\n".into(),
            format!("{name}:\n"),
            format!("{name}: null # keep\n"),
            format!("{name}: ~\n"),
            format!("{name}:\n  {{}}\n"),
        ] {
            let node = json!({"name":"中文🙂", "apiKey":"fake", "note":"Search & Fetch * x ! What?! a-!b"});
            let added =
                patch_provider(Some(before.as_bytes()), section, "a.b:'quoted'", Some(&node))
                    .unwrap();
            let value: serde_yaml::Value = serde_yaml::from_slice(&added).unwrap();
            assert_eq!(value[name]["a.b:'quoted'"], serde_yaml::to_value(&node).unwrap());
            let removed = patch_provider(Some(&added), section, "a.b:'quoted'", None).unwrap();
            let value: serde_yaml::Value = serde_yaml::from_slice(&removed).unwrap();
            assert!(value[name].as_mapping().unwrap().is_empty());
            assert_eq!(patch_provider(Some(&removed), section, "missing", None).unwrap(), removed);
            assert!(patch_provider(Some(&removed), section, "new", Some(&node)).is_ok());
        }
        let before = format!(
            "# 中文\r\n{name}: # keep\r\n  p: {{}}\r\n\r\n# trailing\r\nother: 'bytes'\r\n"
        );
        let after =
            patch_provider(Some(before.as_bytes()), section, "new", Some(&json!({}))).unwrap();
        let after = std::str::from_utf8(&after).unwrap();
        assert!(after.starts_with(&format!("# 中文\r\n{name}: # keep\r\n  p: {{}}\r\n")));
        assert!(after.ends_with("\r\n# trailing\r\nother: 'bytes'\r\n"));
    }
    assert_eq!(patch_provider(None, ProviderSection::Providers, "missing", None).unwrap(), b"");
}

#[test]
fn cc04_patch_yaml_rejects_invalid_nodes_and_input_limits() {
    use super::patch_yaml::PatchError;
    for node in [json!(null), json!([]), json!("scalar"), json!(true)] {
        assert_eq!(
            patch_provider(None, ProviderSection::Providers, "p", Some(&node)),
            Err(PatchError::InvalidNode)
        );
    }
    assert_eq!(
        patch_provider(None, ProviderSection::Providers, "", Some(&json!({}))),
        Err(PatchError::InvalidNode)
    );
    assert!(
        patch_provider(Some(&[0xff]), ProviderSection::Providers, "p", Some(&json!({}))).is_err()
    );
    assert!(
        patch_provider(
            Some(&vec![b' '; 1024 * 1024 + 1]),
            ProviderSection::Providers,
            "p",
            Some(&json!({}))
        )
        .is_err()
    );
    assert!(
        patch_provider(
            None,
            ProviderSection::Providers,
            "p",
            Some(&json!({"name":"x".repeat(1024*1024)}))
        )
        .is_err()
    );
    // A token in another root node may reference the edited subtree; reject globally.
    let before = b"other: &a {}\nproviders:\n  p: {}\n";
    assert!(
        patch_provider(Some(before), ProviderSection::Providers, "p", Some(&json!({}))).is_err()
    );
    for before in ["providers:\n  3: {}\n", "providers:\n  p: {}\n---\nother: {}\n"] {
        assert!(
            patch_provider(
                Some(before.as_bytes()),
                ProviderSection::Providers,
                "p",
                Some(&json!({}))
            )
            .is_err()
        );
    }
}
