use std::{fs, io::ErrorKind, net::TcpListener, path::Path};

use serde_json::json;
use vw_agent_apps::mcp::import::NativeMcpSource;
use vw_api_types::{
    TenantScope,
    agent_apps::{AppId, AppInstallationId},
    mcp_management::{McpAppTarget, McpCatalogScope, McpFailureCode, McpNativeStatus},
};

use super::{
    McpCatalogStore, McpImportOutcome, McpImportSource,
    catalog_create::catalog_location,
    catalog_create_tests::{http_draft, with_database},
};
use crate::gateway::settings_store::GatewaySettingsStore;

fn source(app_id: AppId, home: &Path) -> McpImportSource<'_> {
    McpImportSource {
        target: McpAppTarget {
            app_id,
            installation_id: AppInstallationId(format!("{app_id:?}")),
            target_host_id: "host".into(),
        },
        native: NativeMcpSource::HostHome(home),
    }
}

#[test]
fn cc_mcp_t23_import_merges_equal_definitions_and_reports_partial_failure() -> anyhow::Result<()> {
    with_database(|database| {
        let home = tempfile::tempdir()?;
        let home = home.path();
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}/mcp", listener.local_addr()?);
        let marker = home.join("started");
        let existing = catalog.create(&scope, http_draft("taken", &url)).unwrap();
        for dir in [".grok", ".gemini", ".codex"] {
            fs::create_dir(home.join(dir))?;
        }
        let paths = [
            home.join(".grok/config.toml"),
            home.join(".gemini/settings.json"),
            home.join(".codex/config.toml"),
        ];
        let bytes = [
            "[".into(),
            json!({"mcpServers":{
                "shared":{"httpUrl":url},
                "taken":{"httpUrl":"http://127.0.0.1:1/other"},
                "bad":{"command":42},
                "secret":{"httpUrl":url,"headers":{"Authorization":"fixture-value"}},
                "stdio":{"command":"/bin/sh","args":["-c","touch \"$1\"","fixture",marker]},
                "events":{"url":format!("{url}/events")}
            }})
            .to_string(),
            format!("[mcp_servers.shared]\nurl = {url:?}\nenabled = false\n"),
        ];
        for (path, document) in paths.iter().zip(&bytes) {
            fs::write(path, document)?;
        }
        let sources = [
            source(AppId::GrokBuild, home),
            source(AppId::Gemini, home),
            source(AppId::Codex, home),
        ];
        let reports = catalog.import_from_apps(&scope, &sources);
        assert_eq!(reports.len(), sources.len());
        for (report, source) in reports.iter().zip(&sources) {
            assert_eq!(report.target, source.target);
        }
        assert_eq!(reports[0].entries, Err(McpFailureCode::InvalidDefinition));
        let gemini = reports[1].entries.as_ref().unwrap();
        let outcome = |id: &str| {
            &gemini
                .iter()
                .find(|entry| entry.id.as_ref().is_some_and(|record| record.0 == id))
                .unwrap()
                .outcome
        };
        assert_eq!(outcome("taken"), &Err(McpFailureCode::Conflict));
        assert_eq!(outcome("bad"), &Err(McpFailureCode::InvalidDefinition));
        assert_eq!(outcome("secret"), &Err(McpFailureCode::InvalidDefinition));
        for id in ["shared", "stdio", "events"] {
            assert!(matches!(outcome(id), Ok(McpImportOutcome::Added(_))));
        }
        let codex = &reports[2].entries.as_ref().unwrap()[0];
        let Ok(McpImportOutcome::Merged(shared)) = &codex.outcome else {
            panic!("identical Codex definition must merge: {:?}", codex.outcome);
        };
        assert_eq!(shared.record.revision, 2);
        assert_eq!(shared.connection_revision, 1);

        let (tenant, namespace) = catalog_location(&scope).unwrap();
        let settings = GatewaySettingsStore::new(database.clone())?;
        let before = settings.list(tenant, &namespace)?;
        assert_eq!(before.len(), 4);
        assert!(before.iter().all(|row| !row.payload_json.contains("fixture-value")));
        let retry = McpCatalogStore::new(database).unwrap().import_from_apps(&scope, &sources);
        assert!(matches!(
            retry[2].entries.as_ref().unwrap()[0].outcome,
            Ok(McpImportOutcome::Unchanged(_))
        ));
        assert_eq!(before, settings.list(tenant, &namespace)?);
        assert!(catalog.list(&scope).unwrap().contains(&existing));
        let revocations = catalog.delete(&scope, &shared.record).unwrap();
        assert_eq!(revocations.len(), 2);
        assert_eq!(revocations[0].binding.target, sources[1].target);
        assert_eq!(revocations[1].binding.target, sources[2].target);
        assert!(revocations[0].binding.enabled);
        assert!(!revocations[1].binding.enabled);
        assert!(
            revocations.iter().all(|item| item.binding.native_status == McpNativeStatus::Present)
        );
        for (path, document) in paths.iter().zip(&bytes) {
            assert_eq!(fs::read_to_string(path)?, *document);
        }
        assert!(!marker.exists());
        assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
        Ok(())
    })
}

#[test]
fn cc_mcp_t23_import_reads_native_profiles_and_checks_claude_scope() -> anyhow::Result<()> {
    use std::os::unix::fs::{PermissionsExt, symlink};
    use vw_agent_apps::auth::credential_file::{AccountCredential, CredentialFileStore};

    with_database(|database| {
        let home = tempfile::tempdir()?;
        let home = home.path();
        let tenant = TenantScope::new("platform", "company")?;
        let scope = McpCatalogScope::Tenant { scope: tenant.clone() };
        let catalog = McpCatalogStore::new(database).unwrap();
        let files = [
            (AppId::GrokBuild, ".grok/config.toml", "[mcp_servers.grok]\ncommand = \"fixture\""),
            (
                AppId::OpenCode,
                ".config/opencode/opencode.jsonc",
                r#"{"mcp":{"opencode":{"type":"local","command":["fixture"]}}}"#,
            ),
            (AppId::Hermes, ".hermes/config.yaml", "mcp_servers:\n  hermes: {command: fixture}"),
            (AppId::Pi, ".pi/agent/mcp.json", r#"{"mcpServers":{"pi":{"command":"fixture"}}}"#),
            (
                AppId::Mcode,
                ".minimax/mcp.json",
                r#"{"mcpServers":{"mcode":{"command":"fixture"}}}"#,
            ),
            (
                AppId::Kimi,
                ".kimi-code/mcp.json",
                r#"{"mcpServers":{"kimi":{"command":"fixture"}}}"#,
            ),
            (
                AppId::Dsh,
                ".dsh/profiles/web/cordis.patch.yml",
                "- id: fixture\n  name: '@deepseek-ai/dsh-mcp-client'\n  config: {serverName: dsh, transport: stdio, command: fixture}",
            ),
            (
                AppId::Agy,
                ".gemini/config/mcp_config.json",
                r#"{"mcpServers":{"agy":{"command":"fixture"}}}"#,
            ),
            (AppId::Omp, ".omp/agent/mcp.json", r#"{"mcpServers":{"omp":{"command":"fixture"}}}"#),
        ];
        for (_, path, document) in files {
            let path = home.join(path);
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(path, document)?;
        }
        // The import only needs profile patches, even if another DSH resource is unreadable.
        symlink("unavailable", home.join(".dsh/.credentials.yaml"))?;
        let account_root = tempfile::tempdir()?;
        fs::set_permissions(account_root.path(), fs::Permissions::from_mode(0o700))?;
        let store = CredentialFileStore::new(account_root.path())
            .and_then(|store| store.for_owner("member"))
            .map_err(anyhow::Error::msg)?;
        store.register_claude_directory(&tenant, "host", "account").map_err(anyhow::Error::msg)?;
        let lease = store
            .lease(&tenant, "host", AppId::Claude, "account", false)
            .map_err(anyhow::Error::msg)?;
        let AccountCredential::ClaudeDirectory(directory) =
            lease.read().map_err(anyhow::Error::msg)?
        else {
            anyhow::bail!("expected Claude directory");
        };
        let profile = directory.path().join(".claude.json");
        let claude = r#"{"mcpServers":{"claude":{"command":"fixture"}}}"#;
        fs::write(&profile, claude)?;

        let mut sources = files.map(|(app, _, _)| source(app, home)).to_vec();
        let mut target = source(AppId::Claude, home).target;
        target.target_host_id = "other-host".into();
        sources.push(McpImportSource { target, native: NativeMcpSource::ClaudeAccount(&lease) });
        sources.push(McpImportSource {
            target: source(AppId::Claude, home).target,
            native: NativeMcpSource::ClaudeAccount(&lease),
        });
        sources.push(source(AppId::ClaudeDesktop, home));
        sources.push(source(AppId::Codex, home));
        let reports = catalog.import_from_apps(&scope, &sources);
        for index in (0..files.len()).chain([10]) {
            let report = &reports[index];
            let entries = report
                .entries
                .as_ref()
                .unwrap_or_else(|err| panic!("{:?}: {err:?}", report.target));
            assert_eq!(entries.len(), 1);
            assert!(matches!(entries[0].outcome, Ok(McpImportOutcome::Added(_))), "{entries:?}");
        }
        assert_eq!(reports[9].entries, Err(McpFailureCode::PermissionDenied));
        assert_eq!(reports[11].entries, Err(McpFailureCode::TargetUnavailable));
        assert_eq!(reports[12].entries, Ok(Vec::new()));
        assert_eq!(catalog.list(&scope).unwrap().len(), 10);
        assert!(!home.join(".codex").exists());
        assert!(!home.join(".dsh/profiles/desktop").exists());
        assert_eq!(fs::read_to_string(profile)?, claude);
        for (_, path, document) in files {
            assert_eq!(fs::read_to_string(home.join(path))?, document);
        }
        Ok(())
    })
}
