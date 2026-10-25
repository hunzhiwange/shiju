
#[cfg(test)]
#[tokio::test]
async fn channel_session_lifecycle_new_preserves_old() { runtime_command::session_commands_tests::channel_session_lifecycle_new_preserves_old().await; }
#[cfg(test)]
#[test]
fn channel_session_lifecycle_lists_are_parsed() { runtime_command::session_commands_tests::channel_session_lifecycle_lists_are_parsed(); }
#[cfg(test)]
#[test]
fn channel_session_lifecycle_mutations_are_parsed() { runtime_command::session_commands_tests::channel_session_lifecycle_mutations_are_parsed(); }
#[cfg(test)]
#[test]
fn channel_session_lifecycle_clear_is_separate() { runtime_command::session_commands_tests::channel_session_lifecycle_clear_is_separate(); }
#[cfg(test)]
#[tokio::test]
async fn channel_session_lifecycle_new_clears_cache() { runtime_command::session_commands_tests::channel_session_lifecycle_new_clears_cache().await; }
#[cfg(test)]
#[test]
fn channel_session_lifecycle_help_explains_clear() { runtime_command::session_commands_tests::channel_session_lifecycle_help_explains_clear(); }

#[tokio::test]
async fn channel_session_lifecycle_resume_reloads_persisted_state() { runtime_command::session_commands_tests::channel_session_lifecycle_resume_reloads_persisted_state().await; }

#[tokio::test]
async fn channel_session_lifecycle_resume_rejects_unowned_and_invalid_ids() { runtime_command::session_commands_tests::channel_session_lifecycle_resume_rejects_unowned_and_invalid_ids().await; }

#[tokio::test]
async fn channel_session_lifecycle_resume_isolates_all_chat_dimensions() { runtime_command::session_commands_tests::channel_session_lifecycle_resume_isolates_all_chat_dimensions().await; }

#[tokio::test]
async fn channel_session_lifecycle_archive_requires_matching_confirmation_and_preserves_history() { runtime_command::session_commands_tests::channel_session_lifecycle_archive_requires_matching_confirmation_and_preserves_history().await; }

#[tokio::test]
async fn channel_session_lifecycle_archive_rejects_stale_and_busy_state() { runtime_command::session_commands_tests::channel_session_lifecycle_archive_rejects_stale_and_busy_state().await; }

#[tokio::test]
async fn channel_session_lifecycle_continue_and_rename_use_owned_sessions() { runtime_command::session_commands_tests::channel_session_lifecycle_continue_and_rename_use_owned_sessions().await; }

#[tokio::test]
async fn channel_session_lifecycle_pipeline_persists_and_refreshes_model_options() { runtime_command::session_commands_tests::channel_session_lifecycle_pipeline_persists_and_refreshes_model_options().await; }

#[tokio::test]
async fn channel_session_lifecycle_resume_isolates_projects_and_tenants() { runtime_command::session_commands_tests::channel_session_lifecycle_resume_isolates_projects_and_tenants().await; }

#[tokio::test]
async fn channel_session_lifecycle_failed_creation_and_rename_preserve_binding() { runtime_command::session_commands_tests::channel_session_lifecycle_failed_creation_and_rename_preserve_binding().await; }

#[tokio::test]
async fn channel_session_lifecycle_invalid_saved_model_preserves_current_state() { runtime_command::session_commands_tests::channel_session_lifecycle_invalid_saved_model_preserves_current_state().await; }

#[tokio::test]
async fn channel_session_lifecycle_lists_are_stable_bounded_and_owned() { runtime_command::session_commands_tests::channel_session_lifecycle_lists_are_stable_bounded_and_owned().await; }

#[tokio::test]
async fn channel_session_lifecycle_new_rejects_invalid_model_options() {
    runtime_command::session_commands_tests::channel_session_lifecycle_new_rejects_invalid_model_options().await;
}
