//! 库内代理配置测试的串行锁；宿主测试使用宿主同名支撑模块。
#[allow(dead_code)] // 宿主内包含源码时，测试共用宿主代理锁。
pub(crate) fn global_runtime_proxy_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    &LOCK
}
