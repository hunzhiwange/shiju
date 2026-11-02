use crate::plugins::install::{remove_installed_plugin, set_plugin_enabled};
use crate::plugins::{PluginDiscoveryOptions, discover_plugins, install_plugin_from_marketplace};
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
fn package(p: &Path) {
    fs::create_dir_all(p.join(".codex-plugin")).unwrap();
    fs::write(p.join(".codex-plugin/plugin.json"), r#"{"name":"original","version":"1"}"#).unwrap();
    fs::write(p.join("payload"), b"unchanged original bytes").unwrap();
}
#[test]
fn codex_plugin_compat_t61_executes_original_package_without_mutation() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "market").unwrap();
    assert_eq!(digest(&installed.installed_dir), before);
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    assert_eq!(discover_plugins(&options).unwrap().enabled.len(), 1);
    set_plugin_enabled(&plugins, "original@market", false).unwrap();
    assert!(discover_plugins(&options).unwrap().enabled.is_empty());
    set_plugin_enabled(&plugins, "original@market", true).unwrap();
    assert_eq!(discover_plugins(&options).unwrap().enabled.len(), 1);
    remove_installed_plugin(&plugins, "original@market").unwrap();
    assert!(discover_plugins(&options).unwrap().installed.is_empty());
    assert_eq!(digest(&source), before);
}
#[test]
fn codex_plugin_compat_t61_rejects_invalid_input() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    fs::write(source.join(".codex-plugin/plugin.json"), b"{broken").unwrap();
    assert!(
        install_plugin_from_marketplace(
            source.to_str().unwrap(),
            &t.path().join("plugins"),
            "market"
        )
        .is_err()
    );
    assert_eq!(fs::read(source.join("payload")).unwrap(), b"unchanged original bytes");
}
#[test]
fn codex_plugin_compat_t61_preserves_isolation() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    let plugins = t.path().join("plugins");
    let a = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    let b = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "beta").unwrap();
    remove_installed_plugin(&plugins, "original@alpha").unwrap();
    assert!(!a.installed_dir.exists());
    assert!(b.installed_dir.exists());
    assert_eq!(digest(&source), digest(&b.installed_dir));
}

#[tokio::test]
async fn codex_plugin_compat_t61_runs_installed_mcp_side_effect() {
    use crate::tools::{
        Tool,
        mcp_tool::{McpTool, discover_mcp_tool_specs},
    };
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/plugin_compat/t61/package");
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "market").unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let pool = Arc::new(super::mcp_connections::McpSessionConnections::default());
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "writer", None, &[]).await })
        })
        .await
        .unwrap();
    assert_eq!(specs.len(), 1);
    let tool = McpTool::from_session_server(
        server,
        "write",
        specs.into_iter().next().unwrap(),
        pool.clone(),
        None,
    );
    let output = t.path().join("output.txt");
    let result = tool.call(serde_json::json!({"output":output})).await.unwrap();
    assert_eq!(result.model_text(), "written");
    assert_eq!(fs::read_to_string(&output).unwrap(), "original package executed\n");
    pool.shutdown();
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&installed.installed_dir), before);
    set_plugin_enabled(&plugins, "original@market", false).unwrap();
    let mut disabled = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut disabled, &options).unwrap();
    assert!(disabled.is_empty());
    remove_installed_plugin(&plugins, "original@market").unwrap();
    assert!(output.exists());
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

#[tokio::test]
#[ignore = "live original npm dependency; set T61_EVERYTHING_DEPS to separately installed pinned dependencies"]
async fn codex_plugin_compat_t61_public_stdio_original() {
    use crate::tools::{
        Tool,
        mcp_tool::{McpTool, discover_mcp_tool_specs},
    };
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "pi-agent-plugins-smoke");
    let before = digest(&source);
    let config = t.path().join("config");
    let installed = install_plugin_from_marketplace(
        source.to_str().unwrap(),
        &config.join("plugins"),
        "public",
    )
    .unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let agent_client_protocol::McpServer::Stdio(ref stdio) = server else {
        panic!("expected stdio")
    };
    let data = super::mcp_environment::runtime_cwd(stdio).unwrap().unwrap();
    let dependencies =
        std::env::var("T61_EVERYTHING_DEPS").expect("provide pinned external npm dependency root");
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(
            Path::new(&dependencies)
                .join("node_modules/@modelcontextprotocol/server-everything/package.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(metadata["version"], "2.1.0");
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        Path::new(&dependencies).join("node_modules"),
        data.join("node_modules"),
    )
    .unwrap();
    let pool = Arc::new(super::mcp_connections::McpSessionConnections::default());
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "everything", None, &[]).await })
        })
        .await
        .unwrap();
    let spec = specs
        .into_iter()
        .find(|s| s.id == crate::tools::mcp_tool::mcp_tool_id("everything", "echo").unwrap())
        .expect("original server exposes echo");
    let tool = McpTool::from_session_server(server, "echo", spec, pool.clone(), None);
    let result = tool.call(serde_json::json!({"message":"t61-original-stdio"})).await.unwrap();
    assert!(result.model_text().contains("t61-original-stdio"));
    pool.shutdown();
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&installed.installed_dir), before);
}

#[test]
fn codex_plugin_compat_t61_public_marketplace_cli_lifecycle() {
    use clap::Parser;
    #[derive(Parser)]
    struct Args {
        #[command(subcommand)]
        command: crate::cli_commands::PluginCommands,
    }
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "superpowers-7.0.1");
    let before = digest(&source);
    let mut config = crate::config::Config::default();
    config.config_path = t.path().join("config/config.json");
    config.workspace_dir = t.path().join("workspace");
    let run = |args: &[&str]| {
        let parsed =
            Args::try_parse_from(std::iter::once("plugin").chain(args.iter().copied())).unwrap();
        crate::plugins::handle_command(parsed.command, &config).unwrap();
    };
    let market = source.join(".claude-plugin/marketplace.json");
    run(&["marketplace", "add", market.to_str().unwrap()]);
    run(&["install", "superpowers@superpowers-dev"]);
    let installed = t.path().join("config/plugins/superpowers@superpowers-dev");
    assert_eq!(digest(&installed), before);
    run(&["disable", "superpowers@superpowers-dev"]);
    run(&["enable", "superpowers@superpowers-dev"]);
    run(&["remove", "superpowers@superpowers-dev"]);
    assert!(!installed.exists());
    assert_eq!(digest(&source), before);
}

#[tokio::test]
#[ignore = "requires dedicated XDG_CACHE_HOME and XDG_CONFIG_HOME to isolate original hook state"]
async fn codex_plugin_compat_t61_public_hook_original() {
    use crate::plugins::{PluginHookTrustStore, PluginHooksManifest, load_plugin_hook_source};
    let cache =
        std::path::PathBuf::from(std::env::var("XDG_CACHE_HOME").expect("set isolated cache"));
    assert!(cache.file_name().unwrap().to_string_lossy().starts_with("shiju-t61-"));
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "notify");
    let before = digest(&source);
    let installed = install_plugin_from_marketplace(
        source.to_str().unwrap(),
        &t.path().join("plugins"),
        "public",
    )
    .unwrap();
    let parsed =
        load_plugin_hook_source(&installed.manifest.hooks[0], &installed.installed_dir, t.path())
            .unwrap();
    let PluginHooksManifest::Codex(manifest) = parsed else { panic!("expected Codex hooks") };
    let mut trust = PluginHookTrustStore::load_or_default(&t.path().join("trust.json")).unwrap();
    assert!(
        crate::plugins::hook_handler::codex_handlers(
            "notify",
            &installed.installed_dir,
            &manifest,
            &trust
        )
        .unwrap()
        .is_empty()
    );
    trust.trust_codex_hooks("notify", &installed.installed_dir, &manifest).unwrap();
    let handlers = crate::plugins::hook_handler::codex_handlers(
        "notify",
        &installed.installed_dir,
        &manifest,
        &trust,
    )
    .unwrap();
    let input = serde_json::json!({"hook_event_name":"UserPromptSubmit","session_id":"t61-original-hook","turn_id":"t61-turn","model":"fixture","cwd":t.path(),"prompt":"sandbox"});
    let state_root = cache.join("agent-harness-notify");
    let directories = || -> std::collections::BTreeSet<std::path::PathBuf> {
        fs::read_dir(&state_root).into_iter().flatten().map(|e| e.unwrap().path()).collect()
    };
    let existing = directories();
    for handler in handlers {
        handler.user_prompt_submit(&input).await.unwrap();
    }
    let after = directories();
    let created = after.difference(&existing).cloned().collect::<Vec<_>>();
    assert_eq!(created.len(), 1, "original hook must create one isolated session state directory");
    let state = &created[0];
    assert!(state.is_dir(), "original hook must create isolated session state");
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&installed.installed_dir), before);
    remove_installed_plugin(&t.path().join("plugins"), "notify@public").unwrap();
    assert!(state.is_dir(), "plugin removal must preserve external session state");
    fs::remove_dir(&state).unwrap();
}

#[tokio::test]
#[ignore = "real public Context7 network integration; no credential or mock"]
async fn codex_plugin_compat_t61_public_http_original() {
    use crate::tools::{
        Tool,
        mcp_tool::{McpTool, discover_mcp_tool_specs},
    };
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "context7");
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "public").unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let pool = Arc::new(super::mcp_connections::McpSessionConnections::default());
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "context7", None, &[]).await })
        })
        .await
        .unwrap();
    let id = crate::tools::mcp_tool::mcp_tool_id("context7", "resolve-library-id").unwrap();
    let spec =
        specs.into_iter().find(|s| s.id == id).expect("public service exposes documented resolver");
    let tool = McpTool::from_session_server(
        server,
        "resolve-library-id",
        spec,
        pool.clone(),
        Some(std::time::Duration::from_secs(30)),
    );
    let result = tool.call(serde_json::json!({"libraryName":"tokio","query":"Rust Tokio async runtime task spawn API"})).await.unwrap();
    assert!(
        result.model_text().to_lowercase().contains("tokio"),
        "public resolver must return requested library"
    );
    pool.shutdown();
    set_plugin_enabled(&plugins, "context7@public", false).unwrap();
    let mut disabled = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut disabled, &options).unwrap();
    assert!(disabled.is_empty());
    remove_installed_plugin(&plugins, "context7@public").unwrap();
    assert!(!installed.installed_dir.exists());
    assert_eq!(digest(&source), before);
}

#[test]
#[ignore = "run with PATH=/usr/bin:/bin so the unchanged original package lacks its npx dependency"]
fn codex_plugin_compat_t61_original_reports_missing_dependency() {
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "pi-agent-plugins-smoke");
    let before = digest(&source);
    let observations = crate::plugins::inspect_plugin_dependencies(&source).unwrap();
    assert!(
        observations
            .iter()
            .any(|o| o.state == crate::plugins::DependencyState::MissingHost
                && o.dependency == "npx")
    );
    assert_eq!(digest(&source), before);
}

#[test]
fn codex_plugin_compat_t61_public_sources_match_locked_hashes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/plugin_compat/t61/public");
    let sources: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("sources.json")).unwrap()).unwrap();
    for package in sources["packages"].as_array().unwrap() {
        let directory = package["directory"].as_str().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let actual = digest(&public_original(temp.path(), directory))
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        let expected = package["files"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(path, hash)| (path.clone(), hash.as_str().unwrap().to_owned()))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(actual, expected, "original source drift: {directory}");
    }
}
