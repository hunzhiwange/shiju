use super::*;
use acp_server::server::SessionBackend;
use agent_app_services::mcp::McpCatalogStore;
use llm_runtime::acp::host::Host;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};
use vw_api_mcp_management::{
    McpCatalogDraft, McpCatalogScope, McpConnectionDefinition, McpLinuxStdioSelection,
};

const STDIO: &str = r#"
count=0
while IFS= read -r line; do
    case "$line" in
        *'"id":'*) id=${line#*\"id\":}; id=${id%%,*}; id=${id%%\}*};;
        *) continue;;
    esac
    case "$line" in
        *'"method":"initialize"'*)
            result='{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}';;
        *'"method":"tools/list"'*)
            result='{"tools":[{"name":"read_fixture","inputSchema":{"type":"object"}}]}';;
        *'"method":"tools/call"'*)
            case "$line" in *'"shijuCallId"'*) exit 3;; esac
            count=$((count+1))
            result='{"content":[{"type":"text","text":"execution-'${count}'"}]}';;
        *) exit 2;;
    esac
    printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$id" "$result"
done
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires non-root GNU Linux with Bubblewrap, dash, Node/npm and the pinned iFlow packages"]
async fn acp_platform_iflow_contract_host_project_mcp_revision_and_removal() {
    const FIXTURE: &str = "SHIJU_TEST_IFLOW_PROJECT_MCP";
    let Some(root) = std::env::var_os(FIXTURE).map(std::path::PathBuf::from) else {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let module = module_path!().split_once("::").unwrap().1;
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                &format!(
                    "{module}::acp_platform_iflow_contract_host_project_mcp_revision_and_removal"
                ),
            ])
            .env(FIXTURE, root.path())
            .env("HOME", &home)
            .env("VIBEWINDOW_TEST_ROOT", &home)
            .env("VIBEWINDOW_CONFIG_DIR", root.path().join("config"))
            .status()
            .unwrap();
        assert!(status.success(), "iFlow production Host MCP test failed");
        return;
    };

    acp_agent_registry::install_builtin_agent("iflow").await.unwrap();
    let (command, args) = acp_agent_registry::reviewed_agent_command("iflow").unwrap().unwrap();
    assert!(command.contains("iflow-0.5.19-model-source-v4"));
    let config = AcpAgentConfig { command, args, env: Default::default() };
    let tenant = crate::session::ui_store::current_tenant_scope();
    let workspace = root.join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &workspace).await.unwrap();
    crate::config::update_project_for_tenant(
        &tenant,
        &workspace,
        json!({
            "providers":{"provider":{"api":"http://127.0.0.1:1/v1","models":{
                "model":{"status":"active","tool_call":true,"temperature":true}
            }}}
        }),
    )
    .await
    .unwrap();
    let context = instance::Context {
        directory: workspace.to_string_lossy().into_owned(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let db =
        crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    let store = McpCatalogStore::new(db).unwrap();
    let project_scope =
        vw_api_transport::ProjectScope::new(tenant.clone(), project.id.into()).unwrap();
    let scope = McpCatalogScope::Project { scope: project_scope.clone() };
    let executable = Path::new("/bin/dash").canonicalize().unwrap();
    let argv = vec!["-c".to_owned(), STDIO.to_owned()];
    let connection = McpConnectionDefinition::Stdio {
        command: executable.to_str().unwrap().into(),
        args: argv.clone(),
        env: Default::default(),
        cwd: None,
    };
    let record = store
        .create(
            &scope,
            McpCatalogDraft {
                id: "selected".into(),
                name: None,
                description: None,
                homepage: None,
                documentation: None,
                tags: vec![],
                connection: connection.clone(),
            },
        )
        .unwrap()
        .record;
    let review = || McpLinuxStdioSelection::Select {
        sha256: hex::encode(Sha256::digest(std::fs::read(&executable).unwrap())),
        allowed_tools: vec!["read_fixture".into()],
    };
    let definition = acp_types::AcpMcpServer::Stdio(
        agent_client_protocol::McpServerStdio::new("selected", &executable).args(argv).meta(
            serde_json::Map::from_iter([(
                "shiju.iflow.toolPolicy".into(),
                json!({"allow":[],"deny":[]}),
            )]),
        ),
    );
    let selected = [definition];
    let prepare = || async {
        instance::provide_context(context.clone(), async {
            let model = provider::get_model("provider", "model").await?;
            super::super::request::RuntimeHost::prepare_stdio_mcp_launch(
                "iflow",
                &config,
                &model,
                "project-mcp-host-test",
                &workspace,
                &selected,
            )
            .await
        })
        .await
    };
    assert!(prepare().await.is_err(), "unreviewed project service started");
    store.configure_linux_stdio(&project_scope, &record, review()).unwrap();
    let profile = prepare().await.unwrap();

    let (servers, issue) = profile.managed_mcp.as_ref().unwrap();
    let [acp_types::AcpMcpServer::Http(server)] = servers.as_slice() else {
        panic!("Expected one host HTTP projection");
    };
    assert_eq!(server.meta.as_ref().unwrap()["shiju.iflow.mcp-v4"], "project");
    assert_eq!(
        profile.config().env["SHIJU_IFLOW_MCP_RELAY"].as_str(),
        server.url.split("/shiju-vtcode-mcp/").next().unwrap()
    );
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap();
    let call = |server: &agent_client_protocol::McpServerHttp, id: &str| {
        client
            .post(&server.url)
            .header("Authorization", &server.headers[0].value)
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
                "name":"read_fixture","arguments":{},"_meta":{"shijuCallId":id}
            }}))
            .send()
    };
    assert_eq!(call(server, "rejected").await.unwrap().status(), 403);
    assert!(issue("mcp__selected__read_fixture", "approved", &json!({}), Box::new(|| true)));
    let response = call(server, "approved").await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(response["result"]["content"][0]["text"], "execution-1");
    assert_eq!(call(server, "approved").await.unwrap().status(), 403);
    assert!(issue("mcp__selected__read_fixture", "after-replay", &json!({}), Box::new(|| true)));
    let response = call(server, "after-replay").await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(response["result"]["content"][0]["text"], "execution-2");

    assert!(issue("mcp__selected__read_fixture", "old-revision", &json!({}), Box::new(|| true)));
    let updated = store.update_connection(&scope, &record, connection).unwrap().record;
    assert_eq!(call(server, "old-revision").await.unwrap().status(), 403);
    assert!(!issue("mcp__selected__read_fixture", "late", &json!({}), Box::new(|| true)));
    assert!(!(profile.managed_mcp_current.as_ref().unwrap())());
    assert!(prepare().await.is_err(), "stale review enabled a new launch");

    store.configure_linux_stdio(&project_scope, &updated, review()).unwrap();
    let replacement = prepare().await.unwrap();
    let (servers, _) = replacement.managed_mcp.as_ref().unwrap();
    let [acp_types::AcpMcpServer::Http(server)] = servers.as_slice() else {
        panic!("Expected a new projection");
    };
    store.configure_linux_stdio(&project_scope, &updated, McpLinuxStdioSelection::Disable).unwrap();
    let removed = client
        .post(&server.url)
        .header("Authorization", &server.headers[0].value)
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(removed.status(), 403);
    assert!(prepare().await.is_err(), "withdrawn project service opened");
    assert!(!(replacement.managed_mcp_current.as_ref().unwrap())());

    store.configure_linux_stdio(&project_scope, &updated, review()).unwrap();
    let backend = crate::acp::VibeWindowBackend::new(tenant);
    let session = backend.create_session(workspace.clone()).await.unwrap();
    let id = session.id.as_str();
    let mount = json!({"scope":scope,"record":updated});
    let update = |revision, mounted| {
        serde_json::from_value(json!({
            "sessionId":id, "expectedRevision":revision, "skills":[], "mcpServers":[],
            "mcpCatalog":if mounted { vec![mount.clone()] } else { vec![] },
        }))
        .unwrap()
    };
    assert_eq!(backend.set_session_mounts(update(0, true)).await.unwrap().revision.0, 1);
    assert_eq!(backend.session_tool_count(id), Some(0));

    let tool_context = std::sync::Arc::new(
        crate::tools::ToolUseContext::new(id, Some(workspace.display().to_string()))
            .with_channel("acp"),
    );
    let mounted_session = format!("{id}:acp-source-0:fixture");
    let prepare_mounted = || async {
        instance::provide_context(context.clone(), async {
            crate::tools::scope_tool_use_context(tool_context.clone(), async {
                let model = provider::get_model("provider", "model").await?;
                super::super::request::RuntimeHost::prepare_stdio_mcp_launch(
                    "iflow",
                    &config,
                    &model,
                    &mounted_session,
                    &workspace,
                    &[],
                )
                .await
            })
            .await
        })
        .await
    };
    let mounted = prepare_mounted().await.unwrap();
    let (servers, approve) = mounted.managed_mcp.as_ref().unwrap();
    let [acp_types::AcpMcpServer::Http(server)] = servers.as_slice() else {
        panic!("Expected one committed session projection");
    };
    assert_eq!(server.meta.as_ref().unwrap()["shiju.iflow.mcp-v4"], "mounted");
    assert_eq!(call(server, "mounted-rejected").await.unwrap().status(), 403);
    for (id, expected) in [("mounted-once", "execution-1"), ("mounted-after-replay", "execution-2")] {
        assert!(approve("mcp__selected__read_fixture", id, &json!({}), Box::new(|| true)));
        let response = call(server, id).await.unwrap().json::<Value>().await.unwrap();
        assert_eq!(response["result"]["content"][0]["text"], expected);
        assert_eq!(call(server, id).await.unwrap().status(), 403);
    }
    assert!(approve("mcp__selected__read_fixture", "mounted-call", &json!({}), Box::new(|| true)));
    assert!(backend.set_session_mounts(update(0, false)).await.is_err());
    assert!((mounted.managed_mcp_current.as_ref().unwrap())());
    assert_eq!(backend.set_session_mounts(update(1, false)).await.unwrap().revision.0, 2);
    assert_eq!(call(server, "mounted-call").await.unwrap().status(), 403);
    assert!(!approve("mcp__selected__read_fixture", "after-removal", &json!({}), Box::new(|| true)));
    assert!(!(mounted.managed_mcp_current.as_ref().unwrap())());
    assert!(prepare_mounted().await.unwrap().managed_mcp.is_none());
    profile.binding.revoke();
    replacement.binding.revoke();
}
