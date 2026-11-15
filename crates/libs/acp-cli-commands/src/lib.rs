//! ACP 命令行配置、状态查询与会话目标定位。

pub mod cli;

#[cfg(test)]
static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
