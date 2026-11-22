use super::retain_vtcode_tools;
use serde_json::json;

#[test]
fn vtcode_relay_exposes_only_sandboxed_read_and_patch_tools() {
    let model = "alibaba-token-plan-cn/deepseek-v4.1-flash-1017";
    let tool = |name| json!({"type":"function","function":{"name":name,"parameters":{}}});
    let mut request = json!({
        "model": model,
        "messages": [{"role":"user","content":"Create a file"}],
        "tools": [tool("list_files"),tool("apply_patch"), tool("load_skill"), tool("load_skill_resource"), tool("exec_command"), tool("write_stdin"), tool("run_skill"), tool("web_fetch")]
    });
    retain_vtcode_tools(&mut request, &[], false);
    assert_eq!(
        request["tools"],
        json!([
            tool("list_files"),
            tool("apply_patch"),
            tool("load_skill"),
            tool("load_skill_resource")
        ])
    );
    assert_eq!(request["model"], model);
    assert_eq!(request["messages"][0]["content"], "Create a file");
}

#[test]
fn vtcode_relay_retains_only_this_sessions_mcp_catalog_alongside_official_tools() {
    let tool = |name| json!({"type":"function","function":{"name":name,"parameters":{}}});
    let mounted = "mcp__ruejs_graph_probe__rustcodegraph_query";
    let mut request = json!({"tools":[tool("list_files"),tool(mounted),tool("mcp__unmounted__query"),tool("exec_command")]});
    retain_vtcode_tools(&mut request, &[mounted.into()], false);
    assert_eq!(request["tools"], json!([tool("list_files"), tool(mounted)]));
    retain_vtcode_tools(&mut request, &[], false);
    assert_eq!(request["tools"], json!([tool("list_files")]));
}


#[test]
fn vtcode_unix_relay_hides_tools_outside_the_empty_read_only_workspace() {
    let tool = |name| json!({"type":"function","function":{"name":name}});
    let mounted = "mcp__selected__query";
    let mut request = json!({"tools":[
        tool("list_files"), tool("read_file"), tool("apply_patch"), tool("load_skill"),
        tool("load_skill_resource"), tool("exec_command"), tool(mounted), tool("mcp__unmounted__query")
    ]});
    retain_vtcode_tools(&mut request, &[mounted.into()], true);
    assert_eq!(request["tools"], json!([tool("list_files"), tool(mounted)]));
}
