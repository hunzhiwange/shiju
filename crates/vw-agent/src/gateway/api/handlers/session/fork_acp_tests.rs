use super::*;
#[test]
#[should_panic(expected = "missing retained-user-1")]
fn fork_acp_negative_missing_history_is_detected() {
    gateway_session::fork_acceptance::fork_acp_negative_missing_history_is_detected();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fork_acp_first_prompt_and_restart_preserve_history() {
    fn app() -> axum::Router {
        crate::gateway::api::handlers::session::router().with_state(state()).layer(axum::Extension(ui_store::current_tenant_scope()))
    }
    let name = concat!(module_path!(), "::fork_acp_first_prompt_and_restart_preserve_history");
    gateway_session::fork_acceptance::fork_acp_first_prompt_and_restart_preserve_history::<crate::gateway::api::handlers::session::message_ops::Host, crate::gateway::api::handlers::session::stream::Host>(app, name.split_once("::").unwrap().1).await;
}
