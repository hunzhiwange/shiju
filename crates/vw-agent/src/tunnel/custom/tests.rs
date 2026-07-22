//! 自定义隧道模块的单元测试
//!
//! 本模块包含对 `CustomTunnel` 的各项测试用例，覆盖以下场景：
//! - 命令验证：空命令应返回错误
//! - URL 生成：带/不带模式的 URL 提取与占位符替换
//! - 健康检查：不可达 URL 的处理

use super::*;

/// 测试：启动命令为空时应返回错误
///
/// # 场景
/// - 创建一个使用空格作为启动命令的 `CustomTunnel`
/// - 调用 `start` 方法
///
/// # 断言
/// - 结果应为错误
/// - 错误消息应包含 "start_command is empty"
#[tokio::test]
async fn start_with_empty_command_returns_error() {
    let tunnel = CustomTunnel::new("   ".into(), None, None);
    let result = tunnel.start("127.0.0.1", 8080).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("start_command is empty"));
}

/// 测试：未提供模式时应返回本地 URL
///
/// # 场景
/// - 创建一个使用 "sleep 1" 作为启动命令的 `CustomTunnel`
/// - 不提供健康检查 URL
/// - 不提供 URL 提取模式
///
/// # 断言
/// - `start` 返回的 URL 应为 `http://127.0.0.1:4455`
/// - `public_url` 应返回相同值
/// - `stop` 应正常完成
#[tokio::test]
async fn start_without_pattern_returns_local_url() {
    let tunnel = CustomTunnel::new("sleep 1".into(), None, None);

    let url = tunnel.start("127.0.0.1", 4455).await.unwrap();
    assert_eq!(url, "http://127.0.0.1:4455");
    assert_eq!(tunnel.public_url().as_deref(), Some("http://127.0.0.1:4455"));

    tunnel.stop().await.unwrap();
}

/// 测试：提供模式时应从命令输出提取 URL
///
/// # 场景
/// - 创建一个使用 "echo https://public.example" 作为启动命令的 `CustomTunnel`
/// - 提供 URL 提取模式 "public.example"
///
/// # 断言
/// - `start` 应从命令输出中提取 URL
/// - 返回的 URL 应为 `https://public.example`
/// - `public_url` 应返回相同值
/// - `stop` 应正常完成
#[tokio::test]
async fn start_with_pattern_extracts_url() {
    let tunnel = CustomTunnel::new(
        "echo https://public.example".into(),
        None,
        Some("public.example".into()),
    );

    let url = tunnel.start("localhost", 9999).await.unwrap();

    assert_eq!(url, "https://public.example");
    assert_eq!(tunnel.public_url().as_deref(), Some("https://public.example"));

    tunnel.stop().await.unwrap();
}

/// 测试：命令中的占位符 {host} 和 {port} 应被替换
///
/// # 场景
/// - 创建一个使用 "echo http://{host}:{port}" 作为启动命令的 `CustomTunnel`
/// - 提供 URL 提取模式 "http://"
///
/// # 断言
/// - 启动命令中的 {host} 应被替换为实际的主机地址
/// - 启动命令中的 {port} 应被替换为实际的端口
/// - 返回的 URL 应为 `http://10.1.2.3:4321`
/// - `stop` 应正常完成
#[tokio::test]
async fn start_replaces_host_and_port_placeholders() {
    let tunnel =
        CustomTunnel::new("echo http://{host}:{port}".into(), None, Some("http://".into()));

    let url = tunnel.start("10.1.2.3", 4321).await.unwrap();

    assert_eq!(url, "http://10.1.2.3:4321");
    tunnel.stop().await.unwrap();
}

/// 测试：健康检查 URL 不可达时应返回 false
///
/// # 场景
/// - 创建一个使用 "sleep 1" 作为启动命令的 `CustomTunnel`
/// - 提供一个无效的健康检查 URL `http://127.0.0.1:9/healthz`
///   - 端口 9 是无效端口（Discard 协议端口，通常不会响应 HTTP 请求）
///
/// # 断言
/// - `health_check` 应返回 `false`，表示健康检查失败
#[tokio::test]
async fn health_check_with_unreachable_health_url_returns_false() {
    let tunnel =
        CustomTunnel::new("sleep 1".into(), Some("http://127.0.0.1:9/healthz".into()), None);

    assert!(!tunnel.health_check().await);
}

#[tokio::test]
async fn public_url_returns_none_while_process_state_is_locked() {
    let tunnel = CustomTunnel::new("echo ready".into(), None, None);
    let _guard = tunnel.proc.lock().await;

    assert!(tunnel.public_url().is_none());
}
