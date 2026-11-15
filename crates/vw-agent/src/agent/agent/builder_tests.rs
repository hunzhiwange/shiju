//! 以真实宿主能力运行所有者库的原测试断言。
#[test]
fn build_reports_first_missing_required_dependency() {
    agent_loop::agent::test_support::builder_tests::build_reports_first_missing_required_dependency::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn build_reports_missing_provider_after_tools_are_present() {
    agent_loop::agent::test_support::builder_tests::build_reports_missing_provider_after_tools_are_present::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn build_reports_missing_memory_after_provider_is_present() {
    agent_loop::agent::test_support::builder_tests::build_reports_missing_memory_after_provider_is_present::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn build_reports_missing_observer_after_memory_is_present() {
    agent_loop::agent::test_support::builder_tests::build_reports_missing_observer_after_memory_is_present::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn build_fills_defaults_when_optional_fields_are_absent() {
    agent_loop::agent::test_support::builder_tests::build_fills_defaults_when_optional_fields_are_absent::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn build_collects_tool_specs_in_tool_order() {
    agent_loop::agent::test_support::builder_tests::build_collects_tool_specs_in_tool_order::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn build_uses_configured_optional_fields() {
    agent_loop::agent::test_support::builder_tests::build_uses_configured_optional_fields::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[tokio::test]
async fn build_uses_configured_memory_loader() {
    agent_loop::agent::test_support::builder_tests::build_uses_configured_memory_loader::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
