use crate::plugins::{PluginHookSource, PluginMcpServers, load_plugin_manifest};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";

fn write_json(root: &Path, path: &str, value: &Value) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn package(root: &Path) -> Value {
    fs::create_dir_all(root.join("skills")).unwrap();
    fs::write(root.join("mcp.json"), "{\"mcpServers\":{}}\n").unwrap();
    serde_json::from_str(include_str!("../../../testdata/plugin_compat/t04/portable.json")).unwrap()
}

#[test]
fn codex_plugin_compat_t04_prefers_inline_openai_overlay() {
    let temp = tempfile::tempdir().unwrap();
    let value = package(temp.path());
    write_json(temp.path(), "plugin.json", &value);
    write_json(
        temp.path(),
        ".codex-plugin/plugin.json",
        &json!({"name": "overlay-identity", "skills": "./missing", "interface": {"displayName": "Overlay"}, "hooks": {"overlay": true}}),
    );
    let manifest = load_plugin_manifest(temp.path()).expect("portable root manifest must load");
    assert_eq!(manifest.name, "portable-example");
    assert_eq!(manifest.version.as_deref(), Some("1.2.3"));
    assert_eq!(manifest.skills[0].raw, "./skills");
    assert!(
        matches!(manifest.mcp_servers, Some(PluginMcpServers::Path(ref path)) if path.raw == "./mcp.json")
    );
    assert_eq!(manifest.interface.unwrap().display_name.as_deref(), Some("Inline"));
    assert!(
        matches!(&manifest.hooks[..], [PluginHookSource::Inline(hooks)] if hooks.get("inline") == Some(&json!(true)))
    );
    assert!(manifest.apps.is_none(), "the overlay must not be merged into inline extensions");
}

#[test]
fn codex_plugin_compat_t04_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    package(temp.path());
    write_json(temp.path(), ".codex-plugin/plugin.json", &json!({"name": "legacy"}));
    write_json(
        temp.path(),
        "plugin.json",
        &json!({"$schema": "https://agent-plugins.org/schemas/9.0.0/plugin.schema.json", "name": "future"}),
    );
    let error = load_plugin_manifest(temp.path()).expect_err("unknown schema cannot fall back");
    assert!(error.to_string().contains("unsupported Agent Plugins schema"), "{error:#}");
    write_json(temp.path(), "plugin.json", &json!({"name": "npm-package"}));
    assert_eq!(load_plugin_manifest(temp.path()).unwrap().name, "legacy");
    fs::remove_dir_all(temp.path().join(".codex-plugin")).unwrap();
    assert!(load_plugin_manifest(temp.path()).is_err(), "ordinary npm metadata is not a plugin");
    write_json(
        temp.path(),
        "plugin.json",
        &json!({"$schema": SCHEMA, "name": "valid", "version": null}),
    );
    assert!(load_plugin_manifest(temp.path()).is_err(), "explicit null metadata is rejected");
}

#[test]
fn codex_plugin_compat_t04_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["first", "second"] {
        let root = temp.path().join(name);
        let mut value = package(&root);
        value["name"] = json!(name);
        write_json(&root, "plugin.json", &value);
        let before = fs::read(root.join("plugin.json")).unwrap();
        let manifest = load_plugin_manifest(&root).expect("independent portable package loads");
        assert_eq!(manifest.name, name);
        assert_eq!(manifest.root, root.canonicalize().unwrap());
        assert_eq!(fs::read(root.join("plugin.json")).unwrap(), before);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 3);
        assert!(!root.join(".vibewindow-plugin").exists());
    }
}

#[test]
fn codex_plugin_compat_t04_keeps_fixed_components_and_default_interface() {
    let temp = tempfile::tempdir().unwrap();
    write_json(
        temp.path(),
        "plugin.json",
        &json!({
            "$schema": SCHEMA, "name": "portable.name-1", "version": " \t",
            "description": " Description ", "author": {"name": " Developer "},
            "homepage": " https://example.com ", "repository": "validated only", "license": "MIT",
            "keywords": ["", "tag"], "skills": "./not-a-component", "mcpServers": false,
            "agents": {"unexpected": "not executed"}, "extensions": {"other.vendor": {"hooks": true}}
        }),
    );
    let manifest = load_plugin_manifest(temp.path()).unwrap();
    let root = temp.path().canonicalize().unwrap();
    assert_eq!(manifest.manifest_path, root.join("plugin.json"));
    assert_eq!(manifest.version, None);
    assert_eq!(manifest.description.as_deref(), Some("Description"));
    assert_eq!(manifest.skills[0].canonical_path, root.join("skills"));
    assert!(
        matches!(manifest.mcp_servers, Some(PluginMcpServers::Path(ref path)) if path.canonical_path == root.join("mcp.json"))
    );
    assert_eq!(manifest.keywords, ["", "tag"]);
    assert!(manifest.author.is_none());
    assert!(manifest.repository.is_none());
    assert!(manifest.license.is_none());
    assert!(manifest.hooks.is_empty());
    let interface = manifest.interface.unwrap();
    assert_eq!(interface.display_name.as_deref(), Some("portable.name-1"));
    assert_eq!(interface.short_description.as_deref(), Some("Description"));
    assert_eq!(interface.long_description.as_deref(), Some("Description"));
    assert_eq!(interface.developer_name.as_deref(), Some("Developer"));
    assert_eq!(interface.website_url.as_deref(), Some("https://example.com"));
    assert_eq!(interface.category.as_deref(), Some("Other"));
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1, "loading creates no resources");
}

#[test]
fn codex_plugin_compat_t04_uses_overlay_only_without_inline_object() {
    let temp = tempfile::tempdir().unwrap();
    let mut value = package(temp.path());
    fs::create_dir_all(temp.path().join("other-skills")).unwrap();
    fs::write(temp.path().join("other-mcp.json"), "{}").unwrap();
    fs::write(temp.path().join(".app.json"), "{}").unwrap();
    fs::write(temp.path().join("skills/SKILL.md"), "# Setup").unwrap();
    write_json(
        temp.path(),
        ".codex-plugin/plugin.json",
        &json!({
            "name": "ignored-overlay-name", "version": "9", "description": "ignored description",
            "skills": "./other-skills", "mcpServers": "./other-mcp.json",
            "apps": "./.app.json", "hooks": {"overlay": true},
            "interface": {"displayName": "Overlay"},
            "extensions": {"com.openai": {"onboardingSkill": "skills/SKILL.md"}}
        }),
    );
    let overlay_before = fs::read(temp.path().join(".codex-plugin/plugin.json")).unwrap();
    for extension in [Value::Null, json!([]), json!({"com.openai": false}), json!({})] {
        value["extensions"] = extension;
        write_json(temp.path(), "plugin.json", &value);
        let manifest = load_plugin_manifest(temp.path()).unwrap();
        assert_eq!(manifest.name, "portable-example");
        assert_eq!(manifest.version.as_deref(), Some("1.2.3"));
        assert_eq!(manifest.description.as_deref(), Some("Portable example"));
        assert_eq!(manifest.skills[0].raw, "./skills");
        assert!(
            matches!(manifest.mcp_servers, Some(PluginMcpServers::Path(ref path)) if path.raw == "./mcp.json")
        );
        assert_eq!(manifest.apps.unwrap().raw, "./.app.json");
        assert_eq!(manifest.onboarding_skill.unwrap().raw, "./skills/SKILL.md");
        let interface = manifest.interface.unwrap();
        assert_eq!(interface.display_name.as_deref(), Some("Overlay"));
        assert!(interface.category.is_none(), "overlay interface replaces the default interface");
        assert!(
            matches!(&manifest.hooks[..], [PluginHookSource::Inline(hooks)] if hooks.get("overlay") == Some(&json!(true)))
        );
    }
    value["extensions"] = json!({"com.openai": {}});
    write_json(temp.path(), "plugin.json", &value);
    let manifest = load_plugin_manifest(temp.path()).unwrap();
    assert_eq!(manifest.interface.unwrap().display_name.as_deref(), Some("portable-example"));
    assert!(manifest.apps.is_none());
    assert!(manifest.onboarding_skill.is_none());
    assert!(manifest.hooks.is_empty(), "even an empty inline object replaces the overlay");
    assert_eq!(fs::read(temp.path().join(".codex-plugin/plugin.json")).unwrap(), overlay_before);
    value.as_object_mut().unwrap().remove("extensions");
    write_json(temp.path(), "plugin.json", &value);
    assert_eq!(load_plugin_manifest(temp.path()).unwrap().apps.unwrap().raw, "./.app.json");
}

#[test]
fn codex_plugin_compat_t04_reads_inline_onboarding_from_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let mut value = package(temp.path());
    fs::write(temp.path().join("skills/SKILL.md"), "# Setup").unwrap();
    for path in ["skills/SKILL.md", "./skills/SKILL.md"] {
        value["extensions"] = json!({"com.openai": {"onboardingSkill": path}});
        write_json(temp.path(), "plugin.json", &value);
        assert_eq!(
            load_plugin_manifest(temp.path()).unwrap().onboarding_skill.unwrap().raw,
            "./skills/SKILL.md"
        );
    }
    value["extensions"] = json!({"com.openai": {"onboardingSkill": "../outside"}});
    write_json(temp.path(), "plugin.json", &value);
    assert!(
        load_plugin_manifest(temp.path()).unwrap_err().to_string().contains("escapes plugin root")
    );
}

#[test]
fn codex_plugin_compat_t04_validates_metadata_and_name_boundaries() {
    let temp = tempfile::tempdir().unwrap();
    let base = json!({"$schema": SCHEMA, "name": "valid"});
    for name in ["", "Upper", "has space", "bad--name", "bad..name", ".start", "end-", "中文"] {
        let mut value = base.clone();
        value["name"] = json!(name);
        write_json(temp.path(), "plugin.json", &value);
        assert!(
            load_plugin_manifest(temp.path())
                .unwrap_err()
                .to_string()
                .contains("invalid Agent Plugins name"),
            "{name}"
        );
    }
    for (length, accepted) in [(64, true), (65, false)] {
        let mut value = base.clone();
        value["name"] = json!("a".repeat(length));
        write_json(temp.path(), "plugin.json", &value);
        assert_eq!(load_plugin_manifest(temp.path()).is_ok(), accepted);
    }
    for field in
        ["version", "description", "author", "homepage", "repository", "license", "keywords"]
    {
        for invalid in [Value::Null, json!(42), json!([])] {
            if field == "keywords" && invalid == json!([]) {
                continue;
            }
            let mut value = base.clone();
            value[field] = invalid;
            write_json(temp.path(), "plugin.json", &value);
            assert!(load_plugin_manifest(temp.path()).is_err(), "invalid {field}: {value}");
        }
    }
    for author in [
        json!("author"),
        json!({"name": null}),
        json!({"email": null}),
        json!({"url": null}),
        json!({"name": false}),
        json!({"unknown": "field"}),
    ] {
        let mut value = base.clone();
        value["author"] = author;
        write_json(temp.path(), "plugin.json", &value);
        assert!(load_plugin_manifest(temp.path()).is_err(), "invalid author: {value}");
    }
    for author in [json!({}), json!({"email": "author@example.com", "url": "https://example.com"})]
    {
        let mut value = base.clone();
        value["author"] = author;
        write_json(temp.path(), "plugin.json", &value);
        assert!(
            load_plugin_manifest(temp.path()).unwrap().interface.unwrap().developer_name.is_none()
        );
    }
}

#[test]
fn codex_plugin_compat_t04_rejects_nonregular_root_before_legacy_fallback() {
    let temp = tempfile::tempdir().unwrap();
    write_json(temp.path(), ".codex-plugin/plugin.json", &json!({"name": "legacy"}));
    fs::create_dir(temp.path().join("plugin.json")).unwrap();
    assert!(load_plugin_manifest(temp.path()).unwrap_err().to_string().contains("must be a file"));
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t04_rejects_symlink_roots_components_and_overlays() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target.json");
    fs::write(&target, serde_json::to_vec(&json!({"$schema": SCHEMA, "name": "linked"})).unwrap())
        .unwrap();
    let root = temp.path().join("package");
    write_json(&root, ".codex-plugin/plugin.json", &json!({"name": "legacy"}));
    symlink(&target, root.join("plugin.json")).unwrap();
    assert!(load_plugin_manifest(&root).unwrap_err().to_string().contains("symlink"));
    fs::remove_file(root.join("plugin.json")).unwrap();
    write_json(&root, "plugin.json", &json!({"$schema": SCHEMA, "name": "portable"}));
    for component in ["skills", "mcp.json"] {
        symlink(&target, root.join(component)).unwrap();
        assert!(load_plugin_manifest(&root).unwrap_err().to_string().contains("symlink"));
        fs::remove_file(root.join(component)).unwrap();
    }
    fs::remove_file(root.join(".codex-plugin/plugin.json")).unwrap();
    symlink(&target, root.join(".codex-plugin/plugin.json")).unwrap();
    assert!(load_plugin_manifest(&root).unwrap_err().to_string().contains("symlink"));
    fs::remove_file(root.join(".codex-plugin/plugin.json")).unwrap();
    fs::remove_dir(root.join(".codex-plugin")).unwrap();
    symlink(temp.path(), root.join(".codex-plugin")).unwrap();
    assert!(load_plugin_manifest(&root).unwrap_err().to_string().contains("symlink"));
    write_json(
        &root,
        "plugin.json",
        &json!({"$schema": SCHEMA, "name": "portable", "extensions": {"com.openai": {}}}),
    );
    assert!(load_plugin_manifest(&root).is_ok(), "unused overlay is never followed");
}
