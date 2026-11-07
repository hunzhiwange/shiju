use super::access::import_provider_policy;
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use vw_api_types::{agent_apps::AppId, id::TenantScope};
#[test]
fn cc04_import_startup_consumes_only_authorized_empty_catalogs() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::create_dir(root.path().join(".gemini")).unwrap();
    let native = root.path().join(".gemini/.env");
    std::fs::write(&native, b"GEMINI_API_KEY=fake-key\nGEMINI_MODEL=fixture\n").unwrap();
    let policy_path = root.path().join("access.json");
    let mut policy = json!({"principals":[{"id":"fixture","token_sha256":"a".repeat(64),"grants":[{"platform_id":"p","company_id":"a","host_id":"h","provider_config_read":true,"provider_config_write":false}]}],"hosts":[{"id":"h","online":true,"binding":"local","home":root.path(),"applications":root.path(),"search_paths":[]}]});
    std::fs::write(&policy_path, policy.to_string()).unwrap();
    std::fs::set_permissions(&policy_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let store = AppProviderStore::new(db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    import_provider_policy(&policy_path, &db).unwrap();
    assert!(store.list(&tenant, AppId::Gemini).unwrap().is_empty());
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(true);
    std::fs::write(&policy_path, policy.to_string()).unwrap();
    import_provider_policy(&policy_path, &db).unwrap();
    assert_eq!(store.list(&tenant, AppId::Gemini).unwrap().len(), 1);
    let before = AppProviderStore::preview_record(&db, &tenant, AppId::Gemini, "default").unwrap();
    std::fs::write(&native, b"GEMINI_API_KEY=fake-replacement\n").unwrap();
    import_provider_policy(&policy_path, &db).unwrap();
    assert_eq!(
        AppProviderStore::preview_record(&db, &tenant, AppId::Gemini, "default").unwrap(),
        before
    );
    assert!(!root.path().join(".gemini/settings.json").exists());
    assert!(!root.path().join("provider-live").exists());
    assert!(store.list(&TenantScope::new("p", "b").unwrap(), AppId::Gemini).unwrap().is_empty());
}
