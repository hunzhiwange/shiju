use super::build_init_script;
use std::path::Path;

#[test]
fn build_init_script_escapes_json_values_and_embeds_cookie_config() {
    let script = build_init_script(
        Path::new("/tmp/path with \"quote\""),
        r#"[{"name":"sid","domain":"example.com"}]"#,
        "Agent \"A\"",
    )
    .unwrap();

    assert!(script.contains(r#"const dataPath = "/tmp/path with \"quote\"";"#));
    assert!(script.contains(r#"const userAgent = "Agent \"A\"";"#));
    assert!(script.contains(r#"const cookieConfigs = [{"name":"sid","domain":"example.com"}];"#));
    assert!(script.contains("function initSessionSync()"));
    assert!(script.contains("history.pushState = function"));
    assert!(script.contains("history.replaceState = function"));
    assert!(script.contains("window.addEventListener('popstate', reportNavigation)"));
    assert!(script.contains("window.ipc.postMessage"));
}
