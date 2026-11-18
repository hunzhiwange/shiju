use super::retain_vtcode_tools;
use serde_json::json;

#[test]
fn vtcode_relay_exposes_only_sandboxed_read_and_patch_tools() {
    let model = "alibaba-token-plan-cn/deepseek-v4.1-flash-1017";
    let tool = |name| json!({"type":"function","function":{"name":name,"parameters":{}}});
    let mut request = json!({
        "model": model,
        "messages": [{"role":"user","content":"Create a file"}],
        "tools": [tool("list_files"),tool("apply_patch"),tool("exec_command"),tool("write_stdin"),tool("web_fetch")]
    });
    retain_vtcode_tools(&mut request);
    assert_eq!(request["tools"], json!([tool("list_files"), tool("apply_patch")]));
    assert_eq!(request["model"], model);
    assert_eq!(request["messages"][0]["content"], "Create a file");
}
