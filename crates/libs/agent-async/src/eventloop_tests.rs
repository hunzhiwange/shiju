//! 验证事件循环让出执行权时能够正常完成。
//! 这是最小异步烟雾测试，不对调度器的具体唤醒顺序作额外假设。

use super::eventloop::wait;

#[tokio::test]
async fn wait_yields_without_panicking() {
    wait().await;
}
