//! 库内修改代理配置的测试共用同一串行锁。
pub(crate) fn global_runtime_proxy_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    &LOCK
}
