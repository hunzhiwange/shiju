use super::{Approvals, take_approval};
use serde_json::json;

#[test]
fn aionrs_mcp_approval_only_consumes_the_exact_call_once() {
    let approvals = Approvals::default();
    let args = json!({"symbol": "mountRueSiteApp"});
    approvals.lock().unwrap().insert("callers".into(), args.clone());

    assert!(!take_approval(&approvals, "query", &args));
    assert!(!take_approval(&approvals, "callers", &json!({"symbol": "other"})));
    assert!(take_approval(&approvals, "callers", &args));
    assert!(!take_approval(&approvals, "callers", &args));
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[tokio::test]
#[ignore = "requires a real RustCodeGraph binary and indexed RueJS fixture"]
async fn aionrs_mcp_routes_real_graph_results_through_a_read_only_mount() {
    use super::{Catalog, Mount};
    use agent_client_protocol::{McpServer, McpServerStdio};
    use std::path::PathBuf;

    let workspace = PathBuf::from(std::env::var_os("SHIJU_AIONRS_MCP_TEST_WORKSPACE").unwrap())
        .canonicalize()
        .unwrap();
    let executable = PathBuf::from(std::env::var_os("SHIJU_AIONRS_MCP_TEST_GRAPH").unwrap());
    let state = tempfile::tempdir().unwrap();
    let root = state.path().canonicalize().unwrap();
    let policy = json!({"shiju.aionrs.toolPolicy":{
        "allow":["rustcodegraph_query","ruejs_graph_probe__rustcodegraph_callers","rustcodegraph_node","rustcodegraph_files"],
        "deny":["ruejs_graph_probe__rustcodegraph_files"]
    }});
    let mut mount = Mount::new(
        McpServer::Stdio(
            McpServerStdio::new("ruejs_graph_probe", executable)
                .args(vec![
                    "serve".into(),
                    "--mcp".into(),
                    "--path".into(),
                    workspace.to_str().unwrap().into(),
                    "--no-watch".into(),
                ])
                .meta(policy.as_object().cloned()),
        ),
        &workspace,
        &root,
    )
    .unwrap();
    let catalog = Catalog::default();
    let approvals = Approvals::default();
    let init = mount.request(json!({
        "jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
            "protocolVersion":"2024-11-05", "capabilities":{}, "clientInfo":{"name":"probe","version":"1"}
        }
    }), &catalog, &approvals).await.unwrap();
    assert!(init.get("result").is_some(), "{init}");
    mount
        .request(
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            &catalog,
            &approvals,
        )
        .await
        .unwrap();
    let list = mount
        .request(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}), &catalog, &approvals)
        .await
        .unwrap();
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 3);
    assert_eq!(catalog.lock().unwrap().len(), 3);
    for (index, (tool, args, output)) in [
        ("query", json!({"search":"createRueSiteApp"}), "createRueSiteApp"),
        (
            "callers",
            json!({"symbol":"createRueSiteApp", "projectPath":workspace}),
            "mountRueSiteApp",
        ),
        ("node", json!({"name":"mountRueSiteApp"}), "target.replaceChildren()"),
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("mcp__ruejs_graph_probe__rustcodegraph_{tool}");
        let call = json!({
            "jsonrpc":"2.0","id":index+3,"method":"tools/call","params":{"name":name,"arguments":args}
        });
        let denied = mount.request(call.clone(), &catalog, &approvals).await.unwrap();
        assert_eq!(denied["error"]["code"], -32000);
        approvals.lock().unwrap().insert(name, args);
        let result = mount.request(call.clone(), &catalog, &approvals).await.unwrap();
        assert_ne!(result["result"]["isError"], true);
        assert!(
            result["result"]["content"][0]["text"].as_str().unwrap().contains(output),
            "{result}"
        );
        let replay = mount.request(call, &catalog, &approvals).await.unwrap();
        assert_eq!(replay["error"]["code"], -32000);
    }
}
