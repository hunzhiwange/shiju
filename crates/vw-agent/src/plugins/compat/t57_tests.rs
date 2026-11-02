use super::session_snapshot::{PluginTurnSnapshot, discover_for_runtime};
use crate::{config::Config, plugins::*, skills::SkillLoadMode, tools::ToolUseContext};
use std::{fs, path::Path, sync::Arc, time::Duration};
fn package(root: &Path, version: &str) {
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::create_dir_all(root.join("skills/review")).unwrap();
    fs::write(root.join(".codex-plugin/plugin.json"), format!(r#"{{"name":"fixture","version":"{version}","skills":"./skills","mcpServers":{{"public":{{"command":"fixture-{version}"}}}}}}"#)).unwrap();
    fs::write(
        root.join("skills/review/SKILL.md"),
        "---\nname: review\ndescription: Review fixture\n---\nOriginal fixture body\n",
    )
    .unwrap();
}
fn config(root: &Path) -> Config {
    let mut c = Config::default();
    c.workspace_dir = root.join("workspace");
    c.config_path = root.join("config/shiju.json");
    c
}
fn options(c: &Config) -> PluginDiscoveryOptions {
    super::super::mcp::plugin_mcp_discovery_options(c)
}
#[test]
fn codex_plugin_compat_t57_switches_version_at_turn_boundary() {
    let temp = tempfile::tempdir().unwrap();
    let c = config(temp.path());
    let store = c.workspace_dir.join("plugins");
    let source = temp.path().join("original");
    package(&source, "1");
    install_plugin_from_marketplace(source.to_str().unwrap(), &store, "alpha").unwrap();
    package(&source, "2");
    let prepared =
        prepare_plugin_upgrade(source.to_str().unwrap(), &store, "fixture@alpha").unwrap();
    let turn = PluginTurnSnapshot::capture(&c).unwrap();
    let context = Arc::new(
        ToolUseContext::new("turn", Some(c.workspace_dir.to_string_lossy().into_owned()))
            .with_plugin_snapshot(turn.data.clone()),
    );
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let r = activate_plugin_upgrade(prepared);
        done_tx.send(r.map(|_| ())).unwrap();
    });
    started_rx.recv().unwrap();
    let waiting = done_rx.recv_timeout(Duration::from_millis(150));
    let discovery = crate::tools::scope_tool_use_context_sync(context.clone(), || {
        discover_for_runtime(&options(&c))
    })
    .unwrap();
    let old = load_plugin_manifest(&store.join("fixture@alpha")).unwrap().version;
    drop(turn);
    assert!(waiting.is_err(), "upgrade must wait for active turn instead of replacing its package");
    assert_eq!(old.as_deref(), Some("1"));
    assert_eq!(discovery.enabled[0].manifest.version.as_deref(), Some("1"));
    done_rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    worker.join().unwrap();
    // Persistent MCP/context clones contain data only; they must not keep a lifecycle lease.
    drop(context);
    let next = PluginTurnSnapshot::capture(&c).unwrap();
    let next_context =
        Arc::new(ToolUseContext::new("next", None).with_plugin_snapshot(next.data.clone()));
    let d = crate::tools::scope_tool_use_context_sync(next_context, || {
        discover_for_runtime(&options(&c))
    })
    .unwrap();
    assert_eq!(d.enabled[0].manifest.version.as_deref(), Some("2"));
    assert_eq!(
        fs::read_to_string(source.join("skills/review/SKILL.md")).unwrap(),
        "---\nname: review\ndescription: Review fixture\n---\nOriginal fixture body\n"
    );
}
#[test]
fn codex_plugin_compat_t57_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    let mut c = config(root.path());
    c.plugins.sources = vec!["missing-explicit-source".into()];
    assert!(PluginTurnSnapshot::capture(&c).is_err());
    c.plugins.sources.clear();
    let turn = PluginTurnSnapshot::capture(&c).unwrap();
    let context =
        Arc::new(ToolUseContext::new("owner", None).with_plugin_snapshot(turn.data.clone()));
    let mut foreign = options(&c);
    foreign.workspace_dir = root.path().join("other");
    let result = crate::tools::scope_tool_use_context_sync(context.clone(), || {
        discover_for_runtime(&foreign)
    });
    assert!(result.is_err(), "a snapshot must reject a foreign workspace even when it is empty");
    let tenant = vw_api_types::id::TenantScope::new("other-platform", "other-company").unwrap();
    assert!(
        crate::session::ui_store::with_tenant_scope_sync(tenant, || {
            crate::tools::scope_tool_use_context_sync(context.clone(), || {
                discover_for_runtime(&options(&c))
            })
        })
        .is_err()
    );
    drop(turn);
    assert!(
        crate::tools::scope_tool_use_context_sync(context, || discover_for_runtime(&options(&c)))
            .is_err()
    );
}
#[test]
fn codex_plugin_compat_t57_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let mut c = config(root.path());
    let source = root.path().join("original");
    package(&source, "1");
    for market in ["alpha", "beta"] {
        install_plugin_from_marketplace(
            source.to_str().unwrap(),
            &c.workspace_dir.join("plugins"),
            market,
        )
        .unwrap();
    }
    c.plugins.enabled = vec!["fixture@alpha".into()];
    let turn = PluginTurnSnapshot::capture(&c).unwrap();
    let context =
        Arc::new(ToolUseContext::new("owner", None).with_plugin_snapshot(turn.data.clone()));
    c.plugins.enabled = vec!["fixture@beta".into()];
    crate::tools::scope_tool_use_context_sync(context.clone(), || {
        let d = discover_for_runtime(&options(&c)).unwrap();
        assert_eq!(
            d.enabled[0].identity.as_key(),
            "fixture@alpha",
            "current selection must remain pinned despite live config changes"
        );
        let inventory = super::super::skills::load_plugin_skill_inventory(
            &c.workspace_dir,
            &c,
            SkillLoadMode::Full,
        );
        assert_eq!(inventory.skills.len(), 1);
        let skill = &inventory.skills[0];
        assert_eq!(skill.invocation_name(), "fixture@alpha:review");
        let record = crate::skills::InvokedSkillRecord::new(
            skill.invocation_name(),
            skill.location.as_ref().unwrap(),
            None,
        )
        .unwrap();
        let recovered: crate::skills::InvokedSkillRecord =
            serde_json::from_value(serde_json::to_value(&record).unwrap()).unwrap();
        assert!(recovered.is_authorized_by(skill));
        assert!(recovered.replay().unwrap().content.contains("fixture@alpha:review"));
        let mut servers = vec![];
        merge_enabled_plugin_mcp_servers(&mut servers, &options(&c)).unwrap();
        assert_eq!(servers.len(), 1);
        let mut other = options(&c);
        other.config_dir = root.path().join("foreign-config");
        assert!(merge_enabled_plugin_mcp_servers(&mut vec![], &other).is_err());
    });
    drop(turn);
    let next = PluginTurnSnapshot::capture(&c).unwrap();
    let next_context =
        Arc::new(ToolUseContext::new("next", None).with_plugin_snapshot(next.data.clone()));
    let skills = crate::tools::scope_tool_use_context_sync(next_context, || {
        super::super::skills::load_plugin_skill_inventory(&c.workspace_dir, &c, SkillLoadMode::Full)
    });
    assert_eq!(skills.skills[0].invocation_name(), "fixture@beta:review");
    drop(context);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_plugin_compat_t57_refreshes_required_hooks_at_boundary() {
    let root = tempfile::tempdir().unwrap();
    let mut c = config(root.path());
    c.hooks.enabled = false;
    let turn = PluginTurnSnapshot::capture(&c).unwrap();
    let context =
        Arc::new(ToolUseContext::new("owner", None).with_plugin_snapshot(turn.data.clone()));
    c.hooks.required = vec!["unavailable-hook".into()];
    let required = crate::tools::scope_tool_use_context(
        context.clone(),
        crate::hooks::authorize_model_request(&c, "fixture"),
    )
    .await
    .unwrap();
    assert!(!required, "the active turn uses its admitted hook policy");
    drop(turn);
    assert!(
        PluginTurnSnapshot::capture(&c).is_err(),
        "next admission must reject unavailable required hooks"
    );
    assert!(
        crate::tools::scope_tool_use_context(
            context,
            crate::hooks::authorize_model_request(&c, "fixture")
        )
        .await
        .is_err(),
        "expired contexts cannot authorize another model request"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_plugin_compat_t57_joins_cancelled_work_before_releasing_turn() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Finished(Arc<AtomicBool>);
    impl Drop for Finished {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let root = tempfile::tempdir().unwrap();
    let mut turn = PluginTurnSnapshot::capture(&config(root.path())).unwrap();
    let hooks = Arc::new(super::hook_async::AsyncHooks::default());
    turn.own_async_hooks(hooks.clone());
    let finished = Arc::new(AtomicBool::new(false));
    let marker = finished.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    hooks
        .schedule(async move {
            let _guard = Finished(marker);
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        })
        .unwrap();
    ready.await.unwrap();
    drop(turn);
    assert!(
        finished.load(Ordering::Acquire),
        "turn release must wait for the cancelled future's process/resource guards"
    );
    assert!(hooks.is_closed());
    assert!(hooks.schedule(async {}).is_err());
}
#[test]
fn codex_plugin_compat_t57_defers_new_store_until_next_turn() {
    let root = tempfile::tempdir().unwrap();
    let c = config(root.path());
    let turn = PluginTurnSnapshot::capture(&c).unwrap();
    let context =
        Arc::new(ToolUseContext::new("owner", None).with_plugin_snapshot(turn.data.clone()));
    let source = root.path().join("source");
    package(&source, "1");
    install_plugin_from_marketplace(
        source.to_str().unwrap(),
        &c.workspace_dir.join("plugins"),
        "alpha",
    )
    .unwrap();
    assert!(
        crate::tools::scope_tool_use_context_sync(context, || discover_for_runtime(&options(&c)))
            .unwrap()
            .installed
            .is_empty()
    );
    drop(turn);
    let next = PluginTurnSnapshot::capture(&c).unwrap();
    let context =
        Arc::new(ToolUseContext::new("next", None).with_plugin_snapshot(next.data.clone()));
    assert_eq!(
        crate::tools::scope_tool_use_context_sync(context, || discover_for_runtime(&options(&c)))
            .unwrap()
            .installed
            .len(),
        1
    );
}
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_plugin_compat_t57_retires_old_mcp_process_after_turn() {
    let root = tempfile::tempdir().unwrap();
    let c = config(root.path());
    let store = c.workspace_dir.join("plugins");
    let source = root.path().join("source");
    let marker = root.path().join("starts");
    let create = |version: &str| {
        package(&source, version);
        fs::write(
            source.join("server.py"),
            include_bytes!("../../../testdata/plugin_compat/t18/server.py"),
        )
        .unwrap();
        fs::write(source.join(".codex-plugin/plugin.json"),serde_json::to_vec(&serde_json::json!({"name":"fixture","version":version,"mcpServers":{"public":{"command":"python3","cwd":store.join("fixture@alpha"),"args":[store.join("fixture@alpha/server.py"),marker.to_string_lossy(),"normal"]}}})).unwrap()).unwrap();
    };
    create("1");
    install_plugin_from_marketplace(source.to_str().unwrap(), &store, "alpha").unwrap();
    create("2");
    let prepared =
        prepare_plugin_upgrade(source.to_str().unwrap(), &store, "fixture@alpha").unwrap();
    let turn = PluginTurnSnapshot::capture(&c).unwrap();
    let context = Arc::new(
        ToolUseContext::new("mcp-turn", Some(c.workspace_dir.to_string_lossy().into_owned()))
            .with_plugin_snapshot(turn.data.clone()),
    );
    let pool = super::mcp_connection_sessions::connections("mcp-turn", &c.workspace_dir);
    async fn pid(
        pool: &super::mcp_connections::McpSessionConnections,
        server: &agent_client_protocol::McpServer,
    ) -> String {
        pool.with_peer(server, None, |peer| {
            Box::pin(async move {
                Ok(peer.list_all_tools().await?[0].description.as_deref().unwrap().to_owned())
            })
        })
        .await
        .unwrap()
    }
    let (old, old_server) = crate::tools::scope_tool_use_context(context.clone(), async {
        let mut servers = vec![];
        merge_enabled_plugin_mcp_servers(&mut servers, &options(&c)).unwrap();
        (pid(&pool, &servers[0]).await, servers.remove(0))
    })
    .await;
    let (tx, rx) = std::sync::mpsc::channel();
    let worker =
        std::thread::spawn(move || tx.send(activate_plugin_upgrade(prepared).map(|_| ())).unwrap());
    let blocked = rx.recv_timeout(Duration::from_millis(150));
    assert!(blocked.is_err());
    assert_eq!(
        unsafe { libc::kill(old.parse().unwrap(), 0) },
        0,
        "old process remains alive while its turn owns the package"
    );
    drop(turn);
    rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    worker.join().unwrap();
    assert!(
        crate::tools::scope_tool_use_context(
            context.clone(),
            pool.with_peer(&old_server, None, |_peer| Box::pin(async { Ok(()) }))
        )
        .await
        .is_err(),
        "retained contexts cannot restart an old plugin after their turn expires"
    );
    for _ in 0..100 {
        if unsafe { libc::kill(old.parse().unwrap(), 0) } != 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_ne!(
        unsafe { libc::kill(old.parse().unwrap(), 0) },
        0,
        "upgrade must retire the old plugin process"
    );
    let next = PluginTurnSnapshot::capture(&c).unwrap();
    let next_context = Arc::new(
        ToolUseContext::new("mcp-turn", Some(c.workspace_dir.to_string_lossy().into_owned()))
            .with_plugin_snapshot(next.data.clone()),
    );
    let new = crate::tools::scope_tool_use_context(next_context, async {
        let mut servers = vec![];
        merge_enabled_plugin_mcp_servers(&mut servers, &options(&c)).unwrap();
        pid(&pool, &servers[0]).await
    })
    .await;
    assert_ne!(new, old);
    assert_eq!(fs::read_to_string(marker).unwrap().lines().count(), 2);
    super::mcp_connection_sessions::close_session("mcp-turn", &c.workspace_dir);
    drop(context);
}
