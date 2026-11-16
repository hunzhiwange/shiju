#[test]
fn build_reports_first_missing_required_dependency() {
    agent_loop::agent::test_support::builder_tests::build_reports_first_missing_required_dependency::<super::host::DomainHost>();
}
#[test]
fn build_reports_missing_provider_after_tools_are_present() {
    agent_loop::agent::test_support::builder_tests::build_reports_missing_provider_after_tools_are_present::<super::host::DomainHost>();
}
#[test]
fn build_reports_missing_memory_after_provider_is_present() {
    agent_loop::agent::test_support::builder_tests::build_reports_missing_memory_after_provider_is_present::<super::host::DomainHost>();
}
#[test]
fn build_reports_missing_observer_after_memory_is_present() {
    agent_loop::agent::test_support::builder_tests::build_reports_missing_observer_after_memory_is_present::<super::host::DomainHost>();
}
#[test]
fn build_fills_defaults_when_optional_fields_are_absent() {
    agent_loop::agent::test_support::builder_tests::build_fills_defaults_when_optional_fields_are_absent::<super::host::DomainHost>();
}
#[test]
fn build_collects_tool_specs_in_tool_order() {
    agent_loop::agent::test_support::builder_tests::build_collects_tool_specs_in_tool_order::<super::host::DomainHost>();
}
#[test]
fn build_uses_configured_optional_fields() {
    agent_loop::agent::test_support::builder_tests::build_uses_configured_optional_fields::<super::host::DomainHost>();
}
#[tokio::test]
async fn build_uses_configured_memory_loader() {
    agent_loop::agent::test_support::builder_tests::build_uses_configured_memory_loader::<super::host::DomainHost>().await;
}
