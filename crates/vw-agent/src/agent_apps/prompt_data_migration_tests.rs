use super::{PromptsStore, PromptScope};
use crate::global_database::GlobalDatabase;
use serde_json::json;
use vw_agent_apps::data_management::{source::{SourceScope, read_selected_source}, preview::Decision};
use vw_api_types::{id::{ProjectScope, TenantScope}, agent_apps::AppId};

#[test]
fn cc_data_12_owner_import_repeat_skip_replace_and_stale() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let snapshots = tempfile::tempdir().unwrap();
    std::fs::set_permissions(snapshots.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let project = ProjectScope::new(TenantScope::new("fixture-platform", "fixture-company").unwrap(), "fixture-project".into()).unwrap();
    let scope = SourceScope::new(project.clone(), "fixture-host".into(), "fixture-user".into()).unwrap();
    let target = PromptScope { project: &project, host: "fixture-host", app: AppId::Claude };
    let store = PromptsStore::new(GlobalDatabase::for_home(home.path()).unwrap()).unwrap();
    let live = home.path().join("CLAUDE.md");
    std::fs::write(&live, b"untouched").unwrap();
    let path = sources.path().join("source.json");
    let write = |body: &str| std::fs::write(&path, serde_json::to_vec(&json!({"version":2,"prompts":{"claude":{"prompts":{"fixture":{"id":"fixture","name":"Rule","content":body,"enabled":true}}}}})).unwrap()).unwrap();
    write("body");
    let source = read_selected_source(&scope, sources.path(), &path).unwrap();
    let prepared = store.preview_data_migration(&scope, &source).unwrap();
    assert_eq!(store.commit_data_migration(&scope, &source, &prepared, &[], snapshots.path()).unwrap().outcome.unwrap(), 1);
    assert_eq!(store.list(target).unwrap().active_id.as_deref(), Some("fixture"));
    assert!(store.commit_data_migration(&scope, &source, &prepared, &[], snapshots.path()).unwrap().outcome.is_err());
    let prepared = store.preview_data_migration(&scope, &source).unwrap();
    assert_eq!(store.commit_data_migration(&scope, &source, &prepared, &[], snapshots.path()).unwrap().outcome.unwrap(), 0);
    write("replacement");
    let source = read_selected_source(&scope, sources.path(), &path).unwrap();
    let prepared = store.preview_data_migration(&scope, &source).unwrap();
    let id = prepared.preview().entries()[0].id;
    assert_eq!(store.commit_data_migration(&scope, &source, &prepared, &[(id, Decision::Skip)], snapshots.path()).unwrap().outcome.unwrap(), 0);
    assert_eq!(store.commit_data_migration(&scope, &source, &prepared, &[(id, Decision::Replace)], snapshots.path()).unwrap().outcome.unwrap(), 1);
    let catalog = store.list(target).unwrap();
    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].content, "replacement");
    assert_eq!(std::fs::read(live).unwrap(), b"untouched");
}
