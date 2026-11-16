use crate::*;
use agent_config::runtime::Config;
use std::{fs, path::Path, sync::Arc, time::Duration};
fn capture(config: &Config) -> session_snapshot::PluginTurnSnapshot {
 session_snapshot::PluginTurnSnapshot::capture(crate::mcp::plugin_mcp_discovery_options(config), || Ok(None), || false, || vw_api_transport::TenantScope::new("snapshot-fixture", "snapshot-fixture").unwrap(), |hooks| tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(hooks.shutdown()))).unwrap()
}
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
    crate::mcp::plugin_mcp_discovery_options(c)
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
    let turn = capture(&c);
    let context = turn.data.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let r = crate::upgrade::activate_plugin_upgrade(prepared, &|_| {});
        done_tx.send(r.map(|_| ())).unwrap();
    });
    started_rx.recv().unwrap();
    let waiting = done_rx.recv_timeout(Duration::from_millis(150));
    let discovery = context.selection(&options(&c)).map(|(discovery, _)| discovery)
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
    let next = capture(&c);
    let next_context = next.data.clone();
    let d = next_context.selection(&options(&c)).map(|(discovery, _)| discovery)
    .unwrap();
    assert_eq!(d.enabled[0].manifest.version.as_deref(), Some("2"));
    assert_eq!(
        fs::read_to_string(source.join("skills/review/SKILL.md")).unwrap(),
        "---\nname: review\ndescription: Review fixture\n---\nOriginal fixture body\n"
    );
}
#[test]
fn codex_plugin_compat_t57_defers_new_store_until_next_turn() {
    let root = tempfile::tempdir().unwrap();
    let c = config(root.path());
    let turn = capture(&c);
    let context = turn.data.clone();
    let source = root.path().join("source");
    package(&source, "1");
    install_plugin_from_marketplace(
        source.to_str().unwrap(),
        &c.workspace_dir.join("plugins"),
        "alpha",
    )
    .unwrap();
    assert!(
        context.selection(&options(&c)).map(|(discovery, _)| discovery)
            .unwrap()
            .installed
            .is_empty()
    );
    drop(turn);
    let next = capture(&c);
    let context = next.data.clone();
    assert_eq!(
        context.selection(&options(&c)).map(|(discovery, _)| discovery)
            .unwrap()
            .installed
            .len(),
        1
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
    let mut turn = capture(&config(root.path()));
    let hooks = Arc::new(plugin_hooks::hook_async::AsyncHooks::default());
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
