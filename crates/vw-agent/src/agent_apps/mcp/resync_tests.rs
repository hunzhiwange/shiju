use std::{fs, io::ErrorKind, net::TcpListener};

use vw_agent_apps::{
    apps::lifecycle::operations::{Operation, OperationFailure},
    mcp::import::NativeMcpSource,
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    mcp_management::{
        McpBatchOutcome, McpCatalogScope, McpConnectionDefinition, McpDistributionOutcome,
        McpFailureCode, McpRecordRef,
    },
    task::TaskStatus,
};

use super::{
    McpBindingTarget, McpCatalogStore, McpResyncTargetResult, McpResyncTask,
    binding_tests::{installed, private_root},
    catalog_create_tests::{http_draft, with_database},
    resync_targets,
};
use crate::agent_apps::auth_accounts::AuthAccountStore;

#[test]
fn cc_mcp_t26_resync_restores_deleted_managed_entry() -> anyhow::Result<()> {
    with_database(|database| {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let home = tempfile::tempdir()?;
        let private = private_root()?;
        let transactions =
            ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
                .unwrap();
        let accounts = AuthAccountStore::new(database.clone(), private.path()).unwrap();
        let (codex, codex_app) = installed(home.path(), AppId::Codex, "codex")?;
        let (gemini, gemini_app) = installed(home.path(), AppId::Gemini, "gemini")?;
        let (kimi, kimi_app) = installed(home.path(), AppId::Kimi, "kimi")?;
        let source = NativeMcpSource::HostHome(home.path());
        let path = home.path().join(".codex/config.toml");
        let broken = home.path().join(".gemini/settings.json");
        let excluded = home.path().join(".kimi-code/mcp.json");
        for file in [&path, &broken, &excluded] {
            fs::create_dir(file.parent().unwrap())?;
        }
        let external = "# external\n[mcp_servers.other]\ncommand = \"other\"\n";
        fs::write(&path, external)?;
        fs::write(&broken, "{}")?;
        fs::write(&excluded, "{}")?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}/mcp", listener.local_addr()?);
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database).unwrap();
        let bind = |record: &McpRecordRef, target: &McpBindingTarget<'_>| {
            runtime
                .block_on(catalog.set_app_binding(
                    &scope,
                    record,
                    target,
                    true,
                    &accounts,
                    |_, _| None,
                ))
                .unwrap()
        };
        let mut record = catalog.create(&scope, http_draft("managed", &url)).unwrap().record;
        for (installation, target) in
            [(&codex, &codex_app), (&gemini, &gemini_app), (&kimi, &kimi_app)]
        {
            let target =
                McpBindingTarget { target, installation, source, transactions: &transactions };
            let initial = bind(&record, &target);
            assert_eq!(
                initial.native_outcome,
                McpDistributionOutcome::Applied,
                "initial binding for {:?}",
                target.target.app_id
            );
            record = initial.record;
        }
        let gemini_target = McpBindingTarget {
            target: &gemini_app,
            installation: &gemini,
            source,
            transactions: &transactions,
        };
        let mut sse = http_draft("events", &url);
        sse.connection =
            McpConnectionDefinition::Sse { url: url.clone(), headers: Default::default() };
        assert_eq!(
            bind(&catalog.create(&scope, sse).unwrap().record, &gemini_target).native_outcome,
            McpDistributionOutcome::Applied
        );
        let codex_target = McpBindingTarget {
            target: &codex_app,
            installation: &codex,
            source,
            transactions: &transactions,
        };
        let marker = home.path().join("executed");
        let mut draft = http_draft("stdio", &url);
        draft.connection = McpConnectionDefinition::Stdio {
            command: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "touch \"$1\"".into(),
                "fixture".into(),
                marker.display().to_string(),
            ],
            env: Default::default(),
            cwd: None,
        };
        let stdio = bind(&catalog.create(&scope, draft).unwrap().record, &codex_target);
        assert_eq!(stdio.native_outcome, McpDistributionOutcome::Applied);
        let unowned =
            bind(&catalog.create(&scope, http_draft("other", &url)).unwrap().record, &codex_target);
        assert_eq!(
            unowned.native_outcome,
            McpDistributionOutcome::Failed { code: McpFailureCode::Conflict }
        );

        let disabled = bind(
            &catalog.create(&scope, http_draft("disabled", &url)).unwrap().record,
            &codex_target,
        );
        assert_eq!(disabled.native_outcome, McpDistributionOutcome::Applied);
        fs::write(&path, "[broken")?;
        let disabled = runtime
            .block_on(catalog.set_app_binding(
                &scope,
                &disabled.record,
                &codex_target,
                false,
                &accounts,
                |_, _| None,
            ))
            .unwrap();
        assert_eq!(
            disabled.native_outcome,
            McpDistributionOutcome::Failed { code: McpFailureCode::InvalidDefinition }
        );

        fs::write(
            &path,
            format!(
                "{external}[mcp_servers.stdio]\ncommand = \"drift\"\n[mcp_servers.disabled]\nurl = \"{url}\"\n"
            ),
        )?;
        fs::write(&broken, "{")?;
        fs::write(&excluded, "{}")?;

        let operation = Operation::new();
        let mut resolved = Vec::new();
        let mut progress = Vec::new();
        let result = runtime
            .block_on(catalog.resync_apps(
                &scope,
                &["gemini".into(), "codex".into(), "gemini".into()],
                |app| {
                    resolved.push(app);
                    let (installation, target) = match app {
                        AppId::Codex => (&codex, &codex_app),
                        AppId::Gemini => (&gemini, &gemini_app),
                        _ => panic!("unselected app must not be resolved"),
                    };
                    Ok(McpBindingTarget {
                        target,
                        installation,
                        source,
                        transactions: &transactions,
                    })
                },
                &accounts,
                |_, _, _| None,
                McpResyncTask {
                    operation: &operation,
                    on_target: |report: &McpResyncTargetResult, completed, total| {
                        progress.push((report.app_id, completed, total));
                    },
                },
            ))
            .unwrap();
        let native = fs::read_to_string(&path)?;
        assert!(
            native.contains("[mcp_servers.managed]"),
            "resync must restore a deleted managed entry"
        );
        assert!(native.contains(&url));
        assert!(native.contains(external));
        assert!(native.contains("command = \"/bin/sh\""));
        assert!(!native.contains("command = \"drift\""));
        assert!(!native.contains("mcp_servers.disabled"));
        assert_eq!(fs::read_to_string(&broken)?, "{");
        assert_eq!(fs::read_to_string(&excluded)?, "{}");
        assert!(!marker.exists());
        assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);

        assert_eq!(resolved, [AppId::Gemini, AppId::Codex]);
        assert_eq!(progress, [(AppId::Gemini, 1, 2), (AppId::Codex, 2, 2)]);
        assert_eq!(result.outcome, McpBatchOutcome::PartiallySucceeded);
        assert_eq!(result.targets[0].app_id, AppId::Gemini);
        let failed = result.targets[0].distribution.as_ref().unwrap();
        assert_eq!(failed.outcome, McpBatchOutcome::Failed);
        assert_eq!(
            failed.items[0].native_outcome,
            McpDistributionOutcome::Failed { code: McpFailureCode::InvalidDefinition }
        );
        assert_eq!(result.targets[1].app_id, AppId::Codex);
        let repaired = result.targets[1].distribution.as_ref().unwrap();
        assert_eq!(repaired.outcome, McpBatchOutcome::PartiallySucceeded);
        assert_eq!(repaired.items.len(), 4);
        for saved in [record, stdio.record, unowned.record, disabled.record] {
            let item = repaired.items.iter().find(|item| item.record.id == saved.id).unwrap();
            assert_eq!(item.record, saved);
            assert_eq!(item.enabled, saved.id.0 != "disabled");
            assert_eq!(
                item.native_outcome,
                if saved.id.0 == "other" {
                    McpDistributionOutcome::Failed { code: McpFailureCode::Conflict }
                } else {
                    McpDistributionOutcome::Applied
                }
            );
        }
        Ok(())
    })
}

#[test]
fn cc_mcp_t26_resync_defaults_and_rejects_invalid_targets() {
    let all = resync_targets(&[]).unwrap();
    assert_eq!(all.len(), 12);
    assert!(!all.contains(&AppId::ClaudeDesktop));
    assert!(!all.contains(&AppId::OpenClaw));
    assert_eq!(
        resync_targets(&["codex".into(), "unknown".into()]),
        Err(McpFailureCode::InvalidDefinition)
    );
    for unsupported in ["claude-desktop", "openclaw"] {
        assert_eq!(resync_targets(&[unsupported.into()]), Err(McpFailureCode::TargetUnavailable));
    }
}

#[test]
fn cc_mcp_t26_resync_cancel_keeps_completed_writes() -> anyhow::Result<()> {
    with_database(|database| {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let home = tempfile::tempdir()?;
        let private = private_root()?;
        let transactions =
            ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
                .unwrap();
        let accounts = AuthAccountStore::new(database.clone(), private.path()).unwrap();
        let (installation, app) = installed(home.path(), AppId::Codex, "codex")?;
        let path = home.path().join(".codex/config.toml");
        fs::create_dir(path.parent().unwrap())?;
        fs::write(&path, "")?;
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database).unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let url = format!("http://{}/mcp", listener.local_addr()?);
        let record = catalog.create(&scope, http_draft("managed", &url)).unwrap().record;
        runtime.block_on(async {
            let operation = Operation::new();
            let worker = operation.clone();
            let (send, receive) = tokio::sync::oneshot::channel();
            let readback = path.clone();
            operation
                .start_task(async move {
                    let source = NativeMcpSource::HostHome(home.path());
                    let target = McpBindingTarget {
                        target: &app,
                        installation: &installation,
                        source,
                        transactions: &transactions,
                    };
                    let bound = catalog
                        .set_app_binding(&scope, &record, &target, true, &accounts, |_, _| None)
                        .await
                        .unwrap();
                    assert_eq!(bound.native_outcome, McpDistributionOutcome::Applied);
                    fs::write(&path, "").unwrap();
                    let result = catalog
                        .resync_apps(
                            &scope,
                            &["codex".into(), "gemini".into()],
                            |id| {
                                assert_eq!(
                                    id,
                                    AppId::Codex,
                                    "cancellation must skip subsequent targets"
                                );
                                Ok(McpBindingTarget { ..target })
                            },
                            &accounts,
                            |_, _, _| None,
                            McpResyncTask {
                                operation: &worker,
                                on_target: |_: &McpResyncTargetResult, completed, total| {
                                    assert_eq!((completed, total), (1, 2));
                                    assert_eq!(worker.snapshot().status, TaskStatus::Running);
                                    worker.cancel();
                                },
                            },
                        )
                        .await
                        .unwrap();
                    assert!(fs::read_to_string(&readback).unwrap().contains(&url));
                    let cancelled = result.outcome == McpBatchOutcome::Cancelled;
                    send.send(result).unwrap();
                    if cancelled { Err(OperationFailure::Cancelled) } else { Ok(()) }
                })
                .unwrap();
            let result = receive.await?;
            assert_eq!(result.outcome, McpBatchOutcome::Cancelled);
            assert_eq!(result.targets.len(), 2);
            assert_eq!(
                result.targets[0].distribution.as_ref().unwrap().outcome,
                McpBatchOutcome::Succeeded
            );
            let skipped = &result.targets[1];
            assert_eq!(skipped.app_id, AppId::Gemini);
            let skipped = skipped.distribution.as_ref().unwrap();
            assert_eq!(skipped.outcome, McpBatchOutcome::Cancelled);
            assert!(skipped.items.is_empty());
            assert_eq!(operation.wait().await.status, TaskStatus::Cancelled);
            Ok(())
        })
    })
}
