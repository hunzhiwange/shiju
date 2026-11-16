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
