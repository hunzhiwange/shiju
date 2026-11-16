# 任务2：聊天发送与会话生命周期静态交付

承接阶段提交 2141b30c0 后继续完成局部状态转换下沉和真实接线。本轮仅静态交付，未编译、未测试、未运行、未绘制；不承诺编译速度或行为验证通过。

## 实现所有权与调用链

- `send_input::prepare_send_input` 使用既有 `SessionRuntimeState<R>`：空输入、内联模式、手动上下文与附件消费、任务选项、模型/来源、委派、工具投影、历史数量及工作流/目标模式；返回 None / InlineMode / AddTask / Send。
- `lifecycle::gate_start` → 项目身份推断及 `resolve_pending_send` → `initialize_draft` → `prepare_request` → `begin_request`：权限保存时排队、历史加载保护、PendingSessionSend 代际隔离、解析失败恢复附件、草稿与分叉标记、StopAndSend 稳定历史边界、覆盖历史/附件、助手占位、权限投影、请求计时、guide handoff、ACP 重建标记。
- `lifecycle::dispatch` 管理 StopDraft / Replace / Guide / Queue / Start，宿主消费 StopDraft 和 Start 效果；仅 Replace 清除旧请求并更新宿主 pet/popover，保持原分支差异。
- `complete_cancel` 与 `next_queued` 完成计时并提供续接请求；宿主在网关 stop、历史存储与渲染完成的原有顺序中调用。
- `load_send_history` / `resolve_history_load` 管理读取身份、更新本地快照前的请求与缓存竞争；`restore_cached_chat` 保持消息 ID 长度不符时重置；`retain_history`、`session_snapshot` 复用既有历史消息类型。
- `preferences` 拥有模型、智能体切换后的局部重置、分叉选择标记、工具选择前处理、plan-run 身份判断及 ACP 偏好载荷。其它配置域仍使用原配置条目和库存入口。
- `selection::update` 调用既有工具领域状态操作，拥有搜索/范围/分组/分桶/工具/手动上下文/重置转换及拒绝反馈。
- `git_handoff` 拥有 QuickMessageGitCommitAction、QuickMessageHandoffState、生成前校验与提示词、请求 ID 隔离和具名 GitHandoffEffect。宿主映射 Waiting 和 ClearCommitInProgress 到跨域 Git 进度字段。
- `session::compaction_gate`、recover/reset/summarize 与 `fork::fork_session` 承接压缩前置条件和网关操作。恢复、重置、分叉均使用宿主提供的认证 GatewayClient，不在来源中另造裸端点客户端。

`desktop-session/lib.rs` 注册上述模块及 `#[cfg(test)] host_module_2_tests`。真实宿主发送、回执、取消、队列、偏好、工具、Git、分叉、重置入口均直接调用这些领域实现，旧算法已移除，未建立并行算法。无新增 Cargo 依赖，无 App/宿主源码反向引用，无 path/include 反载、全局 App 副本或巨型 trait。

## App 状态与构造

请求运行态、草稿、队列继续使用现有领域类型，PendingSessionSend 继续沿用原消息载荷。App 的 quick_message_handoff 经 app_state.rs 重导出改为领域枚举，保留原 pub / pub(crate) 可见性。字段名、Option 初始化和消息泛型参数不变，因此无需改 new.rs、message/mod.rs 或 Cargo 清单。其它并行任务的 App 字段未纳入本提交。

## 宿主保留函数逐项理由

| 函数 | 保留的真实装配边界 |
| --- | --- |
| clear_quick_message_handoff_for_request | 将领域清理效果映射到 Git 域进度 |
| current_session_directory | 从 App 已有跨域会话索引取得目录 |
| base_tools_for_selection | 领域准备后读取智能体配置域库存 |
| dispatch_queue_item | 将领域队列效果接到 stream stop、pet、popover 和 start |
| selection_message | 工具库存注入及通知效果映射 |
| persist_acp_history_preferences | 映射全局默认值并通过桌面配置存储保存领域载荷 |
| persist_session_reasoning_effort | 会话运行态到全局偏好索引及存储的装配 |
| message_id_for_index | 将活动视图 ID 列表传入既有领域算法 |
| recover_reset_message_ids_task | 认证客户端、Iced Task 与宿主回执构造 |
| reset_runtime_history_snapshots | 从 App 会话定位运行态后调用领域历史转换 |
| reset_session_locally | 协调全局活动视图、fork 菜单、缓存、渲染失效与版本化存储；历史截取与请求快照算法已归库 |
| save_session_task | 复用 stream 持久化协调器以保持写入顺序 |
| restore_cached_chat_for_session | 读取 App 冷/热缓存入口后调用领域恢复算法 |
| send_to_existing_session | 暂存/恢复当前活动视图、缓存和 UI 失效，期间执行目标会话发送；历史读取及竞争规则已归库 |
| send_current_session_text_only | 隔离全局附件 UI 与快捷动作发送 |
| session_info_is_compacting / set_session_compacting | 将跨域会话索引迭代器传入既有领域实现 |
| request_session_compaction_confirmation | native rfd 对话框、翻译、菜单收起与回执 |
| compact_session | 映射确认、领域前置结果、认证客户端和 Task |
| finish_session_compaction | 将网关结果映射 toast 并清理跨域索引标记 |
| generate_git_commit_message | 将领域 Send/Waiting 效果接到聊天发送与 Git 进度 |
| active_session_is_plan_run_child | 提供活动身份、运行态与两个会话索引给领域判断 |
| check_agent_installation | gateway-config 客户端及带身份的安装回执 |
| send_current_session_input | 选择安装检查边界或已检查的发送入口 |
| send_current_session_input_checked | 注入工具库存、导航目录和时间，映射领域发送效果到任务看板或队列 |
| update | 宿主消息到领域操作的映射；同时协调智能体配置条目、ACP 来源设置、模型选项目录、全局偏好、popover、项目打开、缓存和持久化 |
| start | 系统 ID/时间、项目解析 Task、活动视图/冷缓存、渲染估高、stream 持久化协调器、pet、滚动焦点与 submit/todo 调度；请求与草稿转换已归库 |

状态字段组：请求/队列/草稿/计时归既有 SessionRuntimeState；快捷提交 handoff 归领域枚举；App 的全局配置镜像与配置服务属于跨域保存；active_session_id/chat/message_ids 是活动视图投影并与渲染缓存协同；navigation/project/session indexes 是项目域输入；files/input_editor 镜像、popover、pet、toast、scroll 属于宿主跨域界面装配。未引入新宿主替代文件。

## 断言迁移清单

按固定基线核对 81 个原测试名：20 个领域用例迁入，61 个宿主跨域契约保留，缺失 0、两边重复名字 0。迁移用例的 assert/assert_eq/assert_ne 数量逐项与基线一致；这是源码审计，不是运行通过数。

| 原测试名 | 当前归属 | 原断言数 | 保留理由/转换 |
| --- | --- | ---: | --- |
| `session_tests_module_is_wired` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 1 | 宿主模块注册契约 |
| `chat_reasoning_effort_is_isolated_by_session` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 8 | 跨域接线：active_session_id, current_session_runtime, get_session_runtime, reasoning_effort, session_reasoning_efforts, session_runtime_states, sync_active_session_preferences |
| `compact_action_requests_confirmation_without_sending_chat_or_creating_session` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：active_session_id, chat, chat_message_ids, get_session_runtime, navigation_tabs, sessions |
| `compact_action_rejects_a_busy_session` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 1 | 跨域接线：active_session_id, active_toast, chat, get_session_runtime_mut |
| `confirmed_compaction_marks_only_the_target_session_in_progress` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：active_session_id, sessions |
| `compaction_success_preserves_session_and_shows_feedback_without_reloading_history` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：active_session_id, active_toast, sessions |
| `compaction_failure_clears_guard_keeps_session_and_shows_error` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：active_session_id, active_toast, sessions |
| `git_commit_quick_action_asks_ai_for_title_and_summary_before_commit` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 6 | 跨域接线：active_session_id, get_session_runtime, git, navigation_tabs, quick_message_handoff, sessions |
| `selected_git_commit_quick_action_requires_a_selection_before_ai_request` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_toast, git, navigation_tabs, quick_message_handoff |
| `apply_inline_mode_command_toggles_task_mode_and_clears_runtime_editor` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 3 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `session_agent_selection_normalizes_value_and_resets_tool_context` | `crates/libs/desktop-session/src/host_module_2_tests/selection_tests.rs` | 5 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `session_agent_selection_switches_selectable_tools_to_selected_agent_range` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2 | 跨域接线：agents_settings, current_session_runtime, session_tool_inventory |
| `open_session_tool_selector_tab_selects_and_toggles_popover` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 10 | 跨域接线：current_session_runtime, popover_visibility |
| `tool_selector_messages_update_query_scope_group_and_manual_context` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 6 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `gateway_endpoint_defaults_empty_host_to_loopback` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 3 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `current_session_directory_uses_active_session_directory` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 2 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `restore_cached_chat_for_session_uses_matching_cached_message_ids` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 2 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `restore_cached_chat_for_session_resets_ids_when_cached_ids_have_wrong_len` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 4 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `queue_messages_reorder_remove_and_ignore_out_of_bounds_indices` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 1 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `queue_copy_keeps_items_and_ignores_out_of_bounds_or_empty_queries` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 1 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `queue_edit_restores_only_text_and_removes_the_selected_item` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：active_session_id, current_session_runtime, current_session_runtime_mut, files |
| `queue_guide_removes_item_and_dispatches_with_stop_and_send` | `crates/libs/desktop-session/src/host_module_2_tests/selection_tests.rs` | 5 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `dispatch_queue_item_appends_queue_requests_and_prepends_guidance` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 2 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `submit_tick_resets_when_idle_and_cycles_when_requesting` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 2 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `recovered_message_ids_ignore_stale_session_and_store_exact_match` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, cached_chat_message_ids, chat, chat_message_ids |
| `reset_session_failure_reports_operation_specific_notification` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 1 | 跨域接线：notifications |
| `fork_desktop_missing_second_user_id_cannot_borrow_an_earlier_id` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：active_session_id, active_toast, chat, chat_fork_dialog, chat_message_ids, sessions |
| `fork_desktop_ignores_confirmation_after_switching_sessions` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2 | 跨域接线：active_session_id, chat, chat_message_ids, session_view_cache, sessions |
| `reset_session_locally_marks_boundary_and_next_send_uses_trimmed_history` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 6 | 跨域接线：active_session_id, active_session_view_state, chat, chat_message_ids, current_session_runtime_mut, get_session_runtime, navigation_tabs, session_history_reset_boundary, sessions |
| `preference_messages_update_runtime_and_global_chat_defaults` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 19 | 跨域接线：acp_agent, acp_recent_count, agents_settings, auto_model, current_session_runtime, current_session_runtime_mut, model, popover_visibility |
| `agent_unified_selection_locks_chat_queue_and_active_request` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：acp_agent, active_session_id, active_toast, chat, current_session_runtime, current_session_runtime_mut |
| `existing_session_model_selection_preserves_pending_history_restore` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2 | 跨域接线：active_session_id, get_session_runtime, prepare_historical_session_model, restore_session_model_from_history |
| `historical_session_keeps_its_model_when_switches_are_requested` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, chat, current_session_runtime, current_session_runtime_mut |
| `cancel_pressed_ignores_idle_sessions_and_dispatches_next_queued_item_when_busy` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 9 | 跨域接线：active_session_id, chat, current_session_runtime, get_session_runtime, get_session_runtime_mut, popover_visibility, sessions |
| `cancel_pressed_schedules_save_and_gateway_stop_without_blocking_update` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2 | 跨域接线：active_session_id, get_session_runtime, get_session_runtime_mut, sessions |
| `processing_timing_start_uses_runtime_request_start_and_cancel_creates_independent_next_cycle` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：active_session_id, get_session_runtime, get_session_runtime_mut, sessions |
| `send_starts_turn_focused_on_new_user_message` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 8 | 跨域接线：active_session_id, chat, chat_auto_scroll, chat_virtualizer, sessions |
| `start_new_session_appends_user_message_without_a_gateway_snapshot` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 8 | 跨域接线：active_session_id, chat, current_session_is_requesting, current_session_runtime, navigation_tabs, project_id |
| `start_without_project_reports_error_before_creating_session` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：active_session_id, chat, error_message, sessions |
| `send_current_session_input_task_mode_clears_files_and_leaves_chat_unstarted` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：chat, chat_message_ids, current_session_runtime, current_session_runtime_mut, files |
| `send_current_session_input_starts_existing_session_with_selected_context_and_attachments` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 23 | 跨域接线：acp_recent_count, active_session_id, agents_settings, chat, chat_send_behavior, files, get_session_runtime, get_session_runtime_mut, sessions |
| `expert_package_selection_is_consumed_once` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 3 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `expert_package_selection_does_not_leak_between_sessions` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 2 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `send_current_session_input_disables_workflow_mode_for_plan_run_child_session` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 4 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `send_current_session_input_keeps_workflow_mode_for_root_session` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 1 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `send_current_session_input_keeps_workflow_mode_for_any_query_when_toggle_on` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 2 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `send_current_session_input_keeps_explicit_actions_in_workflow_mode` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 1 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `send_current_session_input_keeps_goal_mode_for_root_session` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 2 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `request_preserves_registered_agent_tool_without_desktop_prompt` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, agents_settings, chat_send_behavior, get_session_runtime, get_session_runtime_mut, sessions |
| `request_does_not_add_unregistered_agent_tool` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, agents_settings, chat_send_behavior, get_session_runtime, get_session_runtime_mut, sessions |
| `active_session_is_plan_run_child_uses_persisted_source` | `crates/libs/desktop-session/src/host_module_2_tests.rs` | 3 | 真实局部运行态/领域入口；保留原名和全部断言，去除 App 夹具 |
| `recovered_reset_mismatch_falls_back_to_local_history` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, chat, chat_message_ids, session_history_reset_boundary, sessions |
| `reset_session_finished_success_updates_session_indexes_and_cached_chat` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 7 | 跨域接线：active_session_id, cached_chat_message_ids, chat, chat_message_ids, chat_reset_menu_idx, project_sessions, sessions |
| `start_registers_user_and_assistant_for_delta_save` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, chat, get_session_runtime, sessions |
| `start_preserves_absolute_sequences_for_paged_history` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 1 | 跨域接线：active_session_id, chat, get_session_runtime, get_session_runtime_mut, sessions |
| `new_session_project_resolution_continues_first_send` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 7 | 跨域接线：agent_stream_id, chat, current_session_is_requesting, error_message, navigation_tabs, project_id, sessions |
| `new_session_project_resolution_ignores_stale_responses` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, agent_stream_id, chat, navigation_tabs, project_id, sessions |
| `new_session_project_resolution_failure_preserves_attachments` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2 | 跨域接线：agent_stream_id, files, navigation_tabs, sessions |
| `agent_unified_selection_dispatches_selected_runtime` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：agents_settings, current_session_runtime |
| `agent_unified_selection_rejects_disabled_and_started_sessions` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2 | 跨域接线：agents_settings, chat, current_session_runtime |
| `agent_unified_selection_uninstalled_cli_preserves_draft` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：acp_settings, active_session_id, chat, current_session_runtime, current_session_runtime_mut |
| `agent_unified_selection_ignores_install_result_from_another_session` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：acp_settings, active_session_id, chat, current_session_runtime, current_session_runtime_mut |
| `acp_model_source_ui_selection_preserves_native_and_provider_request_identity` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 8 | 跨域接线：current_session_runtime, current_session_runtime_mut, navigation_tabs, project_id |
| `acp_model_source_ui_cannot_change_during_an_active_request` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 1 | 跨域接线：current_session_runtime, current_session_runtime_mut |
| `agent_config_cleanup_full_access_initializes_new_session_only` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 7 | 跨域接线：active_session_id, agents_settings, current_session_full_access, current_session_runtime, current_session_runtime_mut, sessions |
| `full_access_toggle_is_remembered_for_new_llm_sessions` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：active_session_id, agents_settings, current_session_full_access, current_session_runtime, navigation_tabs |
| `session_entry_recommendations_select_original_index` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：agents_settings, current_session_runtime, input_popovers |
| `desktop_first_response_keeps_unanswered_session_out_of_history` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, chat, navigation_tabs, project_id, project_sessions, sessions |
| `desktop_first_response_failure_and_cancel_retry_on_the_same_id_until_success` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 25 | 跨域接线：active_session_id, agent_stream_id, chat, current_session_is_requesting, current_session_runtime, navigation_tabs, project_id, session_previews, sessions |
| `session_history_survives_failed_publication_lookup_and_multiple_followups` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：chat, current_session_runtime, navigation_tabs, project_id |
| `session_history_missing_background_cache_does_not_start_an_empty_continuation` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：active_session_id, cached_chat_messages, chat, get_session_runtime, navigation_tabs, sessions |
| `session_history_loaded_before_background_send_preserves_prior_turns` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：active_session_id, cached_chat_messages, chat, get_session_runtime, navigation_tabs, sessions |
| `session_history_load_failure_does_not_send_or_replace_active_history` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, cached_chat_messages, chat, get_session_runtime |
| `session_history_pending_foreground_load_blocks_send_without_clearing_input` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, active_session_view_state, chat, current_session_runtime, current_session_runtime_mut |
| `queue_guide_preserves_draft_history_across_repeated_handoffs` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 10 | 跨域接线：active_session_id, chat, chat_message_ids, current_session_runtime, navigation_tabs, project_id |
| `fork_selection_ui_switches_engines_preserving_history_and_input` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 8 | 跨域接线：active_session_id, agents_settings, chat, current_session_runtime, current_session_runtime_mut, files, navigation_tabs |
| `fork_selection_ui_first_send_uses_latest_agent_source_and_history` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 9 | 跨域接线：active_session_id, chat, current_session_has_started, current_session_runtime, current_session_runtime_mut, navigation_tabs, sessions |
| `fork_selection_ui_source_save_pending_blocks_send_and_started_changes` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 5 | 跨域接线：active_session_id, current_session_runtime, current_session_runtime_mut |
| `fork_selection_ui_discards_previous_agent_source_result` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2 | 跨域接线：active_session_id, current_session_runtime, current_session_runtime_mut |
| `fork_selection_ui_ignores_stale_source_after_returning_to_same_agent` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 3 | 跨域接线：active_session_id, current_session_runtime, current_session_runtime_mut |
| `fork_selection_ui_closed_modal_waits_for_owned_options_read` | `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 4 | 跨域接线：active_session_id, current_session_runtime, current_session_runtime_mut |

原 test 模块的 unused_must_use 范围随领域用例保留；领域夹具只有真实 SessionRuntimeState、既有消息及临时 Vec，不复制 App。删除来源中已无调用者的 persisted_user_message 夹具；保留仍用于跨域契约的 chat_message/session_info/queue_item/agent_request/confirm_pending_model_source。

与前轮 host_split 逐断言核对：host_split_6 的五项断言验证 buffered tool 换行、workflow tool 尾消息索引/数量/文本/ID，host_split_7 验证压缩状态的 system 文本/ID/pending-index，均不覆盖本次输入准备、队列顺序、智能体上下文消费或模式投影。未复制或删除它们。host_module_1 保留活动视图元数据的原契约，本次不重做其算法。

## 基线统计（pathlib splitlines，含注释空行）

| 来源 | 基线 | 当前 | 宿主净减少 | 分类 |
| --- | ---: | ---: | ---: | --- |
| `crates/vw-desktop/src/app/message/chat/session.rs` | 1896 | 1601 | 295 | 生产候选 |
| `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2431 | 1956 | 475 | 测试 |

无新增宿主替代文件；App 公共类型重导出还减少了宿主枚举定义（单列，不计进上述两个独占来源的净减量）。不将测试减少计作生产减少，不卡配额，不承诺编译提速。

## 当前静态验证及限制

- 已检查本任务变更的 `git diff --check`，无输出；全工作区检查曾发现其它并行任务 send_controls_tests.rs EOF 空行，未修改该文件。
- 独立 rustfmt 解析本任务新增/修改文件；源码核对生产 mod、cfg(test)、所有调用者、认证客户端和无反向依赖。
- 固定基线用例与当前归属集合及迁移断言数量核对如上，未运行测试。
- 按用户要求，未执行任何 Cargo 命令、单元测试、实机、绘制或统一回归。泛型/借用/跨 crate 类型及实际运行仍须以后授权验证；本记录不表示编译或行为通过。

以后另行授权：`python3 scripts/cargo_slot.py cargo test -p desktop-session --lib host_module_2_tests -- --test-threads=1`（20 个源码用例，不能将此数量写成通过数）；宿主 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。不得本轮自动运行。
