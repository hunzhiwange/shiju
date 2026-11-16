mod host;
mod builder_tests;
mod core_tests;

#[tokio::test]
async fn run_propagates_agent_factory_error() {
    crate::agent::test_support::run_tests::run_propagates_agent_factory_error::<host::DomainHost>().await;
}

#[test]
fn run_single_method_is_available_on_agent_type() {
    crate::agent::test_support::turn_tests::run_single_method_is_available_on_agent_type::<host::DomainHost>();
}
