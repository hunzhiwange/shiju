#[cfg(not(target_arch = "wasm32"))]
mod persistence {
    #[tokio::test]
    async fn archived_chat_delete_strict_enumeration_rejects_a_file_prefix() {
        gateway_session::archived_acceptance::archived_chat_delete_strict_enumeration_rejects_a_file_prefix::<super::super::message_ops::Host>(
            super::super::tests::test_app_for_tenant,
            std::env::temp_dir().join(format!("vw-agent-test-global-store-{}.sqlite3", std::process::id())),
        ).await;
    }
    #[tokio::test]
    async fn archived_chat_delete_http_removes_multilevel_tree_and_preserves_other_scopes() {
        gateway_session::archived_acceptance::archived_chat_delete_http_removes_multilevel_tree_and_preserves_other_scopes::<super::super::message_ops::Host>(
            super::super::tests::test_app_for_tenant,
            std::env::temp_dir().join(format!("vw-agent-test-global-store-{}.sqlite3", std::process::id())),
        ).await;
    }
    #[tokio::test]
    async fn archived_chat_delete_http_rejects_live_root_and_restore_while_waiting_for_history() {
        gateway_session::archived_acceptance::archived_chat_delete_http_rejects_live_root_and_restore_while_waiting_for_history::<super::super::message_ops::Host>(
            super::super::tests::test_app_for_tenant,
            std::env::temp_dir().join(format!("vw-agent-test-global-store-{}.sqlite3", std::process::id())),
        ).await;
    }
    #[tokio::test]
    async fn archived_chat_delete_storage_failure_keeps_root_and_allows_retry() {
        gateway_session::archived_acceptance::archived_chat_delete_storage_failure_keeps_root_and_allows_retry::<super::super::message_ops::Host>(
            super::super::tests::test_app_for_tenant,
            std::env::temp_dir().join(format!("vw-agent-test-global-store-{}.sqlite3", std::process::id())),
        ).await;
    }
}
