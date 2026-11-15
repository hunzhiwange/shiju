//! 库内修改代理配置的测试共用同一串行锁。
pub(crate) fn global_runtime_proxy_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    &LOCK
}
