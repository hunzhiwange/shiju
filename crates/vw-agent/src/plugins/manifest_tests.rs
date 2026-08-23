//! 验证插件清单必填字段、组件路径、钩子顺序与解析错误上下文。

use super::load_plugin_manifest;
use serde_json::json;
use std::fs;
use std::path::Path;

fn write_plugin_manifest(plugin_root: &Path, manifest: serde_json::Value) {
    let manifest_dir = plugin_root.join(".vibewindow-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    fs::write(
        manifest_dir.join("plugin.json"),
        serde_json::to_vec_pretty(&manifest).expect("manifest json"),
    )
    .expect("write manifest");
}

#[test]
fn plugin_manifest_accepts_vibewindow_plugin_json() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("my-plugin");
    fs::create_dir_all(plugin_root.join("skills/research")).expect("skills dir");
    fs::create_dir_all(plugin_root.join("hooks")).expect("hooks dir");
    fs::create_dir_all(plugin_root.join("assets")).expect("assets dir");
    fs::write(plugin_root.join(".mcp.json"), "{}").expect("mcp config");
    fs::write(plugin_root.join("hooks/hooks.json"), "{}").expect("hooks config");
    fs::write(plugin_root.join("assets/icon.png"), "icon").expect("icon");
    fs::write(plugin_root.join("assets/logo.png"), "logo").expect("logo");
    fs::write(plugin_root.join("assets/screenshot-1.png"), "screenshot").expect("screenshot");

    write_plugin_manifest(
        &plugin_root,
        json!({
            "name": "my-plugin",
            "version": "0.1.0",
            "description": "Bundle reusable skills and app integrations.",
            "author": {
                "name": "Your team",
                "email": "team@example.com",
                "url": "https://example.com"
            },
            "homepage": "https://example.com/plugins/my-plugin",
            "repository": "https://github.com/example/my-plugin",
            "license": "MIT",
            "keywords": ["research", "crm"],
            "skills": "./skills/",
            "mcpServers": "./.mcp.json",
            "hooks": "./hooks/hooks.json",
            "interface": {
                "displayName": "My Plugin",
                "shortDescription": "Reusable skills and apps",
                "longDescription": "Distribute skills and app integrations together.",
                "developerName": "Your team",
                "category": "Productivity",
                "capabilities": ["Read", "Write"],
                "websiteURL": "https://example.com",
                "privacyPolicyURL": "https://example.com/privacy",
                "termsOfServiceURL": "https://example.com/terms",
                "defaultPrompt": [
                    "Use My Plugin to summarize new CRM notes.",
                    "Use My Plugin to triage new customer follow-ups."
                ],
                "brandColor": "#10A37F",
                "composerIcon": "./assets/icon.png",
                "logo": "./assets/logo.png",
                "screenshots": ["./assets/screenshot-1.png"]
            }
        }),
    );

    let manifest = load_plugin_manifest(&plugin_root).expect("manifest parses");

    assert_eq!(manifest.name, "my-plugin");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.description, "Bundle reusable skills and app integrations.");
    assert_eq!(manifest.keywords, vec!["research".to_string(), "crm".to_string()]);
    assert_eq!(
        manifest.skills.as_ref().expect("skills").canonical_path,
        plugin_root.join("skills").canonicalize().expect("canonical skills")
    );
    assert_eq!(
        manifest.mcp_servers.as_ref().expect("mcp").canonical_path,
        plugin_root.join(".mcp.json").canonicalize().expect("canonical mcp")
    );
    assert_eq!(manifest.hooks.len(), 1);
    assert_eq!(
        manifest.hooks[0].canonical_path,
        plugin_root.join("hooks/hooks.json").canonicalize().expect("canonical hooks")
    );
    let interface = manifest.interface.expect("interface");
    assert_eq!(interface.display_name.as_deref(), Some("My Plugin"));
    assert_eq!(
        interface.composer_icon.expect("composer icon").canonical_path,
        plugin_root.join("assets/icon.png").canonicalize().expect("canonical icon")
    );
    assert_eq!(
        interface.logo.expect("logo").canonical_path,
        plugin_root.join("assets/logo.png").canonicalize().expect("canonical logo")
    );
    assert_eq!(interface.screenshots.len(), 1);
    assert_eq!(
        interface.screenshots[0].canonical_path,
        plugin_root.join("assets/screenshot-1.png").canonicalize().expect("canonical screenshot")
    );
}

#[test]
fn plugin_manifest_rejects_path_escape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("escape-plugin");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    fs::create_dir_all(dir.path().join("outside-skills")).expect("outside skills");
    write_plugin_manifest(
        &plugin_root,
        json!({
            "name": "escape-plugin",
            "version": "0.1.0",
            "description": "Attempt to escape the plugin root.",
            "skills": "./../outside-skills"
        }),
    );

    let err = load_plugin_manifest(&plugin_root).expect_err("path escape rejected").to_string();

    assert!(err.contains("skills"), "{err}");
    assert!(err.contains("./../outside-skills"), "{err}");
    assert!(err.contains("escapes plugin root"), "{err}");
}

#[test]
fn plugin_manifest_requires_name_version_description() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("incomplete-plugin");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    write_plugin_manifest(&plugin_root, json!({ "name": "" }));

    let err = load_plugin_manifest(&plugin_root).expect_err("required fields rejected").to_string();

    assert!(err.contains("name"), "{err}");
    assert!(err.contains("version"), "{err}");
    assert!(err.contains("description"), "{err}");
}

#[test]
fn plugin_manifest_accepts_hook_array_in_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("multi-hook-plugin");
    fs::create_dir_all(plugin_root.join("hooks")).expect("hooks dir");
    fs::write(plugin_root.join("hooks/first.json"), "{}").expect("first hook");
    fs::write(plugin_root.join("hooks/second.json"), "{}").expect("second hook");
    write_plugin_manifest(
        &plugin_root,
        json!({
            "name": "multi-hook-plugin",
            "version": "0.1.0",
            "description": "Load multiple hook manifests.",
            "hooks": ["./hooks/first.json", "./hooks/second.json"]
        }),
    );

    let manifest = load_plugin_manifest(&plugin_root).expect("manifest parses");

    assert_eq!(manifest.hooks.len(), 2);
    assert_eq!(manifest.hooks[0].raw, "./hooks/first.json");
    assert_eq!(manifest.hooks[1].raw, "./hooks/second.json");
    assert_eq!(
        manifest.hooks[0].canonical_path,
        plugin_root.join("hooks/first.json").canonicalize().expect("canonical first hook")
    );
    assert_eq!(
        manifest.hooks[1].canonical_path,
        plugin_root.join("hooks/second.json").canonicalize().expect("canonical second hook")
    );
}

#[test]
fn plugin_manifest_reports_json_parse_context() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("invalid-json-plugin");
    let manifest_dir = plugin_root.join(".vibewindow-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    let manifest_path = manifest_dir.join("plugin.json");
    fs::write(&manifest_path, r#"{"name": "invalid-json-plugin""#).expect("write manifest");

    let err = load_plugin_manifest(&plugin_root).expect_err("invalid json rejected").to_string();

    assert!(err.contains("failed to parse plugin manifest"), "{err}");
    assert!(err.contains(&manifest_path.display().to_string()), "{err}");
}
