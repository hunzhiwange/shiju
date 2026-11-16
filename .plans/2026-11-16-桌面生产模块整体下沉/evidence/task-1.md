# 任务1：静态交付证据

已完成完整局部入口接线与静态职责核对；未编译、未测试、未绘制、未运行最终回归。前序提交 858ac6b3f 为阶段提交，本证据覆盖其后完整收口。严格没有调用任何 Cargo 命令。

## 生产归属及真实调用链

- App.active_session_view_state 使用 desktop-session::session::active::ActiveSessionViewState<P>，P 为现有 desktop-chat-view 的 TrajectoryProjection。领域持有所有步骤、原始载荷、稳定键索引、问题导航、消息元数据、首屏准备字段；宿主字段类型别名维持所有既有调用者。真实轨迹 project_trajectory 由宿主作为已有绘制实现传入，避免 session -> chat-view -> session 循环。
- chat::stream/trajectory 和项目会话打开调用原 App 方法，薄委派进入领域活动状态；replace/upsert/clear/raw 合并在同一转换中重建轨迹。保留原始载荷不覆盖、键索引并集、空 turn 归一化、时间单调递增及工具消息不计入 assistant 步骤的语义。
- App 的 get_session_runtime/get_session_runtime_mut 捕获局部 RuntimeDefaults，再调用 read_runtime/initialize；权限初始化、历史模型恢复、ACP 重建与请求定位使用领域真实实现。read_runtime 对已有状态保持延迟创建默认值，避免每次读取额外克隆全局模型。
- 工具选择读取现有 AgentsSettingsState，selected_static_allowed_tools/tool_inventory/request_tools 归 session::runtime；静态空限制、注册表交集、内置工具和 AgentTool 规则复用原算法，没有第二套实现。
- cache_active_session_chat/prune/known_directory/store_snapshot 使用 session::cache；签名生成、后台热视图失效、冷快照保留和请求会话筛选归领域。历史重置边界由 HistoryResetBoundaries 持有；App 字段和 new 初始化已同步，既有 lifecycle::remove 调用接口保持兼容。
- prepare_session_ui_projection 只映射宿主 Message 为 ProjectionChange 和跨域刷新信号；SessionUiProjection::prepare 拥有实际缓存失效。置顶去重、代际递增和回执仍在领域，宿主只装配 spawn_blocking、Task 与 toast。
- 冷加载首屏的分块边界与准备归 desktop-chat-view::chat_ui::prepare_initial_session_chunk；宿主 prepare_base 仅保留跨 crate 的宿主测试计数钩子。动画帧推进与过期归 desktop-chat-view::stream_render，宿主注入真实时钟及现有持续时间。

## 状态与依赖

没有新增 Cargo 依赖、特性、循环、App 复制品、反向 path/include 或独立控制器。SessionRuntimeState 沿用已有泛型领域类型；RuntimeDefaults 只保存所选会话初始化所需值。测试 Fixture 仅持有入口局部输入，调用生产领域函数；聊天 UI 直接复用既有 desktop_split_04_tests::Fixture，未复制大型 UI 夹具。轨迹重建计数保留可见字段以兼容下游宿主 cfg(test)；依赖 crate 的 cfg(test) 与宿主不同，字段不能仅在库单测配置下存在。

## 来源逐文件与残留理由

| 来源 | 基线行 | 当前行 | 保留职责 |
|---|---:|---:|---|
| `active.rs` | 396 | 302 | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active_tests.rs` | 311 | 59 | 仅窗口工作集移动及项目重载契约 |
| `agent_children.rs` | 2 | 2 | 已有聊天输入领域重导出 |
| `chat_timeline.rs` | 2 | 2 | 已有会话领域重导出 |
| `chat_ui.rs` | 123 | 113 | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui_tests.rs` | 48 | 0 | 已删除，整组归 chat-view 局部夹具 |
| `hover_details.rs` | 22 | 22 | 网关句柄与 Task/Message 映射 |
| `loading.rs` | 117 | 145 | session 加载与 chat-view 准备的组合、宿主性能计数、项目重载跨域生命周期与 Task 映射 |
| `loading_tests.rs` | 99 | 69 | 仅 App 项目重载重置及返回 Task 契约 |
| `runtime.rs` | 396 | 281 | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime_tests.rs` | 913 | 375 | 仅全局偏好/配置桥接及真实 chat::update ACP 回执隔离 |
| `stream_render.rs` | 1 | 1 | 已有 chat-view 重导出 |
| `tests.rs` | 203 | 0 | 已删除，全部测试迁入各真实领域入口 |
| `ui_projection.rs` | 218 | 222 | 宿主 Message 分类、刷新消息映射、阻塞任务与 toast 诊断 |
| `ui_projection_tests.rs` | 40 | 40 | 仅真实跨域 Message 分类契约 |
| `view_cache.rs` | 88 | 88 | 窗口编辑器/滚动状态等 UI 资源的 move/restore 字段映射 |
| `view_cache_tests.rs` | 41 | 41 | 仅真实窗口资源移动、正文指针及面板模式恢复 |

所有残留函数按下表逐项记录；同类薄委派的判定以当前函数体为准。

| 宿主函数 | 保留理由 |
|---|---|
| `active.rs::insert_active_session_step_llm_raw` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::insert_missing_active_session_step_llm_raw` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::stash_active_session_view` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::close_session_transient_ui` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::rebuild_active_session_message_meta` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::sync_active_session_user_question_index` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::refresh_active_session_tail_message_meta` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::clear_active_session_steps` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::replace_active_session_steps` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::replace_active_session_calls` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::replace_active_session_step_llm_raw` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::replace_active_session_llm_raw_step_indices` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::upsert_active_session_step` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::sync_active_session_from_chat` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::cache_active_session_chat` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::prune_inactive_session_chat_cache` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::restore_chat_for_session` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::known_session_directory` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::cached_chat_messages` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::cached_chat_message_ids` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::mark_session_history_reset_boundary` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::session_history_reset_boundary` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::active_shared_chat_messages` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `active.rs::store_session_chat_snapshot` | App 窗口工作集移动、瞬态 composer/菜单清理、选中会话、领域委派及内容无关诊断 |
| `chat_ui.rs::chat_ui` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::chat_ui_schedule` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::sync_completed_stream_text` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::finalize_stream_text` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::sync_chat_virtualizer_compat_fields` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::apply_stream_tail_estimated_height` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::sync_chat_scroll_compat_fields` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::protected_chat_ui_chunk_starts` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::pin_chat_ui_chunk` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::unload_chat_ui_chunk` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::visible_chat_message_window` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::prune_chat_heavy_editor_caches` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::prune_chat_ui_chunks` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::preferred_base_chat_ui_chunk_start` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::collect_chat_ui_chunk_starts` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::pending_chat_ui_chunk_starts` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::mark_chat_ui_chunks_preparing` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::invalidate_chat_ui_chunk` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::invalidate_chat_ui_for_message_idx` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::sync_chat_message_estimated_heights_len` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::refine_chat_message_estimated_heights` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::rebuild_chat_message_estimated_heights` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::invalidate_chat_ui_state` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::apply_prepared_chat_ui_phase` | App 多个 UI 资源字段借用及领域方法转发 |
| `chat_ui.rs::pending_chat_message_editors_window_chunks` | App 多个 UI 资源字段借用及领域方法转发 |
| `hover_details.rs::load_session_hover_details` | 网关句柄与 Task/Message 映射 |
| `loading.rs::prepare_base` | session 加载与 chat-view 准备的组合、宿主性能计数、项目重载跨域生命周期与 Task 映射 |
| `loading.rs::prepare_session_open` | session 加载与 chat-view 准备的组合、宿主性能计数、项目重载跨域生命周期与 Task 映射 |
| `loading.rs::prepare_bounded_session_open` | session 加载与 chat-view 准备的组合、宿主性能计数、项目重载跨域生命周期与 Task 映射 |
| `loading.rs::prepare_session_open_from_ui_store` | session 加载与 chat-view 准备的组合、宿主性能计数、项目重载跨域生命周期与 Task 映射 |
| `loading.rs::reload_sessions_for_project` | session 加载与 chat-view 准备的组合、宿主性能计数、项目重载跨域生命周期与 Task 映射 |
| `loading.rs::rebuild_session_previews` | session 加载与 chat-view 准备的组合、宿主性能计数、项目重载跨域生命周期与 Task 映射 |
| `runtime.rs::session_runtime_defaults` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::new_session_full_access` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::session_full_access` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::current_session_full_access` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::restore_session_full_access` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::load_session_full_access` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::reset_draft_full_access` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::get_session_runtime` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::get_session_runtime_mut` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::current_session_runtime` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::current_session_runtime_ref` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::current_session_runtime_mut` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::current_session_has_started` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::mark_session_viewed` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::session_tool_inventory` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::session_allowed_tools_for_request` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::mark_active_session_viewed` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::current_session_is_requesting` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::session_is_requesting` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::any_session_requesting` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::has_active_explore_summary_animation` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::advance_status_animation_frame` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::find_session_by_request_id` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::sync_active_session_preferences` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::prepare_historical_session_model` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::restore_session_model_from_steps` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::restore_session_model_from_history` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::cancel_session_model_history_restore` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `runtime.rs::mark_session_acp_rebuild_required` | 选中会话与捕获全局默认输入、网关 Task 消息映射、全局偏好回写；转换与权限算法已委派 |
| `ui_projection.rs::preserves_projection` | 宿主 Message 分类、刷新消息映射、阻塞任务与 toast 诊断 |
| `ui_projection.rs::set_session_pin` | 宿主 Message 分类、刷新消息映射、阻塞任务与 toast 诊断 |
| `ui_projection.rs::apply_session_pin` | 宿主 Message 分类、刷新消息映射、阻塞任务与 toast 诊断 |
| `ui_projection.rs::prepare_session_ui_projection` | 宿主 Message 分类、刷新消息映射、阻塞任务与 toast 诊断 |
| `ui_projection.rs::refresh_session_pins_task` | 宿主 Message 分类、刷新消息映射、阻塞任务与 toast 诊断 |
| `ui_projection.rs::apply_session_pins` | 宿主 Message 分类、刷新消息映射、阻塞任务与 toast 诊断 |
| `view_cache.rs::take_from` | 窗口编辑器/滚动状态等 UI 资源的 move/restore 字段映射 |
| `view_cache.rs::restore_into` | 窗口编辑器/滚动状态等 UI 资源的 move/restore 字段映射 |
| `view_cache.rs::chat` | 窗口编辑器/滚动状态等 UI 资源的 move/restore 字段映射 |
| `view_cache.rs::content_signature` | 窗口编辑器/滚动状态等 UI 资源的 move/restore 字段映射 |
| `view_cache.rs::estimated_bytes` | 窗口编辑器/滚动状态等 UI 资源的 move/restore 字段映射 |
| `view_cache.rs::content_signature` | 窗口编辑器/滚动状态等 UI 资源的 move/restore 字段映射 |
| `view_cache.rs::estimated_bytes` | 窗口编辑器/滚动状态等 UI 资源的 move/restore 字段映射 |

## 全部旧断言映射

基线 56 个测试各有唯一归属。表内原断言序号按函数体 assert!/assert_eq!/assert_ne! 的源码顺序；对应新函数内相同序号，除特别注明之外，原断言期望值和先后顺序保持。App 字段访问替换为领域状态/局部输入；未删断言。时间测试注入 2000ms，超过现有 1100ms 过期阈值，u64::MAX 保持未来时间场景。全局当前模型不参与历史消息元数据：该测试的原最终断言对应新断言2，新增断言1只确认局部真实 runtime 的 model-b 输入。

| 原文件/测试 | 原断言数 | 新归属 | 新断言数 |
|---|---:|---|---:|
| `active_tests.rs::refresh_tail_message_meta_appends_then_replaces_changed_label` | 3 | `crates/libs/desktop-session/src/host_module_1_tests.rs::refresh_tail_message_meta_appends_then_replaces_changed_label` | 3 |
| `active_tests.rs::refresh_tail_message_meta_rebuilds_gaps_and_truncates_out_of_range_entries` | 3 | `crates/libs/desktop-session/src/host_module_1_tests.rs::refresh_tail_message_meta_rebuilds_gaps_and_truncates_out_of_range_entries` | 3 |
| `active_tests.rs::question_index_updates_incrementally_for_appended_user_message` | 4 | `crates/libs/desktop-session/src/host_module_1_tests.rs::question_index_updates_incrementally_for_appended_user_message` | 4 |
| `active_tests.rs::active_session_step_mutations_keep_steps_calls_and_index_map_consistent` | 7 | `crates/libs/desktop-session/src/host_module_1_tests.rs::active_session_step_mutations_keep_steps_calls_and_index_map_consistent` | 7 |
| `active_tests.rs::active_session_step_mutations_normalize_blank_turn_ids` | 2 | `crates/libs/desktop-session/src/host_module_1_tests.rs::active_session_step_mutations_normalize_blank_turn_ids` | 2 |
| `active_tests.rs::active_session_clear_removes_step_raw_payloads` | 3 | `crates/libs/desktop-session/src/host_module_1_tests.rs::active_session_clear_removes_step_raw_payloads` | 3 |
| `active_tests.rs::sync_active_session_from_chat_only_advances_session_timestamp` | 3 | `crates/libs/desktop-session/src/host_module_1_tests/cache_tests.rs::sync_active_session_from_chat_only_advances_session_timestamp` | 3 |
| `active_tests.rs::cache_pruning_retains_requesting_sessions_and_removes_inactive_sessions` | 6 | `crates/libs/desktop-session/src/host_module_1_tests/cache_tests.rs::cache_pruning_retains_requesting_sessions_and_removes_inactive_sessions` | 6 |
| `active_tests.rs::known_session_directory_prefers_loaded_sessions_then_searches_projects` | 3 | `crates/libs/desktop-session/src/host_module_1_tests/cache_tests.rs::known_session_directory_prefers_loaded_sessions_then_searches_projects` | 3 |
| `active_tests.rs::history_reset_boundary_replaces_previous_value_and_is_session_scoped` | 2 | `crates/libs/desktop-session/src/host_module_1_tests/cache_tests.rs::history_reset_boundary_replaces_previous_value_and_is_session_scoped` | 2 |
| `active_tests.rs::switch_stashes_previous_chat_without_cloning_payload` | 2 | `crates/vw-desktop/src/app/session/active_tests.rs::switch_stashes_previous_chat_without_cloning_payload` | 2 |
| `active_tests.rs::repeated_stash_and_project_reload_preserve_running_session_history` | 6 | `crates/vw-desktop/src/app/session/active_tests.rs::repeated_stash_and_project_reload_preserve_running_session_history` | 6 |
| `chat_ui_tests.rs::invalidate_chat_ui_state_clears_user_question_index_for_history_replacement` | 2 | `crates/libs/desktop-chat-view/src/host_module_1_tests.rs::invalidate_chat_ui_state_clears_user_question_index_for_history_replacement` | 2 |
| `chat_ui_tests.rs::resolve_render_cache_height_requires_matching_message_content_hash` | 3 | `crates/libs/desktop-chat-view/src/host_module_1_tests.rs::resolve_render_cache_height_requires_matching_message_content_hash` | 3 |
| `loading_tests.rs::cold_session_load_prepares_base_chunk_off_ui_thread` | 4 | `crates/libs/desktop-chat-view/src/host_module_1_tests.rs::cold_session_load_prepares_base_chunk_off_ui_thread` | 4 |
| `loading_tests.rs::reload_sessions_for_project_clears_current_session_state_before_loading` | 8 | `crates/vw-desktop/src/app/session/loading_tests.rs::reload_sessions_for_project_clears_current_session_state_before_loading` | 8 |
| `loading_tests.rs::reload_sessions_for_project_with_path_returns_runnable_task` | 2 | `crates/vw-desktop/src/app/session/loading_tests.rs::reload_sessions_for_project_with_path_returns_runnable_task` | 2 |
| `runtime_tests.rs::runtime_tests_module_is_wired` | 1 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::runtime_tests_module_is_wired` | 1 |
| `runtime_tests.rs::selected_static_allowed_tools_uses_selected_agent_and_normalizes_entries` | 2 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::selected_static_allowed_tools_uses_selected_agent_and_normalizes_entries` | 2 |
| `runtime_tests.rs::selected_static_allowed_tools_treats_blank_main_configuration_as_empty_restriction` | 2 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::selected_static_allowed_tools_treats_blank_main_configuration_as_empty_restriction` | 2 |
| `runtime_tests.rs::session_tool_inventory_handles_unrestricted_intersection_and_static_fallbacks` | 7 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::session_tool_inventory_handles_unrestricted_intersection_and_static_fallbacks` | 7 |
| `runtime_tests.rs::session_tool_inventory_includes_registered_builtin_tools_locked_by_settings` | 3 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::session_tool_inventory_includes_registered_builtin_tools_locked_by_settings` | 3 |
| `runtime_tests.rs::session_tool_inventory_preserves_selected_custom_registered_tools` | 2 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::session_tool_inventory_preserves_selected_custom_registered_tools` | 2 |
| `runtime_tests.rs::restricted_request_preserves_registered_agent_tool` | 1 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::restricted_request_preserves_registered_agent_tool` | 1 |
| `runtime_tests.rs::restricted_request_does_not_invent_unregistered_agent_tool` | 1 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::restricted_request_does_not_invent_unregistered_agent_tool` | 1 |
| `runtime_tests.rs::empty_restricted_request_keeps_only_registered_agent_tool` | 1 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::empty_restricted_request_keeps_only_registered_agent_tool` | 1 |
| `runtime_tests.rs::get_session_runtime_prefers_existing_state_and_uses_global_defaults_for_missing_state` | 8 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::get_session_runtime_prefers_existing_state_and_uses_global_defaults_for_missing_state` | 8 |
| `runtime_tests.rs::get_session_runtime_mut_initializes_and_refreshes_acp_preferences_without_overwriting_agent` | 8 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::get_session_runtime_mut_initializes_and_refreshes_acp_preferences_without_overwriting_agent` | 8 |
| `runtime_tests.rs::current_session_accessors_switch_between_empty_and_active_runtime` | 4 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::current_session_accessors_switch_between_empty_and_active_runtime` | 4 |
| `runtime_tests.rs::current_session_started_covers_chat_active_request_and_queue` | 5 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::current_session_started_covers_chat_active_request_and_queue` | 5 |
| `runtime_tests.rs::viewed_and_requesting_helpers_update_and_query_per_session_state` | 7 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::viewed_and_requesting_helpers_update_and_query_per_session_state` | 7 |
| `runtime_tests.rs::find_session_by_request_id_returns_matching_session_only` | 2 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::find_session_by_request_id_returns_matching_session_only` | 2 |
| `runtime_tests.rs::sync_active_session_preferences_preserves_global_model_and_syncs_other_values` | 8 | `crates/vw-desktop/src/app/session/runtime_tests.rs::sync_active_session_preferences_preserves_global_model_and_syncs_other_values` | 8 |
| `runtime_tests.rs::historical_session_model_restore_uses_latest_recorded_model` | 9 | `crates/vw-desktop/src/app/session/runtime_tests.rs::historical_session_model_restore_uses_latest_recorded_model` | 9 |
| `runtime_tests.rs::historical_session_model_restore_preserves_existing_session_selection` | 2 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::historical_session_model_restore_preserves_existing_session_selection` | 2 |
| `runtime_tests.rs::mark_session_acp_rebuild_required_sets_flag_and_clears_effective_agent` | 2 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::mark_session_acp_rebuild_required_sets_flag_and_clears_effective_agent` | 2 |
| `runtime_tests.rs::animation_helpers_detect_future_change_and_wrap_frame_without_expiring_idle_state` | 5 | `crates/libs/desktop-chat-view/src/host_module_1_tests.rs::animation_helpers_detect_future_change_and_wrap_frame_without_expiring_idle_state` | 5 |
| `runtime_tests.rs::advance_status_animation_frame_expires_old_summary_animation` | 3 | `crates/libs/desktop-chat-view/src/host_module_1_tests.rs::advance_status_animation_frame_expires_old_summary_animation` | 3 |
| `runtime_tests.rs::historical_session_restores_persisted_reasoning_effort_without_affecting_model_restore` | 9 | `crates/vw-desktop/src/app/session/runtime_tests.rs::historical_session_restores_persisted_reasoning_effort_without_affecting_model_restore` | 9 |
| `runtime_tests.rs::acp_model_source_ui_selection_waits_for_backend_confirmation` | 2 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_selection_waits_for_backend_confirmation` | 2 |
| `runtime_tests.rs::acp_model_source_ui_round_trip_preserves_model_and_open_popover` | 4 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_round_trip_preserves_model_and_open_popover` | 4 |
| `runtime_tests.rs::acp_model_source_ui_unsupported_cli_uses_native_and_rejects_client_switch` | 2 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_unsupported_cli_uses_native_and_rejects_client_switch` | 2 |
| `runtime_tests.rs::acp_model_source_ui_late_options_do_not_recreate_deleted_session` | 1 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_late_options_do_not_recreate_deleted_session` | 1 |
| `runtime_tests.rs::acp_model_source_ui_failure_preserves_confirmed_model_and_other_session` | 5 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_failure_preserves_confirmed_model_and_other_session` | 5 |
| `runtime_tests.rs::acp_model_source_ui_confirmation_updates_only_originating_session` | 4 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_confirmation_updates_only_originating_session` | 4 |
| `runtime_tests.rs::acp_model_source_ui_busy_session_rejects_model_and_source_changes` | 3 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_busy_session_rejects_model_and_source_changes` | 3 |
| `runtime_tests.rs::acp_model_source_ui_manual_model_draft_does_not_change_confirmed_model` | 3 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_manual_model_draft_does_not_change_confirmed_model` | 3 |
| `runtime_tests.rs::acp_model_source_ui_new_project_ignores_previous_project_confirmation` | 3 | `crates/vw-desktop/src/app/session/runtime_tests.rs::acp_model_source_ui_new_project_ignores_previous_project_confirmation` | 3 |
| `runtime_tests.rs::fork_selection_runtime_inherited_history_does_not_start_fork` | 4 | `crates/libs/desktop-session/src/host_module_1_tests/runtime_tests.rs::fork_selection_runtime_inherited_history_does_not_start_fork` | 4 |
| `tests.rs::chunk_scheduler_retains_anchor_and_expanded_text` | 5 | `crates/libs/desktop-chat-view/src/host_module_1_tests.rs::chunk_scheduler_retains_anchor_and_expanded_text` | 5 |
| `tests.rs::apply_prepared_chat_ui_phase_preserves_explore_summary_animation_history` | 6 | `crates/libs/desktop-chat-view/src/host_module_1_tests.rs::apply_prepared_chat_ui_phase_preserves_explore_summary_animation_history` | 6 |
| `tests.rs::rebuild_active_session_message_meta_keeps_tool_outside_step_indexing` | 4 | `crates/libs/desktop-session/src/host_module_1_tests.rs::rebuild_active_session_message_meta_keeps_tool_outside_step_indexing` | 4 |
| `tests.rs::rebuild_active_session_message_meta_does_not_relabel_unrecorded_model` | 1 | `crates/libs/desktop-session/src/host_module_1_tests.rs::rebuild_active_session_message_meta_does_not_relabel_unrecorded_model` | 2 |
| `ui_projection_tests.rs::scroll_and_animation_preserve_projection_but_data_events_invalidate` | 2 | `crates/vw-desktop/src/app/session/ui_projection_tests.rs::scroll_and_animation_preserve_projection_but_data_events_invalidate` | 2 |
| `view_cache_tests.rs::session_view_cache_moves_snapshot_without_reallocating_chat` | 1 | `crates/vw-desktop/src/app/session/view_cache_tests.rs::session_view_cache_moves_snapshot_without_reallocating_chat` | 1 |
| `view_cache_tests.rs::session_view_snapshot_keeps_chat_panel_mode_independent` | 2 | `crates/vw-desktop/src/app/session/view_cache_tests.rs::session_view_snapshot_keeps_chat_panel_mode_independent` | 2 |

领域测试 37，保留宿主跨域契约 19。新增一个投影状态机用例，单独覆盖 Preserve/Agent/All。现有 host_split_6/7 为工具流/压缩投影，与本组控制器入口无同名或同组重复；active/runtime 既有函数级测试继续覆盖底层算法，本次状态测试覆盖步骤/派生索引组合不变量，未保留重复大型 App 夹具。

## 真实净减量

按 baseline splitlines：来源 3020 -> 1762，净减少 1258；生产候选 1365 -> 1178，净减少 187，其余为测试减少。没有新建宿主替代文件。必要公共 state/agent.rs 类型别名及 session.rs 注册变化另计，不将领域新增代码计为仓库总代码减少，不承诺编译提速。

## 静态验证及后续限制

本任务路径 git diff --check 无输出；独立 rustfmt（skip_children）仅解析/格式化本次文件。静态确认原56用例唯一归属、断言数不减、测试模块注册、真实调用者及依赖方向。无编译、测试、实际字体/绘制或原生权限证据。

静态发现原 get_session_runtime 默认路径未复制全局 acp_agent，而两个既有测试期望复制；此差异在迁移前代码即存在。本轮保持原初始化与原断言，不混入行为修复；后续运行时应重点核验，不宣称这些用例通过。

只有以后用户授权才运行：
- python3 scripts/cargo_slot.py cargo test -p desktop-session --lib host_module_1_tests -- --test-threads=1
- python3 scripts/cargo_slot.py cargo test -p desktop-chat-view --lib desktop_split_04_tests::host_module_1_tests -- --test-threads=1
- python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib

本轮不自动运行上述命令。宿主剩余跨域契约需在后续授权的对应 active_tests/runtime_tests/loading_tests/ui_projection_tests/view_cache_tests 过滤下验证。
