# Task 11 static evidence

Code migration complete; compilation and behavior NOT verified. Cargo, builds, tests and device checks were prohibited and none were run.

Production migration gain: 0. Production already lives in gateway_session::stream; native wrappers still call that implementation with Host. Dependency direction remains host -> domain, never domain -> vw-agent. Task 1 evidence has no additional external path mount for these sources.

37 functions moved: 36 tests plus pending_stream_ui_session fixture. Names, inputs and assertions preserved; only equivalent processor_events/message/PromptEvent paths changed. Domain lib self-registers host_migration_11_tests and three child modules. Old moved definitions are removed. Existing stream_value_tests (12 tests) cover different inputs/fields and remain unchanged. Native tempfile dev-dependency is supplied by concurrent task 10; no duplicate declaration needed.

## Physical lines

- Host stream.rs: 568 -> 566.
- Host stream_tests.rs: 3250 -> 2222.
- Domain test crates/libs/gateway-session/src/host_migration_11_tests.rs: 9 added lines; zero production lines.
- Domain test crates/libs/gateway-session/src/host_migration_11_tests/group_1.rs: 450 added lines; zero production lines.
- Domain test crates/libs/gateway-session/src/host_migration_11_tests/group_2.rs: 443 added lines; zero production lines.
- Domain test crates/libs/gateway-session/src/host_migration_11_tests/group_3.rs: 138 added lines; zero production lines.

Net host reduction approximately 27%, BELOW 70%; requires closing-task review, not a claim of overall success. stream.rs retains native ports and typed forwarding, no residual production algorithm identified. stream_tests retains real tool execution, filesystem/git changes, config/skill loading, tenant storage, workflow routing and disconnect replay integration. Each retained function and its exact native dependencies is listed below. Further candidates: four history conversion tests and three delegated model policy tests currently require generic StreamHost; decouple their real algorithm inputs before moving rather than cloning a no-op Host. Persistence/trajectory assertions can be split further when real store fixtures cover instance boundaries.

## Every source function

| Original stream_tests function | Lines | Destination or retained reason |
|---|---:|---|
| `gateway_chat_persists_and_reports_session_git_line_counts` | 112 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `codex_skills_defaults_gateway_main_and_child_preload_before_first_request` | 86 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `codex_skills_defaults_gateway_empty_defaults_replace_untrusted_request_allowance` | 13 | gateway_session::host_migration_11_tests::group_1::codex_skills_defaults_gateway_empty_defaults_replace_untrusted_request_allowance |
| `codex_skills_transport_gateway_preserves_directory_and_untrusted_body_roles` | 20 | gateway_session::host_migration_11_tests::group_4::codex_skills_transport_gateway_preserves_directory_and_untrusted_body_roles |
| `forced_child_model_overrides_parent_selection` | 15 | gateway_session::host_migration_11_tests::group_6 |
| `incomplete_forced_child_binding_is_ignored` | 14 | gateway_session::host_migration_11_tests::group_6 |
| `child_soft_preference_does_not_bypass_profile_fit` | 12 | gateway_session::host_migration_11_tests::group_6 |
| `prepare_stream_history_maps_roles_tool_results_and_last_user_query` | 28 | gateway_session::host_migration_11_tests::group_4::prepare_stream_history_maps_roles_tool_results_and_last_user_query |
| `prepare_stream_history_promotes_last_prior_user_unless_resume_history_only` | 18 | gateway_session::host_migration_11_tests::group_4::prepare_stream_history_promotes_last_prior_user_unless_resume_history_only |
| `prepare_stream_history_accepts_structured_attachment_blocks` | 24 | gateway_session::host_migration_11_tests::group_4::prepare_stream_history_accepts_structured_attachment_blocks |
| `stream_payload_for_non_done_event_encodes_delta_step_finish_and_error_state` | 78 | gateway_session::host_migration_11_tests::group_1::stream_payload_for_non_done_event_encodes_delta_step_finish_and_error_state |
| `compaction_lifecycle_events_encode_stable_gateway_payloads` | 29 | gateway_session::host_migration_11_tests::group_1::compaction_lifecycle_events_encode_stable_gateway_payloads |
| `gateway_stream_preserves_existing_tool_payloads` | 35 | gateway_session::host_migration_11_tests::group_1::gateway_stream_preserves_existing_tool_payloads |
| `gateway_tool_event_preserves_step_and_payload` | 84 | gateway_session::host_migration_11_tests::group_1::gateway_tool_event_preserves_step_and_payload |
| `stream_payload_for_non_done_event_handles_step_start_post_tool_round_and_done` | 50 | gateway_session::host_migration_11_tests::group_1::stream_payload_for_non_done_event_handles_step_start_post_tool_round_and_done |
| `merge_gateway_request_options_forwards_allowed_tools_to_acp` | 21 | gateway_session::host_migration_11_tests::group_1::merge_gateway_request_options_forwards_allowed_tools_to_acp |
| `delegated_gateway_options_preserve_tool_registration_and_pass_skill_policy` | 29 | gateway_session::host_migration_11_tests::group_1::delegated_gateway_options_preserve_tool_registration_and_pass_skill_policy |
| `merge_gateway_request_options_preserves_explicit_options_over_agent_defaults` | 41 | gateway_session::host_migration_11_tests::group_1::merge_gateway_request_options_preserves_explicit_options_over_agent_defaults |
| `merge_gateway_request_options_replaces_non_object_options_with_object` | 19 | gateway_session::host_migration_11_tests::group_1::merge_gateway_request_options_replaces_non_object_options_with_object |
| `chat_reasoning_effort_reaches_acp_and_model_options` | 50 | gateway_session::host_migration_11_tests::group_1::chat_reasoning_effort_reaches_acp_and_model_options |
| `stream_part_builders_fill_expected_message_boundaries` | 57 | gateway_session::host_migration_11_tests::group_2::stream_part_builders_fill_expected_message_boundaries |
| `generated_stream_parts_and_preallocated_ids_are_non_empty` | 23 | gateway_session::host_migration_11_tests::group_2::generated_stream_parts_and_preallocated_ids_are_non_empty |
| `collect_gateway_chat_stream_payloads_returns_output_on_done` | 26 | gateway_session::host_migration_11_tests::group_2::collect_gateway_chat_stream_payloads_returns_output_on_done |
| `collect_gateway_chat_stream_payloads_rejects_close_without_done` | 14 | gateway_session::host_migration_11_tests::group_2::collect_gateway_chat_stream_payloads_rejects_close_without_done |
| `resolve_delegate_request_overrides_without_agent_returns_explicit_values` | 18 | Host: real Host-bound entry or direct fixture for retained integration tests |
| `resolve_main_request_overrides_preserves_registered_tool_selection` | 17 | gateway_session::host_migration_11_tests::group_6 |
| `resolve_delegate_request_overrides_rejects_unknown_agent` | 14 | Host: real Host-bound entry or direct fixture for retained integration tests |
| `merge_gateway_request_options_fills_missing_agent_defaults` | 22 | gateway_session::host_migration_11_tests::group_2::merge_gateway_request_options_fills_missing_agent_defaults |
| `merge_gateway_request_options_existing_acp_request_reuses_allowed_tools` | 21 | gateway_session::host_migration_11_tests::group_2::merge_gateway_request_options_existing_acp_request_reuses_allowed_tools |
| `merge_gateway_request_options_non_acp_removes_reasoning_aliases` | 25 | gateway_session::host_migration_11_tests::group_2::merge_gateway_request_options_non_acp_removes_reasoning_aliases |
| `merge_gateway_request_options_explicit_acp_tools_enable_acp` | 18 | gateway_session::host_migration_11_tests::group_2::merge_gateway_request_options_explicit_acp_tools_enable_acp |
| `stream_patch_recorder_no_active_step_is_a_noop` | 11 | gateway_session::host_migration_11_tests::group_2::stream_patch_recorder_no_active_step_is_a_noop |
| `stream_patch_recorder_begin_step_records_start_and_active_step` | 14 | gateway_session::host_migration_11_tests::group_2::stream_patch_recorder_begin_step_records_start_and_active_step |
| `stream_patch_recorder_mismatched_step_discards_active_step_without_snapshot_io` | 16 | gateway_session::host_migration_11_tests::group_2::stream_patch_recorder_mismatched_step_discards_active_step_without_snapshot_io |
| `stream_patch_recorder_take_parts_drains_recorded_parts` | 20 | gateway_session::host_migration_11_tests::group_2::stream_patch_recorder_take_parts_drains_recorded_parts |
| `patch_line_stats_accumulate_all_file_diffs` | 24 | gateway_session::host_migration_11_tests::group_2::patch_line_stats_accumulate_all_file_diffs |
| `collect_gateway_chat_stream_payloads_concatenates_deltas_and_ignores_other_events` | 18 | gateway_session::host_migration_11_tests::group_2::collect_gateway_chat_stream_payloads_concatenates_deltas_and_ignores_other_events |
| `collect_gateway_chat_stream_payloads_returns_stream_error` | 14 | gateway_session::host_migration_11_tests::group_2::collect_gateway_chat_stream_payloads_returns_stream_error |
| `collect_gateway_chat_stream_payloads_uses_fallback_for_malformed_error` | 14 | gateway_session::host_migration_11_tests::group_2::collect_gateway_chat_stream_payloads_uses_fallback_for_malformed_error |
| `docs_stream_request` | 13 | Host: real Host-bound entry or direct fixture for retained integration tests |
| `make_docs_workspace` | 8 | Host: real Host-bound entry or direct fixture for retained integration tests |
| `chat_stream_rejects_missing_workspace_without_retryable_status` | 16 | gateway_session::stream_acceptance (real host registration retained) |
| `native_workflow_gateway_persists_bound_graph_without_llm_artifacts` | 72 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `stream_trajectory_commit_rejects_llm_without_raw_artifacts` | 35 | gateway_session::host_migration_11_tests::group_4::stream_trajectory_commit_rejects_llm_without_raw_artifacts |
| `collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids` | 31 | gateway_session::stream_acceptance (real host registration retained) |
| `gateway_first_turn_registers_parent_before_thread_tool_use` | 47 | gateway_session::stream_acceptance (real host registration retained) |
| `stream_trajectory_commit_persists_steps_tools_and_raw_without_ui_save` | 102 | gateway_session::host_migration_11_tests::group_4::stream_trajectory_commit_persists_steps_tools_and_raw_without_ui_save |
| `stream_processing_elapsed_persists_and_reloads_explicit_request_bounds` | 50 | gateway_session::host_migration_11_tests::group_5::stream_processing_elapsed_persists_and_reloads_explicit_request_bounds |
| `persist_stream_turn_uses_request_start_for_user_timestamp` | 50 | gateway_session::host_migration_11_tests::group_5::persist_stream_turn_uses_request_start_for_user_timestamp |
| `chat_stream_returns_sse_response_for_docs_request` | 68 | gateway_session::stream_acceptance (real host registration retained) |
| `pending_stream_ui_session` | 24 | gateway_session::host_migration_11_tests::group_2::pending_stream_ui_session |
| `stream_ui_commit_replaces_current_draft_despite_tool_markers_or_partial_text` | 35 | gateway_session::host_migration_11_tests::group_2::stream_ui_commit_replaces_current_draft_despite_tool_markers_or_partial_text |
| `stream_ui_commit_does_not_claim_another_turns_identical_answer` | 25 | gateway_session::host_migration_11_tests::group_2::stream_ui_commit_does_not_claim_another_turns_identical_answer |
| `shell_session_running_update_retains_gateway_metadata_without_finishing` | 21 | gateway_session::host_migration_11_tests::group_2::shell_session_running_update_retains_gateway_metadata_without_finishing |
| `streamed_acp_tool_part_retains_display_title_and_input` | 28 | gateway_session::host_migration_11_tests::group_3::streamed_acp_tool_part_retains_display_title_and_input |
| `recovery_event_preserves_status_without_assistant_content` | 39 | gateway_session::host_migration_11_tests::group_3::recovery_event_preserves_status_without_assistant_content |
| `agcollab_gateway_keeps_inherited_skill_ceiling` | 20 | gateway_session::host_migration_11_tests::group_3::agcollab_gateway_keeps_inherited_skill_ceiling |
| `agcollab_delivery_reaches_next_model_request` | 118 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `video_edit_entry_gateway_unbound_projects_template_edit_menu_and_persistence` | 165 | Host: crate::gateway::api::handlers::workflow::applications_router, crate::global::paths, crate::global_database::GlobalDatabase::for_home, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::session::ui_store::load_session_scoped_for_tenant, crate::session::ui_store::resolve_session_scope_id_for_tenant, crate::test_support::global_async_env_lock, crate::workflow:: |
| `video_edit_entry_gateway_explicit_unavailable_model_does_not_start_graph` | 26 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `collect_video_edit_test_stream` | 26 | Host: crate::session::ui_store::current_tenant_scope |
| `agent_native_defaults_reach_registered_tools_and_prompt` | 136 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `agent_native_defaults_do_not_grant_disabled_mcp` | 28 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `agent_session_identity_rejects_forged_ui_binding` | 27 | gateway_session::stream_acceptance (real host registration retained) |
| `agent_session_identity_survives_profile_edits` | 129 | Host: agent_session::llm::StreamEvent::Delta, agent_session::llm::StreamEvent::Done, agent_session::processor::enqueue_test_llm_stream, agent_session::processor::take_test_llm_messages, agent_session::processor::test_llm_stream_guard, agent_session::session::fork, crate::config::get_current, crate::config::update, crate::permission::next::Action::Deny, crate::session::agent_binding::apply_to_config, crate::session::agent_binding::load, crate::session::capability_policy::native_agent_defaults, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_config::set_config_field_result, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `agent_session_identity_derives_backend_and_rejects_conflicts` | 31 | gateway_session::stream_acceptance (real host registration retained) |
| `acp_background_approval_gateway_uses_turn_grant_after_agent_resolution` | 46 | Host: crate::session::agent_binding::confirmed_full_access, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `agent_model_inheritance_preserves_explicit_source_in_processor_options` | 23 | gateway_session::host_migration_11_tests::group_4::agent_model_inheritance_preserves_explicit_source_in_processor_options |
| `acp_model_source_rejects_invalid_and_conflicting_runtime_options` | 17 | gateway_session::host_migration_11_tests::group_4::acp_model_source_rejects_invalid_and_conflicting_runtime_options |
| `agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice` | 44 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `native_tool_search_core_loop_gateway_encodes_typed_model_calls` | 25 | gateway_session::host_migration_11_tests::group_4::native_tool_search_core_loop_gateway_encodes_typed_model_calls |
| `native_tool_search_core_loop_gateway_history_reaches_model_as_native_item` | 32 | Host: crate::session::llm::native_tools::NativeHistoryItem, crate::session::processor::llm_messages::session_message_to_llm_message, crate::session::session::Message, crate::session::session::Role::Tool, crate::session::session::Role::User |
| `agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority` | 101 | gateway_session::stream_acceptance (real host registration retained) |
| `gateway_first_response_replays_the_same_session_and_rejects_other_directories` | 61 | gateway_session::stream_acceptance (real host registration retained) |
| `gateway_first_response_failure_preserves_history_until_same_session_retries` | 116 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `gateway_first_response_missing_replay_does_not_start_a_new_turn` | 16 | gateway_session::stream_acceptance (real host registration retained) |
| `gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing` | 105 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `todo_continuation_emits_status_without_changing_assistant_text` | 21 | gateway_session::host_migration_11_tests::group_3::todo_continuation_emits_status_without_changing_assistant_text |
| `task_pool_agent_uses_existing_definition_and_defaults` | 50 | Host: crate::agent::permission_rules, crate::config::update, crate::permission::next::Action::Deny, crate::permission::next::evaluate, crate::session::agent_binding::load, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `task_pool_agent_rejects_unknown_or_disabled_definition` | 25 | gateway_session::stream_acceptance (real host registration retained) |
| `task_pool_recovery_collector_waits_for_producer_shutdown` | 13 | gateway_session::host_migration_11_tests::group_3::task_pool_recovery_collector_waits_for_producer_shutdown |
| `task_pool_recovery_collector_keeps_shutdown_failure_after_done` | 15 | gateway_session::host_migration_11_tests::group_3::task_pool_recovery_collector_keeps_shutdown_failure_after_done |

## Every stream.rs symbol

| Symbol | Retained boundary |
|---|---|
| `current_tenant_scope` | Native tenant, context, config, prompt, storage or lifecycle port |
| `resolve_directory` | Native tenant, context, config, prompt, storage or lifecycle port |
| `with_tenant_instance` | Native tenant, context, config, prompt, storage or lifecycle port |
| `tool_context` | Native tenant, context, config, prompt, storage or lifecycle port |
| `approval_manager` | Native tenant, context, config, prompt, storage or lifecycle port |
| `with_tenant_scope` | Native tenant, context, config, prompt, storage or lifecycle port |
| `now_ms` | Native tenant, context, config, prompt, storage or lifecycle port |
| `is_successful_finish_reason` | Native tenant, context, config, prompt, storage or lifecycle port |
| `first_response_source` | Native tenant, context, config, prompt, storage or lifecycle port |
| `delegated_model_ref` | Native tenant, context, config, prompt, storage or lifecycle port |
| `agent` | Native tenant, context, config, prompt, storage or lifecycle port |
| `config` | Native tenant, context, config, prompt, storage or lifecycle port |
| `project_config` | Native tenant, context, config, prompt, storage or lifecycle port |
| `native_agent_defaults` | Native tenant, context, config, prompt, storage or lifecycle port |
| `prepare_binding` | Native tenant, context, config, prompt, storage or lifecycle port |
| `resolve_full_access` | Native tenant, context, config, prompt, storage or lifecycle port |
| `fork_execution` | Native tenant, context, config, prompt, storage or lifecycle port |
| `start_fork_execution` | Native tenant, context, config, prompt, storage or lifecycle port |
| `locked_acp_model` | Native tenant, context, config, prompt, storage or lifecycle port |
| `forked_session` | Native tenant, context, config, prompt, storage or lifecycle port |
| `load_fork_history` | Native tenant, context, config, prompt, storage or lifecycle port |
| `project_from_directory` | Native tenant, context, config, prompt, storage or lifecycle port |
| `refresh_stream_diff_summary` | Native tenant, context, config, prompt, storage or lifecycle port |
| `refresh_session_diff_summary` | Native tenant, context, config, prompt, storage or lifecycle port |
| `spawn_first_response_title` | Native tenant, context, config, prompt, storage or lifecycle port |
| `load_session_scoped_for_tenant_fallible` | Native tenant, context, config, prompt, storage or lifecycle port |
| `load_agent_session_scoped_for_tenant` | Native tenant, context, config, prompt, storage or lifecycle port |
| `load_visible_task_states` | Native tenant, context, config, prompt, storage or lifecycle port |
| `commit_first_response` | Native tenant, context, config, prompt, storage or lifecycle port |
| `save_agent_session_scoped_for_tenant` | Native tenant, context, config, prompt, storage or lifecycle port |
| `save_session_ui_scoped_for_tenant` | Native tenant, context, config, prompt, storage or lifecycle port |
| `commit_session_ui_snapshot_scoped_for_tenant` | Native tenant, context, config, prompt, storage or lifecycle port |
| `reserve_pending_session` | Native tenant, context, config, prompt, storage or lifecycle port |
| `child_agent_handoff_from_output` | Native tenant, context, config, prompt, storage or lifecycle port |
| `task_summary` | Native tenant, context, config, prompt, storage or lifecycle port |
| `start_prompt` | Native tenant, context, config, prompt, storage or lifecycle port |
| `collect_gateway_chat_stream` | Host type binding for existing production callers or retained test-only bridge |
| `collect_gateway_chat_stream_with_observer` | Host type binding for existing production callers or retained test-only bridge |
| `collect_background_gateway_chat_stream` | Host type binding for existing production callers or retained test-only bridge |
| `collect_scheduler_gateway_chat_stream` | Host type binding for existing production callers or retained test-only bridge |
| `persist_stream_chat_turn` | Host type binding for existing production callers or retained test-only bridge |
| `persist_child_agent_turn` | Host type binding for existing production callers or retained test-only bridge |
| `commit_collected_child_agent_turn` | Host type binding for existing production callers or retained test-only bridge |
| `reserve_stream_agent_session` | Host type binding for existing production callers or retained test-only bridge |
| `chat_stream` | Host type binding for existing production callers or retained test-only bridge |
| `chat_stream_stop` | Host type binding for existing production callers or retained test-only bridge |
| `resolve_delegate_request_overrides` | Host type binding for existing production callers or retained test-only bridge |
| `prepare_gateway_agent_request` | Host type binding for existing production callers or retained test-only bridge |
| `session_model` | Host type binding for existing production callers or retained test-only bridge |
| `start_gateway_chat_stream` | Host type binding for existing production callers or retained test-only bridge |
| `start_gateway_chat_producer` | Host type binding for existing production callers or retained test-only bridge |
| `prepare_stream_history` | Host type binding for existing production callers or retained test-only bridge |
| `persist_stream_chat_turn_for_agent` | Host type binding for existing production callers or retained test-only bridge |
| `commit_stream_session_trajectory` | Host type binding for existing production callers or retained test-only bridge |
| `delegated_model_override` | Host type binding for existing production callers or retained test-only bridge |

Host is the concrete StreamHost/MessageHost implementation. current_tenant_scope/resolve_directory/with_tenant_instance inject tenant and instance; tool_context/approval_manager/with_tenant_scope preserve runtime capabilities. now_ms/is_successful_finish_reason/first_response_source delegate existing session policy. No moved production algorithm claimed. Stream stop delegates the same domain entry. All retained wrappers and ports above preserve behavior.

## Future checks

`python3 scripts/cargo_slot.py cargo test -p gateway-session --lib host_migration_11_tests -- --test-threads=1` (expected 36, must be nonzero and failed=0).

`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::session::stream::stream_tests -- --test-threads=1` (confirm actual fully-qualified prefix with real --list first; expected 42 retained tests). Verify all 36 migrated names disappear from host list, not a zero-test pass. Every retained name is in the table for exact filtering.

Static checks only: module registration, equivalent type sources, diff and assertions. All compilation/behavior checks unrun.

## Recovery pass (supersedes earlier retained-boundary decisions)

Read evidence/final.md and evidence/remaining-host.json: task 11 originally retained 2788 lines, whole-plan gate remained open. This pass removes the unnecessary StreamHost binding from prepare_stream_history, gateway_content_to_title_source, commit_stream_session_trajectory and reserve_stream_agent_session. Existing production callers now reach these concrete domain functions directly. Store operations directly use the SAME session_store::ui_store functions formerly re-exported by host; title and child-agent helpers directly use the SAME session_engine functions. Removed 11 trait methods and matching native implementations; no permission, tenant selection or resource capture moved or bypassed.

persist_stream_messages is a real production writer extracted from persist_stream_chat_turn_for_agent, which still captures the native clock and invokes summary IO. The returned OwnedMutexGuard preserves the original history lock across summary refresh. Both timestamp tests use this writer and real agent_project::instance and session_engine::message persistence. Test clock is explicit 9000; asserted request timing remains 1000/1250 and 1000/9000 respectively. No mock Host, fake storage, path mount or reverse dependency added.

Additional tests migrated (all prior assertions retained):
- `codex_skills_transport_gateway_preserves_directory_and_untrusted_body_roles`
- `prepare_stream_history_maps_roles_tool_results_and_last_user_query`
- `prepare_stream_history_promotes_last_prior_user_unless_resume_history_only`
- `prepare_stream_history_accepts_structured_attachment_blocks`
- `stream_trajectory_commit_rejects_llm_without_raw_artifacts`
- `stream_trajectory_commit_persists_steps_tools_and_raw_without_ui_save`
- `agent_model_inheritance_preserves_explicit_source_in_processor_options`
- `acp_model_source_rejects_invalid_and_conflicting_runtime_options`
- `native_tool_search_core_loop_gateway_encodes_typed_model_calls`
- `stream_processing_elapsed_persists_and_reloads_explicit_request_bounds`
- `persist_stream_turn_uses_request_start_for_user_timestamp`

Previous event tests now use public session_port types rather than private imports inside processor_events. ToolCall remains the public processor_events re-export; Search kind is deserialized from its stable serialized value to avoid adding a dependency solely for a test enum.

Current physical source counts:
- Host stream.rs: 422 lines.
- Host stream_tests.rs: 1830 lines.
- Domain test host_migration_11_tests.rs: 11 lines.
- Domain test group_1.rs: 450 lines.
- Domain test group_2.rs: 443 lines.
- Domain test group_3.rs: 140 lines.
- Domain test group_4.rs: 295 lines.
- Domain test group_5.rs: 103 lines.

Cumulative host reduction: 1566/3818 (41.0%); current 2252 lines. Still below 70%; do not count this task as meeting the gate. Domain migration filter now expects 47 tests; host stream_tests expects 31. All Cargo/build/behavior checks remain UNRUN per explicit instruction. Static source/diff checks only.

Remaining host responsibilities: real configured processor streams, skill preload/collaboration delivery, tool file/git mutations, approval grants, trusted binding config mutation, workflow applications router integration, title provider failure/retry, replay and cancellation. Three delegated model tests still use native model selection through Host. The mapping table above lists exact remaining dependencies. The native_tool_search history integration also still checks model SDK conversion and requires its owning runtime fixtures. These are explicit continuing candidates; the remaining 70% gap is not resolved by this pass.

Future commands remain the same precise package/lib filters above, now expected 47 domain tests and 31 retained host tests; verify actual list and zero failures when authorized. No compile or behavior pass claimed.

## Third recovery pass (latest mapping supersedes previous snapshots)

Read latest final.md: host 51915 lines, gap16915; task11 prioritized model selection and override assertions.

Production: delegated_model_override now directly formats trimmed provider/model only after has_complete_model_binding. Inspected agent_loop::definitions::resolve_model_ref: its first branch does exactly that when both fields are nonempty, and its combined-model fallback is unreachable under this gate. Removed delegated_model_ref from StreamHost and concrete Host. No new dependency, widened fallback, or behavior intended. resolve_delegate_policy now owns the environment-free force-model conflict, effective/default model choice, main/delegate tool filtering and capability options. Existing async resolver obtains actual native defaults then invokes this function. The main tool-selection test supplies explicit default AgentDefinitionConfig and no resolved model default; all original assertions remain. Three forced/soft model tests moved unchanged to self-registered group_6.

stream_acceptance is feature=test-support, native-only. Ten named assertion bodies use the existing StreamHost and real message/UI stores. Original host tests retain attributes, global environment lock, RecoveryConfig and config mutation where needed, and call the matching generic assertion function with the ACTUAL Host. These are still HOST-REGISTERED acceptance tests, NOT independently registered domain unit tests. No mock or duplicate App state added. Two docs request/filesystem fixtures now shared here. Domain HTTP handler is called directly with gateway_instance query; only the former trivial query field conversion wrapper is bypassed, native directory/admission/prompt ports are retained. No HTTP router/authentication coverage claim made.

Removed all five test-only async forwarding wrappers (resolve_delegate_request_overrides, prepare_gateway_agent_request, session_model, start_gateway_chat_stream, start_gateway_chat_producer), their SharedPromptCancel alias, and delegated_model_override wrapper. Remaining native tests call real domain entries with Host explicitly.

Mapping:
- `forced_child_model_overrides_parent_selection` -> `gateway_session::host_migration_11_tests::group_6`
- `incomplete_forced_child_binding_is_ignored` -> `gateway_session::host_migration_11_tests::group_6`
- `child_soft_preference_does_not_bypass_profile_fit` -> `gateway_session::host_migration_11_tests::group_6`
- `chat_stream_rejects_missing_workspace_without_retryable_status` -> `gateway_session::stream_acceptance (real host registration retained)`
- `collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids` -> `gateway_session::stream_acceptance (real host registration retained)`
- `gateway_first_turn_registers_parent_before_thread_tool_use` -> `gateway_session::stream_acceptance (real host registration retained)`
- `chat_stream_returns_sse_response_for_docs_request` -> `gateway_session::stream_acceptance (real host registration retained)`
- `agent_session_identity_rejects_forged_ui_binding` -> `gateway_session::stream_acceptance (real host registration retained)`
- `agent_session_identity_derives_backend_and_rejects_conflicts` -> `gateway_session::stream_acceptance (real host registration retained)`
- `agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority` -> `gateway_session::stream_acceptance (real host registration retained)`
- `gateway_first_response_replays_the_same_session_and_rejects_other_directories` -> `gateway_session::stream_acceptance (real host registration retained)`
- `gateway_first_response_missing_replay_does_not_start_a_new_turn` -> `gateway_session::stream_acceptance (real host registration retained)`
- `task_pool_agent_rejects_unknown_or_disabled_definition` -> `gateway_session::stream_acceptance (real host registration retained)`
- `resolve_main_request_overrides_preserves_registered_tool_selection` -> `gateway_session::host_migration_11_tests::group_6`

Current physical counts:
- Host stream.rs: 330 lines.
- Host stream_tests.rs: 1420 lines.
- Domain stream_acceptance.rs: 401 lines.
- Domain host_migration_11_tests/group_6.rs: 62 lines.

Cumulative reduction 2068/3818 = 54.2%; still below70%. Pure domain test filter now expects 51; original host stream_tests filter expects 27, including ten thin acceptance invocations. Do not count relocated acceptance bodies as new independent test registration or compilation isolation.

Remaining exact functions and native references (thin acceptance invocations are identifiable from stream_acceptance mapping above):

| Host function | Lines | Native dependencies |
|---|---:|---|
| `gateway_chat_persists_and_reports_session_git_line_counts` | 112 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `codex_skills_defaults_gateway_main_and_child_preload_before_first_request` | 86 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `resolve_delegate_request_overrides_without_agent_returns_explicit_values` | 19 | Actual Host assertion invocation / dependent helper |
| `resolve_delegate_request_overrides_rejects_unknown_agent` | 14 | Actual Host assertion invocation / dependent helper |
| `chat_stream_rejects_missing_workspace_without_retryable_status` | 5 | Actual Host assertion invocation / dependent helper |
| `native_workflow_gateway_persists_bound_graph_without_llm_artifacts` | 72 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids` | 5 | Actual Host assertion invocation / dependent helper |
| `gateway_first_turn_registers_parent_before_thread_tool_use` | 5 | Actual Host assertion invocation / dependent helper |
| `chat_stream_returns_sse_response_for_docs_request` | 5 | Actual Host assertion invocation / dependent helper |
| `agcollab_delivery_reaches_next_model_request` | 118 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `video_edit_entry_gateway_unbound_projects_template_edit_menu_and_persistence` | 165 | crate::gateway::api::handlers::workflow::applications_router, crate::global::paths, crate::global_database::GlobalDatabase::for_home, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::session::ui_store::load_session_scoped_for_tenant, crate::session::ui_store::resolve_session_scope_id_for_tenant, crate::test_support::global_async_env_lock, crate::workflow:: |
| `video_edit_entry_gateway_explicit_unavailable_model_does_not_start_graph` | 25 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `collect_video_edit_test_stream` | 27 | crate::session::ui_store::current_tenant_scope |
| `agent_native_defaults_reach_registered_tools_and_prompt` | 136 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `agent_native_defaults_do_not_grant_disabled_mcp` | 28 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `agent_session_identity_rejects_forged_ui_binding` | 11 | crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |
| `agent_session_identity_survives_profile_edits` | 129 | agent_session::llm::StreamEvent::Delta, agent_session::llm::StreamEvent::Done, agent_session::processor::enqueue_test_llm_stream, agent_session::processor::take_test_llm_messages, agent_session::processor::test_llm_stream_guard, agent_session::session::fork, crate::config::get_current, crate::config::update, crate::permission::next::Action::Deny, crate::session::agent_binding::apply_to_config, crate::session::agent_binding::load, crate::session::capability_policy::native_agent_defaults, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_config::set_config_field_result, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `agent_session_identity_derives_backend_and_rejects_conflicts` | 11 | crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |
| `acp_background_approval_gateway_uses_turn_grant_after_agent_resolution` | 46 | crate::session::agent_binding::confirmed_full_access, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice` | 44 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `native_tool_search_core_loop_gateway_history_reaches_model_as_native_item` | 32 | crate::session::llm::native_tools::NativeHistoryItem, crate::session::processor::llm_messages::session_message_to_llm_message, crate::session::session::Message, crate::session::session::Role::Tool, crate::session::session::Role::User |
| `agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority` | 18 | crate::config::update, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |
| `gateway_first_response_replays_the_same_session_and_rejects_other_directories` | 12 | crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |
| `gateway_first_response_failure_preserves_history_until_same_session_retries` | 116 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `gateway_first_response_missing_replay_does_not_start_a_new_turn` | 6 | crate::test_support::global_async_env_lock |
| `gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing` | 105 | stream_runtime_acceptance via actual native Host (host registration retained) |
| `task_pool_agent_uses_existing_definition_and_defaults` | 50 | crate::agent::permission_rules, crate::config::update, crate::permission::next::Action::Deny, crate::permission::next::evaluate, crate::session::agent_binding::load, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `task_pool_agent_rejects_unknown_or_disabled_definition` | 10 | crate::config::update, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |

Remaining non-thin functions exercise concrete test LLM stream queue/capture, registry tool execution/skills, native binding scope/config mutation, workflow application HTTP router, title-provider socket failure/retry, and cancellation of an executing shell tool. They cannot be replaced with a simulated host to meet the numeric gate. This pass closes all model override pure tests and removes every test-only forwarding function; further independent registration would require moving the real native runner fixture API with its owning subsystem rather than cloning it here.

Static source checks: old generic delegated_model_override call sites and removed StreamHost method references absent; exact registration and feature present; git diff --check clean for task files. No Cargo, build, test, test listing or executable invocation run. Future commands above unchanged; verify new expected counts and actual host list when authorized. Existing gateway-session native test-support uuid feature/dependency is reused, no Cargo manifest/lock modifications owned by this pass.

## Fourth recovery pass (latest physical snapshot and mapping)

Read task-59-recovery-priorities.json owner11 priority2: explicit collaboration, video snapshot, native defaults, profile edits, title retry and cancellation candidates. No blanket host-coupling exemption applied.

RuntimeAcceptanceHost is native feature=test-support only. It extends existing StreamHost with actual env lock/recovery guard, original stream guard/queue, captured request projection, tool ids, config mutation and channel boundary register/start/release. Native implementation calls the SAME original functions. CapturedRequest projects only fields asserted here (messages/system/model/tool descriptions), not App or AppState. No mock admission, substituted queue, alternate producer, reverse dependency or host source mount. Existing optional llm-runtime/model-types/agent-env/futures-util and tokio test-support features suffice; no manifest or lock changes owned.

All original assertions retained in domain modules but still run through original registered host tests and REAL native stream; independent unit test count remains51. These are domain assertions + host boundary, NOT independent domain tests, compilation isolation or measured speedup. Former synchronous runtime scenarios execute their unchanged runtime inside spawn_blocking, called by native tokio test; original runtime/lock and guard lifetimes remain within the worker. No tests were run.

Full-body mappings:
- `stream_tests::codex_skills_defaults_gateway_main_and_child_preload_before_first_request` -> `gateway_session::stream_runtime_acceptance::codex_skills_defaults_gateway_main_and_child_preload_before_first_request` through actual Host. Original native registration retained.
- `stream_tests::agcollab_delivery_reaches_next_model_request` -> `gateway_session::stream_runtime_acceptance::agcollab_delivery_reaches_next_model_request` through actual Host. Original native registration retained.
- `stream_tests::agent_native_defaults_reach_registered_tools_and_prompt` -> `gateway_session::stream_runtime_acceptance::agent_native_defaults_reach_registered_tools_and_prompt` through actual Host. Original native registration retained.
- `stream_tests::native_workflow_gateway_persists_bound_graph_without_llm_artifacts` -> `gateway_session::stream_runtime_acceptance::native_workflow_gateway_persists_bound_graph_without_llm_artifacts` through actual Host. Original native registration retained.
- `stream_tests::video_edit_entry_gateway_explicit_unavailable_model_does_not_start_graph` -> `gateway_session::stream_runtime_acceptance::video_edit_entry_gateway_explicit_unavailable_model_does_not_start_graph` through actual Host. Original native registration retained.
- `stream_tests::gateway_first_response_failure_preserves_history_until_same_session_retries` -> `gateway_session::stream_runtime_acceptance::gateway_first_response_failure_preserves_history_until_same_session_retries` through actual Host. Original native registration retained.
- `stream_tests::gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing` -> `gateway_session::stream_runtime_acceptance::gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing` through actual Host. Original native registration retained.
- `stream_tests::agent_native_defaults_do_not_grant_disabled_mcp` -> `gateway_session::stream_runtime_acceptance::agent_native_defaults_do_not_grant_disabled_mcp` through actual Host. Original native registration retained.
- `stream_tests::agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice` -> `gateway_session::stream_runtime_acceptance::agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice` through actual Host. Original native registration retained.
- `stream_tests::gateway_chat_persists_and_reports_session_git_line_counts` -> `gateway_session::stream_runtime_acceptance::gateway_chat_git_line_counts` through actual Host. Original native registration retained.

Partial mappings:
- agent_session_identity_survives_profile_edits: config/profile stream loop and captured system-prompt assertions -> stream_runtime_acceptance::profile_edit_stream. Host retains real fork call, binding reads, trusted permission setting mutation, native capability admission and disabled-session assertions.
- video_edit_entry_gateway_unbound_projects_template_edit_menu_and_persistence: progress/output and UI snapshot assertions -> video_edit_progress_and_snapshot, consuming the actual collected stream, seed digest and real UI storage. Host retains transport installation, seed database setup, real workflow applications router edit/read, source permissions and ledger cross-template/path assertions.
- gateway_chat_persists_and_reports_session_git_line_counts: real repository initialization remains in a native closure; domain function uses the actual stream Host, patch queue and real git-backed workspace, preserves event ordering and durable summary assertions.
- agent_native_defaults_reach_registered_tools_and_prompt: MCP python source remains supplied by host include_str; contents are written by domain fixture and process launch still uses the actual native runtime. No host Rust source borrowing.

Current physical host files:
- stream.rs: 328 lines.
- stream_tests.rs: 547 lines.
- stream_acceptance_host.rs: 27 lines.
- session.rs: 4 added test-only registration lines.
- Original3818 -> 906 incl new native adapter and registration, cumulative reduction 2912 (76.3%), above70% task threshold. Whole-plan gate still owned by task59.

Domain physical test-support files:
- crates/libs/gateway-session/src/stream_runtime_acceptance.rs: 503 lines.
- crates/libs/gateway-session/src/stream_runtime_acceptance_host.rs: 29 lines.
- crates/libs/gateway-session/src/stream_runtime_acceptance/recovery.rs: 397 lines.
- crates/libs/gateway-session/src/stream_runtime_acceptance/identity.rs: 59 lines.

Remaining host function map (current exact declarations):

| Function | Current body lines | Why retained |
|---|---:|---|
| `gateway_chat_persists_and_reports_session_git_line_counts` | 15 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `codex_skills_defaults_gateway_main_and_child_preload_before_first_request` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `resolve_delegate_request_overrides_without_agent_returns_explicit_values` | 19 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `resolve_delegate_request_overrides_rejects_unknown_agent` | 14 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `chat_stream_rejects_missing_workspace_without_retryable_status` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `native_workflow_gateway_persists_bound_graph_without_llm_artifacts` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `gateway_first_turn_registers_parent_before_thread_tool_use` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `chat_stream_returns_sse_response_for_docs_request` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `agcollab_delivery_reaches_next_model_request` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `video_edit_entry_gateway_unbound_projects_template_edit_menu_and_persistence` | 131 | crate::gateway::api::handlers::workflow::applications_router, crate::global::paths, crate::global_database::GlobalDatabase::for_home, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock, crate::workflow:: |
| `video_edit_entry_gateway_explicit_unavailable_model_does_not_start_graph` | 4 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `collect_video_edit_test_stream` | 27 | crate::session::ui_store::current_tenant_scope |
| `agent_native_defaults_reach_registered_tools_and_prompt` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `agent_native_defaults_do_not_grant_disabled_mcp` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `agent_session_identity_rejects_forged_ui_binding` | 11 | crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |
| `agent_session_identity_survives_profile_edits` | 75 | agent_session::processor::test_llm_stream_guard, agent_session::session::fork, crate::config::get_current, crate::config::update, crate::permission::next::Action::Deny, crate::session::agent_binding::apply_to_config, crate::session::agent_binding::load, crate::session::capability_policy::native_agent_defaults, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_config::set_config_field_result, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `agent_session_identity_derives_backend_and_rejects_conflicts` | 11 | crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |
| `acp_background_approval_gateway_uses_turn_grant_after_agent_resolution` | 46 | crate::session::agent_binding::confirmed_full_access, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `native_tool_search_core_loop_gateway_history_reaches_model_as_native_item` | 32 | crate::session::llm::native_tools::NativeHistoryItem, crate::session::processor::llm_messages::session_message_to_llm_message, crate::session::session::Message, crate::session::session::Role::Tool, crate::session::session::Role::User |
| `agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority` | 18 | crate::config::update, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |
| `gateway_first_response_replays_the_same_session_and_rejects_other_directories` | 12 | crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |
| `gateway_first_response_failure_preserves_history_until_same_session_retries` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `gateway_first_response_missing_replay_does_not_start_a_new_turn` | 6 | crate::test_support::global_async_env_lock |
| `gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing` | 5 | Thin actual Host invocation; or actual video template/router boundary shown above |
| `task_pool_agent_uses_existing_definition_and_defaults` | 50 | crate::agent::permission_rules, crate::config::update, crate::permission::next::Action::Deny, crate::permission::next::evaluate, crate::session::agent_binding::load, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::session::ui_store::current_tenant_scope, crate::test_support::global_async_env_lock |
| `task_pool_agent_rejects_unknown_or_disabled_definition` | 10 | crate::config::update, crate::session::llm::recovery_test_support::RecoveryConfig::new, crate::test_support::global_async_env_lock |

Static checks: removed test-only forwarding remains absent, native adapter registered with cfg(test), acceptance modules registered native-only under test-support, native StreamEvent/queue and tool-id return types inspected, git diff --check clean for owned files. All Cargo/build/unit/device tests and test executable/listing remain NOT RUN. Future precise filters: gateway-session --lib host_migration_11_tests expects51, vw-agent --lib gateway::api::handlers::session::stream::stream_tests expects27 including thin native invocations; verify actual fully-qualified list when authorized. No new independent test registrations claimed for acceptance functions.

## Fifth recovery pass (latest snapshot supersedes the stale owner1750 inventory)

Re-read final.md and remaining-host inventory. Source already contained fourth-pass real native runtime acceptance; the 38000-line gap in the dispatch is an older snapshot. This pass splits the remaining concrete assertions without repeating earlier migration credit.

- resolve_delegate_request_overrides_without_agent_returns_explicit_values and resolve_delegate_request_overrides_rejects_unknown_agent: full unchanged request assertions -> stream_acceptance, executed by original host tests with real Host.
- acp_background_approval_gateway_uses_turn_grant_after_agent_resolution: request mutation/default/explicit permission-mode assertions -> background_approval_options. It returns the actual project/session ids; host still reads confirmed_full_access from the real trusted binding store and asserts the turn grant was not persisted.
- task_pool_agent_uses_existing_definition_and_defaults: agent/model/backend/tool/full-access projection assertions -> task_agent_defaults, consuming actual resolved request and actual stored AgentBinding. Native host still loads that binding and evaluates the actual permission rules; no replaced permission or profile lookup.
- video_edit_entry_gateway_unbound_projects_template_edit_menu_and_persistence: video_template_ledger consumes rows read from the actual production preparation ledger and canonical real source paths, preserving both source/template digest assertions. Native source permissions, actual template edits, HTTP router, transport and database reads stay in host.
- agent_session_identity_survives_profile_edits: resumed_profile_selection contains original same-session rules/model conflict assertions, called after the native fork, trusted permission mutation and agent-disable update. Host still obtains and reapplies the actual stored config binding, resolves native capability defaults and checks revocation. No fork/publication/permission operation simulated.

No new production algorithm or independent test registration claimed. Acceptance helpers still require original native test invocation; domain unit filter remains51, host filter27. Native history SDK conversion assertion remains in the host because its complete assertion matches the actual SDK Message enum, with no added SDK dependency merely for relocation. Remaining native test bodies now consist of actual git initialization, workflow route/edit and transport/database/path permission operations, actual fork/binding/config revocation operations, SDK integration and thin invocations. Every previous full-function mapping is retained; the partial mappings above supersede previous retained-body tables.

Latest physical count:
- stream.rs: 328 lines.
- stream_tests.rs: 472 lines.
- stream_acceptance_host.rs: 27 lines.
- Parent test registration:4 lines. Total task host footprint 831; reduction 2987/3818 (78.2%).
- Domain stream_acceptance.rs: 462 lines.
- Domain stream_acceptance/projections.rs: 46 lines.

Static verification: inspected remaining direct call sites and real return types; original host wrappers still invoke each extracted assertion, public helper registration is local under existing native test-support cfg, git diff --check clean. No Cargo, build, tests, test listing, device run or test binary invoked. Future commands/counts remain as above. The numbered task file was already deleted after prior committed completion and is not recreated for bookkeeping.
