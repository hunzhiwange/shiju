//! 验证浏览器后端名称解析、别名与错误显示契约。

use super::*;

#[test]
fn parse_accepts_stable_aliases() {
    assert_eq!(
        BrowserBackendKind::parse("agent-browser").unwrap(),
        BrowserBackendKind::AgentBrowser
    );
    assert_eq!(BrowserBackendKind::parse("native").unwrap(), BrowserBackendKind::RustNative);
    assert_eq!(BrowserBackendKind::parse("computer_use").unwrap(), BrowserBackendKind::ComputerUse);
    assert_eq!(BrowserBackendKind::parse(" AUTO ").unwrap(), BrowserBackendKind::Auto);
    assert_eq!(
        BrowserBackendKind::parse("agentbrowser").unwrap(),
        BrowserBackendKind::AgentBrowser
    );
    assert_eq!(BrowserBackendKind::parse("computeruse").unwrap(), BrowserBackendKind::ComputerUse);
    assert_eq!(BrowserBackendKind::parse("Rust-Native").unwrap(), BrowserBackendKind::RustNative);
}

#[test]
fn names_are_stable_for_errors_and_config() {
    assert_eq!(BrowserBackendKind::AgentBrowser.as_str(), "agent_browser");
    assert_eq!(BrowserBackendKind::RustNative.as_str(), "rust_native");
    assert_eq!(BrowserBackendKind::ComputerUse.as_str(), "computer_use");
    assert_eq!(BrowserBackendKind::Auto.as_str(), "auto");
    assert_eq!(backend_name(ResolvedBackend::AgentBrowser), "agent_browser");
    assert_eq!(backend_name(ResolvedBackend::RustNative), "rust_native");
    assert_eq!(backend_name(ResolvedBackend::ComputerUse), "computer_use");
    assert_eq!(
        unavailable_action_for_backend_error("click", ResolvedBackend::AgentBrowser),
        "Action 'click' is unavailable for backend 'agent_browser'"
    );
}

#[test]
fn parse_rejects_unknown_backend() {
    let err = BrowserBackendKind::parse("mystery").expect_err("unknown backend should fail");
    assert!(err.to_string().contains("Unsupported browser backend"));
}
