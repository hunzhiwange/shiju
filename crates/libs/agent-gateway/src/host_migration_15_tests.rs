//! Domain-owned global and integration handler regression coverage.
mod global;
mod misc;
mod agent_copy;
mod integrations;
#[cfg(unix)]
mod hermes_memory;

fn global_async_env_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    &LOCK
}
fn global_env_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    &LOCK
}
