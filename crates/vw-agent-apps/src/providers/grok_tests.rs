use super::{
    grok::*,
    live::{commit::*, write_guard::WriteError},
};
use std::{fs, os::unix::fs::PermissionsExt};
use toml_edit::DocumentMut;
use vw_api_types::{agent_apps::AppId, app_providers::grok::*, id::TenantScope};
fn third(name: &str) -> GrokProfile {
    GrokProfile {
        name: "Fixture".into(),
        config: GrokConfig::ThirdParty {
            toml: format!(
                "[models]\ndefault = '{name}'\n[model.'{name}']\nmodel = 'remote'\nname = 'Fixture'\nbase_url = 'https://fixture.test/v1'\napi_key = 'fake-secret'\napi_backend = 'responses'\ncontext_window = 1000\n"
            ),
        },
    }
}
fn official() -> GrokProfile {
    GrokProfile { name: "Official".into(), config: GrokConfig::Official {} }
}
struct Fixture {
    home: tempfile::TempDir,
    root: tempfile::TempDir,
    tenant: TenantScope,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(home.path().join(".grok")).unwrap();
        fs::write(home.path().join(".grok/config.toml"), "# keep\n[ui]\ntheme = 'dark'\n[model.mine]\nmodel = 'user' # keep model\n[mcp_servers.local]\ncommand = 'fixture'\n").unwrap();
        Self { home, root, tenant: TenantScope::new("p", "a").unwrap() }
    }
    fn tx(&self) -> ProviderTransactions {
        ProviderTransactions::new(self.root.path(), ExecutionHostId::new("host").unwrap()).unwrap()
    }
    fn read(&self) -> String {
        fs::read_to_string(self.home.path().join(".grok/config.toml")).unwrap()
    }
    fn apply(
        &self,
        tx: &ProviderTransactions,
        files: &mut GrokFiles,
        profile: &GrokProfile,
    ) -> Result<(), WriteError> {
        files.apply(tx, &self.tenant, &tx.state(AppId::GrokBuild).unwrap(), "one", profile, || true)
    }
}
#[test]
fn cc04_grok_switch_preserves_unowned_model_tables() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GrokFiles::open(f.home.path()).unwrap();
    f.apply(&tx, &mut files, &third("old")).unwrap();
    // Grok changes default independently; only the persisted write record proves ownership.
    fs::write(
        f.home.path().join(".grok/config.toml"),
        f.read().replace("default = \"old\"", "default = 'builtin'"),
    )
    .unwrap();
    f.apply(&tx, &mut files, &third("new")).unwrap();
    let text = f.read();
    let doc: DocumentMut = text.parse().unwrap();
    assert!(doc["model"].get("old").is_none(), "old written table remains");
    assert_eq!(doc["model"]["mine"]["model"].as_str(), Some("user"));
    assert_eq!(doc["models"]["default"].as_str(), Some("new"));
    assert_eq!(doc["models"]["session_summary"].as_str(), Some("new"));
    assert!(text.starts_with("# keep\n[ui]\ntheme = 'dark'\n"));
    assert!(text.contains("model = 'user' # keep model"));
    assert!(text.contains("[mcp_servers.local]\ncommand = 'fixture'"));
    f.apply(&tx, &mut files, &official()).unwrap();
    let doc: DocumentMut = f.read().parse().unwrap();
    assert!(doc["model"].get("new").is_none());
    assert!(doc.get("models").is_none());
    assert_eq!(tx.state(AppId::GrokBuild).unwrap().pointer.unwrap().id, "one");
    assert_eq!(
        fs::metadata(f.home.path().join(".grok/config.toml")).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[test]
fn cc04_grok_editor_schema_rejects_bad_rows() {
    for text in [
        "invalid",
        "",
        "[models]\ndefault='missing'\n",
        "[model.one]\nmodel='x'\n",
        "[model.a]\nmodel='x'\n[model.b]\nmodel='y'",
    ] {
        assert!(
            settings(&GrokProfile {
                name: "Fixture".into(),
                config: GrokConfig::ThirdParty { toml: text.into() }
            })
            .is_err()
        );
    }
    let mut p = third("one");
    if let GrokConfig::ThirdParty { toml } = &mut p.config {
        *toml = toml.replace("context_window = 1000", "context_window = -1");
    }
    assert!(settings(&p).is_err());
}
#[test]
fn cc04_grok_failures_recover_file_pointer_and_written_record() {
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let f = Fixture::new();
        let tx = f.tx();
        let mut files = GrokFiles::open(f.home.path()).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        assert!(f.apply(&tx, &mut files, &third("old")).is_err());
        HOOK.with_borrow_mut(|h| *h = None);
        drop(files);
        drop(tx);
        let tx = f.tx();
        let mut files = GrokFiles::open(f.home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        if phase == Phase::Intent {
            assert!(tx.state(AppId::GrokBuild).unwrap().pointer.is_none());
        } else {
            let state = tx.state(AppId::GrokBuild).unwrap();
            assert_eq!(state.pointer.unwrap().id, "one");
            assert_eq!(state.mode.unwrap().contract.unwrap().exclusive["grokWrittenTable"], "old");
            f.apply(&tx, &mut files, &official()).unwrap();
            assert!(!f.read().contains("[model.old]"));
        }
    }
}
#[test]
fn cc04_grok_input_conflict_and_symlink_never_publish() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GrokFiles::open(f.home.path()).unwrap();
    let before = f.read();
    assert!(
        files
            .apply(
                &tx,
                &f.tenant,
                &tx.state(AppId::GrokBuild).unwrap(),
                "one",
                &third("one"),
                || false
            )
            .is_err()
    );
    assert_eq!(f.read(), before);
    fs::remove_file(f.home.path().join(".grok/config.toml")).unwrap();
    let outside = f.home.path().join("outside");
    fs::write(&outside, b"unchanged").unwrap();
    std::os::unix::fs::symlink(&outside, f.home.path().join(".grok/config.toml")).unwrap();
    assert!(f.apply(&tx, &mut files, &third("one")).is_err());
    assert_eq!(fs::read(outside).unwrap(), b"unchanged");
}
#[test]
fn cc04_grok_whole_table_replacement_and_user_summary() {
    let pre = "[models]\nsession_summary='user-summary' # titles\n[model.same]\nenv_key='STALE_KEY'\n[model.proxy]\napi_key='PROXY_MANAGED'\n";
    let bytes = patch(Some(pre.as_bytes()), &third("same"), None).unwrap();
    let doc: DocumentMut = std::str::from_utf8(&bytes).unwrap().parse().unwrap();
    assert!(doc["model"]["same"].get("env_key").is_none());
    assert!(doc["model"].get("proxy").is_none());
    assert_eq!(doc["models"]["session_summary"].as_str(), Some("user-summary"));
}
#[test]
fn cc04_grok_external_change_is_preserved_on_recovery() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GrokFiles::open(f.home.path()).unwrap();
    let path = f.home.path().join(".grok/config.toml");
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, at| {
            if at == Phase::BeforePublish {
                fs::write(&path, b"# external\n").unwrap();
            }
            Ok(())
        }))
    });
    assert!(f.apply(&tx, &mut files, &third("one")).is_err());
    HOOK.with_borrow_mut(|h| *h = None);
    // Single-file rejection occurs before any file is published: the existing
    // transaction owner discards intent instead of settling a foreign write.
    assert!(tx.recover(&mut files.guard).unwrap().is_none());
    assert!(tx.state(AppId::GrokBuild).unwrap().pointer.is_none());
    assert_eq!(f.read(), "# external\n");
}
#[test]
fn cc04_grok_nested_vendor_tables_follow_parent_without_reordering_user_sections() {
    let mut profile = third("vendor.with.dot");
    if let GrokConfig::ThirdParty { toml } = &mut profile.config {
        *toml = format!(
            "# row\n[global]\nx = 1\n{toml}\n[model.'vendor.with.dot'.extra_headers]\nX-Team='fixture'\n[model.'vendor.with.dot'.vendor]\noption=true\n"
        );
    }
    let before = "# live\n[a]\nx=1\n[b]\ny=2\n[c]\nz=3\n";
    let bytes = patch(Some(before.as_bytes()), &profile, None).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.starts_with(before), "{text}");
    let doc: DocumentMut = text.parse().unwrap();
    assert_eq!(
        doc["model"]["vendor.with.dot"]["extra_headers"]["X-Team"].as_str(),
        Some("fixture")
    );
    assert_eq!(doc["model"]["vendor.with.dot"]["vendor"]["option"].as_bool(), Some(true));
    assert!(doc.get("global").is_none());
}
#[test]
fn cc04_grok_changed_input_after_intent_does_not_publish() {
    use std::{cell::Cell, rc::Rc};
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GrokFiles::open(f.home.path()).unwrap();
    let before = f.read();
    let current = Rc::new(Cell::new(true));
    let at_intent = current.clone();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::Intent {
                at_intent.set(false);
            }
            Ok(())
        }))
    });
    let result = files.apply(
        &tx,
        &f.tenant,
        &tx.state(AppId::GrokBuild).unwrap(),
        "one",
        &third("one"),
        || current.get(),
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(result, Err(WriteError::Conflict));
    assert!(!tx.pending(AppId::GrokBuild).unwrap().unwrap().published);
    tx.recover(&mut files.guard).unwrap();
    assert_eq!(f.read(), before);
    assert!(tx.state(AppId::GrokBuild).unwrap().pointer.is_none());
}
