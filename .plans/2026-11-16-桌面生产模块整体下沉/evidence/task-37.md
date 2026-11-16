# 任务37：聊天其它消息与会话桥接

静态交付；未编译、未测试。遵照计划不执行任何 Cargo、测试或实机操作。

## 生产调用链

- App chat::update → ChatMessage::route（desktop-session/message/chat/routing.rs）→ 六个宿主效果适配入口。完整穷举从宿主移出，无 fallback；领域枚举新增分支时必须补齐路由。
- agent_stream → desktop-session::agent_stream → StreamMessage → From<StreamMessage> for ChatMessage → Message::Chat。全部19类流事件字段保持原样，宿主只选择配置端点与包装外层消息。
- ACP refresh → SessionRuntimeState::begin_acp_source_refresh → 网关任务；能力回包 → apply_acp_source_loaded；持久化确认 → confirm_acp_source_saved → 宿主同步偏好及 toast。保留 pending、request_id、请求中隔离、未启动 fork 不读继承绑定的规则。
- fork 锚点两入口 → 已有 orchestration::fork_session_anchor / fork_session_anchor_is_current；加载结果泛型使用 session::PreparedSessionOpen，不引入 App 到领域库。
- context::update → desktop-session::context_input_text → desktop-chat-view::chat::append_line；宿主仅投影当前 runtime 到空会话兼容编辑器，或衔接预览编辑器 Copy/系统 clipboard。
- trajectory::update → desktop-chat-view::trajectory::update；raw_state → desktop-session::trajectory::RawLoadState；宿主执行网关 Task、定时消息、活动投影插入与详情失效的跨域效果。
- processing_timing.rs 仅借用 SessionRuntimeState 两个领域计时器；旧空测试挂载删除，原整组计时算法测试已在 desktop-session/message/chat/processing_timing_tests.rs，未删断言。
- session.rs 为模块注册及 session/chat-view 类型出口；无新增本地算法。agent_stream 配置、session gateway 的 tenant/project 端点绑定均为系统边界。

## 残留函数与字段组

- chat 根：now_ms 提供系统时钟；arm_autoscroll_hold/throttled_stream_autoscroll_task 调用领域时钟规则并包装 widget Task；focus_chat_on_turn、两种 scroll_to_bottom、scroll_to_turn_top 联结 virtualizer 与兼容视图字段；measure_visible_chat_messages_task 为 Iced operate；refresh_active_trajectory_raw_task 为面板模式与加载任务桥接；两 fork 函数只装配当前目录/代际；update 联结各领域效果、预热可见编辑器和输入兼容投影。
- trajectory：update 映射宿主泛型视图消息；refresh_trajectory_task/raw_load_task 为网关异步任务；apply_snapshot_result/apply_raw_result 将领域结果插入活动缓存并失效视图；refresh_missing_raw_task/retry_raw_load_task 只装配目录、当前会话和 Task；sync_controls_session/sync_details_values 调 chat-view；raw_state 是精确字段借用，无 App 传入领域算法。
- ACP：next_source_request_id 兼容转发；update 绑定活动会话、项目目录、网关任务和通知；select_source 绑定当前项目并提交领域 begin_source 的持久化效果。确认与刷新策略在领域运行时，不扩展权限。
- 状态归属沿用已下沉的 SessionRuntimeState（ACP、输入、请求生命周期）、RawLoadState（精确借用 raw 元数据和重试表）、chat-view trajectory controls/details、fork 锚点与 SessionViewCache 代际。宿主仅持有并联结不同领域实例，不新增 App 克隆或巨型 trait。

## 逐文件 splitlines

| 文件 | 固定基线 | 交付 | 去向/保留理由 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/impls/agent_stream.rs` | 35 | 23 | 领域流事件转换；宿主端点/外层消息包装 |
| `crates/vw-desktop/src/app/message/chat/acp_options.rs` | 231 | 228 | 领域 ACP 运行时；宿主异步效果 |
| `crates/vw-desktop/src/app/message/chat/context.rs` | 83 | 83 | 已有领域文本规则与 chat-view 编辑器；预览/clipboard 桥接 |
| `crates/vw-desktop/src/app/message/chat/context_tests.rs` | 242 | 242 | 兼容编辑器隔离与预览 Copy Task 契约 |
| `crates/vw-desktop/src/app/message/chat/mod.rs` | 569 | 318 | 领域穷举路由；宿主跨域效果 |
| `crates/vw-desktop/src/app/message/chat/processing_timing.rs` | 4 | 4 | 领域计时器字段借用 |
| `crates/vw-desktop/src/app/message/chat/processing_timing_tests.rs` | 1 | 0 | 删除空占位；有效整组测试已在领域 |
| `crates/vw-desktop/src/app/message/chat/tests.rs` | 287 | 287 | widget Task、流缓存保存与延迟代际契约 |
| `crates/vw-desktop/src/app/message/chat/trajectory.rs` | 209 | 208 | 领域 raw/视图更新；宿主异步及投影失效 |
| `crates/vw-desktop/src/app/message/chat/trajectory_tests.rs` | 734 | 721 | 空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约 |
| `crates/vw-desktop/src/app/session.rs` | 54 | 49 | 父模块注册和类型导出 |
| `crates/vw-desktop/src/app/session_gateway_tests.rs` | 69 | 69 | 配置端点、活动 tenant/project 装配契约 |

基线 2518 行，交付 2232 行；按固定基线净减 286 行。生产候选和测试不能按改名互相抵扣；新领域 routing/流转换/ACP 方法计入领域，仓库总行数不作为验收指标。

## 原断言逐用例映射

除下列明确迁移外，原用例及其中全部断言原位置保留；按用例列出断言宏数量以便核查，未以同名推断去重。


### crates/vw-desktop/src/app/message/chat/context_tests.rs

- `context_tests_module_is_wired`：1 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `append_text_adds_text_to_current_runtime_editor`：2 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `append_text_ignores_empty_text`：3 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `append_text_updates_active_session_without_overwriting_legacy_editor`：2 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `append_text_to_empty_active_session_avoids_leading_newline_and_preserves_legacy_editor`：3 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `insert_position_uses_project_relative_path_and_closes_context_menu`：4 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `insert_position_normalizes_path_and_isolates_active_session`：3 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `insert_selection_positions_uses_context_target_when_present`：4 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `insert_selection_positions_updates_active_session_only`：2 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `insert_selection_positions_without_target_only_closes_menu`：3 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `insert_selected_without_active_preview_only_closes_menu`：2 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `insert_selected_without_matching_tab_only_closes_menu`：2 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `insert_selected_with_matching_tab_closes_menu_and_schedules_copy_chain`：2 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `explicit_noop_context_messages_preserve_state`：4 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。
- `unrelated_message_hits_fallback_without_changing_context_state`：3 个断言宏 → 原路径保留：兼容编辑器隔离与预览 Copy Task 契约。

### crates/vw-desktop/src/app/message/chat/processing_timing_tests.rs


### crates/vw-desktop/src/app/message/chat/tests.rs

- `tests_module_is_wired`：1 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `throttled_stream_autoscroll_respects_follow_mode_and_window`：3 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `scroll_chat_to_bottom_task_snaps_virtualizer_and_syncs_compat_fields`：5 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `scroll_chat_to_bottom_with_followups_snaps_virtualizer_and_syncs_compat_fields`：5 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `turn_top_scroll_task_focuses_requested_user_and_requests_measurement`：4 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `measure_visible_chat_messages_task_schedules_widget_operation`：1 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `update_routes_append_text_to_context_handler`：2 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `stream_delta_syncs_and_prunes_think_block_state`：8 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `stream_flush_updates_tail_and_schedules_save_without_chunk_preparation`：6 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `stream_delta_with_new_open_think_refreshes_autoscroll_throttle`：3 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `composer_update_skips_chunks_but_transcript_event_prepares_visible_window`：5 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。
- `delayed_scroll_rejects_old_session_request_and_manual_scroll`：7 个断言宏 → 原路径保留：widget Task、流缓存保存与延迟代际契约。

### crates/vw-desktop/src/app/message/chat/trajectory_tests.rs

- `trajectory_raw_cache_keys_include_turn_id`：3 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_raw_refresh_normalizes_blank_turn_id`：3 个断言宏 → desktop-session::host_module_37_tests 同名用例；Task.units()>0 对应 missing 非空，其余 loading/attempt 断言保留，使用真实 RawLoadState。
- `raw_modal_and_trajectory_share_reconstructed_payload`：4 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_mode_toggle_defaults_to_conversation`：3 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_mode_toggle_refreshes_missing_completed_raw`：3 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_mode_refreshes_known_raw_steps_without_session_step_metadata`：2 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_snapshot_refresh_populates_raw_without_opening_usage_modal`：4 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_snapshot_refresh_ignores_inactive_session`：2 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_interactions_update_state`：5 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_details_json_interactions_update_state_and_copy`：3 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `select_expands_record_ancestors`：3 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `messages_update_controls_and_session_change_clears_collapsed_groups`：10 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_cache_reuses_projection_for_view_interactions`：1 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_cache_invalidates_for_steps_and_raw_payloads`：3 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_cache_survives_hot_session_restore`：2 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_raw_refresh_ignores_stale_session`：13 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_raw_refresh_error_is_recoverable`：5 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_raw_none_schedules_bounded_retry`：10 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_raw_retry_stops_after_success`：6 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。
- `trajectory_raw_retry_exhaustion_reports_error`：8 个断言宏 → 原路径保留：空轮次筛选移入领域；其余消息/Task/投影缓存跨域契约。

### crates/vw-desktop/src/app/session_gateway_tests.rs

- `gateway_client_binds_configured_endpoint_without_network_access`：3 个断言宏 → 原路径保留：配置端点、活动 tenant/project 装配契约。
- `desktop_project_calls_use_active_tenant_scope`：1 个断言宏 → 原路径保留：配置端点、活动 tenant/project 装配契约。
- `active_project_scope_preserves_active_tenant_and_project_id`：2 个断言宏 → 原路径保留：配置端点、活动 tenant/project 装配契约。
- `active_project_scope_rejects_empty_project_id`：1 个断言宏 → 原路径保留：配置端点、活动 tenant/project 装配契约。
- `active_project_scope_rejects_whitespace_project_id`：1 个断言宏 → 原路径保留：配置端点、活动 tenant/project 装配契约。
- `desktop_agent_scheduler_calls_use_resolved_active_scope`：3 个断言宏 → 原路径保留：配置端点、活动 tenant/project 装配契约。
- `active_agent_scheduler_scope_propagates_invalid_project_id`：1 个断言宏 → 原路径保留：配置端点、活动 tenant/project 装配契约。

## 注册与静态核查

- desktop-session/lib.rs 注册 message::chat::routing 和 cfg(test) host_module_37_tests；无 path/include 反向加载宿主。
- host_module_37_tests 新增六域路由、流载荷、ACP fork refresh 隔离，并承接空轮次 raw 键断言。已有领域算法整组测试继续原 cfg 注册。
- 独立 rustfmt 解析本次新模块及流/ACP 接线；git diff --check 无空白错误。此证据不代表类型检查或行为通过。
- 后续仅用户另行要求时运行：`python3 scripts/cargo_slot.py cargo test -p desktop-session --lib host_module_37_tests -- --test-threads=1`。
- 宿主保留的真实系统边界测试以后可用 `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::message::chat:: -- --test-threads=1` 和 `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib session_gateway_tests -- --test-threads=1`；本轮均未执行。
