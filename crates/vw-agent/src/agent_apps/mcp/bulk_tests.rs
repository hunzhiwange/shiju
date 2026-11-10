use std::{fs, io::ErrorKind, net::TcpListener, os::unix::fs::MetadataExt};

use vw_agent_apps::{
    mcp::import::NativeMcpSource,
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    mcp_management::{
        McpBatchOutcome, McpCatalogScope, McpConnectionDefinition, McpDistributionOutcome,
        McpFailureCode, McpRecordId, McpRecordRef,
    },
};

use super::{
    McpBindingTarget, McpCatalogStore,
    binding_tests::{installed, private_root},
    catalog_create_tests::{http_draft, with_database},
};
use crate::agent_apps::auth_accounts::AuthAccountStore;

#[test]
fn cc_mcp_t25_bulk_keeps_siblings_and_reports_conflicts_in_order() -> anyhow::Result<()> {
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
        let original = "# external\n[mcp_servers.external]\ncommand = \"external\"\n";
        fs::write(&path, original)?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}/mcp", listener.local_addr()?);
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database).unwrap();
        let alpha = catalog.create(&scope, http_draft("alpha", &url)).unwrap().record;
        let external = catalog.create(&scope, http_draft("external", &url)).unwrap().record;
        let marker = home.path().join("executed");
        let mut draft = http_draft("bravo", &url);
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
        let bravo = catalog.create(&scope, draft).unwrap().record;
        let missing = McpRecordRef { id: McpRecordId("missing".into()), revision: 1 };

        let set = |records: &[McpRecordRef], enabled| {
            runtime.block_on(catalog.set_app_bindings(
                &scope,
                records,
                &target,
                enabled,
                &accounts,
                |_, _| None,
            ))
        };
        let requested = [alpha.clone(), external.clone(), missing.clone(), bravo.clone(), alpha];
        let enabled = set(&requested, true);
        assert_eq!(enabled.outcome, McpBatchOutcome::PartiallySucceeded);
        let enabled = enabled.items;
        assert_eq!(enabled.len(), requested.len());
        for (item, request) in enabled.iter().zip(&requested) {
            assert_eq!(item.record.id, request.id);
            assert_eq!(item.target, app);
            assert!(item.enabled);
        }
        assert!(enabled[0].desired_committed);
        assert_eq!(enabled[0].native_outcome, McpDistributionOutcome::Applied);
        assert_eq!(enabled[0].record.revision, requested[0].revision + 1);
        assert!(enabled[1].desired_committed);
        assert_eq!(enabled[1].record.revision, external.revision + 1);
        assert_eq!(
            enabled[1].native_outcome,
            McpDistributionOutcome::Failed { code: McpFailureCode::Conflict }
        );
        assert!(!enabled[2].desired_committed);
        assert_eq!(enabled[2].record, missing);
        assert_eq!(
            enabled[2].native_outcome,
            McpDistributionOutcome::Failed { code: McpFailureCode::NotFound }
        );
        assert!(enabled[3].desired_committed);
        assert_eq!(enabled[3].native_outcome, McpDistributionOutcome::Applied);
        assert_eq!(enabled[3].record.revision, bravo.revision + 1);

        let native = fs::read_to_string(&path)?;
        assert!(native.contains(original));
        assert!(native.contains("[mcp_servers.alpha]"));
        assert!(native.contains("[mcp_servers.bravo]"));
        assert_eq!(enabled[4], enabled[0], "a duplicate must reuse the first attempt");

        let inode = fs::metadata(&path)?.ino();
        let replayed = set(&requested, true);
        assert_eq!(replayed.outcome, McpBatchOutcome::Failed);
        let replayed = replayed.items;
        assert_eq!(
            replayed[0].native_outcome,
            McpDistributionOutcome::Failed { code: McpFailureCode::StaleRevision }
        );
        assert_eq!(fs::metadata(&path)?.ino(), inode);
        assert_eq!(fs::read_to_string(&path)?, native);

        let bound =
            [enabled[0].record.clone(), enabled[1].record.clone(), enabled[3].record.clone()];
        let disabled = set(&bound, false);
        assert_eq!(disabled.outcome, McpBatchOutcome::Succeeded);
        let disabled = disabled.items;
        assert_eq!(disabled[0].native_outcome, McpDistributionOutcome::Applied);
        assert_eq!(disabled[1].native_outcome, McpDistributionOutcome::Unchanged);
        assert_eq!(disabled[2].native_outcome, McpDistributionOutcome::Applied);
        for (item, previous) in disabled.iter().zip(&bound) {
            assert!(!item.enabled);
            assert!(item.desired_committed);
            assert_eq!(item.record.revision, previous.revision + 1);
        }
        assert_eq!(fs::read_to_string(&path)?, original);

        let disabled_refs: Vec<_> = disabled.iter().map(|item| item.record.clone()).collect();
        let inode = fs::metadata(&path)?.ino();
        let repeated = set(&disabled_refs, false);
        assert_eq!(repeated.outcome, McpBatchOutcome::Succeeded);
        let repeated = repeated.items;
        for (item, previous) in repeated.iter().zip(&disabled_refs) {
            assert_eq!(&item.record, previous);
            assert_eq!(item.native_outcome, McpDistributionOutcome::Unchanged);
        }
        assert_eq!(fs::metadata(&path)?.ino(), inode);
        assert!(!marker.exists());
        assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
        Ok(())
    })
}
