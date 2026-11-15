//! 系统调用异常检测模块 - 守护进程 Shell/进程执行监控
//!
//! 本模块实现了一个有状态的系统调用异常检测器，主要用于：
//! - 监控命令执行的标准输出和标准错误流
//! - 从输出中提取系统调用相关的遥测信息（如 seccomp/audit 日志行）
//! - 检测偏离配置基线的异常模式
//! - 生成结构化的异常告警并记录到日志文件和审计系统
//!
//! # 主要功能
//!
//! - **未知系统调用检测**：检测不在基线白名单中的系统调用
//! - **拒绝系统调用检测**：在严格模式下检测被拒绝的系统调用
//! - **频率异常检测**：监控被拒绝事件和总事件的速率，防止滥用
//! - **告警去重与冷却**：避免相同告警的重复发送
//!
//! # 使用示例
//!
//! ```ignore
//! use syscall_anomaly::SyscallAnomalyDetector;
//! use vw_config_types::security::{SyscallAnomalyConfig, AuditConfig};
//!
//! let config = SyscallAnomalyConfig::default();
//! let audit_config = AuditConfig::default();
//! let detector = SyscallAnomalyDetector::new(config, "/var/lib/vibewindow", audit_config);
//!
//! // 检查命令输出
//! let alerts = detector.inspect_command_output(
//!     "ls -la",
//!     "file1.txt\nfile2.txt",
//!     "",
//!     Some(0)
//! );
//! ```

use security_audit::{AuditEvent, AuditEventType, AuditLogger};
use vw_config_types::security::{AuditConfig, SyscallAnomalyConfig};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// 速率统计的时间窗口（60秒）
/// 用于计算每分钟的事件数量和告警频率
const RATE_WINDOW: Duration = Duration::from_secs(60);

/// 告警样本的最大字符数
/// 防止日志中出现过长的样本内容
const MAX_ALERT_SAMPLE_CHARS: usize = 240;

/// 系统调用异常告警类型
///
/// 定义了系统调用异常检测器可能发出的告警类别。
/// 每种类型代表一种不同的异常模式。
///
/// # 告警类型
///
/// - `UnknownSyscall`: 检测到不在基线白名单中的系统调用
/// - `DeniedSyscall`: 在严格模式下检测到被拒绝的系统调用
/// - `DeniedRateExceeded`: 被拒绝事件的频率超过阈值
/// - `EventRateExceeded`: 总事件的频率超过阈值
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyscallAnomalyKind {
    /// 未知系统调用 - 不在配置的基线白名单中
    UnknownSyscall,
    /// 被拒绝的系统调用 - 在严格模式下被 seccomp 等机制拒绝
    DeniedSyscall,
    /// 被拒绝事件频率超限 - 每分钟被拒绝的次数超过配置阈值
    DeniedRateExceeded,
    /// 总事件频率超限 - 每分钟的系统调用事件总数超过配置阈值
    EventRateExceeded,
}

/// 系统调用异常告警结构
///
/// 表示一个完整的系统调用异常告警记录，包含告警的所有相关信息。
/// 该结构会被序列化为 JSON 并写入异常日志文件。
///
/// # 字段说明
///
/// - `timestamp`: 告警产生的时间戳（UTC）
/// - `kind`: 告警类型
/// - `command`: 触发告警的命令字符串
/// - `syscall`: 相关的系统调用名称（如果可以识别）
/// - `denied_events_last_minute`: 最近一分钟内被拒绝的事件数量
/// - `total_events_last_minute`: 最近一分钟内的总事件数量
/// - `sample`: 触发告警的原始日志行样本
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyscallAnomalyAlert {
    /// 告警产生的时间戳（UTC 时区）
    pub timestamp: DateTime<Utc>,
    /// 告警类型
    pub kind: SyscallAnomalyKind,
    /// 触发告警的命令字符串
    pub command: String,
    /// 相关的系统调用名称（可选）
    pub syscall: Option<String>,
    /// 最近一分钟内被拒绝的事件数量
    pub denied_events_last_minute: u32,
    /// 最近一分钟内的总事件数量
    pub total_events_last_minute: u32,
    /// 触发告警的原始日志行样本（截断至 MAX_ALERT_SAMPLE_CHARS 字符）
    pub sample: String,
}

/// 从命令输出中解析出的系统调用信号
///
/// 表示从标准错误或标准输出中提取的一个系统调用相关事件。
/// 包含系统调用名称、是否被拒绝以及原始日志行。
#[derive(Debug, Clone)]
struct ParsedSyscallSignal {
    /// 系统调用名称（如果可以识别）
    syscall: Option<String>,
    /// 该系统调用是否被拒绝（如被 seccomp 拦截）
    denied: bool,
    /// 原始日志行内容
    raw_line: String,
}

/// 观察到的事件记录
///
/// 用于在时间窗口内跟踪系统调用事件的发生时间和状态。
/// 这些记录会被存储在滑动窗口队列中，用于频率统计。
#[derive(Debug, Clone)]
struct ObservedEvent {
    /// 事件发生的时间点
    at: Instant,
    /// 该事件是否是被拒绝的系统调用
    denied: bool,
}

/// 告警去重键
///
/// 用于标识和去重告警的组合键。
/// 相同键的告警会根据冷却时间配置进行抑制。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AlertKey {
    /// 告警类型
    kind: SyscallAnomalyKind,
    /// 相关的系统调用名称（可选）
    syscall: Option<String>,
    /// 命令标识（命令的第一个词）
    command_id: String,
}

/// 检测器内部状态
///
/// 维护检测器的运行时状态，包括事件队列和告警历史。
/// 该状态受 Mutex 保护，支持并发访问。
///
/// # 字段说明
///
/// - `events`: 观察到的事件队列（滑动窗口）
/// - `alert_timestamps`: 告警时间戳队列（用于频率限制）
/// - `last_alert_by_key`: 每个告警键的最后告警时间（用于冷却）
#[derive(Debug, Default)]
struct DetectorState {
    /// 观察到的事件队列（滑动窗口，保留时间窗口内的事件）
    events: VecDeque<ObservedEvent>,
    /// 告警时间戳队列（用于实现每分钟告警数限制）
    alert_timestamps: VecDeque<Instant>,
    /// 每个告警键的最后告警时间（用于实现告警冷却）
    last_alert_by_key: HashMap<AlertKey, Instant>,
}

/// 系统调用异常检测器
///
/// 有状态的检测器，用于监控命令执行的系统调用行为并发出异常告警。
/// 检测器会：
/// 1. 解析命令输出中的系统调用相关日志
/// 2. 根据配置的基线白名单检测未知系统调用
/// 3. 监控系统调用事件的频率
/// 4. 生成去重的告警并记录到日志和审计系统
///
/// # 线程安全性
///
/// 该检测器使用 `Mutex` 保护内部状态，可以安全地在多线程环境中使用。
///
/// # 示例
///
/// ```ignore
/// let detector = SyscallAnomalyDetector::new(config, vibewindow_dir, audit_config);
/// let alerts = detector.inspect_command_output("ls", "", "syscall=execve", Some(0));
/// ```
pub struct SyscallAnomalyDetector {
    /// 检测器配置
    config: SyscallAnomalyConfig,
    /// 基线系统调用白名单（已规范化）
    baseline: HashSet<String>,
    /// 检测器内部状态（受 Mutex 保护）
    state: Mutex<DetectorState>,
    /// 异常日志文件路径
    anomaly_log_path: PathBuf,
    /// 审计日志记录器（可选）
    audit_logger: Option<AuditLogger>,
}

impl SyscallAnomalyDetector {
    /// 创建新的系统调用异常检测器实例
    ///
    /// 根据提供的配置初始化检测器，包括：
    /// - 规范化基线系统调用白名单
    /// - 解析日志文件路径
    /// - 初始化审计日志记录器
    ///
    /// # 参数
    ///
    /// - `config`: 系统调用异常检测配置
    /// - `vibewindow_dir`: VibeWindow 数据目录路径，用于解析相对路径
    /// - `audit_config`: 审计日志配置
    ///
    /// # 返回
    ///
    /// 返回初始化完成的检测器实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use vw_config_types::security::{SyscallAnomalyConfig, AuditConfig};
    ///
    /// let config = SyscallAnomalyConfig {
    ///     enabled: true,
    ///     alert_on_unknown_syscall: true,
    ///     ..Default::default()
    /// };
    /// let detector = SyscallAnomalyDetector::new(config, "/var/lib/vibewindow", AuditConfig::default());
    /// ```
    pub fn new(
        config: SyscallAnomalyConfig,
        vibewindow_dir: impl AsRef<Path>,
        audit_config: AuditConfig,
    ) -> Self {
        // 规范化基线系统调用名称（转小写、去空白）
        let baseline = normalize_baseline(&config.baseline_syscalls);
        // 解析日志文件路径（支持相对路径和绝对路径）
        let anomaly_log_path = resolve_log_path(vibewindow_dir.as_ref(), config.log_path.as_str());
        // 初始化审计日志记录器（如果失败则为 None）
        let audit_logger =
            AuditLogger::new(audit_config, vibewindow_dir.as_ref().to_path_buf()).ok();

        Self {
            config,
            baseline,
            state: Mutex::new(DetectorState::default()),
            anomaly_log_path,
            audit_logger,
        }
    }

    /// 检查命令输出并发出异常告警
    ///
    /// 这是检测器的主要入口方法，负责：
    /// 1. 解析命令输出中的系统调用相关日志
    /// 2. 更新事件统计和频率
    /// 3. 检测各种类型的异常（未知系统调用、拒绝调用、频率超限等）
    /// 4. 发出去重后的告警（写入日志、发送到审计系统）
    ///
    /// # 参数
    ///
    /// - `command`: 被执行的命令字符串
    /// - `stdout`: 命令的标准输出内容
    /// - `stderr`: 命令的标准错误输出内容
    /// - `exit_code`: 命令的退出码（可选）
    ///
    /// # 返回
    ///
    /// 返回本次检查发出的告警列表（主要用于测试和诊断）
    ///
    /// # 告警类型
    ///
    /// 该方法可能发出以下类型的告警：
    /// - `UnknownSyscall`: 检测到不在基线白名单中的系统调用
    /// - `DeniedSyscall`: 在严格模式下检测到被拒绝的系统调用
    /// - `DeniedRateExceeded`: 被拒绝事件的频率超过阈值
    /// - `EventRateExceeded`: 总事件的频率超过阈值
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let alerts = detector.inspect_command_output(
    ///     "curl https://example.com",
    ///     "HTTP/1.1 200 OK",
    ///     "syscall=connect",
    ///     Some(0)
    /// );
    /// for alert in alerts {
    ///     println!("Alert: {:?}", alert.kind);
    /// }
    /// ```
    pub fn inspect_command_output(
        &self,
        command: &str,
        stdout: &str,
        stderr: &str,
        exit_code: Option<i32>,
    ) -> Vec<SyscallAnomalyAlert> {
        // 如果检测器未启用，直接返回空列表
        if !self.config.enabled {
            return Vec::new();
        }

        // 从输出中提取系统调用信号
        let signals = extract_signals(stderr, stdout);
        if signals.is_empty() {
            return Vec::new();
        }

        let mut alerts: Vec<SyscallAnomalyAlert> = Vec::new();
        let now = Instant::now();
        let timestamp = Utc::now();

        // 获取状态锁并清理过期事件
        let mut state = self.state.lock();
        prune_old_events(&mut state.events, now);

        // 处理每个系统调用信号
        for signal in &signals {
            // 将事件添加到队列并清理过期事件
            state.events.push_back(ObservedEvent { at: now, denied: signal.denied });
            prune_old_events(&mut state.events, now);

            // 统计当前频率
            let denied_count = count_denied(&state.events);
            let total_count = u32::try_from(state.events.len()).unwrap_or(u32::MAX);

            // 检查未知系统调用
            if let Some(syscall) = signal.syscall.as_deref() {
                let normalized = normalize_syscall_name(syscall);
                let unknown = !self.baseline.contains(&normalized);
                // 如果启用了未知系统调用告警且该系统调用不在基线中
                if self.config.alert_on_unknown_syscall && unknown {
                    alerts.push(SyscallAnomalyAlert {
                        timestamp,
                        kind: SyscallAnomalyKind::UnknownSyscall,
                        command: command.to_string(),
                        syscall: Some(normalized),
                        denied_events_last_minute: denied_count,
                        total_events_last_minute: total_count,
                        sample: truncate_sample(&signal.raw_line),
                    });
                }
            }

            // 在严格模式下检查被拒绝的系统调用
            if self.config.strict_mode && signal.denied {
                alerts.push(SyscallAnomalyAlert {
                    timestamp,
                    kind: SyscallAnomalyKind::DeniedSyscall,
                    command: command.to_string(),
                    syscall: signal
                        .syscall
                        .as_deref()
                        .map(normalize_syscall_name)
                        .filter(|name| !name.is_empty()),
                    denied_events_last_minute: denied_count,
                    total_events_last_minute: total_count,
                    sample: truncate_sample(&signal.raw_line),
                });
            }
        }

        // 检查频率超限（被拒绝事件）
        let denied_count = count_denied(&state.events);
        let total_count = u32::try_from(state.events.len()).unwrap_or(u32::MAX);
        if denied_count > self.config.max_denied_events_per_minute {
            // 提取一个被拒绝事件的样本
            let sample = signals
                .iter()
                .find(|signal| signal.denied)
                .map_or_else(String::new, |signal| truncate_sample(&signal.raw_line));
            alerts.push(SyscallAnomalyAlert {
                timestamp,
                kind: SyscallAnomalyKind::DeniedRateExceeded,
                command: command.to_string(),
                syscall: None,
                denied_events_last_minute: denied_count,
                total_events_last_minute: total_count,
                sample,
            });
        }

        // 检查频率超限（总事件数）
        if total_count > self.config.max_total_events_per_minute {
            // 提取第一个事件作为样本
            let sample = signals
                .first()
                .map_or_else(String::new, |signal| truncate_sample(&signal.raw_line));
            alerts.push(SyscallAnomalyAlert {
                timestamp,
                kind: SyscallAnomalyKind::EventRateExceeded,
                command: command.to_string(),
                syscall: None,
                denied_events_last_minute: denied_count,
                total_events_last_minute: total_count,
                sample,
            });
        }

        // 去重并发送告警
        // 对本次检查的告警进行去重，避免重复告警
        let mut seen = HashSet::new();
        let mut emit_queue = Vec::new();
        for alert in alerts {
            let key = (alert.kind, alert.syscall.clone(), alert.sample.clone());
            if !seen.insert(key) {
                continue;
            }
            // 检查是否应该发送该告警（基于频率限制和冷却时间）
            if should_emit_alert(&mut state, &self.config, &alert, now) {
                emit_queue.push(alert);
            }
        }
        drop(state);

        // 发送所有待发送的告警
        for alert in &emit_queue {
            self.emit_alert(alert, exit_code);
        }

        emit_queue
    }

    /// 发送告警到日志和审计系统
    ///
    /// 将告警输出到多个目标：
    /// 1. 使用 tracing 宏输出到日志系统
    /// 2. 追加到异常日志文件
    /// 3. 发送到审计日志系统（如果配置）
    ///
    /// # 参数
    ///
    /// - `alert`: 要发送的告警
    /// - `exit_code`: 命令的退出码（用于审计日志）
    fn emit_alert(&self, alert: &SyscallAnomalyAlert, exit_code: Option<i32>) {
        // 使用 tracing 输出告警到日志
        tracing::warn!(
            target: "security::syscall_anomaly",
            kind = ?alert.kind,
            command = %alert.command,
            syscall = %alert.syscall.as_deref().unwrap_or("-"),
            denied_last_min = alert.denied_events_last_minute,
            total_last_min = alert.total_events_last_minute,
            sample = %alert.sample,
            "syscall anomaly detected"
        );

        // 追加到异常日志文件
        if let Err(error) = self.append_log_line(alert) {
            tracing::debug!("failed to append syscall anomaly log: {error}");
        }

        // 发送到审计日志系统
        if let Some(logger) = &self.audit_logger {
            // 构建审计事件
            let mut event = AuditEvent::new(AuditEventType::SecurityEvent)
                .with_actor("daemon".to_string(), None, None)
                .with_action(alert.command.clone(), "high".to_string(), true, false)
                .with_result(false, exit_code, 0, Some(alert.sample.clone()));
            // 标记为策略违规
            event.security.policy_violation = true;
            let _ = logger.log(&event);
        }
    }

    /// 追加告警到异常日志文件
    ///
    /// 将告警序列化为 JSON 并追加到配置的日志文件中。
    /// 如果日志文件的父目录不存在，会自动创建。
    ///
    /// # 参数
    ///
    /// - `alert`: 要追加的告警
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(())`，失败返回错误
    ///
    /// # 错误
    ///
    /// 可能的错误包括：
    /// - 无法创建父目录
    /// - 无法打开或写入文件
    /// - 序列化失败
    fn append_log_line(&self, alert: &SyscallAnomalyAlert) -> anyhow::Result<()> {
        // 确保父目录存在
        if let Some(parent) = self.anomaly_log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 序列化告警为 JSON
        let line = serde_json::to_string(alert)?;
        // 追加写入日志文件
        let mut file = OpenOptions::new().create(true).append(true).open(&self.anomaly_log_path)?;
        writeln!(file, "{line}")?;
        // 同步到磁盘
        file.sync_all()?;
        Ok(())
    }
}

/// 规范化基线系统调用列表
///
/// 将原始的系统调用名称列表转换为规范化的哈希集合。
/// 规范化包括：去除空白、转换为小写、过滤空字符串。
///
/// # 参数
///
/// - `raw`: 原始的系统调用名称列表
///
/// # 返回
///
/// 返回规范化后的哈希集合
fn normalize_baseline(raw: &[String]) -> HashSet<String> {
    raw.iter().map(|name| normalize_syscall_name(name)).filter(|name| !name.is_empty()).collect()
}

/// 规范化系统调用名称
///
/// 将系统调用名称转换为标准格式：去除前后空白并转换为小写。
///
/// # 参数
///
/// - `name`: 原始系统调用名称
///
/// # 返回
///
/// 返回规范化后的系统调用名称
fn normalize_syscall_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

/// 解析日志文件路径
///
/// 根据配置的路径字符串解析出实际的日志文件路径。
/// 支持绝对路径和相对路径：
/// - 绝对路径：直接使用
/// - 相对路径：相对于基础目录
///
/// # 参数
///
/// - `base_dir`: 基础目录路径
/// - `configured_path`: 配置的路径字符串
///
/// # 返回
///
/// 返回解析后的完整路径
fn resolve_log_path(base_dir: &Path, configured_path: &str) -> PathBuf {
    let trimmed = configured_path.trim();
    let path = Path::new(trimmed);
    // 如果是绝对路径则直接使用，否则相对于基础目录
    if path.is_absolute() { path.to_path_buf() } else { base_dir.join(path) }
}

/// 返回不超过给定索引的 UTF-8 字符边界。
fn floor_utf8_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }

    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// 截断样本字符串
///
/// 将原始日志行截断到最大字符数，防止日志中出现过长内容。
/// 截断时会确保在 UTF-8 字符边界上截断，避免产生无效的 UTF-8 字符串。
///
/// # 参数
///
/// - `raw`: 原始字符串
///
/// # 返回
///
/// 如果原始字符串长度不超过限制，返回原始字符串；
/// 否则返回截断后的字符串，末尾添加 "..." 表示截断。
fn truncate_sample(raw: &str) -> String {
    if raw.len() <= MAX_ALERT_SAMPLE_CHARS {
        return raw.to_string();
    }
    // 找到最大长度处的 UTF-8 字符边界
    let idx = floor_utf8_char_boundary(raw, MAX_ALERT_SAMPLE_CHARS);
    format!("{}...", &raw[..idx])
}

/// 统计被拒绝的事件数量
///
/// 遍历事件队列，统计被拒绝的系统调用事件数量。
///
/// # 参数
///
/// - `events`: 事件队列
///
/// # 返回
///
/// 返回被拒绝事件的数量（转换为 u32，如果溢出则返回 u32::MAX）
fn count_denied(events: &VecDeque<ObservedEvent>) -> u32 {
    let count = events.iter().filter(|event| event.denied).count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// 清理过期的事件
///
/// 从事件队列前端移除超出时间窗口的旧事件。
/// 保持队列中只包含时间窗口内的有效事件。
///
/// # 参数
///
/// - `events`: 事件队列（可变引用）
/// - `now`: 当前时间点
fn prune_old_events(events: &mut VecDeque<ObservedEvent>, now: Instant) {
    // 从队列前端移除所有超出时间窗口的事件
    while let Some(event) = events.front() {
        if now.duration_since(event.at) <= RATE_WINDOW {
            break;
        }
        let _ = events.pop_front();
    }
}

/// 清理过期的告警时间戳
///
/// 从告警时间戳队列前端移除超出时间窗口的旧时间戳。
/// 用于实现每分钟告警数限制。
///
/// # 参数
///
/// - `timestamps`: 告警时间戳队列（可变引用）
/// - `now`: 当前时间点
fn prune_old_alert_timestamps(timestamps: &mut VecDeque<Instant>, now: Instant) {
    // 从队列前端移除所有超出时间窗口的时间戳
    while let Some(at) = timestamps.front() {
        if now.duration_since(*at) <= RATE_WINDOW {
            break;
        }
        let _ = timestamps.pop_front();
    }
}

/// 提取命令标识
///
/// 从命令字符串中提取第一个词作为命令标识。
/// 用于告警去重键的构建。
///
/// # 参数
///
/// - `command`: 完整的命令字符串
///
/// # 返回
///
/// 返回命令的第一个词（小写），长度限制为 64 个字符
fn command_identity(command: &str) -> String {
    // 提取第一个词
    let token = command.split_whitespace().next().unwrap_or("-");
    let lowered = token.to_ascii_lowercase();
    // 限制长度
    if lowered.len() <= 64 {
        lowered
    } else {
        // 在 UTF-8 字符边界处截断
        let boundary = floor_utf8_char_boundary(&lowered, 64);
        lowered[..boundary].to_string()
    }
}

/// 判断是否应该发送告警
///
/// 根据频率限制和冷却时间策略判断告警是否应该被发送。
/// 实现两个层面的限制：
/// 1. 全局频率限制：每分钟最多发送一定数量的告警
/// 2. 去重冷却：相同键的告警在冷却时间内不会重复发送
///
/// # 参数
///
/// - `state`: 检测器状态（可变引用）
/// - `config`: 检测器配置
/// - `alert`: 待发送的告警
/// - `now`: 当前时间点
///
/// # 返回
///
/// 如果应该发送告警返回 `true`，否则返回 `false`
fn should_emit_alert(
    state: &mut DetectorState,
    config: &SyscallAnomalyConfig,
    alert: &SyscallAnomalyAlert,
    now: Instant,
) -> bool {
    // 清理过期的告警时间戳
    prune_old_alert_timestamps(&mut state.alert_timestamps, now);
    // 清理过期的告警键记录
    state.last_alert_by_key.retain(|_, at| now.duration_since(*at) <= RATE_WINDOW);

    // 检查全局频率限制：如果已达到每分钟最大告警数，则不发送
    if state.alert_timestamps.len() >= config.max_alerts_per_minute as usize {
        return false;
    }

    // 构建告警去重键
    let key = AlertKey {
        kind: alert.kind,
        syscall: alert.syscall.clone(),
        command_id: command_identity(&alert.command),
    };

    // 检查冷却时间：如果该键最近发送过告警且还在冷却期内，则不发送
    if let Some(last_at) = state.last_alert_by_key.get(&key) {
        let cooldown = Duration::from_secs(config.alert_cooldown_secs);
        if now.duration_since(*last_at) < cooldown {
            return false;
        }
    }

    // 记录本次告警的时间和键
    state.alert_timestamps.push_back(now);
    state.last_alert_by_key.insert(key, now);
    true
}

/// 从命令输出中提取系统调用信号
///
/// 遍历标准错误和标准输出的所有行，解析出系统调用相关的信号。
///
/// # 参数
///
/// - `stderr`: 标准错误输出内容
/// - `stdout`: 标准输出内容
///
/// # 返回
///
/// 返回解析出的系统调用信号列表
fn extract_signals(stderr: &str, stdout: &str) -> Vec<ParsedSyscallSignal> {
    stderr.lines().chain(stdout.lines()).filter_map(parse_syscall_signal).collect()
}

/// 解析单行日志中的系统调用信号
///
/// 检查日志行是否包含系统调用相关信息，并提取相关数据。
/// 通过关键词匹配判断行是否与系统调用相关以及是否被拒绝。
///
/// # 参数
///
/// - `line`: 单行日志内容
///
/// # 返回
///
/// 如果该行包含系统调用相关信号，返回 `Some(ParsedSyscallSignal)`；
/// 否则返回 `None`。
///
/// # 检测逻辑
///
/// 1. **相关性检测**：行中包含以下关键词之一则认为相关
///    - "syscall"、"seccomp"、"sigsys"、"bad system call"、"audit("
///
/// 2. **拒绝状态检测**：行中包含以下关键词之一则认为被拒绝
///    - "denied"、"blocked"、"forbidden"、"bad system call"、"sigsys"
///    - "operation not permitted"、" eperm"、"killed"
fn parse_syscall_signal(line: &str) -> Option<ParsedSyscallSignal> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 转换为小写以便进行关键词匹配
    let lower = trimmed.to_ascii_lowercase();

    // 检查是否与系统调用相关
    let looks_relevant = lower.contains("syscall")
        || lower.contains("seccomp")
        || lower.contains("sigsys")
        || lower.contains("bad system call")
        || lower.contains("audit(");
    if !looks_relevant {
        return None;
    }

    // 检查是否被拒绝
    let denied = lower.contains("denied")
        || lower.contains("blocked")
        || lower.contains("forbidden")
        || lower.contains("bad system call")
        || lower.contains("sigsys")
        || lower.contains("operation not permitted")
        || lower.contains(" eperm")
        || lower.contains("killed");

    // 尝试提取系统调用名称
    let syscall = extract_syscall_name(trimmed);

    // 如果既无法识别系统调用名称，也不是被拒绝的事件，且不包含 seccomp，则跳过
    if syscall.is_none() && !denied && !lower.contains("seccomp") {
        return None;
    }

    Some(ParsedSyscallSignal { syscall, denied, raw_line: trimmed.to_string() })
}

/// 从日志行中提取系统调用名称
///
/// 使用正则表达式从日志行中提取系统调用名称，并支持多种格式：
/// - 符号名称（如 "__NR_open"、"SYS_open"）
/// - 数字编号（十进制或十六进制）
/// - 直接名称
///
/// # 参数
///
/// - `line`: 日志行内容
///
/// # 返回
///
/// 如果成功提取并识别系统调用名称，返回 `Some(String)`；
/// 否则返回 `None`。
///
/// # 格式支持
///
/// 1. 符号格式：`__NR_open` -> `open`、`SYS_read` -> `read`
/// 2. 十进制数字：`syscall=59` -> `execve`
/// 3. 十六进制数字：`syscall=0x3b` -> `execve`
/// 4. 直接名称：`syscall=open` -> `open`
fn extract_syscall_name(line: &str) -> Option<String> {
    // 使用正则表达式捕获系统调用字段
    let captures = syscall_field_re().captures(line)?;
    let raw = captures.get(1)?.as_str().trim().trim_matches('"');
    if raw.is_empty() {
        return None;
    }

    // 尝试解析为符号名称（如 __NR_open）
    if let Some(symbolic) = normalize_symbolic_syscall(raw) {
        return Some(symbolic);
    }

    // 尝试解析为数字编号
    if let Some(syscall_nr) = parse_syscall_number(raw) {
        // 尝试将编号映射为名称（仅支持 Linux x86_64）
        if let Some(mapped) = map_linux_x86_64_syscall(syscall_nr) {
            return Some(mapped.to_string());
        }
        // 无法映射时返回编号格式
        return Some(format!("syscall#{syscall_nr}"));
    }

    // 直接返回规范化的名称
    Some(normalize_syscall_name(raw))
}

/// 获取系统调用字段提取正则表达式
///
/// 返回一个编译好的正则表达式，用于从日志行中提取系统调用名称或编号。
/// 使用 `OnceLock` 实现懒加载和单例模式，避免重复编译正则表达式。
///
/// # 正则表达式模式
///
/// ```ignore
/// (?i)\b(?:syscall(?:_name)?|system\s+call)\s*(?:nr|number)?\s*(?:=|:)?\s*([A-Za-z0-9_x]+)
/// ```
///
/// 该模式匹配以下格式：
/// - `syscall=open`
/// - `syscall_name=open`
/// - `syscall nr=2`
/// - `syscall number=2`
/// - `system call: open`
///
/// # 返回
///
/// 返回编译好的正则表达式的静态引用
fn syscall_field_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?i)\b(?:syscall(?:_name)?|system\s+call)\s*(?:nr|number)?\s*(?:=|:)?\s*([A-Za-z0-9_x]+)"#,
        )
            .expect("syscall regex must compile")
    })
}

/// 规范化符号形式的系统调用名称
///
/// 处理带有前缀的符号名称，去除前缀并转换为小写。
/// 支持的前缀：
/// - `__NR_` (如 `__NR_open` -> `open`)
/// - `__nr_` (如 `__nr_open` -> `open`)
/// - `SYS_` (如 `SYS_open` -> `open`)
///
/// # 参数
///
/// - `raw`: 原始的系统调用符号名称
///
/// # 返回
///
/// 如果是符号形式，返回去除前缀后的名称；
/// 否则返回 `None`。
fn normalize_symbolic_syscall(raw: &str) -> Option<String> {
    if let Some(stripped) = raw.strip_prefix("__NR_") {
        return Some(stripped.to_ascii_lowercase());
    }
    if let Some(stripped) = raw.strip_prefix("__nr_") {
        return Some(stripped.to_ascii_lowercase());
    }
    if let Some(stripped) = raw.strip_prefix("SYS_") {
        return Some(stripped.to_ascii_lowercase());
    }
    None
}

/// 解析系统调用编号
///
/// 将字符串解析为系统调用编号。
/// 支持十六进制（以 "0x" 开头）和十进制格式。
///
/// # 参数
///
/// - `raw`: 系统调用编号字符串
///
/// # 返回
///
/// 如果解析成功，返回 `Some(i64)`；
/// 否则返回 `None`。
///
/// # 格式支持
///
/// - 十六进制：`0x3b` -> 59
/// - 十进制：`59` -> 59
fn parse_syscall_number(raw: &str) -> Option<i64> {
    let lower = raw.to_ascii_lowercase();
    // 十六进制格式（以 0x 开头）
    if lower.starts_with("0x") {
        i64::from_str_radix(lower.trim_start_matches("0x"), 16).ok()
    // 十进制格式（纯数字）
    } else if lower.chars().all(|ch| ch.is_ascii_digit()) {
        lower.parse::<i64>().ok()
    } else {
        None
    }
}

/// Linux x86_64 系统调用编号到名称的映射表
///
/// 将 Linux x86_64 架构的系统调用编号映射为可读的名称。
/// 只包含了常见的系统调用，未列出的编号返回 `None`。
///
/// # 参数
///
/// - `number`: 系统调用编号
///
/// # 返回
///
/// 如果编号在映射表中，返回 `Some(&'static str)`；
/// 否则返回 `None`。
///
/// # 参考来源
///
/// 系统调用编号基于 Linux 内核源码中的 x86_64 系统调用表。
/// 完整列表可参考：https://github.com/torvalds/linux/blob/master/arch/x86/entry/syscalls/syscall_64.tbl
fn map_linux_x86_64_syscall(number: i64) -> Option<&'static str> {
    match number {
        0 => Some("read"),              // 从文件描述符读取
        1 => Some("write"),             // 写入文件描述符
        2 => Some("open"),              // 打开文件
        3 => Some("close"),             // 关闭文件描述符
        9 => Some("mmap"),              // 内存映射
        10 => Some("mprotect"),         // 设置内存保护
        11 => Some("munmap"),           // 取消内存映射
        12 => Some("brk"),              // 改变数据段大小
        16 => Some("ioctl"),            // 设备控制
        32 => Some("dup"),              // 复制文件描述符
        33 => Some("dup2"),             // 复制文件描述符到指定编号
        39 => Some("getpid"),           // 获取进程 ID
        41 => Some("socket"),           // 创建套接字
        42 => Some("connect"),          // 连接到远程套接字
        43 => Some("accept"),           // 接受连接
        44 => Some("sendto"),           // 发送数据
        45 => Some("recvfrom"),         // 接收数据
        47 => Some("recvmsg"),          // 接收消息
        50 => Some("listen"),           // 监听连接
        51 => Some("getsockname"),      // 获取套接字名称
        52 => Some("getpeername"),      // 获取对端套接字名称
        54 => Some("setsockopt"),       // 设置套接字选项
        55 => Some("getsockopt"),       // 获取套接字选项
        56 => Some("clone"),            // 创建子进程
        57 => Some("fork"),             // 创建子进程（传统）
        59 => Some("execve"),           // 执行程序
        60 => Some("exit"),             // 退出进程
        61 => Some("wait4"),            // 等待进程状态改变
        72 => Some("fcntl"),            // 文件控制
        202 => Some("futex"),           // 快速用户空间互斥锁
        218 => Some("set_tid_address"), // 设置线程 ID 地址
        231 => Some("exit_group"),      // 退出所有线程
        257 => Some("openat"),          // 相对路径打开文件
        262 => Some("newfstatat"),      // 获取文件状态
        273 => Some("set_robust_list"), // 设置健壮列表
        291 => Some("epoll_create1"),   // 创建 epoll 实例
        318 => Some("getrandom"),       // 获取随机数
        332 => Some("statx"),           // 获取扩展文件状态
        435 => Some("clone3"),          // 扩展的进程创建
        _ => None,
    }
}
#[cfg(test)]
#[path = "syscall_anomaly_tests.rs"]
mod syscall_anomaly_tests;
