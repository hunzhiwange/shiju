//! 验证 Cloudflare 隧道实现的基础生命周期行为。
//! 这些测试聚焦未启动状态和构造器语义，避免在单元测试中启动外部进程。

use super::*;

#[test]
fn constructor_stores_token() {
    let tunnel = CloudflareTunnel::new("cf-token".into());
    assert_eq!(tunnel.token, "cf-token");
}

#[test]
fn name_identifies_cloudflare_provider() {
    let tunnel = CloudflareTunnel::new("cf-token".into());
    assert_eq!(tunnel.name(), "cloudflare");
}

#[test]
fn public_url_is_none_before_start() {
    let tunnel = CloudflareTunnel::new("cf-token".into());
    assert!(tunnel.public_url().is_none());
}

#[tokio::test]
async fn stop_without_started_process_is_ok() {
    let tunnel = CloudflareTunnel::new("cf-token".into());
    let result = tunnel.stop().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn start_returns_error_when_cloudflared_command_is_unavailable() {
    let tunnel = CloudflareTunnel::new("cf-token".into());

    let error = tunnel
        .start("localhost", 3000)
        .await
        .expect_err("unit-test cloudflared command should be unavailable")
        .to_string();

    assert!(
        error.contains("No such file")
            || error.contains("not found")
            || error.contains("cannot find")
            || error.contains("os error")
    );
    assert!(tunnel.public_url().is_none());
}

#[tokio::test]
async fn health_check_is_false_before_start() {
    let tunnel = CloudflareTunnel::new("cf-token".into());
    assert!(!tunnel.health_check().await);
}

#[tokio::test]
async fn public_url_and_health_check_reflect_stored_running_process() -> Result<()> {
    let tunnel = CloudflareTunnel::new("cf-token".into());
    let child = tokio_command("sleep").arg("5").kill_on_drop(true).spawn()?;
    {
        let mut guard = tunnel.proc.lock().await;
        *guard = Some(TunnelProcess {
            child,
            public_url: "https://example.trycloudflare.com".to_string(),
        });
    }

    assert_eq!(tunnel.public_url().as_deref(), Some("https://example.trycloudflare.com"));
    assert!(tunnel.health_check().await);

    tunnel.stop().await?;
    assert!(tunnel.public_url().is_none());
    Ok(())
}

#[tokio::test]
async fn public_url_returns_none_while_process_state_is_locked() {
    let tunnel = CloudflareTunnel::new("cf-token".into());
    let _guard = tunnel.proc.lock().await;

    assert!(tunnel.public_url().is_none());
}

#[test]
fn extract_cloudflare_public_url_stops_at_whitespace() {
    assert_eq!(
        extract_cloudflare_public_url(
            "INF request url https://preview.trycloudflare.com/path?q=1 next"
        )
        .as_deref(),
        Some("https://preview.trycloudflare.com/path?q=1")
    );
    assert_eq!(
        extract_cloudflare_public_url("https://preview.trycloudflare.com\tready").as_deref(),
        Some("https://preview.trycloudflare.com")
    );
    assert!(extract_cloudflare_public_url("no public URL yet").is_none());
}
