# 任务20 静态迁移证据

代码迁移与接线已完成；全部编译、行为、Cargo、实机检查未运行，遵守本轮用户约束。减量不足70%，须由收口任务审核下列保留理由及继续可迁项，不能据此宣称整轮达标。

## 调用与依赖

生产算法此前已归属 llm-runtime/acp 与 session-engine/acp_defaults，本轮不计作生产迁移收益。删除无生产调用的 content 包装、宿主重复错误构造器；测试改为调用实际领域实现。vw-agent → llm-runtime/session-engine，无反向依赖、无 path/include 宿主源码。领域通过 acp/mod.rs 自注册测试族；controls 子模块注册私有协议辅助函数测试，未扩大生产可见性。现有依赖足够，无 Cargo 清单变更。

CacheHost 仅固定默认租户、观察 RPC 与日志预览；未使用的 IO 端口显式 panic。它不复制请求、权限或缓存算法，客户端及技能挂载使用真实领域实现。桌面真实授权与 task-local 租户隔离仍由宿主测试验证。

## 文件与行数（splitlines）

| 来源 | 原行数 | 保留行数 | 职责/去向 |

|---|---:|---:|---|

| request.rs | 59 | 57 | RuntimeHost 全部方法保留真实资源、权限、配置、生命周期与用量桥接；do_stream_request_acp 实际调用领域请求。 |

| session.rs | 37 | 31 | 仅保留领域 re-export、其他宿主测试使用的 RuntimeHost 包装与桌面权限边界。 |

| request_tests.rs | 556 | 520 | 保留真实项目上下文、配置更新、模型凭证、未认证驱动阻断与技能/MCP 默认值接线。继续可迁：按现有 Host 端口拆请求前置校验与默认值的领域断言。 |

| fork_replay_tests.rs | 17 | 17 | 验证真实项目上下文存在时空 session 仍不加载持久化；宿主早退契约。 |

| updates.rs | 6 | 6 | 保留宿主实际使用的事件解析重导出。 |

| config.rs | 32 | 32 | DefaultsHost 的 skills/mcp/permission_ceiling/preferences 绑定真实配置、租户与权限；apply_agent_defaults 维持现有端口调用。 |

| source_selection.rs | 12 | 12 | SelectionHost 绑定真实模型解析和 provider 查询，select 调用领域入口。 |

| session_tests.rs | 481 | 46 | 仅保留真实 ToolUseContext 的会话/工作目录/权限授权绑定。 |

| controls.rs | 34 | 26 | host/annotate_model_source/read/select 注入真实租户和存储，调用领域控制入口。 |

| mod.rs | 98 | 56 | 模块装配、ParsedAcpOptions 绑定真实 MountedSkills，以及邻接宿主测试所需缓存类型引用。 |

| content_tests.rs | 84 | 0 | 整组迁往 llm-runtime/src/acp/host_migration_20_tests；旧文件与挂载已删除。 |

| fork_replay.rs | 25 | 25 | ForkReplay::load 校验真实项目与 fork_execution，再打开真实数据库；into_runtime 为请求端口转换。Deref/DerefMut 保持既有薄入口。 |

| tests.rs | 151 | 0 | 整组迁往 llm-runtime/src/acp/host_migration_20_tests；旧文件与挂载已删除。 |

| config_tests.rs | 85 | 85 | 完整 full_access/deny/选择性 deny 场景验证真实配置更新与 native_permission_ceiling，保留宿主集成。继续可迁：把纯配置合并断言拆到 session-engine。 |

| lifecycle.rs | 20 | 20 | 当前租户、全局数据库注入；session_identity/try_lock/Store::open/locked_model 为宿主调用接口。 |

| approval.rs | 96 | 96 | PromptApproval::bind 提取真实 ToolUseContext、审批管理器及消息身份；PendingApproval::drop 清理真实待审批请求，保留宿主端口。 |

| controls_tests.rs | 291 | 51 | 仅保留真实 task-local 租户对客户端缓存、busy lock 的隔离接线。 |

| content.rs | 10 | 0 | 整组迁往 llm-runtime/src/acp/host_migration_20_tests；旧文件与挂载已删除。 |

| replay.rs | 2 | 2 | 保留相邻宿主调用实际使用的领域重导出。 |

宿主合计 2096 → 1082，净减 1014（48.4%）。按文件粗分测试与生产/装配如下；生产列包含转发与文档，不代表算法收益。

生产/装配：431 → 363。

测试：1665 → 719。

新增领域测试/夹具：1017 行。新增生产算法 0 行；领域注册 7 行。

## 全用例映射（原名称不变）

| 原文件/函数 | 当前完整路径 |

|---|---|

| request_tests.rs::acp_local_blank_anchor_resumes_existing_session | vw_agent::session::llm::acp::request::request_tests::acp_local_blank_anchor_resumes_existing_session |

| request_tests.rs::stream_request_reports_abort_before_config_lookup | vw_agent::session::llm::acp::request::request_tests::stream_request_reports_abort_before_config_lookup |

| request_tests.rs::agent_model_inheritance_rejects_missing_credentials_before_launch | vw_agent::session::llm::acp::request::request_tests::agent_model_inheritance_rejects_missing_credentials_before_launch |

| request_tests.rs::acp_model_source_routing_local_reaches_command_lookup_without_client_credentials | vw_agent::session::llm::acp::request::request_tests::acp_model_source_routing_local_reaches_command_lookup_without_client_credentials |

| request_tests.rs::acp_model_source_launch_profile_production_request_rejects_unverified_driver | vw_agent::session::llm::acp::request::request_tests::acp_model_source_launch_profile_production_request_rejects_unverified_driver |

| request_tests.rs::agent_acp_defaults_reject_unsupported_mode | llm_runtime::acp::host_migration_20_tests::request::agent_acp_defaults_reject_unsupported_mode |

| request_tests.rs::acp_skill_mount_outbound_defaults_preserve_rules_without_body | vw_agent::session::llm::acp::request::request_tests::acp_skill_mount_outbound_defaults_preserve_rules_without_body |

| fork_replay_tests.rs::fork_acp_empty_session_preserves_ephemeral_requests | vw_agent::session::llm::acp::fork_replay::tests::fork_acp_empty_session_preserves_ephemeral_requests |

| session_tests.rs::should_abort_reflects_watch_receiver_state | llm_runtime::acp::host_migration_20_tests::session::should_abort_reflects_watch_receiver_state |

| session_tests.rs::acp_session_name_trims_and_namespaces_non_empty_ids | llm_runtime::acp::host_migration_20_tests::session::acp_session_name_trims_and_namespaces_non_empty_ids |

| session_tests.rs::cache_key_is_stable_and_includes_behavioral_inputs | llm_runtime::acp::host_migration_20_tests::session::cache_key_is_stable_and_includes_behavioral_inputs |

| session_tests.rs::cache_key_is_stable_across_environment_insertion_order | llm_runtime::acp::host_migration_20_tests::session::cache_key_is_stable_across_environment_insertion_order |

| session_tests.rs::cache_key_changes_when_session_behavior_changes | llm_runtime::acp::host_migration_20_tests::session::cache_key_changes_when_session_behavior_changes |

| session_tests.rs::acp_client_cache_key_separates_full_access_policy | llm_runtime::acp::host_migration_20_tests::session::acp_client_cache_key_separates_full_access_policy |

| session_tests.rs::cached_client_is_reused_only_for_matching_keys | llm_runtime::acp::host_migration_20_tests::session::cached_client_is_reused_only_for_matching_keys |

| session_tests.rs::close_cached_client_removes_matching_unshared_entry | llm_runtime::acp::host_migration_20_tests::session::close_cached_client_removes_matching_unshared_entry |

| session_tests.rs::close_cached_client_keeps_entry_while_another_request_shares_it | llm_runtime::acp::host_migration_20_tests::session::close_cached_client_keeps_entry_while_another_request_shares_it |

| session_tests.rs::close_cached_client_keeps_replacement_for_stale_entry | llm_runtime::acp::host_migration_20_tests::session::close_cached_client_keeps_replacement_for_stale_entry |

| session_tests.rs::acp_model_source_launch_profile_cache_never_contains_environment_secrets | llm_runtime::acp::host_migration_20_tests::session::acp_model_source_launch_profile_cache_never_contains_environment_secrets |

| session_tests.rs::agent_acp_defaults_reach_new_session_and_prompt | llm_runtime::acp::host_migration_20_tests::session::agent_acp_defaults_reach_new_session_and_prompt |

| session_tests.rs::acp_model_source_session_controls_managed_process_retires_with_readers | llm_runtime::acp::host_migration_20_tests::session::acp_model_source_session_controls_managed_process_retires_with_readers |

| session_tests.rs::acp_skill_mount_outbound_cache_tracks_content_and_client_retains_view | llm_runtime::acp::host_migration_20_tests::session::acp_skill_mount_outbound_cache_tracks_content_and_client_retains_view |

| session_tests.rs::acp_desktop_approval_policy_binds_current_session_and_isolates_cached_grants | vw_agent::session::llm::acp::session::session_tests::acp_desktop_approval_policy_binds_current_session_and_isolates_cached_grants |

| session_tests.rs::acp_onboarding_deepagents_cache_key_preserves_tool_set_identity | llm_runtime::acp::host_migration_20_tests::session::acp_onboarding_deepagents_cache_key_preserves_tool_set_identity |

| content_tests.rs::image_only_current_message_does_not_reuse_previous_text | llm_runtime::acp::host_migration_20_tests::content::image_only_current_message_does_not_reuse_previous_text |

| content_tests.rs::text_markers_and_multiple_image_parts_are_preserved | llm_runtime::acp::host_migration_20_tests::content::text_markers_and_multiple_image_parts_are_preserved |

| content_tests.rs::replay_strategies_select_historical_images | llm_runtime::acp::host_migration_20_tests::content::replay_strategies_select_historical_images |

| content_tests.rs::invalid_images_fail_explicitly | llm_runtime::acp::host_migration_20_tests::content::invalid_images_fail_explicitly |

| content_tests.rs::fork_acp_full_content_preserves_images_roles_and_tools | llm_runtime::acp::host_migration_20_tests::content::fork_acp_full_content_preserves_images_roles_and_tools |

| tests.rs::to_api_error_wraps_display_text_with_acp_prefix | llm_runtime::acp::host_migration_20_tests::contracts::to_api_error_wraps_display_text_with_acp_prefix |

| tests.rs::acp_session_changed_message_detection_is_case_insensitive_and_requires_exact_marker | llm_runtime::acp::host_migration_20_tests::contracts::acp_session_changed_message_detection_is_case_insensitive_and_requires_exact_marker |

| tests.rs::acp_option_error_formats_as_api_unknown_error | llm_runtime::acp::host_migration_20_tests::contracts::acp_option_error_formats_as_api_unknown_error |

| tests.rs::is_acp_session_changed_error_checks_only_known_api_error_variants | llm_runtime::acp::host_migration_20_tests::contracts::is_acp_session_changed_error_checks_only_known_api_error_variants |

| tests.rs::parsed_acp_options_default_keeps_every_optional_field_empty | llm_runtime::acp::host_migration_20_tests::contracts::parsed_acp_options_default_keeps_every_optional_field_empty |

| tests.rs::acp_replay_strategy_values_are_distinct | llm_runtime::acp::host_migration_20_tests::contracts::acp_replay_strategy_values_are_distinct |

| tests.rs::cached_acp_client_stores_client_lock_and_forwards_output | llm_runtime::acp::host_migration_20_tests::contracts::cached_acp_client_stores_client_lock_and_forwards_output |

| tests.rs::cached_acp_client_entry_keeps_key_and_shared_client | llm_runtime::acp::host_migration_20_tests::contracts::cached_acp_client_entry_keeps_key_and_shared_client |

| config_tests.rs::agent_config_cleanup_full_access_acp_preserves_false_and_deny | vw_agent::session::llm::acp::config::config_tests::agent_config_cleanup_full_access_acp_preserves_false_and_deny |

| controls_tests.rs::acp_model_source_defaults_follow_verified_cli_capabilities | llm_runtime::acp::controls::host_migration_20_tests::acp_model_source_defaults_follow_verified_cli_capabilities |

| controls_tests.rs::acp_session_options_follow_capabilities_protocol_and_persistence | llm_runtime::acp::controls::host_migration_20_tests::acp_session_options_follow_capabilities_protocol_and_persistence |

| controls_tests.rs::acp_session_options_follow_capabilities_config_categories_replace_legacy_selectors | llm_runtime::acp::controls::host_migration_20_tests::acp_session_options_follow_capabilities_config_categories_replace_legacy_selectors |

| controls_tests.rs::acp_session_options_follow_capabilities_isolate_tenant_connections | vw_agent::session::llm::acp::controls::tests::acp_session_options_follow_capabilities_isolate_tenant_connections |

| controls_tests.rs::acp_model_source_session_controls_managed_models_are_not_native_choices | llm_runtime::acp::controls::host_migration_20_tests::acp_model_source_session_controls_managed_models_are_not_native_choices |

| controls_tests.rs::acp_model_source_session_controls_model_config_is_read_only_and_mode_is_private | llm_runtime::acp::controls::host_migration_20_tests::acp_model_source_session_controls_model_config_is_read_only_and_mode_is_private |

| controls_tests.rs::acp_model_source_session_controls_busy_guard_is_session_scoped | vw_agent::session::llm::acp::controls::tests::acp_model_source_session_controls_busy_guard_is_session_scoped |

| controls_tests.rs::acp_model_source_session_controls_disconnected_keeps_provider_identity | llm_runtime::acp::controls::host_migration_20_tests::acp_model_source_session_controls_disconnected_keeps_provider_identity |

| controls_tests.rs::acp_public_permission_options_only_expose_host_modes | llm_runtime::acp::controls::host_migration_20_tests::acp_public_permission_options_only_expose_host_modes |

## 未来验证

Unix 预期迁移测试 36 个（其中协议进程用例含 cfg(unix)，其它平台相应减少）；所有场景断言保留，未声称运行通过。

`python3 scripts/cargo_slot.py cargo test -p llm-runtime --lib host_migration_20_tests -- --test-threads=1`

宿主薄用例按上表各完整模块/函数名使用 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <模块::函数> -- --exact --test-threads=1`，每条预期1；Rust crate 展示名由未来真实 list 核对（不能把零测试算通过）。宿主原 content_tests/tests 和迁出的 session/controls/request 函数须未来用真实 list 确认消失。

静态验证仅源码、注册及 git diff --check；未运行测试清单或任何编译，不能给出构建耗时结论。
