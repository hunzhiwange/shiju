use super::*;
#[test]
fn browser_login_waits_for_thirty_minutes() {
    assert_eq!(LOGIN_TIMEOUT, Duration::from_secs(30 * 60));
}

#[test]
fn account_gateway_tasks_remain_registered() {
    let _ = start_browser_login_task;
    let _ = poll_task;
    let _ = validate_task;
    let _ = profile_load_task;
    let _ = profile_update_task;
    let _ = profile_persist_task;
    let _ = password_update_task;
}
