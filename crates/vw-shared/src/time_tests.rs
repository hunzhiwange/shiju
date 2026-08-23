//! 跨平台时间读取与活动计时器状态转换测试。
//!
//! 活动计时仅累计已启动区间，暂停空档和重置后的时间不计入结果。

#[test]
fn now_ms_is_close_to_system_time() {
    let before =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
            as u64;
    let actual = super::now_ms();
    let after =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
            as u64;

    assert!(actual >= before);
    assert!(actual <= after);
}

#[test]
fn now_returns_system_time_after_unix_epoch() {
    assert!(super::now().duration_since(std::time::UNIX_EPOCH).is_ok());
}

#[test]
fn active_timer_accumulates_only_started_intervals() {
    let start = web_time::Instant::now();
    let first_end = start.checked_add(std::time::Duration::from_millis(1_500)).unwrap();
    let second_start = first_end.checked_add(std::time::Duration::from_secs(10)).unwrap();
    let second_end = second_start.checked_add(std::time::Duration::from_millis(2_250)).unwrap();
    let mut timer = super::ActiveTimer::default();

    assert_eq!(timer.elapsed_ms_at(start), 0);
    timer.start_at(start);
    assert_eq!(timer.elapsed_ms_at(first_end), 1_500);
    assert_eq!(timer.pause_at(first_end), 1_500);
    assert_eq!(timer.elapsed_ms_at(second_start), 1_500);

    timer.start_at(second_start);
    assert_eq!(timer.pause_at(second_end), 3_750);
    timer.reset();
    assert_eq!(timer.elapsed_ms_at(second_end), 0);
}
