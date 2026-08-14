use super::capability_policy::{
    CapabilityPolicyResolution, resolve_skill_capabilities, resolve_tool_capabilities,
};
use crate::app::agent::tools::ToolSpec;
use vw_config_types::agent::DelegateCapabilityPolicy;

fn tool(id: &str, aliases: &[&str]) -> ToolSpec {
    ToolSpec::new(id, format!("{id} description"), serde_json::json!({}))
        .with_aliases(aliases.iter().copied())
}

#[test]
fn capability_policy_resolves_all_modes_without_model_self_authorization() {
    let tools = vec![
        tool("file_read", &["read"]),
        tool("shell", &["terminal"]),
        tool("agent", &["delegate"]),
    ];
    let defaults = [" read ", "missing"];
    let trusted_user_selection = ["terminal", "unknown"];
    let hard_disabled = ["delegate"];

    let strict = resolve_tool_capabilities(
        &tools,
        &defaults,
        &trusted_user_selection,
        DelegateCapabilityPolicy::Strict,
        &hard_disabled,
    );
    assert_eq!(strict, CapabilityPolicyResolution::new(["file_read"], ["file_read"]));

    let user_override = resolve_tool_capabilities(
        &tools,
        &defaults,
        &trusted_user_selection,
        DelegateCapabilityPolicy::UserOverride,
        &hard_disabled,
    );
    assert_eq!(
        user_override,
        CapabilityPolicyResolution::new(["file_read", "shell"], ["file_read"])
    );

    let unrestricted = resolve_tool_capabilities(
        &tools,
        &defaults,
        &trusted_user_selection,
        DelegateCapabilityPolicy::Unrestricted,
        &hard_disabled,
    );
    assert_eq!(
        unrestricted,
        CapabilityPolicyResolution::new(["file_read", "shell"], ["file_read"])
    );

    let skills = ["review", "rustcodegraph", "disabled"];
    let strict_skills = resolve_skill_capabilities(
        &skills,
        [" review "],
        ["rustcodegraph"],
        DelegateCapabilityPolicy::Strict,
        ["disabled"],
    );
    assert_eq!(strict_skills, CapabilityPolicyResolution::new(["review"], ["review"]));

    let user_override_skills = resolve_skill_capabilities(
        &skills,
        ["review"],
        [" rustcodegraph "],
        DelegateCapabilityPolicy::UserOverride,
        ["disabled"],
    );
    assert_eq!(
        user_override_skills,
        CapabilityPolicyResolution::new(["review", "rustcodegraph"], ["review"])
    );

    let unrestricted_skills = resolve_skill_capabilities(
        &skills,
        std::iter::empty::<&str>(),
        std::iter::empty::<&str>(),
        DelegateCapabilityPolicy::Unrestricted,
        ["disabled"],
    );
    assert_eq!(
        unrestricted_skills,
        CapabilityPolicyResolution::new(["review", "rustcodegraph"], std::iter::empty::<&str>(),)
    );
}
