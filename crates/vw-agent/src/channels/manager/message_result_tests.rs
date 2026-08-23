use super::*;

#[test]
fn reaction_done_emoji_marks_success_only_for_completed_ok_result() {
    assert_eq!(
        reaction_done_emoji(&LlmExecutionResult::Completed(Ok(Ok("done".to_string())))),
        "\u{2705}"
    );
    assert_eq!(
        reaction_done_emoji(&LlmExecutionResult::Completed(Ok(Err(anyhow::anyhow!("failed"))))),
        "\u{26A0}\u{FE0F}"
    );
    assert_eq!(reaction_done_emoji(&LlmExecutionResult::Cancelled), "\u{26A0}\u{FE0F}");
}

#[tokio::test]
async fn reaction_done_emoji_marks_timeout_as_warning() {
    let elapsed =
        tokio::time::timeout(std::time::Duration::from_millis(1), std::future::pending::<()>())
            .await
            .expect_err("pending future should time out");

    assert_eq!(
        reaction_done_emoji(&LlmExecutionResult::Completed(Err(elapsed))),
        "\u{26A0}\u{FE0F}"
    );
}
// 本文件验证消息执行结果到完成反应表情的映射。
