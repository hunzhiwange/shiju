# 任务27实施证据（静态交付）

## 授权与验证

遵守用户禁测要求，未运行任何Cargo、测试、绘制或回归。独立rustfmt仅处理修改文件；本任务路径的git diff --check无输出。全工作区diff --check发现其它任务的空白问题，未扩大修改。

## 已接线的生产归属

- settings/worktrees/environment::update → desktop-settings-workspace::environment::update：完整刷新、去重、编辑、请求代际、保存、失败保留；具名EnvironmentEffect只要求宿主选择目录、同步项目缓存和显示toast。本机目录选择仍由loopback和平台约束控制。
- settings/archived_chats::update → desktop-settings-sessions::archive_update::update：单聊天删除/恢复的确认、响应身份、代际与补偿刷新均由库处理。Deleted/Restored/ProjectLoaded/Toast由宿主处理跨域缓存、项目消息和通知。项目整组删除原有独立装配未变。
- projects::reconcile_deleted和set_worktree_enabled持有元数据名称重建与单项目patch策略；宿主保留真实路径、保存串行队列、退出等待和项目导航。
- sessions::next_after_delete只读投影下一个可导航会话；session运行时取消、权限加载、chat清理与App生命周期仍由宿主承担。
- workspace引用现有desktop-workspace::RecentProjectMeta；没有反向依赖宿主或include/path挂载宿主源码。

## 最终职责与宿主保留审计

领域测试在workspace的host_module_27_tests及子模块、sessions的host_module_27_tests独立注册。项目编辑、项目列表、中文帮助菜单、字体加载、截图、真实Iced事件/滚动/工作流选择由领域夹具直接调用现有project_edit_view和system_settings_projects。只以局部生命周期状态投影替代整个App；帮助仍调用实际selectable_titled_text，头像仍调用navigation的真实project_avatar。

宿主逐模块保留理由：
- system_settings_projects：仅ViewState借用、用户消息映射及现有头像/help装配；无列表/布局算法。
- system_settings_projects_ui_refresh_30_tests：仅项目取消/保存/重开与真实磁盘配置、脚本不执行的跨域契约，以及与General设置页实际样式比较的跨页面契约。列表与弹窗事件、全部draw/screenshot/帮助断言迁入project_editor_ui；原preserves用例拆分为同名领域UI与宿主持久化两部分。
- managed_tests：现有checkout导航、真实git worktree数、App项目切换与draft创建、导航代际不改变活动会话、显示Project页面等宿主契约；列表稳定/去重/预检/过期刷新/错误保留以及绘制已迁入managed领域测试。混合用例的局部状态断言从宿主删除，导航不变断言保留。
- projects update：消息转Event、存储写入、startup覆盖标记、保存串行队列/退出watch与导航，均涉及宿主生命周期或跨域效果；元数据重建改为apply_with_metadata真实共同入口。environment_command_saved仅同步跨域project-editor缓存并保存。
- projects_tests：只保留当前项目导航、无toast消息映射、实际PATCH顺序/idle watch和无关消息路由契约；元数据重建、越界/缺少编辑条目等领域断言迁入project_events。
- archived_chats update：仅项目整组删除路由、会话/项目缓存同步、项目刷新跨域消息与toast映射、gateway_changed页面路由；单聊天状态与身份校验在archive_update。archived_chats_tests只验证设置路由、网关切换与App缓存/runtime清理的真实跨域契约；HTTP边界用例全部在sessions库。
- sessions update/tests：会话runtime取消、活动chat/usage/preferences切换、权限和消息加载必须留宿主；列表选择使用sessions::next_after_delete只读投影，不建立可写领域session副本。
- App共享项目路径/metadata/list是导航和project-editor共同使用的真实项目资源；settings库使用已有窄State借用。worktree/archived状态已由App持有对应领域类型，没有反向借用App。

新增开发依赖仅用于原生绘制、实际头像、帮助文本与工作流列表测试；没有宿主源码引用、path/include反向加载或新生产开关。由于用户禁测，只完成静态交付，不宣称编译、运行或绘制通过。

## 基线统计

按pathlib.read_text().splitlines()，来源 3621 → 1534，减少 2087；生产候选 537 → 431，减少 106。这是当前工作区来源口径，可能含同期导航接线，不等同本提交独立减量。新增宿主替代文件0。

| 来源 | 基线 | 当前 |
|---|---:|---:|
| system_settings_projects.rs | 54 | 46 |
| system_settings_projects_tests.rs | 29 | 0 |
| system_settings_projects_ui_refresh_30_tests.rs | 931 | 161 |
| environment_tests.rs | 313 | 0 |
| managed_tests.rs | 368 | 193 |
| recovery_tests.rs | 224 | 0 |
| archived_chats.rs | 236 | 133 |
| archived_chats_tests.rs | 785 | 426 |
| projects.rs | 147 | 143 |
| projects_tests.rs | 236 | 125 |
| sessions.rs | 100 | 109 |
| sessions_tests.rs | 198 | 198 |

## 旧用例逐名映射

保留原函数名；迁移位置保留对应断言。恢复测试中宿主的draft/new-session未变化断言改为领域不发Open效果，宿主真实导航契约仍由managed导航用例验证；项目行使用共享help控件的局部装配。宿主保留用例仅承担上述真实跨域契约；同名用例拆分UI与持久化时两边均保留在下列映射。
- `worktree_row_builds_fill_width_help_label_and_control_layout` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/projects_view.rs
- `ui_refresh_30_layout_and_interaction` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_editor_ui.rs
- `ui_refresh_30_preserves_values_and_disabled_states` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_editor_ui.rs；宿主components/system_settings_projects_ui_refresh_30_tests.rs保留对应跨域契约
- `settings_tabs_peripheral_appearance_project` → crates/vw-desktop/src/app/components/system_settings_projects_ui_refresh_30_tests.rs（跨域契约）
- `help_rollout_21_project_settings_render_contextual_help` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_editor_ui.rs
- `help_rollout_21_renders_contextual_help` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_editor_ui.rs
- `worktree_environment_ui_saves_setup_for_selected_project` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_cancel_preserves_command` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_save_does_not_execute` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_clear_failure_retry_and_stale_response` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_rejects_remote_picker_and_deduplicates_additions` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_project_switch_discards_pending_save` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_renders_chinese_editor_in_both_themes` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_paginates_and_sends_only_project_patch` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_retry_blocks_chat_until_refreshed_ready` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_managed_ui_opens_chat_in_selected_existing_checkout` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（跨域契约）
- `worktree_managed_ui_switches_project_before_draft` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（跨域契约）
- `worktree_managed_ui_refresh_keeps_groups_stable` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs
- `worktree_managed_ui_deduplicates_and_preserves_session_on_failure` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs；宿主components/system_settings_worktrees/managed_tests.rs保留对应跨域契约
- `worktree_managed_ui_rejects_stale_refresh_and_navigation` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs；宿主components/system_settings_worktrees/managed_tests.rs保留对应跨域契约
- `worktree_managed_ui_preflight_is_scoped_and_never_deletes` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs；宿主components/system_settings_worktrees/managed_tests.rs保留对应跨域契约
- `worktree_managed_ui_partial_error_retains_rows_and_collapsed_groups` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs
- `worktree_managed_ui_renders_long_chinese_and_errors_in_both_themes` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs
- `worktree_managed_ui_paginates_all_projects_and_keeps_partial_failure` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs
- `worktree_managed_ui_returns_to_project_screen_before_showing_draft` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（跨域契约）
- `worktree_cleanup_flow_cross_project_response_is_ignored` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_old_epoch_response_is_ignored` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_item_failure_requires_new_preflight` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_restores_and_refreshes` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_delete_success_refreshes_and_records_snapshot` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_load_failure_does_not_report_empty_success` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_restore_failure_can_retry_persisted_snapshot` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_cancel_rejects_late_preflight` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_preflight_cannot_cross_project` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_delete_revokes_inflight_snapshot_list` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `archived_chats_restore_http_persists_only_target_and_retries_failure` → crates/libs/desktop-settings-sessions/src/host_module_27_tests.rs
- `archived_chats_restore_routes_sync_caches_without_navigation_and_rejects_late_results` → crates/vw-desktop/src/app/message/settings/archived_chats_tests.rs（跨域契约）
- `archived_chats_browse_task_reads_all_projects_over_http` → crates/libs/desktop-settings-sessions/src/host_module_27_tests.rs
- `archived_chats_browse_settings_routes_and_gateway_switch` → crates/vw-desktop/src/app/message/settings/archived_chats_tests.rs（跨域契约）
- `archived_chats_delete_http_failure_retains_confirmation_then_retry_deletes_tree` → crates/libs/desktop-settings-sessions/src/host_module_27_tests.rs
- `archived_chats_delete_routes_cleanup_only_after_success` → crates/vw-desktop/src/app/message/settings/archived_chats_tests.rs（跨域契约）
- `archived_chats_project_delete_rechecks_archive_before_restricted_http_delete` → crates/libs/desktop-settings-sessions/src/host_module_27_tests.rs
- `search_updates_query_and_dismisses_previous_row_actions` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `delete_request_and_cancel_only_change_confirmation_state` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `delete_request_allows_current_project` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（跨域契约）
- `delete_confirmed_current_project_selects_first_remaining_project` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（跨域契约）
- `delete_confirmed_removes_associated_state_and_rebuilds_display_names` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `worktree_toggles_patch_one_project_in_click_order` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（跨域契约）
- `delete_confirmed_handles_missing_edit_for_existing_project` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `delete_confirmed_rejects_out_of_bounds_index_but_closes_confirmation` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `worktree_toggle_inserts_enabled_entry_and_removes_disabled_entry` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `unrelated_settings_message_leaves_project_state_unchanged` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（跨域契约）
- `session_delete_active_only_session_clears_conversation_and_usage` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（跨域契约）
- `session_delete_active_session_switches_to_first_remaining_and_syncs_preferences` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（跨域契约）
- `session_delete_inactive_session_preserves_active_context` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（跨域契约）
- `session_delete_missing_session_removes_orphan_runtime_without_changing_context` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（跨域契约）
- `session_delete_missing_active_session_switches_to_first_available_session` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（跨域契约）
- `session_copy_leaves_local_session_state_unchanged_until_task_completes` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（跨域契约）
- `unrelated_settings_message_is_ignored` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（跨域契约）

## 后续验证（仅用户另行授权）

- python3 scripts/cargo_slot.py cargo test -p desktop-settings-workspace --lib host_module_27_tests -- --test-threads=1
- python3 scripts/cargo_slot.py cargo test -p desktop-settings-sessions --lib host_module_27_tests -- --test-threads=1
- python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib

当前不宣称类型检查、绘制和行为通过；新增依赖锁文件仅文本同步。


### 迁移时的断言与夹具说明

原进程隔离包装用于规避App构造对全局配置的读取；独立领域UI不构造App，不读取bootstrap，故去掉该包装和对非法host的前置断言。截图/像素比较/中文标签/布局/鼠标键盘事件断言保留。工作流列表加载、失败的绘制改用现有LifecycleStatus局部投影；未替换生产绘制算法。原调度输入退格、保存/取消、color-picker、最近项目移除、空列表与所有主题/locale循环保留。跨域General样式对照继续在宿主实际窗口组合路径验证。

最终静态复核：任务来源注册与真实update/view映射已接齐；lib.rs独立注册测试，领域源码无crate::app或宿主include/path；本任务改动git diff --check无输出。未运行Cargo、测试、draw或截图。任务卡按公共收尾规则删除。
