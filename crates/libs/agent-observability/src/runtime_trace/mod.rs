//! 运行时追踪事件的本地 JSONL 存储。
//!
//! 本模块负责把可审计的运行时事件写入工作区内文件，并提供按事件类型、文本和 ID 查询的读取入口。
//! 写入路径使用私有文件权限和显式存储模式，避免在未配置时悄悄持久化运行时数据。

use config_observability::ObservabilityConfig;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};
use uuid::Uuid;

const DEFAULT_TRACE_REL_PATH: &str = "state/runtime-trace.jsonl";

/// 运行时追踪事件的持久化模式。
///
/// 该枚举由配置字符串解析而来，未知值会回退为 `None`，保持默认不落盘。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTraceStorageMode {
    /// 不写入任何追踪事件。
    None,
    /// 只保留最近的固定数量事件。
    Rolling,
    /// 追加保存所有事件。
    Full,
}

impl RuntimeTraceStorageMode {
    fn from_raw(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "rolling" => Self::Rolling,
            "full" => Self::Full,
            _ => Self::None,
        }
    }
}

/// 单条运行时追踪事件。
///
/// 字段保持扁平结构，便于 UI、CLI 和外部诊断工具按行读取 JSONL。可选字段仅在存在时序列化，
/// 避免将空值误解释为真实事件属性。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeTraceEvent {
    /// 事件唯一 ID。
    pub id: String,
    /// RFC3339 格式的 UTC 时间戳。
    pub timestamp: String,
    /// 事件类型标识。
    pub event_type: String,
    /// 关联通道名。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// 关联模型提供商。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// 关联模型名。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 关联 turn ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    /// 操作是否成功。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// 面向诊断的简短消息。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 事件扩展载荷。
    #[serde(default)]
    pub payload: Value,
}

struct RuntimeTraceLogger {
    mode: RuntimeTraceStorageMode,
    max_entries: usize,
    path: PathBuf,
    write_lock: std::sync::Mutex<()>,
}

impl RuntimeTraceLogger {
    fn new(mode: RuntimeTraceStorageMode, max_entries: usize, path: PathBuf) -> Self {
        Self { mode, max_entries: max_entries.max(1), path, write_lock: std::sync::Mutex::new(()) }
    }

    fn append(&self, event: &RuntimeTraceEvent) -> Result<()> {
        if self.mode == RuntimeTraceStorageMode::None {
            return Ok(());
        }

        // 追踪文件按行追加和裁剪，互斥写入可避免并发事件交错造成 JSONL 损坏。
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let line = serde_json::to_string(event)?;
        let mut options = OpenOptions::new();
        options.create(true).append(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // 追踪可能包含路径、模型名或错误摘要，创建时即限制为用户私有读写。
            options.mode(0o600);
        }

        let mut file = options.open(&self.path)?;
        writeln!(file, "{line}")?;
        file.sync_data()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // 现有文件权限可能来自旧版本或用户手动创建，写入后再次收紧权限。
            let _ = fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
        }

        if self.mode == RuntimeTraceStorageMode::Rolling {
            self.trim_to_last_entries()?;
        }

        Ok(())
    }

    fn trim_to_last_entries(&self) -> Result<()> {
        let raw = fs::read_to_string(&self.path).unwrap_or_default();
        let lines: Vec<&str> = raw.lines().map(str::trim).filter(|line| !line.is_empty()).collect();

        if lines.len() <= self.max_entries {
            return Ok(());
        }

        let keep_from = lines.len().saturating_sub(self.max_entries);
        let kept = &lines[keep_from..];
        let mut rewritten = kept.join("\n");
        rewritten.push('\n');

        // 先写临时文件再 rename，避免裁剪过程中崩溃导致原追踪文件被截断。
        let tmp = self.path.with_extension(format!(
            "tmp.{}.{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::write(&tmp, rewritten)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }

        fs::rename(tmp, &self.path)?;
        Ok(())
    }
}

static TRACE_LOGGER: LazyLock<RwLock<Option<Arc<RuntimeTraceLogger>>>> =
    LazyLock::new(|| RwLock::new(None));

/// 将观测配置解析为运行时追踪存储模式。
///
/// # 参数
///
/// - `config`: 全局观测配置。
///
/// # 返回值
///
/// 返回明确的 `RuntimeTraceStorageMode`。未知配置值会记录 warning 并回退到 `None`。
pub fn storage_mode_from_config(config: &ObservabilityConfig) -> RuntimeTraceStorageMode {
    let mode = RuntimeTraceStorageMode::from_raw(&config.runtime_trace_mode);
    if mode == RuntimeTraceStorageMode::None
        && !config.runtime_trace_mode.trim().is_empty()
        && !config.runtime_trace_mode.eq_ignore_ascii_case("none")
    {
        tracing::warn!(
            mode = %config.runtime_trace_mode,
            "Unknown observability.runtime_trace_mode; falling back to none"
        );
    }
    mode
}

/// 解析运行时追踪文件路径。
///
/// # 参数
///
/// - `config`: 观测配置。
/// - `workspace_dir`: 当前工作区目录。
///
/// # 返回值
///
/// 返回绝对路径；相对配置会落在工作区下，空配置使用默认 `state/runtime-trace.jsonl`。
pub fn resolve_trace_path(config: &ObservabilityConfig, workspace_dir: &Path) -> PathBuf {
    let raw = config.runtime_trace_path.trim();
    let fallback = workspace_dir.join(DEFAULT_TRACE_REL_PATH);
    if raw.is_empty() {
        return fallback;
    }

    let configured = PathBuf::from(raw);
    if configured.is_absolute() { configured } else { workspace_dir.join(configured) }
}

/// 从配置初始化全局运行时追踪 logger。
///
/// # 参数
///
/// - `config`: 观测配置。
/// - `workspace_dir`: 当前工作区目录。
///
/// # 错误处理
///
/// 本函数不返回错误；仅更新内存中的 logger。实际文件写入错误会在记录事件时降级为 warning。
pub fn init_from_config(config: &ObservabilityConfig, workspace_dir: &Path) {
    let mode = storage_mode_from_config(config);
    let logger = if mode == RuntimeTraceStorageMode::None {
        None
    } else {
        Some(Arc::new(RuntimeTraceLogger::new(
            mode,
            config.runtime_trace_max_entries.max(1),
            resolve_trace_path(config, workspace_dir),
        )))
    };

    let mut guard = TRACE_LOGGER.write().unwrap_or_else(|e| e.into_inner());
    *guard = logger;
}

/// 记录一条运行时追踪事件。
///
/// # 参数
///
/// - `event_type`: 事件类型。
/// - `channel`: 可选通道。
/// - `provider`: 可选模型提供商。
/// - `model`: 可选模型名。
/// - `turn_id`: 可选 turn ID。
/// - `success`: 可选成功状态。
/// - `message`: 可选诊断消息。
/// - `payload`: 结构化扩展载荷。
///
/// # 错误处理
///
/// 写入失败不会向上传播，只记录 warning，避免观测路径中断主业务流程。
pub fn record_event(
    event_type: &str,
    channel: Option<&str>,
    provider: Option<&str>,
    model: Option<&str>,
    turn_id: Option<&str>,
    success: Option<bool>,
    message: Option<&str>,
    payload: Value,
) {
    let logger = TRACE_LOGGER.read().unwrap_or_else(|e| e.into_inner()).clone();
    let Some(logger) = logger else {
        // 未启用追踪时保持零副作用，不创建目录或空文件。
        return;
    };

    let event = RuntimeTraceEvent {
        id: Uuid::new_v4().to_string(),
        timestamp: Utc::now().to_rfc3339(),
        event_type: event_type.to_string(),
        channel: channel.map(str::to_string),
        provider: provider.map(str::to_string),
        model: model.map(str::to_string),
        turn_id: turn_id.map(str::to_string),
        success,
        message: message.map(str::to_string),
        payload,
    };

    if let Err(err) = logger.append(&event) {
        tracing::warn!("Failed to write runtime trace event: {err}");
    }
}

/// 从 JSONL 文件加载运行时追踪事件。
///
/// # 参数
///
/// - `path`: 追踪文件路径。
/// - `limit`: 最多返回的最近事件数量。
/// - `event_filter`: 可选事件类型过滤。
/// - `contains`: 可选大小写不敏感文本过滤。
///
/// # 返回值
///
/// 返回按时间倒序排列的事件列表。
///
/// # 错误
///
/// 文件读取失败会返回错误；单行 JSON 解析失败会跳过该行并记录 warning。
pub fn load_events(
    path: &Path,
    limit: usize,
    event_filter: Option<&str>,
    contains: Option<&str>,
) -> Result<Vec<RuntimeTraceEvent>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(path)?;
    let mut events = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<RuntimeTraceEvent>(trimmed) {
            Ok(event) => events.push(event),
            // 单行损坏不应阻断完整追踪读取，尤其是进程异常退出后的半行写入。
            Err(err) => tracing::warn!("Skipping malformed runtime trace line: {err}"),
        }
    }

    if let Some(filter) = event_filter.map(str::trim).filter(|f| !f.is_empty()) {
        let normalized = filter.to_ascii_lowercase();
        events.retain(|event| event.event_type.to_ascii_lowercase() == normalized);
    }

    if let Some(needle) = contains.map(str::trim).filter(|s| !s.is_empty()) {
        let needle = needle.to_ascii_lowercase();
        events.retain(|event| {
            let mut haystack = format!(
                "{} {} {}",
                event.event_type,
                event.message.as_deref().unwrap_or_default(),
                event.payload
            );
            if let Some(channel) = &event.channel {
                haystack.push_str(channel);
            }
            if let Some(provider) = &event.provider {
                haystack.push_str(provider);
            }
            if let Some(model) = &event.model {
                haystack.push_str(model);
            }
            haystack.to_ascii_lowercase().contains(&needle)
        });
    }

    if events.len() > limit {
        let keep_from = events.len() - limit;
        events = events.split_off(keep_from);
    }

    events.reverse();
    Ok(events)
}

/// 按 ID 查找单条追踪事件。
///
/// # 参数
///
/// - `path`: 追踪文件路径。
/// - `id`: 要查找的事件 ID。
///
/// # 返回值
///
/// 找到时返回 `Some(RuntimeTraceEvent)`；文件不存在或未命中时返回 `None`。
///
/// # 错误
///
/// 文件读取失败会返回错误；无法解析的行会被跳过。
pub fn find_event_by_id(path: &Path, id: &str) -> Result<Option<RuntimeTraceEvent>> {
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(path)?;
    for line in raw.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<RuntimeTraceEvent>(trimmed) {
            if event.id == id {
                return Ok(Some(event));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests;
