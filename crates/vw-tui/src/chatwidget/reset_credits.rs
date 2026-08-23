//! 本模块负责速率限制重置额度选项。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。
use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use codex_app_server_protocol::RateLimitResetCreditStatus;
use codex_app_server_protocol::RateLimitResetCreditsSummary;

#[derive(Debug, Eq, PartialEq)]
/// 描述 `ResetCreditOption` 在当前界面流程中传递的数据。
pub(super) struct ResetCreditOption {
    pub(super) credit_id: Option<String>,
    pub(super) name: String,
    pub(super) detail: Option<String>,
    pub(super) description: String,
}

// 更新 `reset_credit_options` 对应的本地展示状态。
pub(super) fn reset_credit_options(
    summary: &RateLimitResetCreditsSummary,
) -> Vec<ResetCreditOption> {
    let available_count = summary.available_count.max(0);
    let detail_limit = usize::try_from(available_count).unwrap_or(usize::MAX);
    let mut available_credits = summary
        .credits
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|credit| credit.status == RateLimitResetCreditStatus::Available)
        .collect::<Vec<_>>();
    available_credits.sort_by_key(|credit| credit.expires_at.unwrap_or(i64::MAX));

    let mut options = available_credits
        .into_iter()
        .take(detail_limit)
        .map(|credit| {
            let expiration = match credit.expires_at {
                Some(expires_at) => DateTime::<Utc>::from_timestamp(expires_at, 0)
                    .map(|expires_at| {
                        format!(
                            "Expires {}",
                            expires_at
                                .with_timezone(&Local)
                                .format("%H:%M on %-d %b %Y")
                        )
                    })
                    .unwrap_or_else(|| "Expiration unavailable".to_string()),
                None => "Does not expire".to_string(),
            };
            let reset_title = credit
                .title
                .as_deref()
                .map(str::trim)
                .filter(|title| !title.is_empty())
                .unwrap_or("Full reset");
            let reset_description = credit
                .description
                .as_deref()
                .map(str::trim)
                .filter(|description| !description.is_empty())
                .unwrap_or("Reset your current usage limits.");
            ResetCreditOption {
                credit_id: Some(credit.id.clone()),
                name: reset_title.to_string(),
                detail: Some(format!("{expiration}.")),
                description: reset_description.to_string(),
            }
        })
        .collect::<Vec<_>>();

    if options.is_empty() {
        options.push(ResetCreditOption {
            credit_id: None,
            name: "Full reset".to_string(),
            detail: None,
            description: "Reset your current usage limits.".to_string(),
        });
    }

    options
}
