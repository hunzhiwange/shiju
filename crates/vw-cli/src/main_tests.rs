//! 主二进制入口的结构与委托测试。
//! 确认测试模块连线、Tokio 入口和 `run_cli` 对异步代理的单次调用。
//! 委托的成功或失败结果应直接传递，入口层不改写业务语义。

#[test]
fn task_611_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("main_tests.rs"));
}

#[test]
fn binary_main_is_tokio_entry_source() {
    let source = include_str!("main.rs");

    assert!(source.contains("#[tokio::main]"));
    assert!(source.contains("run_cli(vw_cli::run).await"));
}

#[tokio::test]
async fn run_cli_returns_delegate_success() {
    let result = super::run_cli(|| async { Ok::<(), anyhow::Error>(()) }).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn run_cli_propagates_delegate_error() {
    let result =
        super::run_cli(|| async { Err::<(), _>(anyhow::anyhow!("delegate failed")) }).await;

    assert_eq!(result.unwrap_err().to_string(), "delegate failed");
}

#[tokio::test]
async fn run_cli_invokes_fn_once_delegate_exactly_once() {
    let mut calls = 0;
    let result = super::run_cli(|| {
        calls += 1;
        async { Ok::<(), anyhow::Error>(()) }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(calls, 1);
}
