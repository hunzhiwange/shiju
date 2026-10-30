use crate::plugins::{PluginHookSource, PluginMcpServers, load_plugin_manifest};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";

fn write_manifest(root: &Path, format: &str, component: Value) {
    fs::create_dir_all(root).unwrap();
    let (path, mut value) = match format {
        "portable" => ("plugin.json", json!({"$schema": SCHEMA, "name": "path-example"})),
        "native" => (
            ".vibewindow-plugin/plugin.json",
            json!({"name": "path-example", "version": "1", "description": "Path audit"}),
        ),
        _ => (".codex-plugin/plugin.json", json!({"name": "path-example"})),
    };
    if format == "portable" {
        value["extensions"] = json!({"com.openai": component});
    } else {
        value.as_object_mut().unwrap().extend(component.as_object().unwrap().clone());
    }
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
fn codex_plugin_compat_t05_rejects_component_escape() {
    let temp = tempfile::tempdir().unwrap();
    for format in ["legacy", "portable", "native"] {
        let root = temp.path().join(format);
        fs::create_dir_all(root.join("icons")).unwrap();
        fs::write(root.join("logo.svg"), "unchanged image").unwrap();
        write_manifest(&root, format, json!({"interface": {"logo": "./icons/../logo.svg"}}));
        let error = load_plugin_manifest(&root)
            .expect_err("parent components cannot become trusted by resolving back inside the root")
            .to_string();
        assert!(error.contains("interface.logo"), "{error}");
        assert!(error.contains("parent component"), "{error}");
        assert_eq!(fs::read_to_string(root.join("logo.svg")).unwrap(), "unchanged image");
    }
}

#[test]
fn codex_plugin_compat_t05_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let cases: Vec<String> =
        serde_json::from_str(include_str!("../../../testdata/plugin_compat/t05/root_aliases.json"))
            .unwrap();
    for format in ["legacy", "portable", "native"] {
        let root = temp.path().join(format);
        for path in &cases {
            write_manifest(&root, format, json!({"interface": {"logo": path}}));
            let error = load_plugin_manifest(&root)
                .expect_err("the package root cannot be declared as a component resource")
                .to_string();
            assert!(error.contains("interface.logo"), "{error}");
            assert!(error.contains("must name a resource"), "{error}");
        }
    }
}

#[test]
fn codex_plugin_compat_t05_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    for format in ["legacy", "portable", "native"] {
        let root = temp.path().join(format).join("package");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("logo.svg"), format).unwrap();
        write_manifest(&root, format, json!({"interface": {"logo": "./../package/logo.svg"}}));
        let entry = if format == "portable" {
            "plugin.json"
        } else if format == "native" {
            ".vibewindow-plugin/plugin.json"
        } else {
            ".codex-plugin/plugin.json"
        };
        let before = fs::read(root.join(entry)).unwrap();
        assert!(
            load_plugin_manifest(&root).is_err(),
            "leaving and re-entering the package is rejected before reading a component"
        );
        assert_eq!(fs::read(root.join(entry)).unwrap(), before);
        write_manifest(&root, format, json!({"interface": {"logo": "./logo.svg"}}));
        let manifest =
            load_plugin_manifest(&root).expect("rejection does not affect the next load");
        let logo = manifest.interface.unwrap().logo.unwrap();
        assert_eq!(logo.canonical_path, root.join("logo.svg").canonicalize().unwrap());
        assert_eq!(fs::read_to_string(&logo.canonical_path).unwrap(), format);
        assert!(!root.join("data").exists());
        assert!(!root.join("cache").exists());
    }
}

fn declarations(path: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("skills", json!({"skills": path})),
        ("skills", json!({"skills": ["./safe", path]})),
        ("mcpServers", json!({"mcpServers": path})),
        ("hooks", json!({"hooks": path})),
        ("hooks", json!({"hooks": ["./safe", path]})),
        ("apps", json!({"apps": path})),
        ("onboardingSkill", json!({"extensions": {"com.openai": {"onboardingSkill": path}}})),
        ("interface.composerIcon", json!({"interface": {"composerIcon": path}})),
        ("interface.logo", json!({"interface": {"logo": path}})),
        ("interface.logoDark", json!({"interface": {"logoDark": path}})),
        ("interface.screenshots", json!({"interface": {"screenshots": ["./safe", path]}})),
    ]
}

#[test]
fn codex_plugin_compat_t05_audits_each_explicit_path_and_array() {
    let temp = tempfile::tempdir().unwrap();
    for format in ["legacy", "portable"] {
        let root = temp.path().join(format);
        fs::create_dir_all(root.join("safe")).unwrap();
        for path in ["./../outside", "./safe/../safe", "./C:/outside", "./missing"] {
            for (field, mut component) in declarations(path) {
                if format == "portable" && field == "onboardingSkill" {
                    component = json!({"onboardingSkill": path});
                }
                write_manifest(&root, format, component);
                let error = load_plugin_manifest(&root)
                    .expect_err(
                        "an invalid array item or explicit source rejects the whole declaration",
                    )
                    .to_string();
                assert!(error.contains(field), "{format} {field} {path}: {error}");
            }
        }
    }
}

#[test]
fn codex_plugin_compat_t05_rejects_absolute_and_windows_spelling_on_every_host() {
    let temp = tempfile::tempdir().unwrap();
    for format in ["legacy", "portable", "native"] {
        let root = temp.path().join(format);
        fs::create_dir_all(&root).unwrap();
        for path in [
            "./C:/logo.svg",
            "./C:logo.svg",
            "././C:/logo.svg",
            "./icons\\..\\logo.svg",
            "./icons\\logo.svg",
            "./\\server\\share\\logo.svg",
            ".//logo.svg",
        ] {
            // These are ordinary filenames on POSIX. Rejection must not rely on their absence.
            #[cfg(unix)]
            {
                let file = root.join(path.strip_prefix("./").unwrap().trim_start_matches('/'));
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(file, "unread resource").unwrap();
            }
            write_manifest(&root, format, json!({"interface": {"logo": path}}));
            let error = load_plugin_manifest(&root)
                .expect_err("host-independent lexical audit rejects Windows-ambiguous paths")
                .to_string();
            assert!(error.contains("interface.logo"), "{error}");
            assert!(
                error.contains("Windows drive") || error.contains("must stay within"),
                "{error}"
            );
        }
        let absolute = root.join("logo.svg").display().to_string();
        for path in [absolute.as_str(), "C:/logo.svg", "\\\\server\\share\\logo.svg", "logo.svg"] {
            write_manifest(&root, format, json!({"interface": {"logo": path}}));
            let error = load_plugin_manifest(&root).unwrap_err().to_string();
            assert!(error.contains("interface.logo"), "{error}");
            assert!(error.contains("must start with './'"), "{error}");
        }
    }
}

#[test]
fn codex_plugin_compat_t05_distinguishes_optional_defaults_from_explicit_missing_paths() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("portable");
    write_manifest(&root, "portable", json!({}));
    let before = fs::read(root.join("plugin.json")).unwrap();
    let manifest = load_plugin_manifest(&root).unwrap();
    let canonical_root = root.canonicalize().unwrap();
    assert_eq!(manifest.skills[0].canonical_path, canonical_root.join("skills"));
    assert!(
        matches!(manifest.mcp_servers, Some(PluginMcpServers::Path(path)) if path.canonical_path == canonical_root.join("mcp.json"))
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(fs::read(root.join("plugin.json")).unwrap(), before);
    for format in ["legacy", "portable"] {
        let root = temp.path().join(format);
        fs::create_dir_all(root.join("safe")).unwrap();
        for (field, mut component) in declarations("./missing") {
            if format == "portable" && field == "onboardingSkill" {
                component = json!({"onboardingSkill": "./missing"});
            }
            write_manifest(&root, format, component);
            let error = load_plugin_manifest(&root).unwrap_err().to_string();
            assert!(error.contains(field), "{error}");
            assert!(error.contains("path does not exist"), "{error}");
            assert!(!root.join("missing").exists());
        }
    }
}

#[test]
fn codex_plugin_compat_t05_retains_file_directory_array_and_inline_origins() {
    let temp = tempfile::tempdir().unwrap();
    for format in ["legacy", "portable"] {
        let root = temp.path().join(format);
        fs::create_dir_all(root.join("skills")).unwrap();
        for file in ["skills/SKILL.md", "mcp.json", "one.json", "two.json", "logo.svg"] {
            fs::write(root.join(file), "unchanged fixture").unwrap();
        }
        let servers =
            json!({"example": {"command": "./../unexecuted", "args": ["--marker", "created"]}});
        let hooks = json!({"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": format!("touch {}", root.join("created").display())}]}]}});
        let mut component = json!({
            "skills": ["./skills", "./skills/SKILL.md"], "mcpServers": servers,
            "hooks": [hooks.clone(), hooks.clone()], "apps": "./one.json",
            "interface": {"composerIcon": "./logo.svg", "logo": "././logo.svg", "logoDark": "./logo.svg", "screenshots": ["./logo.svg", "./logo.svg"]}
        });
        if format == "portable" {
            component["onboardingSkill"] = json!("skills/SKILL.md");
        } else {
            component["extensions"] = json!({"com.openai": {"onboardingSkill": "skills/SKILL.md"}});
        }
        write_manifest(&root, format, component);
        let entry = if format == "portable" { "plugin.json" } else { ".codex-plugin/plugin.json" };
        let before = fs::read(root.join(entry)).unwrap();
        let manifest = load_plugin_manifest(&root).unwrap();
        assert_eq!(manifest.skills.len(), if format == "portable" { 1 } else { 2 });
        if format == "legacy" {
            assert!(
                matches!(manifest.mcp_servers, Some(PluginMcpServers::Inline(map)) if Value::Object(map.clone()) == servers)
            );
            assert!(manifest.skills[1].canonical_path.is_file());
        }
        assert!(manifest.skills[0].canonical_path.is_dir());
        assert!(
            matches!(&manifest.hooks[..], [PluginHookSource::Inline(first), PluginHookSource::Inline(second)] if Value::Object(first.clone()) == hooks && first == second)
        );
        assert_eq!(manifest.apps.unwrap().raw, "./one.json");
        assert_eq!(manifest.onboarding_skill.unwrap().raw, "./skills/SKILL.md");
        let interface = manifest.interface.unwrap();
        for path in [
            interface.composer_icon.unwrap(),
            interface.logo.unwrap(),
            interface.logo_dark.unwrap(),
        ]
        .into_iter()
        .chain(interface.screenshots)
        {
            assert_eq!(path.canonical_path, root.join("logo.svg").canonicalize().unwrap());
        }
        assert_eq!(fs::read(root.join(entry)).unwrap(), before);
        assert!(!root.join("created").exists(), "parsing never executes inline handlers");
        write_manifest(
            &root,
            format,
            json!({"mcpServers": "./mcp.json", "hooks": ["./one.json", "./two.json"]}),
        );
        let manifest = load_plugin_manifest(&root).unwrap();
        assert!(
            matches!(manifest.mcp_servers, Some(PluginMcpServers::Path(path)) if path.raw == "./mcp.json")
        );
        assert!(
            matches!(&manifest.hooks[..], [PluginHookSource::Path(first), PluginHookSource::Path(second)] if first.raw == "./one.json" && second.raw == "./two.json")
        );
    }
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t05_rejects_internal_external_and_broken_symlinks() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside.svg");
    fs::write(&outside, "outside unchanged").unwrap();
    for format in ["legacy", "portable", "native"] {
        let root = temp.path().join(format);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("inside.svg"), "inside unchanged").unwrap();
        for (name, target) in [
            ("internal.svg", root.join("inside.svg")),
            ("external.svg", outside.clone()),
            ("broken.svg", root.join("missing.svg")),
        ] {
            symlink(target, root.join(name)).unwrap();
            write_manifest(&root, format, json!({"interface": {"logo": format!("./{name}")}}));
            let error = load_plugin_manifest(&root).unwrap_err().to_string();
            assert!(error.contains("includes symlink"), "{error}");
        }
        symlink(&root, root.join("linked-directory")).unwrap();
        write_manifest(
            &root,
            format,
            json!({"interface": {"logo": "./linked-directory/inside.svg"}}),
        );
        assert!(load_plugin_manifest(&root).unwrap_err().to_string().contains("includes symlink"));
        assert_eq!(fs::read_to_string(root.join("inside.svg")).unwrap(), "inside unchanged");
    }
    let root = temp.path().join("optional");
    write_manifest(&root, "portable", json!({}));
    for name in ["skills", "mcp.json"] {
        symlink(root.join("missing"), root.join(name)).unwrap();
        let error = load_plugin_manifest(&root).unwrap_err().to_string();
        assert!(error.contains("includes symlink"), "{error}");
        fs::remove_file(root.join(name)).unwrap();
    }
    assert_eq!(fs::read_to_string(outside).unwrap(), "outside unchanged");
}
