use crate::compat_t61_tests::{public_original, digest};
use crate::install_plugin_from_marketplace;
use std::{fs, path::Path};
fn remove_installed_plugin(path: &Path, name: &str) -> anyhow::Result<()> {
    crate::install::remove_installed_plugin(path, name, &plugin_mcp::mcp_connection_sessions::close_plugin)
}
#[test]
fn codex_plugin_compat_t61_public_marketplace_cli_lifecycle() {
    use clap::Parser;
    #[derive(Parser)]
    struct Args {
        #[command(subcommand)]
        command: agent_cli_types::PluginCommands,
    }
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "superpowers-7.0.1");
    let before = digest(&source);
    let mut config = agent_config::runtime::Config::default();
    config.config_path = t.path().join("config/config.json");
    config.workspace_dir = t.path().join("workspace");
    let run = |args: &[&str]| {
        let parsed =
            Args::try_parse_from(std::iter::once("plugin").chain(args.iter().copied())).unwrap();
        super::cli::handle_command(parsed.command, &config).unwrap();
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

#[test]
#[ignore = "run with PATH=/usr/bin:/bin so the unchanged original package lacks its npx dependency"]
fn codex_plugin_compat_t61_original_reports_missing_dependency() {
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "pi-agent-plugins-smoke");
    let before = digest(&source);
    let observations = super::dependencies::inspect_plugin_dependencies(&source).unwrap();
    assert!(
        observations
            .iter()
            .any(|o| o.state == plugin_install::dependencies::DependencyState::MissingHost
                && o.dependency == "npx")
    );
    assert_eq!(digest(&source), before);
}

#[tokio::test]
#[ignore = "requires dedicated XDG_CACHE_HOME and XDG_CONFIG_HOME to isolate original hook state"]
async fn codex_plugin_compat_t61_public_hook_original() {
    use crate::{PluginHookTrustStore, PluginHooksManifest, load_plugin_hook_source};
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
        crate::hook_handler::codex_handlers::<crate::host_migration_38_tests::host::Host>(
            "notify",
            &installed.installed_dir,
            &manifest,
            &trust
        )
        .unwrap()
        .is_empty()
    );
    trust.trust_codex_hooks("notify", &installed.installed_dir, &manifest).unwrap();
    let handlers = crate::hook_handler::codex_handlers::<crate::host_migration_38_tests::host::Host>(
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
