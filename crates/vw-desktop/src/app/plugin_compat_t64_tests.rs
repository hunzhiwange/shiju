//! Explicit macOS UI acceptance, using an isolated source package and control directory.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use vw_desktop::app::components::mcp_app_panel::McpAppPanel;

struct LocalServer(Child);
impl Drop for LocalServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[cfg(target_os = "macos")]
#[ignore = "requires macOS graphical desktop, T64_UI_ROOT, and T64_UI_HELPER; explicit CUA acceptance"]
fn codex_plugin_compat_t64_real_macos_mcp_page() {
    let root = PathBuf::from(std::env::var("T64_UI_ROOT").expect("isolated UI control directory"));
    let helper = PathBuf::from(std::env::var("T64_UI_HELPER").expect("native application helper"));
    assert!(root.is_absolute() && root.is_dir());
    assert!(!root.join("ui-finish").exists(), "stale acceptance marker");
    let source = root.join("中文 MCP 原包 with spaces");
    fs::create_dir(&source).unwrap();
    let fixtures =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vw-agent/testdata/plugin_compat/t31");
    let original: Vec<_> = ["server.py", "app.html"]
        .into_iter()
        .map(|name| {
            let bytes = fs::read(fixtures.join(name)).unwrap();
            fs::write(source.join(name), &bytes).unwrap();
            (name, bytes)
        })
        .collect();
    let mut server = LocalServer(
        Command::new("python3")
            .arg(source.join("server.py"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut input = server.0.stdin.take().unwrap();
    let mut output = BufReader::new(server.0.stdout.take().unwrap());
    let mut request = |method: &str, params: Value, id: u32| {
        writeln!(input, "{}", json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["id"], id);
        assert!(response.get("error").is_none(), "{response}");
        response["result"].clone()
    };
    assert_eq!(request("initialize", json!({}), 1)["serverInfo"]["name"], "panel-qa");
    assert_eq!(request("tools/list", json!({}), 2)["tools"][0]["name"], "show");
    let result = request("tools/call", json!({"name":"show","arguments":{}}), 3);
    assert_eq!(result["content"][0]["text"], "本地服务结果");
    let resource = request("resources/read", json!({"uri":"ui://panel/view"}), 4);
    let app = json!({"server":"panel-qa","tool":"show","execution_binding":"t64-local-ui-only",
        "resource_uri":"ui://panel/view","resource":resource});
    let client = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .unwrap();
    let panel =
        McpAppPanel::launch(&helper, "t64-macos-ui".into(), &app, json!({}), result, client)
            .unwrap();
    fs::write(root.join("ui-ready"), "actual native MCP application launched\n").unwrap();
    println!("T64_MACOS_UI_READY {}", root.display());
    let deadline = Instant::now() + Duration::from_secs(300);
    while !root.join("ui-finish").exists() && Instant::now() < deadline {
        assert!(!panel.failed(), "actual native panel failed before UI acceptance");
        std::thread::sleep(Duration::from_millis(100));
    }
    let accepted = root.join("ui-finish").exists();
    panel.cancel();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !panel.finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(accepted, "manual UI acceptance timed out");
    assert!(panel.finished() && !panel.failed(), "native panel must finish cleanly");
    drop(panel);
    drop(input);
    drop(server);
    for (name, bytes) in original {
        assert_eq!(fs::read(source.join(name)).unwrap(), bytes, "source package changed");
    }
    assert_eq!(fs::read_dir(&source).unwrap().count(), 2, "state leaked into source package");
    fs::write(
        root.join("ui-result.json"),
        json!({"native_panel_finished":true,
        "source_unchanged":true,"server_reaped":true,"unicode_spaced_source":true})
        .to_string(),
    )
    .unwrap();
}
