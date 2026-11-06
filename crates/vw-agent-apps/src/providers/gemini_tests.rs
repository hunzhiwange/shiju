use super::{
    gemini::*,
    live::{commit::*, write_guard::WriteError},
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{agent_apps::AppId, app_providers::gemini::*, id::TenantScope};
fn third() -> GeminiProfile {
    GeminiProfile {
        name: "Fixture".into(),
        config: GeminiConfig::ThirdParty {
            base_url: "https://fixture.test".into(),
            api_key: "fake-key".into(),
        },
        model: Some("model-new".into()),
    }
}
fn official() -> GeminiProfile {
    GeminiProfile { name: "Google".into(), config: GeminiConfig::Official {}, model: None }
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
        fs::create_dir(home.path().join(".gemini")).unwrap();
        fs::write(home.path().join(".gemini/.env"), b"# my comment\nGEMINI_SANDBOX=docker\nDEBUG=1\nGEMINI_API_KEY=old\nGOOGLE_CLOUD_PROJECT=old\n").unwrap();
        fs::write(home.path().join(".gemini/settings.json"), br#"{"mcpServers":{"fixture":{"command":"fixture"}},"ui":{"theme":"dark"},"security":{"auth":{"selectedType":"oauth-personal","useExternal":true}},"model":{"name":"old","compressionThreshold":0.5}}"#).unwrap();
        Self { home, root, tenant: TenantScope::new("p", "a").unwrap() }
    }
    fn tx(&self) -> ProviderTransactions {
        ProviderTransactions::new(self.root.path(), ExecutionHostId::new("host").unwrap()).unwrap()
    }
    fn read(&self, n: &str) -> Vec<u8> {
        fs::read(self.home.path().join(".gemini").join(n)).unwrap()
    }
    fn apply(
        &self,
        tx: &ProviderTransactions,
        files: &mut GeminiFiles,
        p: &GeminiProfile,
        id: &str,
    ) -> Result<(), WriteError> {
        files.apply(tx, &self.tenant, &tx.state(AppId::Gemini).unwrap(), id, p, || true)
    }
}
#[test]
fn cc04_gemini_switch_commits_both_files() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GeminiFiles::open(f.home.path()).unwrap();
    f.apply(&tx, &mut files, &third(), "third").unwrap();
    assert!(String::from_utf8(f.read(".env")).unwrap().contains("fake-key"));
    let v: Value = serde_json::from_slice(&f.read("settings.json")).unwrap();
    assert_eq!(v["security"]["auth"]["selectedType"], "gemini-api-key");
    assert_eq!(v["model"]["name"], "model-new");
    assert_eq!(v["mcpServers"]["fixture"]["command"], "fixture");
    assert_eq!(tx.state(AppId::Gemini).unwrap().pointer.unwrap().id, "third");
    assert!(tx.pending(AppId::Gemini).unwrap().is_none());
    for n in [".env", "settings.json"] {
        assert_eq!(
            fs::metadata(f.home.path().join(".gemini").join(n)).unwrap().permissions().mode()
                & 0o777,
            0o600
        );
    }
}
#[test]
fn cc04_gemini_official_switch_removes_only_managed_keys() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GeminiFiles::open(f.home.path()).unwrap();
    f.apply(&tx, &mut files, &third(), "third").unwrap();
    f.apply(&tx, &mut files, &official(), "official").unwrap();
    assert_eq!(f.read(".env"), b"# my comment\nGEMINI_SANDBOX=docker\nDEBUG=1\n");
    let v: Value = serde_json::from_slice(&f.read("settings.json")).unwrap();
    assert_eq!(v["security"]["auth"]["selectedType"], "oauth-personal");
    assert_eq!(v["security"]["auth"]["useExternal"], true);
    assert_eq!(v["ui"]["theme"], "dark");
    assert_eq!(v["model"]["compressionThreshold"], json!(0.5));
    assert!(v["model"].get("name").is_none());
    assert_eq!(v["mcpServers"]["fixture"]["command"], "fixture");
}
#[test]
fn cc04_gemini_two_file_failures_recover_consistently() {
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let f = Fixture::new();
        let tx = f.tx();
        let mut files = GeminiFiles::open(f.home.path()).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        assert!(f.apply(&tx, &mut files, &third(), "third").is_err());
        HOOK.with_borrow_mut(|h| *h = None);
        let pending = tx.pending(AppId::Gemini).unwrap();
        if let Some(p) = &pending {
            assert_eq!(p.files.len(), 2);
        }
        drop(files);
        drop(tx);
        let tx = f.tx();
        let mut files = GeminiFiles::open(f.home.path()).unwrap();
        tx.recover(&mut files.guard).unwrap();
        if phase == Phase::Intent {
            assert!(tx.state(AppId::Gemini).unwrap().pointer.is_none());
        } else {
            assert_eq!(tx.state(AppId::Gemini).unwrap().pointer.unwrap().id, "third");
            assert!(String::from_utf8(f.read(".env")).unwrap().contains("fake-key"));
            let v: Value = serde_json::from_slice(&f.read("settings.json")).unwrap();
            assert_eq!(v["model"]["name"], "model-new");
        }
    }
}
#[test]
fn cc04_gemini_bad_settings_and_changed_input_do_not_publish() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GeminiFiles::open(f.home.path()).unwrap();
    let env = f.read(".env");
    let config = f.read("settings.json");
    assert!(
        files
            .apply(&tx, &f.tenant, &tx.state(AppId::Gemini).unwrap(), "one", &third(), || false)
            .is_err()
    );
    assert_eq!(f.read(".env"), env);
    assert_eq!(f.read("settings.json"), config);
    fs::write(f.home.path().join(".gemini/settings.json"), b"invalid").unwrap();
    assert!(f.apply(&tx, &mut files, &third(), "one").is_err());
    assert_eq!(f.read(".env"), env);
    assert!(tx.state(AppId::Gemini).unwrap().pointer.is_none());
}

#[test]
fn cc04_gemini_json_comments_and_unmanaged_env_bytes_survive() {
    let pre = b"# keep\r\nGEMINI_SANDBOX='docker'\r\nDEBUG = 1\nGEMINI_API_KEY=old\nGEMINI_API_KEY=older\n";
    let config = br#"{
      // keep this MCP comment
      "mcpServers": {"local": {"command": "fixture"}},
      "security": {"auth": {"selectedType": "oauth-personal", "other": 1}},
      "model": {"name": "old", "compressionThreshold": 0.5},
    }"#;
    let (env, settings) = patches(Some(pre), Some(config), &third()).unwrap();
    let env = String::from_utf8(env).unwrap();
    assert!(env.starts_with("# keep\r\nGEMINI_SANDBOX='docker'\r\nDEBUG = 1\n"));
    assert_eq!(env.matches("GEMINI_API_KEY=").count(), 1);
    let settings = String::from_utf8(settings).unwrap();
    assert!(settings.contains("// keep this MCP comment"));
    assert!(settings.contains(r#""mcpServers": {"local": {"command": "fixture"}}"#));
    assert!(settings.contains(r#""compressionThreshold": 0.5"#));
}
#[test]
fn cc04_gemini_external_change_is_preserved_and_recovery_reports_exception() {
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GeminiFiles::open(f.home.path()).unwrap();
    let config_path = f.home.path().join(".gemini/settings.json");
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, at| {
            if at == Phase::BeforePublish {
                fs::write(&config_path, br#"{"external":true}"#).unwrap();
            }
            Ok(())
        }))
    });
    assert!(f.apply(&tx, &mut files, &third(), "third").is_err());
    HOOK.with_borrow_mut(|h| *h = None);
    let outcome = tx.recover(&mut files.guard).unwrap();
    assert!(
        matches!(outcome, Some(super::live::recovery::RecoveryOutcome::RolledForwardExcept { names }) if names == vec![b"settings.json".to_vec()])
    );
    assert_eq!(f.read("settings.json"), br#"{"external":true}"#);
    assert_eq!(tx.state(AppId::Gemini).unwrap().pointer.unwrap().id, "third");
}
#[test]
fn cc04_gemini_invalid_credentials_and_symlinks_are_rejected() {
    let f = Fixture::new();
    let mut p = third();
    for key in ["", "[redacted]", "****", "line\nbreak"] {
        p.config = GeminiConfig::ThirdParty {
            base_url: "https://fixture.test".into(),
            api_key: key.into(),
        };
        assert!(settings(&p).is_err());
    }
    let external = f.home.path().join("external");
    fs::write(&external, b"untouched").unwrap();
    fs::remove_file(f.home.path().join(".gemini/.env")).unwrap();
    std::os::unix::fs::symlink(&external, f.home.path().join(".gemini/.env")).unwrap();
    let tx = f.tx();
    let mut files = GeminiFiles::open(f.home.path()).unwrap();
    assert!(f.apply(&tx, &mut files, &third(), "third").is_err());
    assert_eq!(fs::read(external).unwrap(), b"untouched");
}

#[test]
fn cc04_gemini_input_changed_after_intent_never_publishes() {
    use std::{cell::Cell, rc::Rc};
    let f = Fixture::new();
    let tx = f.tx();
    let mut files = GeminiFiles::open(f.home.path()).unwrap();
    let env = f.read(".env");
    let config = f.read("settings.json");
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
    let result =
        files.apply(&tx, &f.tenant, &tx.state(AppId::Gemini).unwrap(), "third", &third(), || {
            current.get()
        });
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(result, Err(WriteError::Conflict));
    let pending = tx.pending(AppId::Gemini).unwrap().unwrap();
    assert!(!pending.published);
    tx.recover(&mut files.guard).unwrap();
    assert_eq!(f.read(".env"), env);
    assert_eq!(f.read("settings.json"), config);
    assert!(tx.state(AppId::Gemini).unwrap().pointer.is_none());
}

#[test]
fn cc04_gemini_dotenv_keeps_literal_key_punctuation() {
    let mut p = third();
    p.config = GeminiConfig::ThirdParty {
        base_url: "https://fixture.test".into(),
        api_key: r#"fake"key\name#suffix"#.into(),
    };
    let (env, _) = patches(None, None, &p).unwrap();
    let env = String::from_utf8(env).unwrap();
    assert!(env.contains("GEMINI_API_KEY='fake\"key\\name#suffix'"));
    p.config = GeminiConfig::ThirdParty {
        base_url: "https://fixture.test".into(),
        api_key: "fake'key#suffix".into(),
    };
    let (env, _) = patches(None, None, &p).unwrap();
    assert!(String::from_utf8(env).unwrap().contains("GEMINI_API_KEY=`fake'key#suffix`"));
}
