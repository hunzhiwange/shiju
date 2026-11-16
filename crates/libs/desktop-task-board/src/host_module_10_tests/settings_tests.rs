use super::*;
fn assert_deadline_in_window(deadline_ms: u64, start_ms: u64, end_ms: u64, interval_secs: u64) {
    let interval_ms = interval_secs.saturating_mul(1000);

    assert!(deadline_ms >= start_ms.saturating_add(interval_ms));
    assert!(deadline_ms <= end_ms.saturating_add(interval_ms));
}

#[test]
fn scheduler_deadline_is_preserved_while_still_in_the_future() {
    let (mut app, _task) = super::fixture_pair();
    app.task_board_next_scheduler_tick_at_ms = u64::MAX;

    let _scheduled = crate::helpers::settings::scheduler_deadline(
        &mut app.task_board_next_scheduler_tick_at_ms,
        &app.task_board_settings,
    );

    assert_eq!(app.task_board_next_scheduler_tick_at_ms, u64::MAX);
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。#[test]
fn scheduler_deadline_is_refreshed_after_expiry() {
    let (mut app, _task) = super::fixture_pair();
    app.task_board_settings.scheduler_tick_interval_seconds = 2;
    app.task_board_next_scheduler_tick_at_ms = 0;
    let start_ms = now_ms();

    let _scheduled = crate::helpers::settings::scheduler_deadline(
        &mut app.task_board_next_scheduler_tick_at_ms,
        &app.task_board_settings,
    );

    let end_ms = now_ms();
    assert_deadline_in_window(app.task_board_next_scheduler_tick_at_ms, start_ms, end_ms, 2);
}
#[test]
fn fixed_interval_schedulers_refresh_their_deadlines() {
    let (mut app, _task) = super::fixture_pair();

    let review_start_ms = now_ms();
    let _review_task = crate::helpers::settings::fixed_deadline(
        &mut app.task_board_next_auto_review_tick_at_ms,
        TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS,
    );
    let review_end_ms = now_ms();
    assert_deadline_in_window(
        app.task_board_next_auto_review_tick_at_ms,
        review_start_ms,
        review_end_ms,
        TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS,
    );

    let merge_start_ms = now_ms();
    let _merge_task = crate::helpers::settings::fixed_deadline(
        &mut app.task_board_next_auto_merge_tick_at_ms,
        TASK_AUTO_MERGE_TICK_INTERVAL_SECS,
    );
    let merge_end_ms = now_ms();
    assert_deadline_in_window(
        app.task_board_next_auto_merge_tick_at_ms,
        merge_start_ms,
        merge_end_ms,
        TASK_AUTO_MERGE_TICK_INTERVAL_SECS,
    );
}
#[test]
fn auto_promote_scheduler_uses_sanitized_interval_for_deadline() {
    let (mut app, _task) = super::fixture_pair();
    app.task_board_settings.auto_promote_tick_interval_seconds = 0;
    let start_ms = now_ms();

    let _scheduled = crate::helpers::settings::fixed_deadline(
        &mut app.task_board_next_auto_promote_tick_at_ms,
        crate::helpers::settings::auto_promote_tick_interval_secs(&app.task_board_settings),
    );

    let end_ms = now_ms();
    assert_deadline_in_window(app.task_board_next_auto_promote_tick_at_ms, start_ms, end_ms, 1);
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
