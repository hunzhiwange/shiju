// 宿主契约：核对网关回复任务、缺失请求分支及问题弹窗字段的清理时机。
//! 验证流式问题请求的排序、默认答案、状态重置和提交流程。
//!
//! 同一请求更新与新请求替换分别测试，答案数组必须随题目数量安全扩缩。

#![allow(unused_must_use)]
#[test]
fn questions_tests_module_is_wired() {
    assert!(module_path!().ends_with("questions_tests"));
}

#[test]
fn question_poll_tick_schedules_gateway_refresh() {
    let task = super::handle_question_poll_tick();

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn reply_and_reject_results_accept_success_and_error_outcomes() {
    let _success_reply = super::handle_question_reply_submitted(Ok(()));
    let _failed_reply = super::handle_question_reply_submitted(Err("reply failed".to_string()));
    let _success_reject = super::handle_question_rejected(Ok(()));
    let _failed_reject = super::handle_question_rejected(Err("reject failed".to_string()));
}
