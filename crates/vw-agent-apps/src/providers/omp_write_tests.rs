use super::*;
use crate::providers::live::commit::{ExecutionHostId, ProviderTransactions};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::TenantScope;

const BEFORE: &[u8] =
    b"# directory\r\nproviders:  null    # user comment\r\n\r\nother: {literal: true}\r\n";
const AFTER: &[u8] = b"# directory\r\nproviders:      # user comment\r\n  t38-local:\n    api: openai-completions\n    apiKey: fixture\n    baseUrl: http://127.0.0.1:1/v1\n    models:\n    - id: t38-work-model\n\r\nother: {literal: true}\r\n";

#[test]
fn aggregation_t38_omp_write_preserves_contract() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let agent = home.path().join(".omp/profiles/work/agent");
    let default_agent = home.path().join(".omp/agent");
    fs::create_dir_all(&agent).unwrap();
    fs::create_dir_all(&default_agent).unwrap();
    let models = agent.join("models.yml");
    let default_models = default_agent.join("models.yml");
    let default_before = b"providers: {unrelated: {apiKey: fixture}}\n";
    let settings = b"modelRoles:\n  default: native/model\n";
    let login = b"fixture login data";
    fs::write(&models, BEFORE).unwrap();
    fs::write(&default_models, default_before).unwrap();
    fs::write(agent.join("config.yml"), settings).unwrap();
    fs::write(agent.join("agent.db"), login).unwrap();
    fs::write(agent.join("models.yaml"), b"# unused fallback\n").unwrap();

    let location =
        resolve_location(home.path(), None, None, Some("work"), Some("other"), None).unwrap();
    let mut files = OmpFiles::open_at(home.path(), location).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("project", "actor").unwrap();
    let (name, pre) = files.snapshot().unwrap();
    assert_eq!(name, "models.yml");
    assert_eq!(pre.as_deref(), Some(BEFORE));
    let next = patch(
        pre.as_deref(),
        "t38-local",
        Some(&json!({
            "api": "openai-completions",
            "apiKey": "fixture",
            "baseUrl": "http://127.0.0.1:1/v1",
            "models": [{"id": "t38-work-model"}]
        })),
        OmpActivation::Add,
    )
    .unwrap();
    files
        .apply(&tx, &tenant, &tx.state(AppId::Omp).unwrap(), name, pre.as_deref(), next, || true)
        .unwrap();
    assert_eq!(fs::read(&models).unwrap(), AFTER);

    let next = patch(
        Some(AFTER),
        "t38-local",
        Some(&json!({"apiKey": "replacement"})),
        OmpActivation::IfMember,
    )
    .unwrap();
    let expected = tx.state(AppId::Omp).unwrap();
    let other_tenant = TenantScope::new("project", "other").unwrap();
    assert_eq!(
        files.apply(&tx, &other_tenant, &expected, name, Some(AFTER), next.clone(), || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(
        files.apply(&tx, &tenant, &expected, name, pre.as_deref(), next.clone(), || true),
        Err(WriteError::Conflict)
    );
    assert_eq!(fs::read(&models).unwrap(), AFTER);
    assert!(tx.pending(AppId::Omp).unwrap().is_none());

    files.apply(&tx, &tenant, &expected, name, Some(AFTER), next, || true).unwrap();
    assert_eq!(
        fs::read(&models).unwrap(),
        b"# directory\r\nproviders:      # user comment\r\n  t38-local:\n    apiKey: replacement\n\r\nother: {literal: true}\r\n"
    );
    assert_eq!(fs::metadata(&models).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(fs::read(default_models).unwrap(), default_before);
    assert_eq!(fs::read(agent.join("config.yml")).unwrap(), settings);
    assert_eq!(fs::read(agent.join("agent.db")).unwrap(), login);
    assert_eq!(fs::read(agent.join("models.yaml")).unwrap(), b"# unused fallback\n");
}

#[test]
fn aggregation_t38_omp_write_refuses_ambiguous_yaml() {
    let home = tempfile::tempdir().unwrap();
    let location = resolve_location(home.path(), None, None, None, None, None).unwrap();
    let files = OmpFiles::open_at(home.path(), location).unwrap();
    let models = home.path().join(".omp/agent/models.yml");
    for before in [
        "providers: {p: {apiKey: fixture}}\n",
        "providers:\n  p: &shared\n    apiKey: fixture\n",
        "providers:\n  p: !custom {apiKey: fixture}\n",
        "providers:\n  p:\n    apiKey: fixture\n# keep global comment\n    name: Model\n",
    ] {
        fs::write(&models, before).unwrap();
        let (_, pre) = files.snapshot().unwrap();
        assert_eq!(
            patch(
                pre.as_deref(),
                "p",
                Some(&json!({"apiKey": "replacement"})),
                OmpActivation::IfMember
            ),
            Err(WriteError::InvalidResource)
        );
        assert_eq!(fs::read(&models).unwrap(), before.as_bytes());
    }
    assert_eq!(
        patch(
            Some(b"providers: null"),
            "p",
            Some(&json!({"apiKey": "fixture"})),
            OmpActivation::Add,
        )
        .unwrap(),
        b"providers: \n  p:\n    apiKey: fixture\n"
    );
}
