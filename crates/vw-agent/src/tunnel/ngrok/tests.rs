//! NgrokTunnel 测试模块
//!
//! 本模块包含 `NgrokTunnel` 结构体的单元测试和集成测试，用于验证
//! ngrok 隧道的构造、生命周期管理和健康检查功能。
//!
//! # 测试范围
//!
//! - **构造函数测试**: 验证配置参数（如认证令牌和域名）是否正确存储
//! - **状态管理测试**: 验证隧道启动前的初始状态是否符合预期
//! - **错误处理测试**: 验证在没有启动进程时调用 `stop()` 的行为
//! - **健康检查测试**: 验证未启动状态下的健康检查返回值
//!
//! # 依赖
//!
//! - 使用 `tokio::test` 进行异步测试
//! - 依赖父模块 `NgrokTunnel` 的实现

use super::*;

/// 测试 NgrokTunnel 构造函数是否正确存储域名配置
///
/// # 测试目的
///
/// 验证通过 `NgrokTunnel::new()` 创建实例时，传入的可选域名参数
/// 是否被正确保存在实例的 `domain` 字段中。
///
/// # 测试场景
///
/// - 输入: 认证令牌 `"ngrok-token"` 和域名 `"my.ngrok.app"`
/// - 期望: `tunnel.domain` 应该是 `Some("my.ngrok.app")`
///
/// # 示例
///
/// ```ignore
/// let tunnel = NgrokTunnel::new("token".into(), Some("domain".into()));
/// assert_eq!(tunnel.domain, Some("domain".to_string()));
/// ```
#[test]
fn constructor_stores_domain() {
    // 创建 NgrokTunnel 实例，指定认证令牌和自定义域名
    let tunnel = NgrokTunnel::new("ngrok-token".into(), Some("my.ngrok.app".into()));

    // 验证域名是否被正确存储（解包 Option 并比较字符串内容）
    assert_eq!(tunnel.domain.as_deref(), Some("my.ngrok.app"));
}

#[test]
fn name_identifies_ngrok_provider() {
    let tunnel = NgrokTunnel::new("ngrok-token".into(), None);
    assert_eq!(tunnel.name(), "ngrok");
}

/// 测试在隧道启动前 public_url() 返回 None
///
/// # 测试目的
///
/// 验证在尚未调用 `start()` 方法启动隧道之前，
/// `public_url()` 方法应该返回 `None`，表示没有可用的公共 URL。
///
/// # 测试场景
///
/// - 输入: 认证令牌 `"ngrok-token"`，域名为 `None`
/// - 状态: 隧道未启动
/// - 期望: `public_url()` 返回 `None`
///
/// # 设计原理
///
/// 这个测试确保了隧道的初始状态是干净的，没有残留的 URL 信息，
/// 防止在使用未启动的隧道时产生误导性的 URL。
#[test]
fn public_url_is_none_before_start() {
    // 创建 NgrokTunnel 实例，不指定域名
    let tunnel = NgrokTunnel::new("ngrok-token".into(), None);

    // 验证在未启动状态下，公共 URL 应该是 None
    assert!(tunnel.public_url().is_none());
}

#[test]
fn public_url_returns_none_while_process_state_is_locked() {
    let tunnel = NgrokTunnel::new("ngrok-token".into(), None);
    let _guard = tunnel.proc.try_lock().expect("process state lock");

    assert!(tunnel.public_url().is_none());
}

/// 测试在没有启动进程时调用 stop() 是否安全返回 Ok
///
/// # 测试目的
///
/// 验证 `stop()` 方法的幂等性和容错性：即使隧道进程从未启动，
/// 调用 `stop()` 也不应该返回错误，而是应该优雅地返回 `Ok(())`。
///
/// # 测试场景
///
/// - 输入: 认证令牌 `"ngrok-token"`，域名为 `None`
/// - 状态: 隧道未启动，内部进程句柄应该是 None
/// - 操作: 调用 `stop()` 方法
/// - 期望: 返回 `Ok(())`
///
/// # 异步说明
///
/// 此测试使用 `#[tokio::test]` 宏，因为 `stop()` 是一个异步方法，
/// 需要在 tokio 运行时环境中执行。
///
/// # 设计原理
///
/// 这个测试确保了 `stop()` 方法符合"幂等性"原则：
/// - 可以安全地多次调用
/// - 即使在非预期状态下也不会 panic 或返回错误
/// - 允许调用者无需检查隧道状态即可安全停止
#[tokio::test]
async fn stop_without_started_process_is_ok() {
    // 创建 NgrokTunnel 实例，不启动隧道
    let tunnel = NgrokTunnel::new("ngrok-token".into(), None);

    // 在未启动状态下调用 stop()，验证返回 Ok
    let result = tunnel.stop().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn start_returns_error_when_ngrok_command_is_unavailable() {
    let tunnel = NgrokTunnel::new("ngrok-token".into(), Some("my.ngrok.app".into()));

    let error = tunnel
        .start("localhost", 3000)
        .await
        .expect_err("unit-test ngrok command should be unavailable")
        .to_string();

    assert!(
        error.contains("No such file")
            || error.contains("not found")
            || error.contains("cannot find")
            || error.contains("os error")
    );
    assert!(tunnel.public_url().is_none());
}

/// 测试在隧道启动前 health_check() 返回 false
///
/// # 测试目的
///
/// 验证在隧道尚未启动时，`health_check()` 方法应该返回 `false`，
/// 表示隧道不健康或不可用。
///
/// # 测试场景
///
/// - 输入: 认证令牌 `"ngrok-token"`，域名为 `None`
/// - 状态: 隧道未启动
/// - 期望: `health_check()` 返回 `false`
///
/// # 异步说明
///
/// 此测试使用 `#[tokio::test]` 宏，因为 `health_check()` 是一个异步方法，
/// 可能涉及网络 I/O 或其他异步操作（如检查进程状态、连接检测等）。
///
/// # 设计原理
///
/// 健康检查遵循"默认拒绝"原则：
/// - 未启动的隧道不应该被认为是健康的
/// - 调用者可以依赖 `health_check()` 来判断是否可以使用该隧道
/// - 防止在隧道不可用时错误地认为服务可用
#[tokio::test]
async fn health_check_is_false_before_start() {
    // 创建 NgrokTunnel 实例，不启动隧道
    let tunnel = NgrokTunnel::new("ngrok-token".into(), None);

    // 验证在未启动状态下，健康检查应该返回 false
    assert!(!tunnel.health_check().await);
}

#[tokio::test]
async fn public_url_and_health_check_reflect_stored_running_process() -> Result<()> {
    let tunnel = NgrokTunnel::new("ngrok-token".into(), None);
    let child = tokio_command("sleep").arg("5").kill_on_drop(true).spawn()?;
    {
        let mut guard = tunnel.proc.lock().await;
        *guard = Some(TunnelProcess { child, public_url: "https://example.ngrok.app".to_string() });
    }

    assert_eq!(tunnel.public_url().as_deref(), Some("https://example.ngrok.app"));
    assert!(tunnel.health_check().await);

    tunnel.stop().await?;
    assert!(tunnel.public_url().is_none());
    Ok(())
}

#[test]
fn extract_ngrok_public_url_reads_logfmt_url_field() {
    assert_eq!(
        extract_ngrok_public_url("lvl=info msg=start url=https://demo.ngrok.app region=us")
            .as_deref(),
        Some("https://demo.ngrok.app")
    );
    assert_eq!(
        extract_ngrok_public_url("url=https://demo.ngrok.app/path?q=1\tstatus=ok").as_deref(),
        Some("https://demo.ngrok.app/path?q=1")
    );
    assert!(extract_ngrok_public_url("lvl=info msg=no-url").is_none());
}
