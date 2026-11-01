use serde_json::json;
use vw_desktop::app::mcp_app_bridge::McpAppBridge;
fn bridge() -> McpAppBridge {
    McpAppBridge::new("session".into(), "server".into(), "null".into(), ["allowed".into()].into())
        .unwrap()
}
#[test]
fn codex_plugin_compat_t30_rejects_cross_server_rpc() {
    let mut b = bridge();
    assert!(
        b.receive(
            "session",
            "other",
            "null",
            r#"{"jsonrpc":"2.0","id":1,"method":"ui/initialize"}"#,
            |_, _, _, _| panic!("must not execute")
        )
        .is_err()
    );
}
#[test]
fn codex_plugin_compat_t30_rejects_invalid_input() {
    let mut b = bridge();
    for raw in [
        r#"{"jsonrpc":"2.0","method":"ui/initialize"}"#,
        "[]",
        "null",
        "{",
        r#"{"jsonrpc":"2.0","method":"native/exec"}"#,
    ] {
        assert!(
            b.receive("session", "server", "null", raw, |_, _, _, _| panic!("must not execute"))
                .is_err()
        );
    }
}
#[test]
fn codex_plugin_compat_t30_preserves_isolation() {
    let mut b = bridge();
    b.receive(
        "session",
        "server",
        "null",
        r#"{"jsonrpc":"2.0","id":1,"method":"ui/initialize"}"#,
        |_, _, _, _| unreachable!(),
    )
    .unwrap();
    b.receive(
        "session",
        "server",
        "null",
        r#"{"jsonrpc":"2.0","method":"ui/notifications/initialized"}"#,
        |_, _, _, _| unreachable!(),
    )
    .unwrap();
    let raw = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"allowed","arguments":{}}}"#;
    let result = b
        .receive("session", "server", "null", raw, |session, server, name, args| {
            assert_eq!((session, server, name), ("session", "server", "allowed"));
            assert_eq!(args, &json!({}));
            Ok(json!({"content":[]}))
        })
        .unwrap();
    assert_eq!(result["id"], 2);
    b.close();
    assert!(b.receive("session", "server", "null", raw, |_, _, _, _| panic!("closed")).is_err());
}

#[test]
#[ignore = "requires a built WebView helper and a graphical desktop"]
fn codex_plugin_compat_t30_real_webview_roundtrip() {
    use vw_desktop::app::mcp_app_bridge::McpAppProcess;
    let helper = std::env::var_os("VW_T30_HELPER").expect("VW_T30_HELPER is required");
    let html = r#"<meta charset="utf-8"><h1>真实桥接 · 中文</h1><script>
    parent.postMessage({jsonrpc:'2.0',id:1,method:'ui/initialize'},'*');
    addEventListener('message',e=>{if(e.source!==parent)return;if(e.data.id===1){parent.postMessage({jsonrpc:'2.0',method:'ui/notifications/initialized'},'*');parent.postMessage({jsonrpc:'2.0',id:2,method:'tools/call',params:{name:'allowed',arguments:{value:'中文'}}},'*')}});
    </script>"#;
    let mut process = McpAppProcess::launch(
        std::path::Path::new(&helper),
        "session".into(),
        "server".into(),
        html,
        ["allowed".into()].into(),
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut called = false;
    while !called && std::time::Instant::now() < deadline {
        process
            .poll(|session, server, name, arguments| {
                assert_eq!((session, server, name), ("session", "server", "allowed"));
                assert_eq!(arguments, &json!({"value":"中文"}));
                called = true;
                Ok(json!({"content":[{"type":"text","text":"approved"}]}))
            })
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    process.tool_result(json!({}), json!({"content":[]})).unwrap();
    process.close().unwrap();
    assert!(called, "real iframe must call through production host bridge");
}

#[test]
fn codex_plugin_compat_t30_host_permission_denial_is_preserved() {
    let mut b = bridge();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"ui/initialize"}"#;
    for (session, server, origin) in
        [("other", "server", "null"), ("session", "server", "https://evil.test")]
    {
        assert!(b.receive(session, server, origin, init, |_, _, _, _| unreachable!()).is_err());
    }
    b.receive("session", "server", "null", init, |_, _, _, _| unreachable!()).unwrap();
    b.receive(
        "session",
        "server",
        "null",
        r#"{"jsonrpc":"2.0","method":"ui/notifications/initialized"}"#,
        |_, _, _, _| unreachable!(),
    )
    .unwrap();
    let error = b.receive("session","server","null",r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"allowed","arguments":{}}}"#,|_,_,_,_|anyhow::bail!("permission denied by session")).unwrap_err();
    assert_eq!(error.to_string(), "permission denied by session");
}
