//! 通过真实插件发现与技能加载入口验证多根、拒绝和来源隔离。

use crate::app::agent::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t09/plugin.json");
const ALPHA: &str = include_str!("../../../testdata/plugin_compat/t09/alpha/SKILL.md");
const BETA: &str = include_str!("../../../testdata/plugin_compat/t09/beta/SKILL.md");

struct Fixture {
    temp: tempfile::TempDir,
    workspace: PathBuf,
    config: Config,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let mut config = Config::default();
        config.workspace_dir = workspace.clone();
        config.config_path = temp.path().join("config/vibewindow.json");
        Self { temp, workspace, config }
    }

    fn package(&self, folder: &str) -> PathBuf {
        let root = self.workspace.join("plugins").join(folder);
        write(&root.join(".codex-plugin/plugin.json"), MANIFEST);
        write(&root.join("a-skills/alpha/SKILL.md"), ALPHA.as_bytes());
        write(&root.join("z-skills/beta/SKILL.md"), BETA.as_bytes());
        root
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn codex_plugin_compat_t09_preserves_same_name_marketplace_provenance_in_runtime_merge() {
    let fixture = Fixture::new();
    let package = fixture.package("source");
    let source = fixture.temp.path().join("original");
    fs::rename(package, &source).unwrap();
    let install_dir = fixture.workspace.join("plugins");
    for market in ["first", "second"] {
        crate::plugins::install_plugin_from_marketplace(
            source.to_str().unwrap(),
            &install_dir,
            market,
        )
        .unwrap();
    }
    let skills = crate::app::agent::skills::load_skills_full_with_config(
        &fixture.workspace,
        &fixture.config,
    );
    let plugins = skills.iter().filter(|skill| skill.plugin_source.is_some()).collect::<Vec<_>>();
    assert_eq!(plugins.len(), 4);
    for market in ["first", "second"] {
        let key = format!("root-bundle@{market}");
        let owned = plugins
            .iter()
            .filter(|skill| skill.plugin_source.as_ref().unwrap().plugin_id == key)
            .collect::<Vec<_>>();
        assert_eq!(
            owned.iter().map(|skill| skill.name.as_str()).collect::<Vec<_>>(),
            ["alpha", "beta"]
        );
        assert!(owned.iter().all(|skill| skill.plugin_source.as_ref().unwrap().root
            == install_dir.join(&key).canonicalize().unwrap()));
    }
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
}
