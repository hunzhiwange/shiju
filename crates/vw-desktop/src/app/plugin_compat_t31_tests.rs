use serde_json::{Value, json};
use vw_desktop::app::components::mcp_app_panel::McpAppPanel;
fn resource(html: &str) -> Value {
    json!({"server":"server","tool":"allowed","execution_binding":"host-issued","resource_uri":"ui://app/view","resource":{"contents":[{"uri":"ui://app/view","mimeType":"text/html;profile=mcp-app","text":html}]}})
}
#[test]
fn codex_plugin_compat_t31_rejects_invalid_input() {
    for value in [json!({}), resource(""), {
        let mut v = resource("中文");
        v["resource"]["contents"][0]["uri"] = json!("ui://other");
        v
    }] {
        assert!(McpAppPanel::validate(&value).is_err());
    }
}
#[test]
fn codex_plugin_compat_t31_preserves_isolation() {
    let mut v = resource("中文");
    v["resource"]["contents"][0]["_meta"] =
        json!({"ui":{"csp":{"connectDomains":["https://example.com/*"]}}});
    assert!(McpAppPanel::validate(&v).is_err());
    v["resource"]["contents"][0]["_meta"] = json!({});
    v["execution_binding"] = json!("");
    assert!(McpAppPanel::validate(&v).is_err());
}
#[cfg(unix)]
#[test]
fn codex_plugin_compat_t31_destroys_app_after_session_close() {
    use std::{
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let helper = dir.path().join("helper.py");
    std::fs::write(&helper,"#!/usr/bin/env python3\nimport sys,json,os,time\nv=json.loads(sys.stdin.readline())\nopen(v['html'],'w').write(str(os.getpid()))\nwhile True: time.sleep(1)\n").unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let pidfile = dir.path().join("pid");
    let client = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .unwrap();
    let panel = McpAppPanel::launch(
        &helper,
        "session".into(),
        &resource(pidfile.to_str().unwrap()),
        json!({}),
        json!({}),
        client,
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !pidfile.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let pid = std::fs::read_to_string(pidfile).unwrap();
    let running = || {
        std::process::Command::new("kill")
            .args(["-0", pid.trim()])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    };
    assert!(running());
    drop(panel);
    let deadline = Instant::now() + Duration::from_secs(5);
    while running() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!running(), "session disposal must reap the actual helper process");
}

#[test]
#[ignore = "requires graphical desktop and VW_T31_HELPER; manual UI evidence"]
fn codex_plugin_compat_t31_real_local_mcp_page() {
    use std::io::{BufRead, Write};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vw-agent/testdata/plugin_compat/t31");
    let mut server = std::process::Command::new("python3")
        .arg(root.join("server.py"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = server.stdin.take().unwrap();
    let mut output = std::io::BufReader::new(server.stdout.take().unwrap());
    let mut request = |method: &str, params: Value, id: u32| {
        writeln!(input, "{}", json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str::<Value>(&line).unwrap()["result"].clone()
    };
    assert_eq!(request("initialize", json!({}), 1)["serverInfo"]["name"], "panel-qa");
    assert_eq!(request("tools/list", json!({}), 2)["tools"][0]["name"], "show");
    let mut result = request("tools/call", json!({"name":"show","arguments":{}}), 3);
    let mut raw = request("resources/read", json!({"uri":"ui://panel/view"}), 4);
    if let Ok(path) = std::env::var("VW_T31_PUBLIC_RESOURCE") {
        raw = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        result = serde_json::from_str(
            &std::fs::read_to_string(std::env::var("VW_T31_PUBLIC_RESULT").unwrap()).unwrap(),
        )
        .unwrap();
    }
    let mut app = resource(raw["contents"][0]["text"].as_str().unwrap());
    app["resource_uri"] = raw["contents"][0]["uri"].clone();
    app["resource"] = raw;
    let helper = std::env::var("VW_T31_HELPER").unwrap();
    let client = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .unwrap();
    let panel = McpAppPanel::launch(
        std::path::Path::new(&helper),
        "local-qa-session".into(),
        &app,
        json!({}),
        result,
        client,
    )
    .unwrap();
    println!("REAL_LOCAL_MCP_PAGE_READY");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    while std::time::Instant::now() < deadline
        && !std::path::Path::new("/tmp/shiju-t31-ui-finish").exists()
    {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panel.cancel();
    drop(panel);
    server.kill().unwrap();
    server.wait().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
}

#[test]
fn codex_plugin_compat_t31_limits_network_to_exact_https_origins() {
    let mut app = resource("中文");
    for origin in ["https://assets.example.com", "https://api.example.com:443"] {
        app["resource"]["contents"][0]["_meta"] =
            json!({"ui":{"csp":{"resourceDomains":[origin]}}});
        assert!(McpAppPanel::validate(&app).is_ok());
    }
    for origin in [
        "http://example.com",
        "https://*.example.com",
        "https://example.com/path",
        "https://example.com;script-src *",
        "https://example.com:0",
        "file:///tmp/private",
    ] {
        app["resource"]["contents"][0]["_meta"] = json!({"ui":{"csp":{"connectDomains":[origin]}}});
        assert!(McpAppPanel::validate(&app).is_err(), "{origin}");
    }
}

#[test]
fn codex_plugin_compat_t31_reports_missing_helper() {
    let client = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .unwrap();
    assert!(
        McpAppPanel::launch(
            std::path::Path::new("/nonexistent/t31-helper"),
            "session".into(),
            &resource("中文"),
            json!({}),
            json!({}),
            client
        )
        .is_err()
    );
}
