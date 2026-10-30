//! 验证技能能力策略只解析配置模式，不允许模型自行扩权。

use super::capability_policy::{CapabilityPolicyResolution, resolve_skill_capabilities};
use vw_config_types::agent::DelegateCapabilityPolicy;

#[test]
// 模型只能在预先配置的能力范围内选择，不能借输出内容自行扩大权限。
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

#[test]
fn codex_skills_defaults_disabled_ids_stay_outside_strict_and_unrestricted_capabilities() {
    for policy in [DelegateCapabilityPolicy::Strict, DelegateCapabilityPolicy::Unrestricted] {
        let resolution = resolve_skill_capabilities(
            ["review", "disabled"],
            [" REVIEW ", "disabled"],
            ["disabled"],
            policy,
            ["disabled"],
        );
        assert_eq!(resolution, CapabilityPolicyResolution::new(["review"], ["review"]));
    }
    let resolution = resolve_skill_capabilities(
        ["review"],
        std::iter::empty::<&str>(),
        ["review"],
        DelegateCapabilityPolicy::Strict,
        std::iter::empty::<&str>(),
    );
    assert_eq!(
        resolution,
        CapabilityPolicyResolution::new(std::iter::empty::<&str>(), std::iter::empty::<&str>())
    );
}
