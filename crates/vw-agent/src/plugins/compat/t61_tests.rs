use crate::plugins::install::{remove_installed_plugin, set_plugin_enabled};
use crate::plugins::install_plugin_from_marketplace;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn public_original(root: &Path, name: &str) -> std::path::PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/plugin_compat/t61/public")
        .join(format!("{name}.tar.gz"));
    let archive = fs::File::open(fixture).unwrap();
    tar::Archive::new(flate2::read::GzDecoder::new(archive)).unpack(root).unwrap();
    root.join(name)
}
fn digest(p: &Path) -> Vec<(String, String)> {
    let mut files = walkdir::WalkDir::new(p)
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path().strip_prefix(p).unwrap().to_string_lossy().into_owned(),
                hex::encode(Sha256::digest(fs::read(e.path()).unwrap())),
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[tokio::test]
async fn codex_plugin_compat_t61_public_skill_lifecycle_and_upgrade() {
    use crate::plugins::{activate_plugin_upgrade, prepare_plugin_upgrade};
    use crate::tools::{Tool, ToolUseContext, scope_tool_use_context, skill::SkillTool};
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "superpowers-6.3.0");
    let upgrade = public_original(t.path(), "superpowers-7.0.1");
    let before = digest(&source);
    let next = digest(&upgrade);
    let mut config = crate::config::Config::default();
    config.workspace_dir = t.path().join("workspace");
    config.config_path = t.path().join("config/shiju.json");
    let plugins = t.path().join("config/plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "public").unwrap();
    assert_eq!(digest(&installed.installed_dir), before);
    let config = Arc::new(config);
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::security::SecurityPolicy::default()),
        "t61-public".into(),
        false,
        config.workspace_dir.clone(),
        config.clone(),
    );
    let name = "superpowers:verification-before-completion";
    let context = Arc::new(ToolUseContext::new("t61-public", None).with_full_access_enabled(true));
    let result = scope_tool_use_context(context, tool.call(serde_json::json!({"skill":name})))
        .await
        .unwrap();
    assert_eq!(result.data["success"], true, "{result:?}");
    assert!(result.extra_messages.iter().any(|m| {
        m.content.to_string().contains("NO COMPLETION CLAIMS WITHOUT FRESH VERIFICATION EVIDENCE")
    }));
    set_plugin_enabled(&plugins, "superpowers@public", false).unwrap();
    assert!(
        crate::skills::load_skills_full_with_config(&config.workspace_dir, &config)
            .iter()
            .all(|s| s.name != name)
    );
    set_plugin_enabled(&plugins, "superpowers@public", true).unwrap();
    let preview =
        prepare_plugin_upgrade(upgrade.to_str().unwrap(), &plugins, "superpowers@public").unwrap();
    activate_plugin_upgrade(preview).unwrap();
    assert_eq!(digest(&installed.installed_dir), next);
    remove_installed_plugin(&plugins, "superpowers@public").unwrap();
    assert!(!installed.installed_dir.exists());
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&upgrade), next);
}
