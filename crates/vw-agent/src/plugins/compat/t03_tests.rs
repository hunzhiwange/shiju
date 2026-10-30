use crate::plugins::load_plugin_manifest;
use crate::plugins::{PluginHookSource, PluginMcpServers};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

fn write_manifest(root: &Path, directory: &str, value: &Value) {
    fs::create_dir_all(root.join(directory)).expect("manifest directory");
    fs::write(root.join(directory).join("plugin.json"), serde_json::to_vec(value).unwrap())
        .expect("manifest bytes");
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../testdata/plugin_compat/t03/legacy.json"))
        .expect("legacy fixture")
}

#[test]
fn codex_plugin_compat_t03_loads_legacy_manifest() {
    let temp = tempfile::tempdir().unwrap();
    write_manifest(temp.path(), ".codex-plugin", &fixture());
    let manifest =
        load_plugin_manifest(temp.path()).expect("legacy manifest loads without rewrite");
    let value = serde_json::to_value(manifest).unwrap();
    assert_eq!(value["name"], "legacy-example");
    assert_eq!(value["version"], Value::Null);
    assert_eq!(value["description"], Value::Null);
    assert_eq!(value["interface"]["default_prompt"], json!(["Use this plugin"]));
    assert_eq!(value["interface"]["website_url"], "https://example.com");
}

#[test]
fn codex_plugin_compat_t03_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    write_manifest(temp.path(), ".codex-plugin", &json!({"name": "legacy", "skills": false}));
    let manifest = load_plugin_manifest(temp.path()).expect("invalid optional skills are ignored");
    assert_eq!(serde_json::to_value(manifest).unwrap()["skills"], json!([]));
    for field in ["mcpServers", "hooks"] {
        let mut value = json!({"name": "legacy"});
        value[field] = json!(false);
        write_manifest(temp.path(), ".codex-plugin", &value);
        let manifest =
            load_plugin_manifest(temp.path()).expect("invalid optional shape is ignored");
        assert!(manifest.mcp_servers.is_none());
        assert!(manifest.hooks.is_empty());
    }
    write_manifest(temp.path(), ".codex-plugin", &json!({"name": 12}));
    let error = load_plugin_manifest(temp.path()).expect_err("invalid name must fail");
    assert!(error.to_string().contains("failed to parse plugin manifest"));
}

#[test]
fn codex_plugin_compat_t03_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    for (name, directory) in [("first", ".claude-plugin"), ("second", ".cursor-plugin")] {
        let root = temp.path().join(name);
        write_manifest(&root, directory, &json!({"name": name}));
        let path = root.join(directory).join("plugin.json");
        let before = fs::read(&path).unwrap();
        let loaded = load_plugin_manifest(&root).expect("each legacy package loads independently");
        assert_eq!(loaded.name, name);
        assert_eq!(loaded.root, root.canonicalize().unwrap());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        assert!(!root.join(".vibewindow-plugin").exists());
    }
}

#[test]
fn codex_plugin_compat_t03_selects_legacy_entries_in_frozen_order() {
    let temp = tempfile::tempdir().unwrap();
    let entries = [".codex-plugin", ".claude-plugin", ".cursor-plugin"];
    for entry in entries {
        write_manifest(temp.path(), entry, &json!({"name": entry}));
    }
    write_manifest(
        temp.path(),
        ".vibewindow-plugin",
        &json!({"name": "native", "version": "1", "description": "native package"}),
    );
    assert_eq!(load_plugin_manifest(temp.path()).unwrap().name, "native");
    fs::remove_dir_all(temp.path().join(".vibewindow-plugin")).unwrap();
    for entry in entries {
        let manifest = load_plugin_manifest(temp.path()).unwrap();
        assert_eq!(manifest.name, entry);
        assert_eq!(
            manifest.manifest_path,
            temp.path().join(entry).join("plugin.json").canonicalize().unwrap()
        );
        fs::remove_dir_all(temp.path().join(entry)).unwrap();
    }
    write_manifest(temp.path(), ".cursor-plugin", &json!({"name": "fallback"}));
    fs::create_dir_all(temp.path().join(".codex-plugin/plugin.json")).unwrap();
    let error =
        load_plugin_manifest(temp.path()).expect_err("do not skip malformed selected entry");
    assert!(error.to_string().contains("must be a file"));
}

#[test]
fn codex_plugin_compat_t03_retains_path_and_inline_component_shapes() {
    let temp = tempfile::tempdir().unwrap();
    for directory in ["skills/first", "skills/second", "hooks"] {
        fs::create_dir_all(temp.path().join(directory)).unwrap();
    }
    for file in
        [".mcp.json", ".app.json", "hooks/first.json", "hooks/second.json", "skills/first/SKILL.md"]
    {
        fs::write(temp.path().join(file), "{}").unwrap();
    }
    let server = json!({"sample": {"command": "unexecuted", "args": ["--example"]}});
    let inline_hook = json!({"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": "unexecuted"}]}]}});
    let base = json!({
        "name": "shapes", "skills": ["./skills/first", "./skills/second"],
        "mcpServers": server, "hooks": [inline_hook.clone(), inline_hook.clone()],
        "apps": "./.app.json", "extensions": {"com.openai": {"onboardingSkill": "skills/first/SKILL.md"}}
    });
    write_manifest(temp.path(), ".codex-plugin", &base);
    let before = fs::read(temp.path().join(".codex-plugin/plugin.json")).unwrap();
    let manifest = load_plugin_manifest(temp.path()).unwrap();
    assert_eq!(
        manifest.skills.iter().map(|path| path.raw.as_str()).collect::<Vec<_>>(),
        ["./skills/first", "./skills/second"]
    );
    assert!(
        matches!(manifest.mcp_servers, Some(PluginMcpServers::Inline(ref map)) if Value::Object(map.clone()) == server)
    );
    assert!(
        matches!(&manifest.hooks[..], [PluginHookSource::Inline(first), PluginHookSource::Inline(second)] if Value::Object(first.clone()) == inline_hook && first == second)
    );
    assert_eq!(manifest.apps.unwrap().raw, "./.app.json");
    assert_eq!(manifest.onboarding_skill.unwrap().raw, "./skills/first/SKILL.md");
    assert_eq!(fs::read(temp.path().join(".codex-plugin/plugin.json")).unwrap(), before);
    for hooks in [
        json!("./hooks/first.json"),
        json!(["./hooks/first.json", "./hooks/second.json"]),
        inline_hook,
    ] {
        let mut value = base.clone();
        value["skills"] = json!("./skills/first");
        value["mcpServers"] = json!("./.mcp.json");
        value["hooks"] = hooks.clone();
        write_manifest(temp.path(), ".codex-plugin", &value);
        let manifest = load_plugin_manifest(temp.path()).unwrap();
        assert_eq!(manifest.skills.len(), 1);
        assert!(
            matches!(manifest.mcp_servers, Some(PluginMcpServers::Path(ref path)) if path.raw == "./.mcp.json")
        );
        match hooks {
            Value::Object(_) => assert!(matches!(manifest.hooks[0], PluginHookSource::Inline(_))),
            Value::Array(_) => assert_eq!(manifest.hooks.len(), 2),
            _ => assert!(
                matches!(&manifest.hooks[0], PluginHookSource::Path(path) if path.raw == "./hooks/first.json")
            ),
        }
    }
}

#[test]
fn codex_plugin_compat_t03_normalizes_optional_metadata_and_prompts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("basename");
    write_manifest(
        &root,
        ".codex-plugin",
        &json!({"name": "  ", "version": " \t", "interface": {}}),
    );
    let manifest = load_plugin_manifest(&root).unwrap();
    assert_eq!(manifest.name, "basename");
    assert_eq!(manifest.version, None);
    assert_eq!(manifest.description, None);
    assert!(manifest.keywords.is_empty());
    assert!(manifest.interface.is_none());
    for (url, privacy, terms) in [
        ("websiteUrl", "privacyPolicyUrl", "termsOfServiceUrl"),
        ("websiteURL", "privacyPolicyURL", "termsOfServiceURL"),
    ] {
        fs::write(root.join("dark.png"), b"image").unwrap();
        let long = "字".repeat(129);
        let boundary = "字".repeat(128);
        let mut interface = json!({
            "defaultPrompt": [null, " ", long, " first\n prompt ", boundary, "third", "fourth"],
            "logoDark": "./dark.png"
        });
        interface[url] = json!("https://example.com");
        interface[privacy] = json!("https://example.com/privacy");
        interface[terms] = json!("https://example.com/terms");
        write_manifest(
            &root,
            ".codex-plugin",
            &json!({"name": " raw ", "version": " nightly ", "description": " preserved ", "keywords": ["one"], "interface": interface}),
        );
        let manifest = load_plugin_manifest(&root).unwrap();
        assert_eq!(manifest.name, " raw ");
        assert_eq!(manifest.version.as_deref(), Some("nightly"));
        assert_eq!(manifest.description.as_deref(), Some(" preserved "));
        assert_eq!(manifest.keywords, ["one"]);
        let interface = manifest.interface.unwrap();
        assert_eq!(interface.website_url.as_deref(), Some("https://example.com"));
        assert_eq!(interface.privacy_policy_url.as_deref(), Some("https://example.com/privacy"));
        assert_eq!(interface.terms_of_service_url.as_deref(), Some("https://example.com/terms"));
        assert_eq!(
            interface.default_prompt,
            ["first prompt".to_owned(), boundary, "third".to_owned()]
        );
        assert_eq!(
            interface.logo_dark.unwrap().canonical_path,
            root.join("dark.png").canonicalize().unwrap()
        );
    }
}

#[test]
fn codex_plugin_compat_t03_rejects_null_interface_lists() {
    let temp = tempfile::tempdir().unwrap();
    for field in ["capabilities", "screenshots"] {
        let mut interface = json!({});
        interface[field] = Value::Null;
        write_manifest(temp.path(), ".codex-plugin", &json!({"interface": interface}));
        let error = load_plugin_manifest(temp.path()).expect_err("legacy lists cannot be null");
        assert!(error.to_string().contains(field), "{error}");
    }
}

#[test]
fn codex_plugin_compat_t03_rejects_unsafe_component_paths() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("package");
    fs::create_dir_all(temp.path().join("outside")).unwrap();
    for field in ["skills", "mcpServers", "hooks", "apps"] {
        let mut value = json!({"name": "safe"});
        value[field] = json!("./../outside");
        write_manifest(&root, ".codex-plugin", &value);
        let error =
            load_plugin_manifest(&root).expect_err("unsafe path must fail visibly").to_string();
        assert!(error.contains(field), "{error}");
        assert!(error.contains("escapes plugin root"), "{error}");
    }
    for value in [json!({"version": 5}), json!({"description": []}), json!({"keywords": [1]})] {
        write_manifest(&root, ".codex-plugin", &value);
        assert!(load_plugin_manifest(&root).is_err(), "invalid scalar metadata {value}");
    }
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t03_rejects_symlink_entries_and_components() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    for (index, directory) in
        [".codex-plugin", ".claude-plugin", ".cursor-plugin"].into_iter().enumerate()
    {
        let root = temp.path().join(index.to_string());
        fs::create_dir_all(&root).unwrap();
        let real = root.join("real");
        write_manifest(&root, "real", &json!({"name": "linked"}));
        symlink(&real, root.join(directory)).unwrap();
        assert!(load_plugin_manifest(&root).unwrap_err().to_string().contains("symlink"));
        fs::remove_file(root.join(directory)).unwrap();
        fs::create_dir(root.join(directory)).unwrap();
        symlink(real.join("plugin.json"), root.join(directory).join("plugin.json")).unwrap();
        assert!(load_plugin_manifest(&root).unwrap_err().to_string().contains("symlink"));
        fs::remove_file(root.join(directory).join("plugin.json")).unwrap();
        symlink(&real, root.join("skills")).unwrap();
        write_manifest(&root, directory, &json!({"name": "linked", "skills": "./skills"}));
        assert!(load_plugin_manifest(&root).unwrap_err().to_string().contains("symlink"));
    }
}
