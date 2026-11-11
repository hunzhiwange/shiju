//! Refresh semantics exercise the production publication and file transaction owners.
use super::{claude_publish::publish, preview::*};
use crate::providers::{
    claude::ClaudeFile, live::commit::*, mode_apply::*, mode_state::StackState,
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectId, ProjectScope, TenantScope},
};

struct Runtime;
impl RouteRuntime for Runtime {
    fn prepare(&self, _: &RouteScope<'_>) -> Result<ClientEndpoint, ModeApplyError> {
        ClientEndpoint::new("http://127.0.0.1:12345")
    }
    fn publish(&self, _: &RouteScope<'_>, _: Option<&str>) -> Result<(), ModeApplyError> {
        Ok(())
    }
}
struct Fixture {
    home: tempfile::TempDir,
    _private: tempfile::TempDir,
    tx: ProviderTransactions,
    file: ClaudeFile,
    tenant: TenantScope,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx = ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
            .unwrap();
        let tenant = TenantScope::new("p", "c").unwrap();
        let mut file = ClaudeFile::open(home.path(), true).unwrap();
        tx.commit(
            &tenant,
            &mut file.guard,
            &[],
            PendingTarget {
                pointer: Some("alpha".into()),
                stack: Some(StackState {
                    members: vec!["alpha".into()],
                    keys: [("a".into(), "alpha".into())].into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
        Self { home, _private: private, tx, file, tenant }
    }
    fn write(&self, value: &Value) {
        fs::write(
            self.home.path().join(".claude/settings.json"),
            serde_json::to_vec(value).unwrap(),
        )
        .unwrap();
    }
    fn read(&self) -> Value {
        serde_json::from_slice(&fs::read(self.home.path().join(".claude/settings.json")).unwrap())
            .unwrap()
    }
    fn refresh(&mut self, record: &Value) -> bool {
        let providers = [Provider {
            id: "alpha",
            name: "Alpha",
            official_service: None,
            catalog: Catalog::Claude(record),
        }];
        let state = self.tx.state(AppId::Claude).unwrap();
        let input = || PreviewInput {
            candidate_default: Some("alpha"),
            providers: &providers,
            pending_tenant: None,
            failover_enabled: false,
            direct_restorable: true,
            catalog_policy: CatalogPolicy::Preserve,
        };
        let planned = preview(AppId::Claude, &self.tenant, &state, input()).unwrap();
        assert_eq!(planned.notices, vec![Notice::ModelReselect]);
        let project = ProjectScope::new(self.tenant.clone(), ProjectId::from("project")).unwrap();
        let mut mode = ClaudeMode {
            file: &mut self.file,
            transactions: &self.tx,
            scope: RouteScope { tenant: &self.tenant, host: "host", app: AppId::Claude },
            runtime: Some(&Runtime),
        };
        publish(&mut mode, &project, &state, true, input(), || true).unwrap().configuration_changed
    }
}
fn record(model: &str) -> Value {
    json!({"name":"Alpha", "settingsConfig":{"env":{"ANTHROPIC_MODEL":model}}})
}

#[test]
fn aggregation_t20_claude_refresh_preserves_contract() {
    let mut f = Fixture::new();
    f.write(&json!({"model":"old", "alwaysThinkingEnabled":true, "env":{"MAX_THINKING_TOKENS":"12000", "DISABLE_PROMPT_CACHING":"1", "DISABLE_PROMPT_CACHING_HAIKU":"1"}}));
    assert!(f.refresh(&record("one")));
    let first_contract = f.tx.state(AppId::Claude).unwrap().mode.unwrap().contract;
    let mut selected = f.read();
    selected["model"] = json!("ccs-claude-a--one");
    f.write(&selected);
    // An unchanged catalog must not erase a choice saved by the client.
    assert!(!f.refresh(&record("one")));
    assert_eq!(f.read(), selected);
    // Catalog changes land immediately in settings; the existing session still needs reselect.
    assert!(f.refresh(&record("two[1M]")));
    let live = f.read();
    assert!(live.get("model").is_none());
    assert_eq!(live["modelPicker"]["options"][0]["model"], "ccs-claude-a--two[1M]");
    assert_eq!(live["alwaysThinkingEnabled"], true);
    for key in ["MAX_THINKING_TOKENS", "DISABLE_PROMPT_CACHING", "DISABLE_PROMPT_CACHING_HAIKU"] {
        assert_eq!(live["env"][key], selected["env"][key]);
    }
    assert!(f.tx.state(AppId::Claude).unwrap().mode.unwrap().contract != first_contract);
}

#[test]
fn aggregation_t20_refresh_removes_only_unchanged_managed_thinking() {
    let mut f = Fixture::new();
    let mut saved = record("one");
    saved["settingsConfig"]["env"]["CLAUDE_CODE_DISABLE_THINKING"] = json!("1");
    saved["settingsConfig"]["env"]["DISABLE_INTERLEAVED_THINKING"] = json!("1");
    saved["settingsConfig"]["env"]["DISABLE_PROMPT_CACHING"] = json!("provider-value");
    f.refresh(&saved);
    let mut live = f.read();
    assert!(live["env"].get("DISABLE_PROMPT_CACHING").is_none());
    live["env"]["DISABLE_INTERLEAVED_THINKING"] = json!("0");
    live["env"]["DISABLE_PROMPT_CACHING"] = json!("user-value");
    f.write(&live);
    f.refresh(&record("two"));
    let live = f.read();
    assert!(live["env"].get("CLAUDE_CODE_DISABLE_THINKING").is_none());
    assert_eq!(live["env"]["DISABLE_INTERLEAVED_THINKING"], "0");
    assert_eq!(live["env"]["DISABLE_PROMPT_CACHING"], "user-value");
}
