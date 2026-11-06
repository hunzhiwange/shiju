use axum::{Router, middleware};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path, time::Duration};
use vw_agent_apps::apps::launch::request::LaunchMaterial;
use vw_api_types::{
    agent_apps::*,
    id::{TaskId, TenantScope},
    task::TaskStatus,
};
use vw_gateway_client::{AgentAppsLifecycleError, GatewayClient, GatewayEndpoint};
fn terminal(_: Option<&str>, _: &Path, material: &LaunchMaterial) -> Result<(), String> {
    let result = std::process::Command::new(&material.program)
        .args(&material.args)
        .current_dir(&material.cwd)
        .status()
        .map_err(|e| e.to_string())?;
    if !result.success() {
        return Err("fixture process failed".into());
    }
    Ok(())
}
const TOKEN: &str = "lifecycle-fixture-token-with-explicit-write-grants";
fn script(path: &Path, value: &str) {
    std::fs::write(path, value).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
struct Fixture {
    root: tempfile::TempDir,
    app: Router,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let prefix = root.path().join("prefix");
        std::fs::create_dir_all(prefix.join("bin")).unwrap();
        script(&prefix.join("bin/node"), "#!/bin/sh\necho v24.0.0\n");
        script(
            &prefix.join("bin/npm"),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$@" >> '{root}/argv'
printf '%s\n' "$PWD" "$HOME" "$npm_config_prefix" "$npm_config_engine_strict" >> '{root}/environment'
if [ -f '{root}/delay' ]; then echo $$ > '{root}/pid'; touch '{root}/started'; sleep 20; touch '{root}/escaped'; fi
if [ -f '{root}/fail' ]; then echo secret-fixture-payload >&2; exit 7; fi
if [ "$1" = uninstall ]; then rm -f '{prefix}/bin/codex'; exit 0; fi
mkdir -p '{prefix}/lib/node_modules/@openai/codex'
printf '{{"name":"@openai/codex"}}' > '{prefix}/lib/node_modules/@openai/codex/package.json'
printf '#!/bin/sh\nif [ \"$1\" = --version ]; then echo codex 1.2.3; else printf launched > launched; fi\n' > '{prefix}/lib/node_modules/@openai/codex/cli'
chmod 700 '{prefix}/lib/node_modules/@openai/codex/cli'
ln -sf '{prefix}/lib/node_modules/@openai/codex/cli' '{prefix}/bin/codex'
"#,
                root = root.path().display(),
                prefix = prefix.display()
            ),
        );
        let policy = root.path().join("policy.json");
        let p = json!({"principals":[{"id":"writer","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[
        {"platform_id":"p","company_id":"a","host_id":"host","launch":true,"actions":["install","update","uninstall"]},
        {"platform_id":"p","company_id":"b","host_id":"host","launch":true,"actions":["install","update","uninstall"]}]}],
        "hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"search_paths":[prefix.join("bin")],"applications":root.path().join("Applications"),"lifecycle":{"prefix":prefix,"npm":prefix.join("bin/npm")}}]});
        std::fs::write(&policy, serde_json::to_vec(&p).unwrap()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let db = crate::global_database::GlobalDatabase::for_home(root.path()).unwrap();
        let app = super::lifecycle::router_with_policy(Some(policy.clone()))
            .merge(super::read::router_with_policy(Some(policy.clone())))
            .merge(super::launch::router_with_terminal(Some(policy.clone()), terminal))
            .layer(axum::Extension(db))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        Self { root, app }
    }
}
struct Gateway {
    fixture: Fixture,
    port: u16,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Gateway {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Gateway {
    async fn new() -> Self {
        let fixture = Fixture::new();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let router = Router::new().nest("/v1", fixture.app.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self { fixture, port, server }
    }
    fn client(&self, company: &str) -> GatewayClient {
        GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", self.port)
                .with_tenant_scope(TenantScope::new("p", company).unwrap()),
        )
        .unwrap()
    }
    async fn action(&self, action: AppLifecycleAction, target: AppActionTarget) -> TaskId {
        let c = self.client("a");
        let p = c
            .agent_apps_action_preview_authorized(
                "host",
                &AppActionPreviewRequest { app_id: AppId::Codex, action, target },
                TOKEN,
            )
            .await
            .unwrap();
        c.agent_apps_action_submit_authorized(
            "host",
            &AppActionSubmitRequest { preview_id: p.preview_id, confirmed: true },
            TOKEN,
        )
        .await
        .unwrap()
        .operation_id
    }
    async fn wait(&self, id: &TaskId) -> TaskStatus {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let s = self
                    .client("a")
                    .agent_apps_operation_authorized("host", id, TOKEN)
                    .await
                    .unwrap();
                if s.phase == AppOperationPhase::Terminal {
                    return s.status;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap()
    }
}
fn environment() -> AppActionTarget {
    AppActionTarget::Environment {
        environment: if cfg!(target_os = "macos") {
            AppInstallationEnvironment::Macos
        } else {
            AppInstallationEnvironment::Linux
        },
    }
}
#[tokio::test]
async fn cc_apps_remote_host_isolation_sdk_install_launch_uninstall_and_cancel() {
    let client_root = tempfile::tempdir().unwrap();
    let a = Gateway::new().await;
    let b = Gateway::new().await;
    // Same host ID on two gateways intentionally stresses endpoint selection.
    for (selected, untouched) in [(&a, &b), (&b, &a)] {
        let id = selected.action(AppLifecycleAction::Install, environment()).await;
        assert_eq!(selected.wait(&id).await, TaskStatus::Completed);
        assert!(selected.fixture.root.path().join("prefix/bin/codex").exists());
        assert!(!untouched.fixture.root.path().join("prefix/bin/codex").exists());
        let c = selected.client("a");
        let installations =
            c.agent_apps_installations_authorized("host", AppId::Codex, TOKEN).await.unwrap();
        let installation_id = installations.installations[0].id.clone();
        let request = LaunchRequest {
            app_id: AppId::Codex,
            installation_id: installation_id.clone(),
            cwd: selected.fixture.root.path().to_str().unwrap().into(),
            args: vec![],
            skip_permissions: false,
        };
        let mut invalid = request.clone();
        invalid.cwd = client_root.path().to_str().unwrap().into();
        assert!(c.agent_apps_launch_authorized("host", &invalid, TOKEN).await.is_err());
        assert!(
            untouched
                .client("a")
                .agent_apps_launch_authorized("host", &request, TOKEN)
                .await
                .is_err()
        );
        c.agent_apps_launch_authorized("host", &request, TOKEN).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(selected.fixture.root.path().join("launched")).unwrap(),
            "launched"
        );
        assert!(!untouched.fixture.root.path().join("launched").exists());
        let id = selected
            .action(
                AppLifecycleAction::Uninstall,
                AppActionTarget::Installation { installation_id },
            )
            .await;
        assert_eq!(selected.wait(&id).await, TaskStatus::Completed);
        assert!(!selected.fixture.root.path().join("prefix/bin/codex").exists());
        std::fs::remove_file(selected.fixture.root.path().join("launched")).unwrap();
    }
    std::fs::write(a.fixture.root.path().join("fail"), "").unwrap();
    let failed = a.action(AppLifecycleAction::Install, environment()).await;
    assert_eq!(a.wait(&failed).await, TaskStatus::Failed);
    assert!(!a.fixture.root.path().join("prefix/bin/codex").exists());
    std::fs::remove_file(a.fixture.root.path().join("fail")).unwrap();
    std::fs::write(a.fixture.root.path().join("delay"), "").unwrap();
    let id = a.action(AppLifecycleAction::Install, environment()).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while !a.fixture.root.path().join("started").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    for client in [a.client("b"), b.client("a")] {
        assert!(matches!(
            client.agent_apps_operation_cancel_authorized("host", &id, TOKEN).await,
            Err(AgentAppsLifecycleError::Http { status: 404, .. })
        ));
    }
    assert_ne!(
        a.client("a").agent_apps_operation_authorized("host", &id, TOKEN).await.unwrap().phase,
        AppOperationPhase::Terminal
    );
    assert_eq!(
        a.client("a")
            .agent_apps_operation_cancel_authorized("host", &id, TOKEN)
            .await
            .unwrap()
            .status,
        TaskStatus::Cancelled
    );
    assert!(!a.fixture.root.path().join("escaped").exists());
    let pid = std::fs::read_to_string(a.fixture.root.path().join("pid")).unwrap();
    assert!(
        !std::process::Command::new("/bin/kill")
            .args(["-0", pid.trim()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(std::fs::read_dir(client_root.path()).unwrap().count(), 0);
}
