use super::capability_policy::{CapabilityPolicyResolution, resolve_skill_capabilities};
use vw_config_types::agent::DelegateCapabilityPolicy;

#[test]
fn skill_capability_policy_resolves_both_modes_without_model_self_authorization() {
    let skills = ["review", "rustcodegraph", "disabled"];
    let strict_skills = resolve_skill_capabilities(
        &skills,
        [" review "],
        ["rustcodegraph"],
        DelegateCapabilityPolicy::Strict,
        ["disabled"],
    );
    assert_eq!(strict_skills, CapabilityPolicyResolution::new(["review"], ["review"]));

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
