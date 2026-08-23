//! Rate-limit and credits display shaping for status surfaces.
//!
//! This module maps `RateLimitSnapshot` protocol payloads into display-oriented rows that the TUI
//! can render in `/status` and status-line contexts without duplicating formatting logic.
//!
//! The key contract is that time-sensitive values are interpreted relative to a caller-provided
//! capture timestamp so stale detection and reset labels remain coherent for a given draw cycle.
//!
//! 本模块把协议层 `RateLimitSnapshot` 整理为 `/status` 与状态行共享的展示行，避免重复格式化。
//! 所有时效判断都相对于调用方提供的同一采集时间，以保证一次绘制中的过期状态和重置文案一致。
use crate::chatwidget::fallback_limit_label;
use crate::chatwidget::limit_label_for_window;
use crate::text_formatting::capitalize_first;

use super::helpers::format_reset_timestamp;
use chrono::DateTime;
use chrono::Duration as ChronoDuration;
use chrono::Local;
use chrono::Utc;
use codex_app_server_protocol::CreditsSnapshot as CoreCreditsSnapshot;
use codex_app_server_protocol::RateLimitSnapshot;
use codex_app_server_protocol::RateLimitWindow;
use codex_app_server_protocol::SpendControlLimitSnapshot as CoreSpendControlLimitSnapshot;
use codex_protocol::num_format::format_with_separators;

const STATUS_LIMIT_BAR_SEGMENTS: usize = 20;
const STATUS_LIMIT_BAR_FILLED: &str = "█";
const STATUS_LIMIT_BAR_EMPTY: &str = "░";

#[derive(Debug, Clone)]
pub(crate) struct StatusRateLimitRow {
    /// Human-readable row label, such as `"5h limit"`, `"Monthly limit"`, or `"Credits"`.
    /// 面向用户的行标签，例如“5h limit”“Monthly limit”或“Credits”。
    pub label: String,
    /// Value payload for the row.
    /// 该行的展示值载荷。
    pub value: StatusRateLimitValue,
}

/// Display value variants for a single rate-limit row.
/// 单条限额行可使用的展示值类型。
#[derive(Debug, Clone)]
pub(crate) enum StatusRateLimitValue {
    /// Percent-based usage window with optional reset timestamp text.
    /// 百分比用量窗口，可附带重置时间与详情。
    Window {
        /// Percent of the window that has been consumed.
        /// 当前窗口已经消耗的百分比。
        percent_used: f64,
        /// Localized reset string, or `None` when unknown.
        /// 本地化重置时间，未知时为空。
        resets_at: Option<String>,
        /// Optional detail line rendered beneath the progress bar.
        /// 可选的进度条下方详情行。
        details: Option<String>,
    },
    /// Plain text value used for non-window rows.
    /// 非窗口行使用的纯文本值。
    Text(String),
}

/// Availability state for rate-limit data shown in status output.
/// 状态输出中限额数据的可用性分类。
#[derive(Debug, Clone)]
pub(crate) enum StatusRateLimitData {
    /// Snapshot data is recent enough for normal rendering.
    /// 快照足够新，可正常渲染。
    Available(Vec<StatusRateLimitRow>),
    /// Snapshot data exists but is older than the staleness threshold.
    /// 快照存在，但已超过陈旧阈值。
    Stale(Vec<StatusRateLimitRow>),
    /// The refresh completed, but the response did not include displayable usage data.
    /// 刷新已完成，但响应没有可展示用量。
    Unavailable,
    /// No snapshot data is currently available.
    /// 当前没有任何快照数据。
    Missing,
}

/// Maximum age before a snapshot is considered stale in status output.
/// 状态输出把快照视为陈旧前允许的最大年龄。
pub(crate) const RATE_LIMIT_STALE_THRESHOLD_MINUTES: i64 = 15;

/// Display-friendly representation of one usage window from a snapshot.
/// 单个用量窗口的展示友好形式。
#[derive(Debug, Clone)]
pub(crate) struct RateLimitWindowDisplay {
    /// Percent used for the window.
    /// 当前窗口的已用百分比。
    pub used_percent: f64,
    /// Human-readable local reset time.
    /// 面向用户的本地重置时间。
    pub resets_at: Option<String>,
    /// Window length in minutes when provided by the server.
    /// 服务端提供时记录的窗口分钟数。
    pub window_minutes: Option<i64>,
}

impl RateLimitWindowDisplay {
    fn from_window(window: &RateLimitWindow, captured_at: DateTime<Local>) -> Self {
        let resets_at_utc = window
            .resets_at
            .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
            .map(|dt| dt.with_timezone(&Local));
        let resets_at = resets_at_utc.map(|dt| format_reset_timestamp(dt, captured_at));

        Self {
            used_percent: f64::from(window.used_percent),
            resets_at,
            window_minutes: window.window_duration_mins,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RateLimitSnapshotDisplay {
    /// Canonical limit identifier (for example: `codex` or `codex_other`).
    /// 规范限额标识，例如 `codex` 或 `codex_other`。
    pub limit_name: String,
    /// Local timestamp representing when this display snapshot was captured.
    /// 这份展示快照的本地采集时间。
    pub captured_at: DateTime<Local>,
    /// Primary usage window.
    /// 主用量窗口。
    pub primary: Option<RateLimitWindowDisplay>,
    /// Secondary usage window.
    /// 次用量窗口。
    pub secondary: Option<RateLimitWindowDisplay>,
    /// Optional credits metadata when available.
    /// 可用时附带的积分元数据。
    pub credits: Option<CreditsSnapshotDisplay>,
    /// Optional effective monthly credit limit from workspace spend controls.
    /// 工作区消费控制给出的可选月度有效积分上限。
    pub individual_limit: Option<SpendControlLimitSnapshotDisplay>,
}

/// Display-ready credits state extracted from protocol snapshots.
/// 从协议快照提取的可直接展示积分状态。
#[derive(Debug, Clone)]
pub(crate) struct CreditsSnapshotDisplay {
    /// Whether credits tracking is enabled for the account.
    /// 账户是否启用积分跟踪。
    pub has_credits: bool,
    /// Whether the account has unlimited credits.
    /// 账户积分是否无限。
    pub unlimited: bool,
    /// Raw balance text as provided by the backend.
    /// 后端提供的原始余额文本，后续格式化会严格解析。
    pub balance: Option<String>,
}

/// Display-ready effective monthly limit extracted from spend controls.
/// 从消费控制提取的可展示月度有效上限。
#[derive(Debug, Clone)]
pub(crate) struct SpendControlLimitSnapshotDisplay {
    /// Local timestamp representing when this monthly usage value was captured.
    /// 月度用量值的本地采集时间。
    pub captured_at: DateTime<Local>,
    pub percent_remaining: f64,
    pub used: String,
    pub limit: String,
    pub resets_at: Option<String>,
}

/// Converts a protocol snapshot into UI-friendly display data.
///
/// Pass the timestamp from the same observation point as `snapshot`; supplying a significantly
/// older or newer `captured_at` can produce misleading reset labels and stale classification.
/// 把协议快照转换为 UI 展示数据；`captured_at` 必须与快照来自同一观测点，时间偏差会误导重置文案和陈旧分类。
#[cfg(test)]
pub(crate) fn rate_limit_snapshot_display(
    snapshot: &RateLimitSnapshot,
    captured_at: DateTime<Local>,
) -> RateLimitSnapshotDisplay {
    rate_limit_snapshot_display_for_limit(snapshot, "codex".to_string(), captured_at)
}

pub(crate) fn rate_limit_snapshot_display_for_limit(
    snapshot: &RateLimitSnapshot,
    limit_name: String,
    captured_at: DateTime<Local>,
) -> RateLimitSnapshotDisplay {
    RateLimitSnapshotDisplay {
        limit_name,
        captured_at,
        primary: snapshot
            .primary
            .as_ref()
            .map(|window| RateLimitWindowDisplay::from_window(window, captured_at)),
        secondary: snapshot
            .secondary
            .as_ref()
            .map(|window| RateLimitWindowDisplay::from_window(window, captured_at)),
        credits: snapshot.credits.as_ref().map(CreditsSnapshotDisplay::from),
        individual_limit: snapshot
            .individual_limit
            .as_ref()
            .and_then(|limit| SpendControlLimitSnapshotDisplay::from_limit(limit, captured_at)),
    }
}

impl From<&CoreCreditsSnapshot> for CreditsSnapshotDisplay {
    fn from(value: &CoreCreditsSnapshot) -> Self {
        Self {
            has_credits: value.has_credits,
            unlimited: value.unlimited,
            balance: value.balance.clone(),
        }
    }
}

impl SpendControlLimitSnapshotDisplay {
    fn from_limit(
        value: &CoreSpendControlLimitSnapshot,
        captured_at: DateTime<Local>,
    ) -> Option<Self> {
        Some(Self {
            captured_at,
            percent_remaining: f64::from(value.remaining_percent.clamp(0, 100)),
            used: format_credit_amount(&value.used)?,
            limit: format_credit_amount(&value.limit)?,
            resets_at: DateTime::<Utc>::from_timestamp(value.resets_at, 0)
                .map(|dt| format_reset_timestamp(dt.with_timezone(&Local), captured_at)),
        })
    }
}

/// Builds display rows from a snapshot and marks stale data by capture age.
///
/// Callers should pass `Local::now()` for `now` at render time; using a cached timestamp can make
/// fresh data appear stale or prevent stale warnings from appearing.
/// 从一个或多个快照生成展示行并按采集年龄标记陈旧；渲染时应传入当前本地时间，缓存时间会造成错误分类。
pub(crate) fn compose_rate_limit_data(
    snapshot: Option<&RateLimitSnapshotDisplay>,
    now: DateTime<Local>,
) -> StatusRateLimitData {
    match snapshot {
        Some(snapshot) => compose_rate_limit_data_many(std::slice::from_ref(snapshot), now),
        None => StatusRateLimitData::Missing,
    }
}

pub(crate) fn compose_rate_limit_data_many(
    snapshots: &[RateLimitSnapshotDisplay],
    now: DateTime<Local>,
) -> StatusRateLimitData {
    if snapshots.is_empty() {
        return StatusRateLimitData::Missing;
    }

    let mut rows = Vec::with_capacity(snapshots.len().saturating_mul(3));
    let mut stale = false;

    for snapshot in snapshots {
        stale |= now.signed_duration_since(snapshot.captured_at)
            > ChronoDuration::minutes(RATE_LIMIT_STALE_THRESHOLD_MINUTES);
        stale |= snapshot
            .individual_limit
            .as_ref()
            .map(|limit| {
                now.signed_duration_since(limit.captured_at)
                    > ChronoDuration::minutes(RATE_LIMIT_STALE_THRESHOLD_MINUTES)
            })
            .unwrap_or(false);

        let limit_bucket_label = snapshot.limit_name.clone();
        let show_limit_prefix = !limit_bucket_label.eq_ignore_ascii_case("codex");
        let primary_label = snapshot
            .primary
            .as_ref()
            .map(|window| {
                limit_label_for_window(window.window_minutes, /*is_secondary*/ false)
            })
            .map(|label| capitalize_first(&label));
        let secondary_label = snapshot
            .secondary
            .as_ref()
            .map(|window| limit_label_for_window(window.window_minutes, /*is_secondary*/ true))
            .map(|label| capitalize_first(&label));
        let window_count =
            usize::from(snapshot.primary.is_some()) + usize::from(snapshot.secondary.is_some());
        let combine_non_codex_single_limit = show_limit_prefix && window_count == 1;

        if show_limit_prefix && !combine_non_codex_single_limit {
            rows.push(StatusRateLimitRow {
                label: format!("{limit_bucket_label} limit"),
                value: StatusRateLimitValue::Text(String::new()),
            });
        }

        if let Some(primary) = snapshot.primary.as_ref() {
            let label = if combine_non_codex_single_limit {
                format!(
                    "{} {} limit",
                    limit_bucket_label,
                    primary_label.clone().unwrap_or_else(|| capitalize_first(
                        fallback_limit_label(/*is_secondary*/ false)
                    ))
                )
            } else {
                format!(
                    "{} limit",
                    primary_label.clone().unwrap_or_else(|| capitalize_first(
                        fallback_limit_label(/*is_secondary*/ false)
                    ))
                )
            };
            rows.push(StatusRateLimitRow {
                label,
                value: StatusRateLimitValue::Window {
                    percent_used: primary.used_percent,
                    resets_at: primary.resets_at.clone(),
                    details: None,
                },
            });
        }

        if let Some(secondary) = snapshot.secondary.as_ref() {
            let label = if combine_non_codex_single_limit {
                format!(
                    "{} {} limit",
                    limit_bucket_label,
                    secondary_label.clone().unwrap_or_else(|| capitalize_first(
                        fallback_limit_label(/*is_secondary*/ true)
                    ))
                )
            } else {
                format!(
                    "{} limit",
                    secondary_label.clone().unwrap_or_else(|| capitalize_first(
                        fallback_limit_label(/*is_secondary*/ true)
                    ))
                )
            };
            rows.push(StatusRateLimitRow {
                label,
                value: StatusRateLimitValue::Window {
                    percent_used: secondary.used_percent,
                    resets_at: secondary.resets_at.clone(),
                    details: None,
                },
            });
        }

        if let Some(credits) = snapshot.credits.as_ref()
            && let Some(row) = credit_status_row(credits)
        {
            rows.push(row);
        }
        if let Some(individual_limit) = snapshot.individual_limit.as_ref() {
            rows.push(StatusRateLimitRow {
                label: "Monthly credit limit".to_string(),
                value: StatusRateLimitValue::Window {
                    percent_used: 100.0 - individual_limit.percent_remaining,
                    resets_at: individual_limit.resets_at.clone(),
                    details: Some(format!(
                        "{} of {} credits used",
                        individual_limit.used, individual_limit.limit
                    )),
                },
            });
        }
    }

    if rows.is_empty() {
        StatusRateLimitData::Unavailable
    } else if stale {
        StatusRateLimitData::Stale(rows)
    } else {
        StatusRateLimitData::Available(rows)
    }
}

/// Renders a fixed-width progress bar from remaining percentage.
///
/// This function expects a remaining value in the `0..=100` range and clamps out-of-range input.
/// Passing a used percentage by mistake will invert the bar and mislead users.
/// 从剩余百分比生成固定宽度进度条，并钳制越界输入；误传已用百分比会反转含义并误导用户。
pub(crate) fn render_status_limit_progress_bar(percent_remaining: f64) -> String {
    let ratio = (percent_remaining / 100.0).clamp(0.0, 1.0);
    let filled = (ratio * STATUS_LIMIT_BAR_SEGMENTS as f64).round() as usize;
    let filled = filled.min(STATUS_LIMIT_BAR_SEGMENTS);
    let empty = STATUS_LIMIT_BAR_SEGMENTS.saturating_sub(filled);
    format!(
        "[{}{}]",
        STATUS_LIMIT_BAR_FILLED.repeat(filled),
        STATUS_LIMIT_BAR_EMPTY.repeat(empty)
    )
}

/// Formats a compact textual summary from remaining percentage.
/// 从剩余百分比生成紧凑文本摘要。
pub(crate) fn format_status_limit_summary(percent_remaining: f64) -> String {
    format!("{percent_remaining:.0}% left")
}

/// Builds a single `StatusRateLimitRow` when workspace credits are available.
/// Unlimited credits are shown explicitly; finite credits show their rounded
/// balance or `Available` when the balance is hidden.
/// 在工作区积分可用时生成一行状态；无限积分明确显示，有限积分显示四舍五入余额或隐藏余额时的可用状态。
fn credit_status_row(credits: &CreditsSnapshotDisplay) -> Option<StatusRateLimitRow> {
    if credits.unlimited {
        return Some(StatusRateLimitRow {
            label: "Credits".to_string(),
            value: StatusRateLimitValue::Text("Unlimited".to_string()),
        });
    }
    if !credits.has_credits {
        return None;
    }
    let value = credits
        .balance
        .as_deref()
        .and_then(format_credit_balance)
        .map_or_else(
            || "Available".to_string(),
            |display_balance| format!("{display_balance} credits"),
        );
    Some(StatusRateLimitRow {
        label: "Credits".to_string(),
        value: StatusRateLimitValue::Text(value),
    })
}

fn format_credit_balance(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(int_value) = trimmed.parse::<i64>()
        && int_value > 0
    {
        return Some(int_value.to_string());
    }

    if let Ok(value) = trimmed.parse::<f64>()
        && value.is_finite()
        && value > 0.0
    {
        let rounded = value.round() as i64;
        return Some(rounded.to_string());
    }

    None
}

fn format_credit_amount(raw: &str) -> Option<String> {
    let value = raw.trim().parse::<f64>().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some(format_with_separators(value.round() as i64))
}

#[cfg(test)]
mod tests {
    use super::CreditsSnapshotDisplay;
    use super::RateLimitSnapshotDisplay;
    use super::RateLimitWindowDisplay;
    use super::StatusRateLimitData;
    use super::compose_rate_limit_data_many;
    use chrono::Local;
    use pretty_assertions::assert_eq;

    fn window(used_percent: f64) -> RateLimitWindowDisplay {
        RateLimitWindowDisplay {
            used_percent,
            resets_at: Some("soon".to_string()),
            window_minutes: Some(300),
        }
    }

    #[test]
    // 验证单个非 Codex 限额合并为紧凑行，并保留积分信息。
    fn non_codex_single_limit_renders_combined_row() {
        let now = Local::now();
        let codex = RateLimitSnapshotDisplay {
            limit_name: "codex".to_string(),
            captured_at: now,
            primary: Some(window(/*used_percent*/ 10.0)),
            secondary: None,
            credits: Some(CreditsSnapshotDisplay {
                has_credits: true,
                unlimited: false,
                balance: Some("25".to_string()),
            }),
            individual_limit: None,
        };
        let other = RateLimitSnapshotDisplay {
            limit_name: "codex-other".to_string(),
            captured_at: now,
            primary: Some(window(/*used_percent*/ 20.0)),
            secondary: None,
            credits: Some(CreditsSnapshotDisplay {
                has_credits: true,
                unlimited: false,
                balance: Some("99".to_string()),
            }),
            individual_limit: None,
        };

        let rows = match compose_rate_limit_data_many(&[codex, other], now) {
            StatusRateLimitData::Available(rows) => rows,
            other => panic!("unexpected status: {other:?}"),
        };

        let labels: Vec<String> = rows.iter().map(|row| row.label.clone()).collect();
        assert_eq!(
            labels,
            vec![
                "5h limit".to_string(),
                "Credits".to_string(),
                "codex-other 5h limit".to_string(),
                "Credits".to_string(),
            ]
        );
        assert_eq!(rows.iter().filter(|row| row.label == "Credits").count(), 2);
    }

    #[test]
    // 验证多个非 Codex 限额仍保留分组行，避免语义混合。
    fn non_codex_multi_limit_keeps_group_row() {
        let now = Local::now();
        let other = RateLimitSnapshotDisplay {
            limit_name: "codex-other".to_string(),
            captured_at: now,
            primary: Some(RateLimitWindowDisplay {
                used_percent: 20.0,
                resets_at: Some("soon".to_string()),
                window_minutes: Some(60),
            }),
            secondary: Some(RateLimitWindowDisplay {
                used_percent: 40.0,
                resets_at: Some("later".to_string()),
                window_minutes: Some(2 * 60),
            }),
            credits: None,
            individual_limit: None,
        };

        let rows = match compose_rate_limit_data_many(&[other], now) {
            StatusRateLimitData::Available(rows) => rows,
            other => panic!("unexpected status: {other:?}"),
        };
        let labels: Vec<String> = rows.iter().map(|row| row.label.clone()).collect();
        assert_eq!(
            labels,
            vec![
                "codex-other limit".to_string(),
                "Usage limit".to_string(),
                "Secondary usage limit".to_string(),
            ]
        );
    }
}
