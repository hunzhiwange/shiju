//! Telegram 身份规范化、白名单绑定及托管服务重载提示。
//!
//! 绑定只更新 Telegram 准入列表；服务重启能力尚未实现，因此调用方会收到明确提示。

use super::*;

/// 去除首尾空白及可选的 `@` 前缀，得到可比较的 Telegram 身份。
pub(crate) fn normalize_telegram_identity(value: &str) -> String {
    value.trim().trim_start_matches('@').to_string()
}

/// 尝试重启受管守护进程；当前未发现可管理服务时返回 `false`。
pub(crate) fn maybe_restart_managed_daemon_service() -> anyhow::Result<bool> {
    // TODO: Implement actual service restart logic if needed
    Ok(false)
}

/// 将守护进程重载结果转换为面向命令行用户的后续操作提示。
pub(crate) fn report_managed_daemon_restart_result(result: anyhow::Result<bool>) {
    match result {
        Ok(true) => {
            println!("🔄 Detected running managed daemon service; reloaded automatically.");
        }
        Ok(false) => {
            println!(
                "ℹ️ No managed daemon service detected. If `vibewindow daemon`/`channel start` is already running, restart it to load the updated allowlist."
            );
        }
        Err(e) => {
            eprintln!(
                "⚠️ Allowlist saved, but failed to reload daemon service automatically: {e}\n\
                 Restart service manually with `vibewindow service stop && vibewindow service start`."
            );
        }
    }
}

/// 将身份加入 Telegram 白名单并持久化更新后的配置。
pub(crate) async fn bind_telegram_identity(config: &Config, identity: &str) -> Result<()> {
    let normalized = normalize_telegram_identity(identity);
    if normalized.is_empty() {
        anyhow::bail!("Telegram identity cannot be empty");
    }

    let mut updated = config.clone();
    let Some(telegram) = updated.channels_config.telegram.as_mut() else {
        anyhow::bail!("Telegram channel is not configured.");
    };

    if telegram.allowed_users.iter().any(|u| u == "*") {
        println!(
            "⚠️ Telegram allowlist is currently wildcard (`*`) — binding is unnecessary until you remove '*'."
        );
    }

    if telegram
        .allowed_users
        .iter()
        .map(|entry| normalize_telegram_identity(entry))
        .any(|entry| entry == normalized)
    {
        println!("✅ Telegram identity already bound: {normalized}");
        return Ok(());
    }

    telegram.allowed_users.push(normalized.clone());
    save_config(&updated).await?;
    println!("✅ Bound Telegram identity: {normalized}");
    println!("   Saved to the local tenant system config database");
    report_managed_daemon_restart_result(maybe_restart_managed_daemon_service());
    Ok(())
}

#[cfg(test)]
#[path = "telegram_tests.rs"]
mod telegram_tests;
