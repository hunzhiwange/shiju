use std::{fs, io::ErrorKind, net::TcpListener, os::unix::fs::PermissionsExt, path::Path};

use vw_agent_apps::{
    apps::discovery::read::{self, LocalReadEnvironment},
    mcp::import::NativeMcpSource,
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
use vw_api_types::{
    TenantScope,
    agent_apps::{AppId, AppInstallation, AppInstallationId},
    agent_apps_auth::AuthService,
    mcp_management::{
        McpAppTarget, McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpCredentialRef,
        McpDistributionOutcome, McpFailureCode, McpNativeStatus,
    },
};

use super::{
    McpBindingTarget, McpCatalogStore, McpImportOutcome, McpImportSource,
    catalog_create_tests::{http_draft, with_database},
};
use crate::agent_apps::auth_accounts::AuthAccountStore;

fn private_root() -> std::io::Result<tempfile::TempDir> {
    let root = tempfile::tempdir()?;
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))?;
    Ok(root)
}

fn installed(
    home: &Path,
    app: AppId,
    executable: &str,
) -> anyhow::Result<(AppInstallation, McpAppTarget)> {
    let bin = home.join("bin");
    fs::create_dir_all(&bin)?;
    let path = bin.join(executable);
    fs::write(&path, "#!/bin/sh\nprintf '1.2.3\\n'\n")?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    let env = LocalReadEnvironment {
        host_id: "host".into(),
        home: home.to_owned(),
        search_paths: vec![bin],
        applications: home.join("Applications"),
    };
    let installation = read::read(app, &env)?.installations.remove(0);
    let target = McpAppTarget {
        app_id: app,
        installation_id: installation.id.clone(),
        target_host_id: installation.target_host_id.clone(),
    };
    Ok((installation, target))
}

#[test]
fn cc_mcp_t24_deleted_binding_revokes_native_entry() -> anyhow::Result<()> {
    with_database(|database| {
        let home = tempfile::tempdir()?;
        let private = private_root()?;
        let transactions =
            ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
                .unwrap();
        let directory = home.path().join(".codex");
        fs::create_dir(&directory)?;
        let path = directory.join("config.toml");
        let original = "[mcp_servers.managed]\ncommand = \"fixture\"\n[mcp_servers.other]\ncommand = \"other\"\n";
        fs::write(&path, original)?;
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let (installation, app) = installed(home.path(), AppId::Codex, "codex")?;
        let source = NativeMcpSource::HostHome(home.path());
        let target = McpBindingTarget {
            target: &app,
            installation: &installation,
            source,
            transactions: &transactions,
        };
        let reports = catalog
            .import_from_apps(&scope, &[McpImportSource { target: app.clone(), native: source }]);
        let Ok(McpImportOutcome::Added(record)) = &reports[0]
            .entries
            .as_ref()
            .unwrap()
            .iter()
            .find(|entry| entry.id.as_ref().is_some_and(|id| id.0 == "managed"))
            .unwrap()
            .outcome
        else {
            panic!("managed fixture must be imported");
        };
        assert_eq!(catalog.delete(&scope, &record.record).unwrap().len(), 1);

        let unrelated =
            McpAppTarget { installation_id: AppInstallationId("another".into()), ..app.clone() };
        let unrelated_target = McpBindingTarget { target: &unrelated, ..target };
        assert_eq!(
            catalog.revoke_app_binding(&scope, &record.record, &unrelated_target),
            Err(McpFailureCode::TargetUnavailable)
        );
        fs::write(&path, "[broken")?;
        let failed = catalog.revoke_app_binding(&scope, &record.record, &target).unwrap();
        assert_eq!(failed.outcome, Some(Err(McpFailureCode::InvalidDefinition)));
        assert_eq!(fs::read_to_string(&path)?, "[broken");

        let reopened = McpCatalogStore::new(database).unwrap();
        assert_eq!(reopened.delete(&scope, &record.record), Ok(vec![failed]));
        fs::write(&path, original)?;
        let cleaned = reopened.revoke_app_binding(&scope, &record.record, &target).unwrap();
        assert_eq!(cleaned.outcome, Some(Ok(())));
        let native = fs::read_to_string(&path)?;
        assert!(!native.contains("[mcp_servers.managed]"), "deleted binding still present");
        assert!(native.contains("[mcp_servers.other]"));

        fs::write(&path, original)?;
        assert_eq!(reopened.revoke_app_binding(&scope, &record.record, &target), Ok(cleaned));
        assert_eq!(
            fs::read_to_string(&path)?,
            original,
            "terminal cleanup must not remove a replacement"
        );
        Ok(())
    })
}

#[test]
fn cc_mcp_t24_binding_retries_conflict_and_updates_then_disables() -> anyhow::Result<()> {
    with_database(|database| {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let home = tempfile::tempdir()?;
        let private = private_root()?;
        let transactions =
            ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
                .unwrap();
        let accounts = AuthAccountStore::new(database.clone(), private.path()).unwrap();
        let (installation, app) = installed(home.path(), AppId::Codex, "codex")?;
        let target = McpBindingTarget {
            target: &app,
            installation: &installation,
            source: NativeMcpSource::HostHome(home.path()),
            transactions: &transactions,
        };
        let directory = home.path().join(".codex");
        fs::create_dir(&directory)?;
        let path = directory.join("config.toml");
        let other = "# sibling\n[mcp_servers.other]\ncommand = \"other\"\n";
        let external = format!("{other}[mcp_servers.managed]\ncommand = \"external\"\n");
        fs::write(&path, &external)?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}/mcp", listener.local_addr()?);
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let record = catalog.create(&scope, http_draft("managed", &url)).unwrap();
        let bind = |catalog: &McpCatalogStore, expected, enabled| {
            runtime.block_on(catalog.set_app_binding(
                &scope,
                expected,
                &target,
                enabled,
                &accounts,
                |_, _| None,
            ))
        };

        let rejected = bind(&catalog, &record.record, true).unwrap();
        assert!(rejected.desired_committed);
        assert!(rejected.enabled);
        assert_eq!(rejected.record.revision, record.record.revision + 1);
        assert_eq!(
            rejected.native_outcome,
            McpDistributionOutcome::Failed { code: McpFailureCode::Conflict }
        );
        assert_eq!(fs::read_to_string(&path)?, external);
        let catalog = McpCatalogStore::new(database).unwrap();
        assert_eq!(bind(&catalog, &record.record, true), Err(McpFailureCode::StaleRevision));
        fs::write(&path, other)?;
        let applied = bind(&catalog, &rejected.record, true).unwrap();
        assert_eq!(applied.record, rejected.record);
        assert_eq!(applied.native_outcome, McpDistributionOutcome::Applied);
        let native = fs::read_to_string(&path)?;
        assert!(native.contains(&url));
        assert!(native.contains(other));

        let marker = home.path().join("executed");
        let updated = catalog
            .update_connection(
                &scope,
                &applied.record,
                McpConnectionDefinition::Stdio {
                    command: "/bin/sh".into(),
                    args: vec![
                        "-c".into(),
                        "touch \"$1\"".into(),
                        "fixture".into(),
                        marker.display().to_string(),
                    ],
                    env: Default::default(),
                    cwd: None,
                },
            )
            .unwrap();
        let applied = bind(&catalog, &updated.record, true).unwrap();
        assert_eq!(applied.native_outcome, McpDistributionOutcome::Applied);
        let native = fs::read_to_string(&path)?;
        assert!(native.contains("/bin/sh"));
        assert!(!native.contains(&url));
        let disabled = bind(&catalog, &applied.record, false).unwrap();
        assert!(disabled.desired_committed);
        assert!(!disabled.enabled);
        assert_eq!(disabled.native_outcome, McpDistributionOutcome::Applied);
        assert_eq!(fs::read_to_string(&path)?, other);
        assert!(!marker.exists());
        assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);

        fs::write(&path, &external)?;
        let retired = catalog.delete(&scope, &disabled.record).unwrap();
        assert_eq!(retired[0].binding.native_status, McpNativeStatus::Missing);
        assert_eq!(
            catalog.revoke_app_binding(&scope, &disabled.record, &target).unwrap().outcome,
            Some(Ok(()))
        );
        assert_eq!(fs::read_to_string(&path)?, external);
        Ok(())
    })
}

#[test]
fn cc_mcp_t24_binding_requires_credential_permission_before_sse_projection() -> anyhow::Result<()> {
    use vw_agent_apps::auth::credential_file::{
        AccountCredential, CredentialFileStore, XaiCredential,
    };
    use vw_api_types::TimestampMs;

    use super::catalog_create::catalog_location;
    use crate::{
        agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope},
        gateway::settings_store::GatewaySettingsStore,
    };

    const ACCESS: &str = "fixture-access-value";
    const REFRESH: &str = "fixture-refresh-value";

    with_database(|database| {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let home = tempfile::tempdir()?;
        let private = private_root()?;
        let transactions =
            ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
                .unwrap();
        let wrong_host =
            ProviderTransactions::new(private.path(), ExecutionHostId::new("wrong").unwrap())
                .unwrap();
        let accounts = AuthAccountStore::new(database.clone(), private.path()).unwrap();
        let (installation, app) = installed(home.path(), AppId::Gemini, "gemini")?;
        let target = McpBindingTarget {
            target: &app,
            installation: &installation,
            source: NativeMcpSource::HostHome(home.path()),
            transactions: &transactions,
        };
        let path = home.path().join(".gemini/settings.json");
        fs::create_dir(path.parent().unwrap())?;
        fs::write(&path, "{}")?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}/events", listener.local_addr()?);
        let tenant = TenantScope::new("platform", "company")?;
        let scope = McpCatalogScope::Tenant { scope: tenant.clone() };
        let owner = AuthAccountScope {
            tenant: &tenant,
            principal: "oidc:owner",
            host: "host",
            service: AuthService::XaiOauth,
        };
        accounts
            .insert_metadata(
                &owner,
                "fixture",
                &AuthAccountMetadata::new("fixture-identity", TimestampMs(1)).unwrap(),
            )
            .unwrap();
        CredentialFileStore::new(private.path())
            .unwrap()
            .lease_xai(owner, "fixture", true)
            .unwrap()
            .save(&AccountCredential::XaiOauth(XaiCredential {
                access_token: ACCESS.into(),
                refresh_token: REFRESH.into(),
                expires_at: TimestampMs(i64::MAX),
                requires_reauth: false,
            }))
            .unwrap();
        let storage = GatewaySettingsStore::new(database.clone())?;
        let catalog = McpCatalogStore::new(database).unwrap();
        let reference = McpCredentialRef { service: owner.service, account_id: "fixture".into() };
        let mut draft = http_draft("events", &url);
        draft.connection = McpConnectionDefinition::Sse {
            url: url.clone(),
            headers: [(
                "Authorization".into(),
                McpConfigValue::Credential { reference: reference.clone() },
            )]
            .into(),
        };
        let record = catalog.create(&scope, draft).unwrap();
        let unauthorized_target = McpBindingTarget { transactions: &wrong_host, ..target };
        let bind = |expected, target| {
            runtime.block_on(catalog.set_app_binding(
                &scope,
                expected,
                target,
                true,
                &accounts,
                |_, _| None,
            ))
        };
        assert_eq!(
            bind(&record.record, &unauthorized_target),
            Err(McpFailureCode::PermissionDenied)
        );
        assert_eq!(catalog.list(&scope).unwrap()[0].record, record.record);
        let failure = bind(&record.record, &target).unwrap();
        assert!(failure.desired_committed);
        assert_eq!(
            failure.native_outcome,
            McpDistributionOutcome::Failed { code: McpFailureCode::PermissionDenied }
        );
        assert_eq!(fs::read_to_string(&path)?, "{}");

        let applied = runtime
            .block_on(catalog.set_app_binding(
                &scope,
                &failure.record,
                &target,
                true,
                &accounts,
                |_, candidate| (candidate == &reference).then_some(owner),
            ))
            .unwrap();
        assert_eq!(applied.record, failure.record);
        assert_eq!(applied.native_outcome, McpDistributionOutcome::Applied);
        let native = fs::read_to_string(path)?;
        let document: serde_json::Value = serde_json::from_str(&native)?;
        assert_eq!(document["mcpServers"]["events"]["url"], url);
        assert_eq!(
            document["mcpServers"]["events"]["headers"]["authorization"],
            format!("Bearer {ACCESS}")
        );
        assert!(!native.contains(REFRESH));
        let (_, namespace) = catalog_location(&scope).unwrap();
        let stored = storage.get(&tenant, &namespace, "events")?.unwrap();
        let outcome = serde_json::to_string(&applied)?;
        for text in [stored, outcome] {
            assert!(!text.contains(ACCESS));
            assert!(!text.contains(REFRESH));
        }
        assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
        Ok(())
    })
}

#[test]
fn cc_mcp_t24_imported_disabled_binding_is_reenabled_with_the_current_definition()
-> anyhow::Result<()> {
    with_database(|database| {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let home = tempfile::tempdir()?;
        let private = private_root()?;
        let transactions =
            ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
                .unwrap();
        let accounts = AuthAccountStore::new(database.clone(), private.path()).unwrap();
        let catalog = McpCatalogStore::new(database).unwrap();
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let (installation, app) = installed(home.path(), AppId::OpenCode, "opencode")?;
        let source = NativeMcpSource::HostHome(home.path());
        let target = McpBindingTarget {
            target: &app,
            installation: &installation,
            source,
            transactions: &transactions,
        };
        let path = home.path().join(".config/opencode/opencode.jsonc");
        fs::create_dir_all(path.parent().unwrap())?;
        let before =
            r#"{"mcp":{"managed":{"type":"local","command":["fixture"],"enabled":false}}}"#;
        fs::write(&path, before)?;
        let report = catalog
            .import_from_apps(&scope, &[McpImportSource { target: app.clone(), native: source }]);
        let Ok(McpImportOutcome::Added(record)) = &report[0].entries.as_ref().unwrap()[0].outcome
        else {
            panic!("disabled fixture must be imported");
        };
        let edited = catalog
            .update_connection(
                &scope,
                &record.record,
                McpConnectionDefinition::Stdio {
                    command: "replacement".into(),
                    args: vec!["argument".into()],
                    env: Default::default(),
                    cwd: None,
                },
            )
            .unwrap();
        let applied = runtime
            .block_on(catalog.set_app_binding(
                &scope,
                &edited.record,
                &target,
                true,
                &accounts,
                |_, _| None,
            ))
            .unwrap();
        assert_eq!(applied.native_outcome, McpDistributionOutcome::Applied);
        let native: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        assert_eq!(
            native["mcp"]["managed"]["command"],
            serde_json::json!(["replacement", "argument"])
        );
        assert_eq!(native["mcp"]["managed"]["enabled"], true);

        let disabled = runtime
            .block_on(catalog.set_app_binding(
                &scope,
                &applied.record,
                &target,
                false,
                &accounts,
                |_, _| None,
            ))
            .unwrap();
        assert_eq!(disabled.native_outcome, McpDistributionOutcome::Applied);
        let native: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        assert_eq!(native["mcp"]["managed"]["enabled"], false);
        let deleted = catalog.delete(&scope, &disabled.record).unwrap();
        assert_eq!(deleted[0].binding.native_status, McpNativeStatus::Present);
        let revoked = catalog.revoke_app_binding(&scope, &disabled.record, &target).unwrap();
        assert_eq!(revoked.outcome, Some(Ok(())));
        let native: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        assert!(native["mcp"].get("managed").is_none());
        Ok(())
    })
}
