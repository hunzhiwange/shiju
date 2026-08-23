//! 验证超时包装器区分及时完成与超过期限的异步任务。

use super::timeout::with_timeout;
use std::time::Duration;

#[tokio::test]
async fn returns_value_before_timeout_and_error_after_timeout() {
    assert_eq!(with_timeout(async { 1 }, 50).await, Ok(1));
    let err = with_timeout(async { tokio::time::sleep(Duration::from_millis(20)).await }, 1)
        .await
        .expect_err("timeout");
    assert!(err.contains("1ms"));
}
