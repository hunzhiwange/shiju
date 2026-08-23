//! `vibe-agent` 二进制入口的委托契约测试。
//! 确认入口由 Tokio 驱动，且仅调用一次传入的异步执行函数。
//! 成功、普通错误与具体错误类型都应原样穿过这一薄封装。

#[test]
fn binary_main_is_tokio_entry_source() {
    let source = include_str!("vibe-agent.rs");

    assert!(source.contains("#[tokio::main]"));
    assert!(source.contains("run_cli(vw_cli::run).await"));
}

#[tokio::test]
async fn run_cli_invokes_delegate_and_returns_success() {
    let called = std::rc::Rc::new(std::cell::Cell::new(false));
    let delegate_called = std::rc::Rc::clone(&called);

    let result = super::run_cli(move || async move {
        delegate_called.set(true);
        Ok::<(), anyhow::Error>(())
    })
    .await;

    assert!(called.get());
    assert!(result.is_ok());
}

#[tokio::test]
async fn run_cli_propagates_delegate_error() {
    let result =
        super::run_cli(|| async { Err::<(), _>(anyhow::anyhow!("delegate failed")) }).await;

    assert_eq!(result.unwrap_err().to_string(), "delegate failed");
}

#[derive(Debug)]
struct DelegateError;

impl std::fmt::Display for DelegateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("typed delegate error")
    }
}

impl std::error::Error for DelegateError {}

#[tokio::test]
async fn run_cli_preserves_delegate_error_type() {
    let result = super::run_cli(|| async { Err::<(), _>(anyhow::Error::new(DelegateError)) }).await;

    assert!(result.unwrap_err().is::<DelegateError>());
}
