//! 验证事件循环让出执行权时能够正常完成。

use super::eventloop::wait;

#[tokio::test]
async fn wait_yields_without_panicking() {
    wait().await;
}
