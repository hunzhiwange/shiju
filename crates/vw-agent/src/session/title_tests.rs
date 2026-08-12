use super::*;

#[test]
fn title_source_at_character_limit_is_preserved() {
    let source = "界".repeat(CONTENT_TRUNCATE_CHARS);

    assert_eq!(truncate_title_source(source.clone()), source);
}

#[test]
fn title_source_over_character_limit_is_truncated_on_unicode_boundary() {
    let source = "界".repeat(CONTENT_TRUNCATE_CHARS + 1);

    let truncated = truncate_title_source(source);

    assert_eq!(truncated, "界".repeat(CONTENT_TRUNCATE_CHARS));
    assert_eq!(truncated.chars().count(), CONTENT_TRUNCATE_CHARS);
}

#[test]
fn title_prompt_includes_source_verbatim() {
    let prompt = build_title_prompt("first line\n第二行");

    assert_eq!(
        prompt,
        "Generate a concise title (max 50 characters) for this conversation. Only output the title, nothing else.\n\nUser message:\nfirst line\n第二行"
    );
}

#[test]
fn acp_agent_options_override_reserved_keys_and_preserve_other_options() {
    let mut options = std::collections::HashMap::from([
        ("acp_test".to_string(), serde_json::Value::Bool(false)),
        ("acp_agent".to_string(), serde_json::Value::String("old-agent".to_string())),
        ("temperature_hint".to_string(), serde_json::json!(0.25)),
    ]);

    apply_acp_agent_options(&mut options, Some("fixture-agent"));

    assert_eq!(options.get("acp_test"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(options.get("acp_agent").and_then(serde_json::Value::as_str), Some("fixture-agent"));
    assert_eq!(options.get("temperature_hint"), Some(&serde_json::json!(0.25)));
}

#[test]
fn absent_acp_agent_leaves_options_unchanged() {
    let original = std::collections::HashMap::from([(
        "custom".to_string(),
        serde_json::Value::String("value".to_string()),
    )]);
    let mut options = original.clone();

    apply_acp_agent_options(&mut options, None);

    assert_eq!(options, original);
}

#[test]
fn internal_title_profile_is_fixed_and_has_no_configured_options() {
    let profile = internal_title_profile(None);

    assert_eq!(profile.name, "session-title");
    assert_eq!(profile.mode, INTERNAL_TITLE_MODE);
    assert!(profile.prompt.is_none());
    assert!(profile.temperature.is_none());
    assert!(profile.top_p.is_none());
    assert!(profile.options.is_empty());
}

#[test]
fn generated_title_is_trimmed_without_truncation() {
    assert_eq!(finalize_generated_title("  Rust 标题\n"), Ok("Rust 标题".to_string()));
}

#[test]
fn generated_title_at_character_limit_is_preserved() {
    let title = "界".repeat(MAX_TITLE_CHARS);

    assert_eq!(finalize_generated_title(&title), Ok(title));
}

#[test]
fn generated_title_over_character_limit_keeps_prefix_and_ellipsis() {
    let title = "界".repeat(MAX_TITLE_CHARS + 1);

    let finalized = finalize_generated_title(&title).expect("non-empty title should be accepted");

    assert_eq!(finalized, "界".repeat(TITLE_TRUNCATE_KEEP) + "...");
    assert_eq!(finalized.chars().count(), MAX_TITLE_CHARS);
}

#[test]
fn generated_title_trims_before_enforcing_character_limit() {
    let title = "界".repeat(MAX_TITLE_CHARS);

    assert_eq!(finalize_generated_title(&format!("  {title}\n")), Ok(title));
}

#[test]
fn blank_generated_title_is_rejected() {
    assert_eq!(finalize_generated_title(" \n\t "), Err("Empty title generated".to_string()));
}

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

#[tokio::test]
async fn empty_content_is_rejected_before_provider_lookup() {
    let err =
        generate_from_content("session-title-test".to_string(), "   \n\t".to_string(), None, None)
            .await
            .expect_err("blank content must be rejected locally");

    assert_eq!(err, "Empty title source");
}

#[tokio::test]
async fn whitespace_after_truncation_is_rejected_before_agent_lookup() {
    let err = generate_from_content(
        "session-title-long-blank-test".to_string(),
        " ".repeat(CONTENT_TRUNCATE_CHARS + 5),
        Some("openai/gpt-4".to_string()),
        Some("fixture-agent".to_string()),
    )
    .await
    .expect_err("blank truncated content must be rejected locally");

    assert_eq!(err, "Empty title source");
    assert_eq!(MAX_TITLE_CHARS, TITLE_TRUNCATE_KEEP + 3);
}

#[tokio::test]
async fn content_beyond_truncation_limit_cannot_make_blank_prefix_valid() {
    let err = generate_from_content(
        "session-title-hidden-suffix-test".to_string(),
        format!("{}visible suffix", " ".repeat(CONTENT_TRUNCATE_CHARS)),
        None,
        None,
    )
    .await
    .expect_err("content discarded by truncation must not reach provider lookup");

    assert_eq!(err, "Empty title source");
}
