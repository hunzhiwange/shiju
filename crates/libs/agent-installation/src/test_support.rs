//! 安装测试共享的进程环境锁。
use std::sync::Mutex;

pub(crate) fn global_env_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}
