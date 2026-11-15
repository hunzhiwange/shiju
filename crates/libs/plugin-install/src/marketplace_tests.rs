use super::marketplace::*;
use serde_json::json;
fn fixture(value: serde_json::Value) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".agents/plugins")).unwrap();
    std::fs::write(dir.path().join(".agents/plugins/marketplace.json"), value.to_string()).unwrap();
    dir
}
#[test]
fn codex_plugin_compat_t47_resolves_declared_marketplace_sources() {
    let d = fixture(
        json!({"name":"catalog","plugins":[{"name":"local","source":"./pkg","version":"1","policy":{"installation":"NOT_AVAILABLE","authentication":"ON_USE"}},{"name":"git","source":{"source":"url","url":"owner/repo","ref":"main"}},{"name":"npm","source":{"source":"npm","package":"@scope/pkg","version":"2"}}]}),
    );
    let m = load_plugin_marketplace(d.path()).unwrap();
    assert_eq!(m.name, "catalog");
    assert_eq!(m.plugins.len(), 3);
    assert_eq!(m.plugins[0].identity.as_key(), "local@catalog");
    assert_eq!(
        m.plugins[0].source,
        MarketplaceSource::Local(d.path().canonicalize().unwrap().join("pkg"))
    );
    assert_eq!(m.plugins[0].policy.installation, InstallationPolicy::NotAvailable);
    assert_eq!(m.plugins[0].policy.authentication, AuthenticationPolicy::OnUse);
    assert_eq!(m.plugins[0].manifest_fallback["version"], "1");
    assert!(
        matches!(&m.plugins[1].source,MarketplaceSource::Git{url,..} if url=="https://github.com/owner/repo.git")
    );
    assert!(matches!(&m.plugins[2].source,MarketplaceSource::Npm{version:Some(v),..} if v=="2"));
}
#[test]
fn codex_plugin_compat_t47_rejects_invalid_input() {
    for plugins in [
        json!([{"name":"x","source":"./../escape"}]),
        json!([{"name":"x","source":"./pkg"},{"name":"x","source":"./other"}]),
        json!([{"name":"x","source":{"source":"npm","package":"../bad"}}]),
        json!([{"name":"x","source":"./pkg","policy":{"installation":"ALLOW"}}]),
    ] {
        let d = fixture(json!({"name":"catalog","plugins":plugins}));
        assert!(load_plugin_marketplace(d.path()).is_err());
    }
}
#[test]
fn codex_plugin_compat_t47_preserves_isolation() {
    let d = fixture(json!({"name":"catalog","plugins":[{"name":"x","source":"./pkg"}]}));
    let path = d.path().join(".agents/plugins/marketplace.json");
    let before = std::fs::read(&path).unwrap();
    std::fs::create_dir_all(d.path().join(".claude-plugin")).unwrap();
    std::fs::write(d.path().join(".claude-plugin/marketplace.json"), "invalid").unwrap();
    assert_eq!(load_plugin_marketplace(d.path()).unwrap().plugins.len(), 1);
    assert_eq!(before, std::fs::read(path).unwrap());
    assert!(!d.path().join("pkg").exists());
    let outside = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), d.path().join("pkg")).unwrap();
        assert!(load_plugin_marketplace(d.path()).is_err());
    }
}

#[test]
fn codex_plugin_compat_t47_supports_all_layouts_and_registry_boundaries() {
    for layout in [
        ".agents/plugins/marketplace.json",
        ".agents/plugins/api_marketplace.json",
        ".claude-plugin/marketplace.json",
        ".cursor-plugin/marketplace.json",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(layout);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            json!({"name":"catalog","plugins":[{"name":"x","source":"./pkg"}]}).to_string(),
        )
        .unwrap();
        assert_eq!(load_plugin_marketplace(&path).unwrap().plugins.len(), 1);
        assert_eq!(load_plugin_marketplace(dir.path()).unwrap().name, "catalog");
    }
    for source in [
        json!({"source":"npm","package":"pkg","version":"file:../pkg"}),
        json!({"source":"npm","package":"pkg","registry":"http://registry.example"}),
        json!({"source":"npm","package":"pkg","registry":"https://registry.example/?token=secret"}),
        json!({"source":"npm","package":"_pkg"}),
        json!({"source":"url","url":"../escape"}),
        json!({"source":"git-subdir","url":"owner/repo","path":"../escape"}),
    ] {
        let dir = fixture(json!({"name":"catalog","plugins":[{"name":"x","source":source}]}));
        assert!(load_plugin_marketplace(dir.path()).is_err());
    }
}
