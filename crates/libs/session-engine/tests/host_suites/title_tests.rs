//! 宿主入口契约：通过 vw-agent 安装 provider/LLM Host 后进入标题引擎。
//! 此测试依赖未配置模型的宿主环境，不能作为 session-engine 的纯库测试接入。

use super::*;

#[tokio::test]
async fn title_generation_does_not_require_a_title_agent_configuration() {
    let err = generate_from_content(
        "session-title-no-agent-config".to_string(),
        "Generate a title without a title agent configuration".to_string(),
        None,
        None,
    )
    .await
    .expect_err("the test environment intentionally has no configured model");

    assert_ne!(err, "No title agent found");
}
