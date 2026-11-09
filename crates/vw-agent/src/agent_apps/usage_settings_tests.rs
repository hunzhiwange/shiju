use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

use super::usage_settings::{
    UsageDisplay, UsageSettings, UsageSettingsError, UsageSettingsScope, UsageSettingsStore,
};
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

fn saved_settings() -> Value {
    json!({
        "template": "token_plan",
        "enabled": true,
        "refreshIntervalMinutes": 15,
        "display": "used",
        "credentials": {
            "signing": {
                "accessKeyIdRef": "a0a0a0a0-0000-4000-8000-000000000001",
                "secretAccessKeyRef": "a0a0a0a0-0000-4000-8000-000000000002"
            }
        }
    })
}

fn scope(project: &ProjectScope) -> UsageSettingsScope<'_> {
    UsageSettingsScope {
        project,
        principal: "user-a",
        target_host: "host-a",
        app: AppId::Claude,
        provider: "provider-a",
        account: Some("account-a"),
    }
}

fn verify_round_trip(database: GlobalDatabase) -> anyhow::Result<()> {
    let project =
        ProjectScope::new(TenantScope::new("platform-a", "company-a")?, "project-a".into())?;
    let other_platform =
        ProjectScope::new(TenantScope::new("platform-b", "company-a")?, "project-a".into())?;
    let other_company =
        ProjectScope::new(TenantScope::new("platform-a", "company-b")?, "project-a".into())?;
    let other_project = ProjectScope::new(project.tenant_scope().clone(), "project-b".into())?;
    let key = scope(&project);
    let store = UsageSettingsStore::new(database.clone())?;
    assert_eq!(store.get(key)?, None);

    let input = saved_settings();
    let settings: UsageSettings = serde_json::from_value(input.clone())?;
    store.save(key, &settings)?;
    let store = UsageSettingsStore::new(database.clone())?;
    assert_eq!(store.get(key)?, Some(settings.clone()));
    let rows = GatewaySettingsStore::new(database)?
        .list(project.tenant_scope(), "agent_apps.usage_settings.v1")?;
    assert_eq!(rows.len(), 1);
    let payload: Value = serde_json::from_str(&rows[0].payload_json)?;
    assert_eq!(payload["credentials"], input["credentials"]);

    for isolated in [
        scope(&other_platform),
        scope(&other_company),
        scope(&other_project),
        UsageSettingsScope { target_host: "host-b", ..key },
        UsageSettingsScope { principal: "user-b", ..key },
        UsageSettingsScope { app: AppId::Codex, ..key },
        UsageSettingsScope { provider: "provider-b", ..key },
        UsageSettingsScope { account: Some("account-b"), ..key },
        UsageSettingsScope { account: None, ..key },
    ] {
        assert_eq!(store.get(isolated)?, None);
    }
    let updated: UsageSettings = serde_json::from_value(json!({
        "template": "general",
        "enabled": false,
        "refreshIntervalMinutes": 0
    }))?;
    assert_eq!(updated.display, UsageDisplay::Left);
    let other_host = UsageSettingsScope { target_host: "host-b", ..key };
    store.save(other_host, &updated)?;
    assert_eq!(store.get(other_host)?, Some(updated.clone()));
    assert_eq!(store.get(key)?, Some(settings));
    store.save(key, &updated)?;
    assert_eq!(store.get(key)?, Some(updated));
    Ok(())
}

#[test]
fn usage_usage_settings_returns_expected_result() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    verify_round_trip(GlobalDatabase::for_home(home.path())?)
}

#[test]
fn usage_usage_settings_postgres_returns_expected_result() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    verify_round_trip(fixture.database().clone())
}

#[test]
fn usage_usage_settings_rejects_invalid_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let store = UsageSettingsStore::new(GlobalDatabase::for_home(home.path())?)?;
    let project = ProjectScope::new(TenantScope::default(), "project-a".into())?;
    let key = scope(&project);
    let original = serde_json::from_value(saved_settings())?;
    store.save(key, &original)?;

    let mut invalid = saved_settings();
    invalid["refreshIntervalMinutes"] = json!(1441);
    assert_eq!(
        store.save(key, &serde_json::from_value(invalid)?),
        Err(UsageSettingsError::InvalidRefreshInterval)
    );
    let mut invalid = saved_settings();
    invalid["credentials"]["signing"]["secretAccessKey"] = json!("raw-secret-fixture");
    assert!(serde_json::from_value::<UsageSettings>(invalid).is_err());
    let mut invalid = saved_settings();
    invalid["credentials"]["signing"]["accessKeyIdRef"] = json!("raw-access-key-fixture");
    let error = serde_json::from_value::<UsageSettings>(invalid).unwrap_err();
    assert!(!error.to_string().contains("raw-access-key-fixture"));
    assert_eq!(
        store.save(UsageSettingsScope { target_host: " ", ..key }, &original),
        Err(UsageSettingsError::InvalidScope)
    );
    assert_eq!(store.get(key)?, Some(original));
    Ok(())
}
