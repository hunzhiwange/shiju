//! Daemon 运行状态诊断。
//!
//! 本模块读取 daemon 写出的状态快照，检查主心跳、调度器与各 channel 的
//! 最近成功时间。诊断只消费本地状态文件，不主动连接 daemon，因此适合在
//! 故障排查时提供低风险的只读健康信号。

use super::{
    CHANNEL_STALE_SECONDS, DAEMON_STALE_SECONDS, DiagItem, SCHEDULER_STALE_SECONDS,
    utils::parse_rfc3339,
};
use agent_config::runtime::Config;
use chrono::{DateTime, Utc};

/// 检查 daemon 状态文件并追加诊断项。
///
/// 参数：
/// - `config`：用于定位 daemon 状态文件的运行配置。
/// - `items`：收集诊断结果的输出列表。
///
/// 错误处理：
/// 本函数不会向调用者返回错误；状态文件缺失、读取失败、JSON 解析失败或时间戳
/// 无效都会转换为 `DiagItem::error`，让 doctor 汇总输出保持完整。
pub(super) fn check_daemon_state(config: &Config, items: &mut Vec<DiagItem>) {
    let cat = "daemon";
    let state_file = agent_daemon::state_file_path(config);

    if !state_file.exists() {
        items.push(DiagItem::error(
            cat,
            format!("state file not found: {} — is the daemon running?", state_file.display()),
        ));
        return;
    }

    let raw = match std::fs::read_to_string(&state_file) {
        Ok(raw) => raw,
        Err(err) => {
            items.push(DiagItem::error(cat, format!("cannot read state file: {err}")));
            return;
        }
    };

    let snapshot: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            items.push(DiagItem::error(cat, format!("invalid state JSON: {err}")));
            return;
        }
    };

    let updated_at = snapshot.get("updated_at").and_then(serde_json::Value::as_str).unwrap_or("");
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(updated_at) {
        let age = Utc::now().signed_duration_since(timestamp.with_timezone(&Utc)).num_seconds();
        if age <= DAEMON_STALE_SECONDS {
            items.push(DiagItem::ok(cat, format!("heartbeat fresh ({age}s ago)")));
        } else {
            items.push(DiagItem::error(cat, format!("heartbeat stale ({age}s ago)")));
        }
    } else {
        items.push(DiagItem::error(cat, format!("invalid daemon timestamp: {updated_at}")));
    }

    if let Some(components) = snapshot.get("components").and_then(serde_json::Value::as_object) {
        if let Some(scheduler) = components.get("scheduler") {
            let scheduler_ok = scheduler
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|status| status == "ok");
            let scheduler_age = scheduler
                .get("last_ok")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_rfc3339)
                .map_or(i64::MAX, |dt| Utc::now().signed_duration_since(dt).num_seconds());

            if scheduler_ok && scheduler_age <= SCHEDULER_STALE_SECONDS {
                items.push(DiagItem::ok(
                    cat,
                    format!("scheduler healthy (last ok {scheduler_age}s ago)"),
                ));
            } else {
                items.push(DiagItem::error(
                    cat,
                    format!("scheduler unhealthy (ok={scheduler_ok}, age={scheduler_age}s)"),
                ));
            }
        } else {
            // 旧版本 daemon 可能尚未上报调度器组件；这里用警告保留兼容信号，
            // 避免把“未接入观测”误判为运行失败。
            items.push(DiagItem::warn(cat, "scheduler component not tracked yet"));
        }

        let mut channel_count = 0u32;
        let mut stale = 0u32;
        for (name, component) in components {
            if !name.starts_with("channel:") {
                continue;
            }

            channel_count += 1;
            let status_ok = component
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|status| status == "ok");
            let age = component
                .get("last_ok")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_rfc3339)
                .map_or(i64::MAX, |dt| Utc::now().signed_duration_since(dt).num_seconds());

            if status_ok && age <= CHANNEL_STALE_SECONDS {
                items.push(DiagItem::ok(cat, format!("{name} fresh ({age}s ago)")));
            } else {
                stale += 1;
                items.push(DiagItem::error(
                    cat,
                    format!("{name} stale (ok={status_ok}, age={age}s)"),
                ));
            }
        }

        if channel_count == 0 {
            items.push(DiagItem::warn(cat, "no channel components tracked yet"));
        } else if stale > 0 {
            items.push(DiagItem::warn(cat, format!("{channel_count} channels, {stale} stale")));
        }
    }
}
