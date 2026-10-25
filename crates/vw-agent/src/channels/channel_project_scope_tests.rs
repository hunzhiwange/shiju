#[test]
fn channel_project_scope_rooms_have_independent_history() {
    session::session_tests::channel_project_scope_rooms_have_independent_history();
}
#[test]
fn channel_project_scope_instances_have_independent_history() {
    session::session_tests::channel_project_scope_instances_have_independent_history();
}
#[test]
fn channel_project_scope_threads_have_independent_history() {
    session::session_tests::channel_project_scope_threads_have_independent_history();
}
#[test]
fn channel_project_scope_senders_have_independent_history() {
    session::session_tests::channel_project_scope_senders_have_independent_history();
}
#[test]
fn channel_project_scope_identity_delimiters_do_not_collide() {
    session::session_tests::channel_project_scope_identity_delimiters_do_not_collide();
}
#[tokio::test]
async fn channel_project_scope_binding_priority_and_immutable_snapshot() {
    session::session_tests::channel_project_scope_binding_priority_and_immutable_snapshot().await;
}
#[tokio::test]
async fn channel_project_scope_invalid_binding_preserves_state() {
    session::session_tests::channel_project_scope_invalid_binding_preserves_state().await;
}
#[tokio::test]
async fn channel_project_scope_sessions_follow_snapshot_and_isolate_rooms() {
    session::session_tests::channel_project_scope_sessions_follow_snapshot_and_isolate_rooms()
        .await;
}
#[tokio::test]
async fn channel_project_scope_project_history_and_routes_are_independent() {
    session::session_tests::channel_project_scope_project_history_and_routes_are_independent()
        .await;
}
#[tokio::test]
async fn channel_project_scope_unavailable_directory_does_not_cache_fake_session() {
    session::session_tests::channel_project_scope_unavailable_directory_does_not_cache_fake_session().await;
}
#[tokio::test]
async fn channel_project_scope_tenant_cannot_resolve_other_tenant_binding() {
    session::session_tests::channel_project_scope_tenant_cannot_resolve_other_tenant_binding()
        .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn channel_project_scope_processor_uses_captured_directory() {
    session::session_tests::channel_project_scope_processor_uses_captured_directory().await;
}
#[tokio::test]
async fn channel_project_scope_memory_reads_only_matching_scope() {
    session::session_tests::channel_project_scope_memory_reads_only_matching_scope().await;
}
#[tokio::test]
async fn channel_project_scope_snapshot_rejects_different_sender() {
    session::session_tests::channel_project_scope_snapshot_rejects_different_sender().await;
}
