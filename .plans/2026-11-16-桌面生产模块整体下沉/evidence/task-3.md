# 任务3：聊天流事件与终态静态交付

本轮遵守用户禁测要求：未执行任何Cargo命令、单元测试、实机绘制或回归。不声称编译或行为通过。仅源码审查、独立rustfmt和限定git diff --check。

## 生产归属与调用链

宿主 message::chat::stream::update → 宿主跨域适配 → desktop-session::message::chat::stream 各局部入口。
- stream_events 持有缓冲队列、工具块、压缩状态、终态计时/消息结算、请求代际过滤与后续效果决策。
- draft 持有发布校验、幂等插入、同项目空标题更新和标题轮询；宿主负责端点构造、Iced消息、项目列表/预览/宠物投影与缓存发布。
- projection 持有活动/权限保存屏障下的队列消费、取消字段结算、快照选择、摘要投影、网关真实停止。
- permissions/questions/steps/todos 复用已有生产入口，补齐权限保存回执、回复准备和步骤费用合并；没有新依赖、反向App导入或平行算法。
- App继续持有既有 desktop-session::session::agent::SessionRuntimeState<RenderState>，不复制状态，真实取消仍调用gateway stop，迟到请求仍按活动id匹配。

## 来源计数（pathlib.read_text().splitlines，含空行注释）

|来源|基线|交付|基线生产候选|
|---|---:|---:|---|
|delta_scheduler.rs|2|2|True|
|draft.rs|191|177|True|
|mod.rs|647|654|True|
|permission_tests.rs|128|128|False|
|permissions.rs|257|286|True|
|permissions_tests.rs|315|192|False|
|questions.rs|130|146|True|
|questions_tests.rs|167|25|False|
|steps.rs|232|220|True|
|steps_tests.rs|382|281|False|
|stream_events.rs|1431|1369|True|
|stream_events_tests.rs|2221|2140|False|
|tests.rs|458|428|False|
|todos.rs|89|89|True|
|todos_tests.rs|83|83|False|

来源总量 6733 → 6220，宿主净减少 513 行；生产候选 2979 → 2943，净减少 36 行。没有新增宿主替代文件；测试减少单独统计，不冒充生产减少。独立rustfmt会影响空白行，行数不是业务迁移规模。

## mod/draft与状态字段残留审查

- delta_scheduler：仅领域调度器re-export，无宿主算法。
- mod：模块注册/消息分派、gateway保存Task、测试save hook（系统边界）、领域snapshot与queue借用装配、子会话等待/取消后的宠物与UI同步、真实stop效果。领域消费/取消已经改接projection，不借用全App。
- draft：reload_after_response/recover_expired_replay为Task与消息映射；complete_recovered为请求定位、缓存/UI投影并调用终态；publish为项目列表/预览/宠物投影和Task；apply_published_title委托领域转换，仅宠物效果留宿主；publication_fixture只供宿主跨域契约。
- register_stream_session_messages/now_ms/session_directory_for_save/load_session_or_default/start_next：保存效果装配、时钟/目录资源、活动缓存局部借用、请求启动跨域调用。无独立领域算法副本。
- 保存hook与gateway客户端、消息路由、滚动/虚拟列表/预热、Git表单提交、语音与系统通知、MCP原生面板资源仍由宿主拥有。
- 本任务没有新增App字段；流缓冲、压缩、保存、计时、恢复、请求生命周期字段原已属领域SessionRuntimeState，继续复用。

## 主模块断言映射

- tests::desktop_first_response_title_updates_only_untitled_copies_for_its_project → host_module_3_tests同名：手工标题、其它会话、原项目空标题、外项目不污染4项保留；移除宿主无效果Task计数。
- tests其它用例保留路由、项目列表、缓存、真实保存hook和活动视图跨域契约。
- permission_tests保留App会话元数据、缓存、权限屏障与真实队列启动契约。
- 新增host_module_3_tests::cancelled_runtime_clears_request_and_animation覆盖取消字段结算。


# Interactions migration notes
静态检查：独立rustfmt与git diff --check；禁测，未编译。
## 迁移断言（保留原名与断言）

### crates/libs/desktop-session/src/message/chat/stream/permission_reply_tests.rs
- handle_permission_list_loaded_applies_sorted_request_and_clears_on_error
- handle_permission_list_loaded_clears_stale_modal_for_empty_success
- handle_permission_select_request_updates_current_when_found_and_clears_when_missing
- handle_permission_approve_all_always_clears_modal_state
- handle_permission_approve_once_advances_modal_to_next_request
- handle_permission_approve_once_without_selection_preserves_pending_requests
- approve_always_advances_modal_to_remaining_request
- reject_advances_modal_to_remaining_request

### crates/libs/desktop-session/src/message/chat/stream/question_reply_tests.rs
- list_loaded_empty_or_error_clears_modal_state
- submit_and_reject_without_request_are_noops
- submit_with_request_id_but_missing_request_preserves_pending_id
- submit_clears_modal_and_combines_allowed_custom_answers
- submit_with_distinct_custom_multi_answer_clears_modal_after_building_reply
- reject_with_pending_request_clears_modal_state

### crates/libs/desktop-session/src/message/chat/stream/step_cost_tests.rs
- handle_agent_step_cost_loaded_returns_none_for_empty_session_id
- handle_agent_step_cost_loaded_noop_when_step_missing_and_both_fields_none
- handle_agent_step_cost_loaded_creates_step_when_model_provided
- handle_agent_step_cost_loaded_creates_step_when_cost_provided
- handle_agent_step_cost_loaded_updates_existing_step_model_and_cost
- handle_agent_step_cost_loaded_does_not_overwrite_with_none_on_existing_step

## 宿主逐函数残留理由

### permissions
- permission_directory：宿主活动会话、项目导航目录跨域选择
- clear_permission_modal：局部字段借用装配，业务转换调用desktop-session
- current_full_access_session_id：从宿主活动运行时读取权限模式，提供领域自动批准会话范围
- submit_permission_replies：网关异步效果及宿主回复消息映射
- apply_current_permission_request：局部字段借用装配，业务转换调用desktop-session
- apply_permission_request：局部字段借用装配，业务转换调用desktop-session
- handle_permission_poll_tick：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_permission_list_loaded：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_permission_select_request：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_toggle_full_access_permission：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_full_access_saved：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_permission_approve_once：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_permission_approve_always：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_permission_approve_all_always：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_permission_reject：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_permission_reply_submitted：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- submit_permission_reply：局部字段借用装配，业务转换调用desktop-session
- permission_state：局部字段借用装配，业务转换调用desktop-session

### questions
- clear_question_modal：局部字段借用装配，业务转换调用desktop-session
- apply_question_request：局部字段借用装配，业务转换调用desktop-session
- handle_question_poll_tick：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_question_list_loaded：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_question_option_toggled：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_question_custom_changed：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_question_submit：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_question_reject：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_question_reply_submitted：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_question_rejected：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- question_state：局部字段借用装配，业务转换调用desktop-session

### steps
- start_llm_timer_at：局部字段借用装配，业务转换调用desktop-session
- pause_llm_timer_at：局部字段借用装配，业务转换调用desktop-session
- handle_agent_step_start：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_agent_step_finish：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_agent_tool_lifecycle：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_agent_step_cost_loaded：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库

### todos
- panel：局部字段借用装配，业务转换调用desktop-session
- animation_task：Iced动画调度与宿主消息映射
- apply_loaded_session_todos：局部字段借用装配，业务转换调用desktop-session
- todo_list_directory：宿主活动会话、项目导航目录跨域选择
- load_input_panel_todos_task：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_load_input_panel_todos：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_todo_poll_tick：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库
- handle_input_panel_todos_loaded：宿主消息映射、网关Task/计时资源或活动/后台跨域投影装配；局部算法委托领域库

## 宿主测试残留
permissions_tests：目录/运行时选取、自动批准和ACP重建、偏好初始化、网关Task与回执跨域契约。
questions_tests：网关poll及reply/reject结果效果契约；弹窗纯状态6例迁走。
steps_tests：请求定位、活动/后台投影、轨迹刷新任务与运行时计时资源联动；费用纯转换4例迁走。
todos_tests：活动会话与项目目录选择、网关Task调度及缓存异步时机；原领域待办状态转换及测试已在todos库。
permission_tests：会话元数据/缓存选取与队列调度，mod.rs由主代理接管。

## 挂载
permissions.rs cfg(test) mod permission_reply_tests；questions.rs cfg(test) mod question_reply_tests；steps.rs cfg(test) mod step_cost_tests。所有3新增测试文件均正常模块加载，无path/include反向依赖。

# 流事件测试断言迁移记录
新归属：desktop-session/src/stream_event_transition_tests.rs，主任务注册至host_module_3_tests。
未运行Cargo、测试或draw。rustfmt仅修改新文件；git diff --check仅静态。
迁移保留原用例名；tail比较完整解析器改为四组既定输入的显式期望(0,false)/(1,false)/(2,false)/(1,false)，避免领域反向依赖desktop-chat-view。

- `stream_events_tests_module_is_wired`：宿主保留：测试模块挂载契约。
- `compaction_status_started_then_completed_updates_one_message`：整例迁至领域；原断言均保留（App字段改真实runtime/消息容器），删除宿主旧用例。
- `background_stream_mutation_invalidates_cached_session_view`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `running_session_switch_preserves_history_after_background_delta`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `running_session_cross_project_switch_preserves_history_after_background_delta`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `running_session_switch_preserves_history_on_terminal_without_delta`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `generated_git_commit_message_fills_form_then_dispatches_commit_action`：宿主保留：Git表单/工作树/Task消息路由真实跨域契约；未删原断言。
- `invalid_generated_git_commit_message_does_not_dispatch_commit`：宿主保留：Git表单/工作树/Task消息路由真实跨域契约；未删原断言。
- `generated_git_commit_message_rejects_a_switched_worktree`：宿主保留：Git表单/工作树/Task消息路由真实跨域契约；未删原断言。
- `plan_run_session_started_registers_without_leaving_parent`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `plan_run_subtask_session_registers_streams_and_finishes_in_background`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `stream_tail_state_handles_split_think_tags_from_deltas`：整例迁至领域；原断言均保留（App字段改真实runtime/消息容器），删除宿主旧用例。
- `chat_scroll_semantics_stream_growth_keeps_manual_scroll_top`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `stream_delta_defers_session_save_off_update_path`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `active_stream_followup_deltas_are_coalesced_until_flush`：局部队列/终态/去重状态断言迁至领域并从宿主移除；Task.units、UI用量/步骤、toast及空聊天契约保留宿主。
- `active_stream_applies_first_delta_without_flush_timer`：局部队列/终态/去重状态断言迁至领域并从宿主移除；Task.units、UI用量/步骤、toast及空聊天契约保留宿主。
- `buffered_tool_event_starts_on_its_own_line`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `active_stream_terminal_flushes_pending_delta`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `stream_session_delta_save_sends_dirty_entry_only`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_forces_terminal_flush`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_preserves_newer_revision`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_never_loads_full_session`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_retries_after_failure`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stale_stream_session_delta_rebases_before_retry`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `active_stream_delta_updates_tail_without_replacing_chat_vector`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `active_stream_following_tail_preserves_height_storage_and_borrows_runtime`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `active_stream_save_snapshot_preserves_replacements_and_restarts_after_ack`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `tail_message_prewarm_does_not_mark_chunk_state`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `active_stream_growth_preserves_turn_top_focus`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `stream_handlers_reject_unknown_request_without_mutating_chat`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `inactive_stream_updates_cached_snapshot_without_touching_active_chat`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `active_workflow_update_appends_tool_block_and_preserves_message_id`：整例迁至领域；原断言均保留（App字段改真实runtime/消息容器），删除宿主旧用例。
- `stream_done_clears_active_request_and_applies_ids_and_usage`：局部队列/终态/去重状态断言迁至领域并从宿主移除；Task.units、UI用量/步骤、toast及空聊天契约保留宿主。
- `processing_timing_stream_done_closes_active_and_background_cycles`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `inactive_stream_done_updates_cached_ids_and_marks_unseen_success`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `active_stream_error_appends_system_message_and_clears_request_state`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `processing_timing_stream_error_closes_and_saves_background_cycle`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `processing_timing_post_tool_round_closes_old_cycle_and_starts_new_cycle`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `active_stream_error_humanizes_payload_too_large_and_shows_toast`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `missing_request_stream_error_clears_active_requesting_and_shows_toast`：局部队列/终态/去重状态断言迁至领域并从宿主移除；Task.units、UI用量/步骤、toast及空聊天契约保留宿主。
- `post_tool_round_rejects_mismatched_or_inactive_session`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `context_loaded_adds_visible_agents_read_card`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `streamed_tail_invalidates_stale_display_and_copy_caches_before_done`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `compaction_error_and_cancel_settle_active_and_background_status`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `recovery_notice_waiting_deduplicates_and_keeps_request_busy`：局部队列/终态/去重状态断言迁至领域并从宿主移除；Task.units、UI用量/步骤、toast及空聊天契约保留宿主。
- `recovery_notice_recovered_cancelled_and_done_clear_waiting`：宿主保留：toast投影、中文真实draw及取消/迟到请求消息路由契约；未删原断言。另有领域新例验证新请求不被unmatched error结算。
- `recovery_notice_stopped_preserves_actual_attempts_and_action`：宿主保留：toast投影、中文真实draw及取消/迟到请求消息路由契约；未删原断言。另有领域新例验证新请求不被unmatched error结算。
- `recovery_notice_background_and_old_request_do_not_pollute_active_request`：宿主保留：toast投影、中文真实draw及取消/迟到请求消息路由契约；未删原断言。另有领域新例验证新请求不被unmatched error结算。
- `recovery_notice_cancel_terminal_and_old_error_keep_new_request`：宿主保留：toast投影、中文真实draw及取消/迟到请求消息路由契约；未删原断言。另有领域新例验证新请求不被unmatched error结算。
- `recovery_notice_chinese_text_draws_in_light_and_dark_themes`：宿主保留：toast投影、中文真实draw及取消/迟到请求消息路由契约；未删原断言。另有领域新例验证新请求不被unmatched error结算。
- `stream_session_delta_save_keeps_retrying_when_conflict_recovery_advances`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_stops_when_conflict_recovery_makes_no_progress`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_stops_after_three_failures`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_repairs_missing_user_prefix_for_active_and_background`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_uses_absolute_page_sequences`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_rejects_permanent_errors_without_retry`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `stream_session_delta_save_stops_when_missing_prefix_is_unavailable`：宿主保留：真实网关保存效果、缓存前缀、重试调度Task和快照装配契约，使用领域StreamSaveState实现；未删原断言。
- `completed_stream_blocks_are_selectable_before_request_finishes`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `compaction_failure_preserves_prior_statuses_and_persists_completion`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `completion_notification_is_opt_in_skips_history_and_ignores_replayed_done`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `finish`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `todo_continuation_preserves_stream_order_for_active_and_background_sessions`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。
- `background_stream_error_without_cache_does_not_create_error_only_history`：宿主保留：聊天投影与缓存、项目/会话切换、虚拟列表、计时和持久化、消息路由或通知跨域联动；未删原断言。


# stream_events 生产迁移

既有领域入口 `message::chat::stream::stream_events` 扩展，复用 `SessionRuntimeState<R>`，未引入 App trait、反向依赖或平行算法。

- 缓冲状态：queue_runtime_delta/flush_runtime_delta，宿主仅调度计时消息和投影。
- 消息更新：apply_delta/AppliedDelta 整组尾消息、旧长度及块边界；apply_tool_blocks 整批工具块更新。
- 压缩状态：apply_runtime_compaction 完整pending index转换，settle_compaction沿用既有领域算法。
- 终态：begin_terminal 统一截取请求代际、队列状态与双计时器；complete_terminal_messages完成消息耗时；finish_runtime完成请求与未读状态；bind_terminal_turn、complete_draft_boundary处理最终turn与草稿历史边界。
- 工具轮交接：yield_tool_round 按guide边界插入续接请求并取下一项。
- 迟到错误：settle_unmatched_error禁止清除新请求；display_stream_error保持错误提示；model_recovery_matches保持会话和请求代际过滤。
- 计划子会话：start_plan_runtime/plan_initial_chat。
- Git完成：complete_commit_handoff返回Unrelated/Warning/Commit具名结果，包含请求会话匹配、JSON标题摘要校验、工作树校验。
- 完成效果策略：completion_effects返回通知/后续请求/消息刷新标志；reload_terminal_messages承接真实gateway客户端及两次读的既有结果次序。

## 宿主剩余函数逐项说明

- stream_delta_flush_interval: App活动会话身份适配至已有库入口。
- calibrate_stream_tail_at_terminal/apply_active_terminal_tail_calibration: 消息视图split_think/高度与App虚拟列表字段的跨域投影。
- completed_handoff_prompt: 消息视图think剥离到Git完成领域输入。
- complete_quick_message_handoff: 将领域CommitHandoff映射Git面板字段、编辑器与Git路由、toast。
- prepare_scroll_for_active_stream_growth/finish_active_stream_tail_update/refresh_stream_replacement_projection: 虚拟列表、渲染缓存、宠物、正文投影与保存跨域装配。
- append_stream_delta_for_performance_probe: cfg(test)性能探针入口，复用领域函数。
- schedule_agent_stream_session_save/mark_agent_stream_session_dirty/flush_agent_stream_session_save/clear_agent_stream_session_save_flush/save_latest_agent_stream_session/handle_agent_stream_session_delta_saved/force_save_agent_stream_session: 既有StreamSaveState拥有状态算法；宿主仅选择App前后台缓存/目录、记录性能探针、映射保存结果及toast与网关资源效果。
- queue_agent_stream_delta/queue_agent_tool_delta/queue_agent_session_delta/flush_agent_stream_delta/flush_agent_stream_delta_for_request: App请求身份解析和领域缓冲结果到Task调度的适配。
- handle_agent_stream_delta/handle_agent_session_delta: cfg(test)跨域直接投影适配。
- upsert_compaction_status: 宿主旧兼容包装已删除；真实入口使用apply_runtime_compaction。
- settle_interrupted_compaction/handle_agent_todo_continuation/handle_agent_compaction_status: 领域更新映射App前后台消息缓存、虚拟列表、保存与scroll。
- apply_agent_session_delta: 领域AppliedDelta映射正文渲染器、前后台缓存和save装配。
- handle_plan_run_session_started/handle_plan_run_session_finished: 父请求匹配，前后台缓存初始化及宠物状态、保存投影。
- handle_agent_workflow_node_update/handle_agent_tool_blocks_update/handle_agent_context_loaded: 网关事件映射领域工具更新，再映射缓存、渲染器、保存和宠物。
- handle_agent_stream_done: 草稿网关publication reload与终态入口装配。
- complete_agent_stream: 领域终态结果映射前后台缓存、voice、Git、draft publication、notifications、todos、trajectory、gateway刷新及请求启动效果，保持顺序。
- handle_agent_post_tool_round: 领域yield_tool_round结果映射保存、正文渲染和下一请求启动。
- handle_agent_stream_error: 领域错误和终态结果映射toast、前后台缓存、保存、下一请求启动；缺失后台缓存继续保护不覆盖历史。
- clear_model_recovery/handle_agent_model_recovery: recovery事件与App全局toast去重/清除的跨域投影，匹配策略在领域。

静态检查：两个修改文件独立rustfmt与git diff --check，无Cargo/测试/渲染运行。类型与行为尚未验证。


## 验证边界与后续命令（未执行）

- host_module_3_tests 已在领域lib.rs独立cfg(test)注册，stream_event_transition_tests作为其子模块；权限/问题/费用测试在各领域模块cfg(test)注册。
- 后续授权后：python3 scripts/cargo_slot.py cargo test -p desktop-session --lib host_module_3_tests -- --test-threads=1
- 另需覆盖真实过滤 message::chat::stream::permissions、message::chat::stream::questions、message::chat::stream::steps 及既有stream_events_tests，不能只跑主模块冒充全部。
- 宿主后续授权检查：python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib。真实中文/暗黑draw测试源码保留，本轮未执行。
- 未验证风险：泛型局部状态借用与跨模块类型/可见性未由编译器确认；网关、系统通知、物理语音、原生MCP面板没有运行证据。
