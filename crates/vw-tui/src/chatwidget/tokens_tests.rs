//! 验证令牌活动加载完成后冻结图表锚点日期，避免后续渲染跨日漂移。
//! 测试使用固定完成时刻，确保已加载状态复用同一时间边界。

use super::*;
use codex_app_server_protocol::AccountTokenUsageSummary;
use pretty_assertions::assert_eq;

#[test]
fn loaded_state_freezes_chart_anchor_date_at_completion() {
    // 使用显式日期完成加载，隔离测试运行当天对图表窗口的影响。
    let state = Arc::new(RwLock::new(TokenActivityState::Loading));
    let handle = TokenActivityHandle {
        state: Arc::clone(&state),
    };
    let today =
        NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 5, /*day*/ 29).expect("valid date");

    handle.finish_with_today(
        Ok(GetAccountTokenUsageResponse {
            summary: AccountTokenUsageSummary {
                lifetime_tokens: None,
                peak_daily_tokens: None,
                longest_running_turn_sec: None,
                current_streak_days: None,
                longest_streak_days: None,
            },
            daily_usage_buckets: None,
            thread_usage: None,
        }),
        today,
    );

    let state = state.read().expect("token activity state poisoned");
    match &*state {
        // 已加载状态保存完成日，后续读取不再重新查询当前日期。
        TokenActivityState::Loaded {
            today: loaded_today,
            ..
        } => {
            assert_eq!(*loaded_today, today);
        }
        other => panic!("expected loaded state, got {other:?}"),
    }
}
