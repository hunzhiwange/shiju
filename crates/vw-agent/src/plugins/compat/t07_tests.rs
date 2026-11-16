//! Real discovery entry-point checks for manifest formats, priority and rejected-package isolation.

use crate::plugins::{PluginDiscoveryOptions, discover_plugins};
use std::fs;
use std::path::{Path, PathBuf};
use config_plugins::PluginsConfig;

const NATIVE: &[u8] = include_bytes!("../../../testdata/plugin_compat/t07/native.json");

fn write_manifest(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn codex_plugin_compat_t07_preserves_healthy_skill_and_mcp_consumers() {
    use crate::app::agent::config::Config;
    use crate::app::agent::skills::SkillLoadMode;

    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let config_dir = temp.path().join("config");
    let source = workspace.join("plugins");
    let healthy = source.join("a-healthy");
    let skill_path = healthy.join("skills/kept-skill/SKILL.md");
    fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
    fs::write(&skill_path, "# kept-skill\nHealthy plugin skill\n").unwrap();
    fs::write(
        healthy.join("mcp.json"),
        br#"[{"name":"kept-mcp","command":"unused-test-command"}]"#,
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(NATIVE).unwrap();
    value["skills"] = "./skills".into();
    value["mcpServers"] = "./mcp.json".into();
    write_manifest(
        &healthy,
        ".vibewindow-plugin/plugin.json",
        &serde_json::to_vec(&value).unwrap(),
    );
    write_manifest(&source.join("b-broken"), ".codex-plugin/plugin.json", b"{");
    let mut config = Config::default();
    config.workspace_dir = workspace.clone();
    config.config_path = config_dir.join("vibewindow.json");
    let skills = crate::plugins::load_plugin_skills(&workspace, &config, SkillLoadMode::Full);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "kept-skill");
    let mut servers = Vec::new();
    crate::plugins::merge_enabled_plugin_mcp_servers(
        &mut servers,
        &PluginDiscoveryOptions::new(workspace, config_dir, PluginsConfig::default()),
    )
    .unwrap();
    assert_eq!(servers.len(), 1);
    assert!(
        matches!(&servers[0], agent_client_protocol::McpServer::Stdio(server) if server.name == "kept-mcp")
    );
    assert_eq!(fs::read_to_string(skill_path).unwrap(), "# kept-skill\nHealthy plugin skill\n");
}
