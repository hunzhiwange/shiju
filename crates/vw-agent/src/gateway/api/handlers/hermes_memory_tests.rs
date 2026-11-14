use super::*;

#[test]
fn hermes_api_isolates_memory_denies_without_project_grant() {
    let scope = Scope { project_id: "project".into(), target_host_id: "remote".into() };
    for write in [false, true] {
        assert!(matches!(authorize(&Policy(None), TenantScope::new("p", "c").unwrap(), &HeaderMap::new(), &scope, write), Err(StatusCode::UNAUTHORIZED)));
    }
}

#[test]
fn hermes_api_isolates_memory_model_omits_credentials() {
    let snapshot = hermes::model_snapshot(Some(b"model:\n  default: test\n  provider: custom\n  api_key: secret\n  base_url: https://secret@example.com\n")).unwrap().unwrap();
    let json = serde_json::to_value(snapshot).unwrap();
    assert_eq!(json, serde_json::json!({"default":"test", "provider":"custom"}));
    assert_eq!(hermes::model_snapshot(None).unwrap(), None);
    assert!(hermes::model_snapshot(Some(b"model: []")).is_err());
}

#[test]
fn hermes_api_isolates_memory_tenant_host_and_native_files() {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let policy_path = dir.path().join("access.json");
    let token = "a".repeat(64);
    let policy = serde_json::json!({
        "principals": [{"id":"editor", "token_sha256":hex::encode(Sha256::digest(token.as_bytes())), "grants":[{
            "platform_id":"p", "company_id":"c", "host_id":"local",
            "hermes_memory_read":["project"], "hermes_memory_write":["project"]
        }]}],
        "hosts":[{"id":"local", "online":true, "binding":"local", "home":dir.path(), "search_paths":[], "applications":dir.path()}]
    });
    std::fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    std::fs::set_permissions(&policy_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let p = Policy(Some(policy_path));
    let mut headers = HeaderMap::new();
    headers.insert("x-agent-apps-token", token.parse().unwrap());
    let mut scope = Scope { project_id:"project".into(), target_host_id:"local".into() };
    let tenant = TenantScope::new("p", "c").unwrap();
    let host = authorize(&p, tenant.clone(), &headers, &scope, true).unwrap();
    assert!(authorize(&p, TenantScope::new("p", "other").unwrap(), &headers, &scope, false).is_err());
    scope.target_host_id = "other".into();
    assert!(authorize(&p, tenant.clone(), &headers, &scope, false).is_err());
    scope.target_host_id = "local".into();
    scope.project_id = "other".into();
    assert!(authorize(&p, tenant, &headers, &scope, true).is_err());
    let root = dir.path().join(".hermes");
    let memory = vw_agent_apps::hermes_memory::HermesMemory::open("local", &host.id, &host.home, &root, true).unwrap();
    memory.write_memory(HermesMemoryKind::Memory, "中文记忆").unwrap();
    memory.write_memory(HermesMemoryKind::User, "用户").unwrap();
    assert_eq!(memory.read_memory(HermesMemoryKind::Memory).unwrap().content, "中文记忆");
    assert_eq!(memory.read_memory(HermesMemoryKind::User).unwrap().content, "用户");
    let yaml = vw_agent_apps::hermes_memory_limits::prepare_memory_enabled(Some(b"memory:\n  memory_char_limit: 99\n"), HermesMemoryKind::User, false).unwrap();
    let limits = vw_agent_apps::hermes_memory_limits::read_memory_limits(Some(&yaml)).unwrap();
    assert_eq!(limits.memory, 99);
    assert!(!limits.user_enabled);
    assert!(limits.memory_enabled);
}
