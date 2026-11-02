use crate::plugins::{handle_command, load_plugin_manifest};
use crate::{cli_commands::PluginCommands, config::Config};
use clap::Parser;
use std::{fs, path::Path};
#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: PluginCommands,
}
fn run(args: &[&str], config: &Config) -> anyhow::Result<()> {
    let args = Args::try_parse_from(std::iter::once("plugin").chain(args.iter().copied()))?;
    handle_command(args.command, config)
}
fn config(root: &Path) -> Config {
    let mut c = Config::default();
    c.config_path = root.join("config/config.json");
    c.workspace_dir = root.join("workspace");
    c
}
fn market(root: &Path, name: &str, version: &str) {
    fs::create_dir_all(root.join(".agents/plugins")).unwrap();
    fs::create_dir_all(root.join("pkg/.codex-plugin")).unwrap();
    fs::write(
        root.join(".agents/plugins/marketplace.json"),
        format!(r#"{{"name":"{name}","plugins":[{{"name":"fixture","source":"./pkg"}}]}}"#),
    )
    .unwrap();
    fs::write(
        root.join("pkg/.codex-plugin/plugin.json"),
        format!(r#"{{"name":"fixture","version":"{version}"}}"#),
    )
    .unwrap();
}
#[test]
fn codex_plugin_compat_t53_installs_from_local_marketplace_command() {
    let temp = tempfile::tempdir().unwrap();
    let c = config(temp.path());
    let source = temp.path().join("catalog");
    market(&source, "catalog", "1");
    run(&["marketplace", "add", source.to_str().unwrap()], &c).unwrap();
    run(&["marketplace", "list"], &c).unwrap();
    run(&["install", "fixture@catalog"], &c).unwrap();
    let installed = temp.path().join("config/plugins/fixture@catalog");
    assert_eq!(load_plugin_manifest(&installed).unwrap().version.as_deref(), Some("1"));
    run(&["read", "fixture@catalog"], &c).unwrap();
    run(&["list"], &c).unwrap();
    market(&source, "catalog", "2");
    run(&["update", "fixture@catalog"], &c).unwrap();
    assert_eq!(load_plugin_manifest(&installed).unwrap().version.as_deref(), Some("2"));
    assert_eq!(
        fs::read(installed.join(".codex-plugin/plugin.json")).unwrap(),
        fs::read(source.join("pkg/.codex-plugin/plugin.json")).unwrap()
    );
}
#[test]
fn codex_plugin_compat_t53_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let c = config(temp.path());
    assert!(run(&["marketplace", "add", "/missing"], &c).is_err());
    assert!(run(&["install", "missing@unknown"], &c).is_err());
    assert!(run(&["update", "../escape"], &c).is_err());
    assert!(!temp.path().join("config/plugins").exists());
}
#[test]
fn codex_plugin_compat_t53_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let c = config(temp.path());
    for name in ["alpha", "beta"] {
        let root = temp.path().join(name);
        market(&root, name, "1");
        run(&["marketplace", "add", root.to_str().unwrap()], &c).unwrap();
        run(&["install", &format!("fixture@{name}")], &c).unwrap();
    }
    run(&["disable", "fixture@alpha"], &c).unwrap();
    assert!(temp.path().join("config/plugins/fixture@alpha.disabled").exists());
    run(&["enable", "fixture@alpha"], &c).unwrap();
    run(&["remove", "fixture@alpha"], &c).unwrap();
    assert!(temp.path().join("config/plugins/fixture@beta").exists());
    assert!(!temp.path().join("config/plugins/fixture@alpha").exists());
}
