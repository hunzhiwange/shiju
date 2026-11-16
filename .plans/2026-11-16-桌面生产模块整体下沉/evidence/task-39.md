# 任务39：国际化与公共表现类型静态验收

静态交付完成。保持禁测：未运行 Cargo、单元测试、实机绘制或回归；不声明编译/行为通过。既有第一阶段提交为 5b51e6297、a8ba95c37，本次继续完成残留实现、领域夹具与审计。统计对照实施前提交 `3c878ce29` 和计划 baseline.json，全部使用 splitlines。

## 生产接线与依赖方向

- vw-desktop::__ 仅重导出 desktop_i18n::__，唯一宏体使用 $crate::format::Argument；宿主 macro.rs 删除。
- locale 更新 → apps::refresh_hub_titles（只装配稳定ID/来源常量与可变标题迭代器）→ desktop-i18n；未知/用户标题不变。
- desktop-editor LSP生产状态写入与宿主top_bar状态展示 → desktop-i18n::editor；原始服务消息保持原文。
- search/input session title → i18n::other::session_title；宿主只借用 preview/feedback，生成文案及重命名保护在领域。
- tool labels → desktop-chat-tools::state::localized_tool_name；宿主 other 只重导出。
- render_input_error → desktop-i18n::input::State::render_error；字段落点选择仍在宿主，因为 General 与 Worktree 属于不同状态库。
- task状态/log/SplitOption → desktop-task-model::i18n → desktop-i18n，未反向依赖模型。
- App::channel_qr_flow_snapshot → desktop-settings-channels::state::ChannelsSettingsState::qr_flow_snapshot；快照、generation匹配与configured判定同归领域。
- CostPriceInput/CostSettingsState → desktop-settings-runtime::state；宿主只有重导出。
- pet_state::State::interaction 构造局部借用，宿主只传导航hover/窗口尺寸/时钟；preview_position、session_title、摘要、排序、折叠与交互均由 task-board 真实入口承担。
- pet宿主保留窗口ID持有、Iced窗口任务与运行时/session/workspace/chat输入装配；不搬App，不新增总控库/trait/特性开关。

## 宠物与应用目录逐函数残留

|函数组|真实归属及保留理由|
|---|---|
|`pet::task_pet_state`|只提供导航hover、窗口size、now_ms给领域State::interaction；局部字段借用构造已移出。|
|`pet::register_window_ids`|Iced原生窗口ID资源所有权。|
|`pet::task_pet_preview_scale`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::task_pet_avatar_bounds`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::resize_task_pet_preview`|宿主重置导航hover，检查窗口ID，调用领域preview_position后执行resize/move系统任务；尺寸与位置算法不在宿主。|
|`pet::task_pet_window_size`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::task_pet_render_collapsed`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::sync_task_pet_from_runtime`|从请求生命周期与session缓存收集活动输入，调用领域sync_items，尺寸变更后安排Iced动画；跨域装配。|
|`pet::request_task_pet_resize`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::task_pet_active_count`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::toggle_task_pet_collapsed`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::advance_task_pet_expand_animation`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::start_task_pet_drag`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::drag_task_pet_to`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::finish_task_pet_drag`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::pulse_task_pet_motion`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::move_task_pet_window_to`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::move_task_pet_to`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::task_pet_is_walking`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::task_pet_facing_left`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::set_task_pet_robot_hovered`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::cycle_task_pet_avatar`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::refresh_task_pet_session_title`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::dismiss_task_pet_item`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::set_task_pet_hovered`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::open_task_pet_reply`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::update_task_pet_reply_input`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::take_task_pet_reply`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::task_pet_item_clicked`|单次转发到task-board的pet_interaction/TaskPetState；保留App调用路径，不含局部状态转换。|
|`pet::task_pet_snapshot_for_request`|选取active/cached chat的输入源，ChatRole转显式assistant标记后调用领域摘要；跨chat/session/pet装配。|
|`pet::task_pet_session_title`|只将session列表和workspace project_sessions按既有顺序借给领域session_title；不在宿主选择/截断标题。|

`agent_apps::{reset_if_context_changed,begin,finish}` 保留为跨域catalog协调：reset先调用catalog_input的context guard；guard成立后才清除model_diagnostics访问。两种启动路径按原顺序invalidate external_sessions、routing、high_availability、hermes_dashboard、openclaw_catalog/model/tools/env/defaults、workspace_files、daily_memory、hermes_memory、prompts、project_profiles；begin最后调用catalog_input.begin，finish仅转发generation/context/result。上述字段分别属于外部会话、路由、高可用、应用仪表、工作区、记忆、提示词和项目配置库，不能让其中一域反向拥有其它领域。未改变失效顺序和权限代际。

## 宿主保留测试逐项理由

|测试|必要系统/跨域契约|
|---|---|
|`crates/vw-desktop/src/app/i18n/gettext_area_10_tests.rs::desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments`|只保留 workspace rename 无效输入经过全局 ProjectMessage 路由到全局 toast 的契约；直接创建 toast 的局部部分迁入 ui-controls。|
|`crates/vw-desktop/src/app/i18n/gettext_area_11_tests.rs::desktop_gettext_area_11_feedback_switches_language`|Git controller 的错误效果经宿主映射到全局 toast；确认原始错误作为参数而非模板传递。|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_feedback_switches_language`|TaskBoard 工作树完成消息效果到全局 toast 的真实映射；纯状态/日志格式已在 task-model。|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_locale_selection_refreshes_cached_logs`|General locale 事件触发 task-board 既有日志编辑器重建；校验两个领域的真实调用链。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_catalog_covers_display_calls_and_descriptors`|宿主全源码目录与领域 descriptors 的目录覆盖契约，include_str 仅静态扫描文案；已将失效路径改到真实 workflow-ui/message 与 page，并补 page 源码，不反向编译宿主。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_save_failure_toast_keeps_nested_error_semantics`|Workflow controller 嵌套失败反馈到全局 toast 的效果映射；局部 workflow 状态和生命周期校验已迁入 workflow-ui。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_locale_selection_refreshes_tab_without_navigation`|General locale → workflow → navigation tab 的跨域装配；保持 active_tab 和用户标题。|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_launch_feedback_switches_language`|Account/runtime 尚未就绪时 apps 领域拒绝启动，宿主转换到全局 toast；纯 launch feedback metadata 已迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_binding_rejection_keeps_display_semantics`|Apps API-key binding 校验反馈同时写入 apps 状态及全局 toast，核对两路效果装配。|
|`crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs::desktop_gettext_area_15_failed_skip_preserves_error_and_switches_language`|原生更新持久化回执通过 ViewMessage 映射到全局 toast；保留服务错误内容。|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_locale_event_updates_existing_and_new_editors`|General locale → navigation 既有预览/editor → git editor，以及后续新 tab 的跨域广播。LSP 格式与 preview OpenFailed 局部夹具已迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_dynamic_feedback_preserves_raw_arguments`|Settings 全局消息分别分派 skills 状态反馈与 hooks 效果到全局 toast；校验两条真实宿主反馈桥，不复制局部算法。|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_search_preserves_chinese_navigation`|聚合 MCP/skills 的真实描述符进入统一搜索；验证跨领域 catalog 装配与稳定检索词。|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`|仅剩 MCP 宿主组合（catalog/bindings/resync/diagnostics/tools 五个真实面板及上层消息映射）；其余9个区域已按域迁出，保留原三语言、双主题、960×800布局/绘制/非透明像素与PNG保存路径。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_general_search_keeps_chinese_and_stable_keywords`|统一 settings catalog 装配 General 描述符；覆盖中文导航词与稳定关键词。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_history_rebuilds_on_locale_selection_without_translating_output`|General LocaleSelected 路由到 cron 历史编辑器重建的跨域连线；保留原输出与状态不被翻译。|
|`crates/vw-desktop/src/app/i18n/project_tests.rs::desktop_gettext_area_10_attachment_errors_switch_after_collection`|真实临时目录/缺失文件进入 workspace 附件采集，再汇总到 navigation notification；文件系统与全局通知边界契约。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_search_matches_locale_stable_ids_and_chinese_navigation`|跨领域统一搜索以 ModelRoutes 目标注册；不是模型路由局部校验夹具。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_builds_current_pages_in_all_locales`|宿主组合 models/providers 与 network/gateway 等7个真实页面并映射全局消息；验证组合入口，不复制领域绘制或状态转换。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_save_waits_for_gateway_acknowledgment`|EmbeddingRoutes Save → 宿主持久化路由 → AgentConfigSaved(tag) 的跨域回执契约，同时验证 gateway tag 不串域。|
|`crates/vw-desktop/src/app/i18n/settings_runtime_tests.rs::desktop_i18n_settings_runtime_views_and_search_follow_locale`|memory/security/runtime/storage/observability 多库组合与统一搜索目标；保留嵌入页面的全局消息映射、Qdrant 条件描述符与 help 内容接线。|
|`crates/vw-desktop/src/app/models_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/app_aux_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/chrome_fields_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/core_fields_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/fields_navigation_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/fields_task_board_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/fields_workspace_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/git_fields_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/settings_preview_fields_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state/workspace_task_fields_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/app_state_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::sync_task_pet_from_runtime_adds_updates_completes_and_sorts_items`|运行时请求生命周期/会话标题/chat消息与 task-board pet 的快照装配，保留实际 AgentRequest、ChatRole 映射；局部摘要、排序、隐藏、缩放、位置断言另已迁出。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_none_keeps_controls`|宿主只保留 companion_navigation 可见状态与实际 task_pet::window 组合入口；几何、折叠、形态断言迁入 task-board。|
|`crates/vw-desktop/src/app/state/presentation_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/settings_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/state/workspace_tests.rs::test_module_is_wired`|仅约束当前宿主模块的cfg(test)注册后缀，无App/局部业务夹具。|
|`crates/vw-desktop/src/app/ui_types_tests.rs::global_messages_exclude_removed_design_domain`|全局消息总线不得恢复已移除领域，直接约束宿主 Message 组成。|

## 完整旧断言映射

A1…An 指原函数按源码顺序出现的 assert!/assert_eq!/assert_ne! 宏，B1…Bn 指目标函数顺序；循环的输入、预期字符串、原始参数、主题/布局/绘制步骤均保留。计数仅为静态完整性核对，不是执行结果。未以新测试替代被测算法。

|原文件/用例|原断言|现位置与映射|
|---|---:|---|
|`crates/vw-desktop/src/app/i18n/apps_tests.rs::desktop_gettext_area_14_hub_title_updates_after_locale_change`|1|`crates/libs/desktop-i18n/src/host_module_39_tests.rs::desktop_gettext_area_14_hub_title_updates_after_locale_change`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_10_tests.rs::desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments`|2|`crates/libs/desktop-ui-controls/src/host_module_39_locale_tests.rs::desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments`; `crates/vw-desktop/src/app/i18n/gettext_area_10_tests.rs::desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments`；A1→ui-controls B1（直接toast）；A2→宿主B1（rename路由）。|
|`crates/vw-desktop/src/app/i18n/gettext_area_11_tests.rs::desktop_gettext_area_11_feedback_switches_language`|1|`crates/vw-desktop/src/app/i18n/gettext_area_11_tests.rs::desktop_gettext_area_11_feedback_switches_language`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_feedback_switches_language`|1|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_feedback_switches_language`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_status_log_keeps_semantics_and_raw_output`|3|`crates/libs/desktop-task-model/src/host_module_39_tests.rs::desktop_gettext_area_12_status_log_keeps_semantics_and_raw_output`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_locale_selection_refreshes_cached_logs`|2|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_locale_selection_refreshes_cached_logs`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_cached_message_preserves_arguments`|2|`crates/libs/desktop-task-model/src/host_module_39_tests.rs::desktop_gettext_area_12_cached_message_preserves_arguments`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_message_feedback_switches_language`|2|`crates/libs/desktop-workflow-ui/src/host_module_39_locale_tests.rs::desktop_gettext_area_13_message_feedback_switches_language`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_catalog_covers_display_calls_and_descriptors`|2|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_catalog_covers_display_calls_and_descriptors`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_save_failure_toast_keeps_nested_error_semantics`|2|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_save_failure_toast_keeps_nested_error_semantics`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_locale_selection_refreshes_tab_without_navigation`|3|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_locale_selection_refreshes_tab_without_navigation`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_lifecycle_validation_keeps_template_and_identifier`|2|`crates/libs/desktop-workflow-ui/src/host_module_39_locale_tests.rs::desktop_gettext_area_13_lifecycle_validation_keeps_template_and_identifier`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_launch_feedback_switches_language`|1|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_launch_feedback_switches_language`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_cache_error_keeps_semantics_until_render`|3|`crates/libs/desktop-skill-hub/src/host_module_39_locale_tests.rs::desktop_gettext_area_14_cache_error_keeps_semantics_until_render`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_status_display_preserves_stable_identifier`|3|`crates/libs/desktop-skill-hub/src/host_module_39_locale_tests.rs::desktop_gettext_area_14_status_display_preserves_stable_identifier`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_launch_error_metadata_rejects_stale_generation`|4|`crates/libs/desktop-apps-ui/src/host_module_39_locale_tests.rs::desktop_gettext_area_14_launch_error_metadata_rejects_stale_generation`；A1…A4→B1…B4，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_binding_rejection_keeps_display_semantics`|2|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_binding_rejection_keeps_display_semantics`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs::desktop_gettext_area_15_failed_skip_preserves_error_and_switches_language`|1|`crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs::desktop_gettext_area_15_failed_skip_preserves_error_and_switches_language`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_lsp_state_switches_without_translating_server_content`|2|`crates/libs/desktop-i18n/src/host_module_39_tests.rs::desktop_gettext_area_16_lsp_state_switches_without_translating_server_content`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_locale_event_updates_existing_and_new_editors`|4|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_locale_event_updates_existing_and_new_editors`；A1…A4→B1…B4，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_preview_loading_and_failure_switch_at_display_time`|2|`crates/libs/desktop-editor/src/host_module_14_tests/host_module_39_locale_tests.rs::desktop_gettext_area_16_preview_loading_and_failure_switch_at_display_time`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_account_labels_follow_current_locale`|1|`crates/libs/desktop-account-ui/src/host_module_39_locale_tests.rs::desktop_gettext_area_18_account_labels_follow_current_locale`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_knowledge_feedback_switches_after_creation`|2|`crates/libs/desktop-expert-hub/src/host_module_39_locale_tests.rs::desktop_gettext_area_18_knowledge_feedback_switches_after_creation`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_builtin_tabs_translate_without_changing_user_titles`|3|`crates/libs/desktop-chat-tools/src/host_module_39_locale_tests.rs::desktop_gettext_area_18_builtin_tabs_translate_without_changing_user_titles`; `crates/libs/desktop-i18n/src/host_module_39_locale_tests.rs::desktop_gettext_area_18_builtin_tabs_translate_without_changing_user_titles`；A1/A2→i18n B1/B2；A3→chat-tools B1。|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_account_validation_retains_semantic_feedback`|2|`crates/libs/desktop-account-ui/src/host_module_39_locale_tests.rs::desktop_gettext_area_18_account_validation_retains_semantic_feedback`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_generated_session_titles_preserve_renames`|3|`crates/libs/desktop-i18n/src/host_module_39_tests.rs::desktop_gettext_area_18_generated_session_titles_preserve_renames`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_dynamic_feedback_preserves_raw_arguments`|3|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_dynamic_feedback_preserves_raw_arguments`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_search_preserves_chinese_navigation`|4|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_search_preserves_chinese_navigation`；A1…A4→B1…B4，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_mcp_validation_keeps_semantic_error`|2|`crates/libs/desktop-settings-mcp/src/host_module_39_locale_tests.rs::desktop_gettext_area_6_mcp_validation_keeps_semantic_error`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`|1|`crates/libs/desktop-settings-agents/src/host_module_16_tests/host_module_39_render.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`; `crates/libs/desktop-settings-channels/src/host_module_39_render.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`; `crates/libs/desktop-settings-network/src/host_module_21_tests/host_module_39_render.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`; `crates/libs/desktop-settings-skills/src/host_module_23_tests/host_module_39_render.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`; `crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`；原像素A1按region完整拆开：network[tunnel/web-search]、agents[agents/tools/skills/identity]、channels、skills[skills/plugins]各B1，MCP留宿主B1；原绘制与PNG保存代码原样复用。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_quick_message_error_switches_locale_without_changing_raw_error`|1|`crates/libs/desktop-settings-appearance/src/host_module_39_locale_tests.rs::desktop_gettext_area_7_quick_message_error_switches_locale_without_changing_raw_error`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_dialogue_feedback_switches_locale`|2|`crates/libs/desktop-settings-agents/src/host_module_39_locale_tests.rs::desktop_gettext_area_7_dialogue_feedback_switches_locale`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_acp_validation_is_localized_without_changing_input`|3|`crates/libs/desktop-settings-agents/src/host_module_39_locale_tests.rs::desktop_gettext_area_7_acp_validation_is_localized_without_changing_input`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_general_search_keeps_chinese_and_stable_keywords`|3|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_general_search_keeps_chinese_and_stable_keywords`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_theme_delete_guard_retains_localized_feedback`|2|`crates/libs/desktop-settings-appearance/src/host_module_39_locale_tests.rs::desktop_gettext_area_7_theme_delete_guard_retains_localized_feedback`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_local_validation_feedback_switches_language`|3|`crates/libs/desktop-settings-network/src/host_module_39_locale_tests.rs::desktop_gettext_area_7_local_validation_feedback_switches_language`; `crates/libs/desktop-settings-runtime/src/host_module_39_locale_tests.rs::desktop_gettext_area_7_local_validation_feedback_switches_language`；A1→network B1；A2/A3→runtime B1/B2。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_history_rebuilds_on_locale_selection_without_translating_output`|3|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_history_rebuilds_on_locale_selection_without_translating_output`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs::desktop_gettext_area_9_existing_toast_renders_current_locale`|1|`crates/libs/desktop-ui-controls/src/host_module_39_locale_tests.rs::desktop_gettext_area_9_existing_toast_renders_current_locale`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs::desktop_gettext_area_9_notifications_and_errors_retain_semantic_source`|3|`crates/libs/desktop-i18n/src/host_module_39_locale_tests.rs::desktop_gettext_area_9_notifications_and_errors_retain_semantic_source`; `crates/libs/desktop-navigation/src/host_module_39_locale_tests.rs::desktop_gettext_area_9_notifications_and_errors_retain_semantic_source`；A1→navigation B1；A2/A3→i18n B1/B2（State::render_error实际生产入口）。|
|`crates/vw-desktop/src/app/i18n/project_tests.rs::desktop_gettext_area_10_attachment_errors_switch_after_collection`|4|`crates/vw-desktop/src/app/i18n/project_tests.rs::desktop_gettext_area_10_attachment_errors_switch_after_collection`；A1…A4→B1…B4，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_validation_and_sync_keep_semantics_after_switch`|5|`crates/libs/desktop-settings-providers/src/host_module_39_locale_tests.rs::desktop_i18n_settings_models_validation_and_sync_keep_semantics_after_switch`；A1…A5→B1…B5，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_search_matches_locale_stable_ids_and_chinese_navigation`|3|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_search_matches_locale_stable_ids_and_chinese_navigation`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_builds_current_pages_in_all_locales`|0|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_builds_current_pages_in_all_locales`；A1…A0→B1…B0，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_save_waits_for_gateway_acknowledgment`|6|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_save_waits_for_gateway_acknowledgment`；A1…A6→B1…B6，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_cached_details_translate_without_mutating_service_json`|5|`crates/libs/desktop-settings-providers/src/host_module_39_locale_tests.rs::desktop_i18n_settings_models_cached_details_translate_without_mutating_service_json`；A1…A5→B1…B5，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/settings_runtime_tests.rs::desktop_i18n_settings_runtime_views_and_search_follow_locale`|4|`crates/vw-desktop/src/app/i18n/settings_runtime_tests.rs::desktop_i18n_settings_runtime_views_and_search_follow_locale`；A1…A4→B1…B4，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/i18n/tasks_tests.rs::desktop_gettext_area_12_split_option_keeps_value`|2|`crates/libs/desktop-task-model/src/host_module_39_tests.rs::desktop_gettext_area_12_split_option_keeps_value`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/models_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/models_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/app_aux_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/app_aux_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/chrome_fields_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/chrome_fields_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/core_fields_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/core_fields_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/fields_navigation_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/fields_navigation_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/fields_task_board_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/fields_task_board_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/fields_workspace_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/fields_workspace_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/git_fields_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/git_fields_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/settings_preview_fields_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/settings_preview_fields_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state/workspace_task_fields_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state/workspace_task_fields_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/app_state_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/app_state_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_ignores_visible_answer`|1|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::newest_open_thinking_text_ignores_visible_answer`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_returns_none_without_open_think`|1|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::newest_open_thinking_text_returns_none_without_open_think`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_skips_user_and_uses_newest_open_assistant`|1|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::newest_open_thinking_text_skips_user_and_uses_newest_open_assistant`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::sync_task_pet_from_runtime_adds_updates_completes_and_sorts_items`|15|`crates/vw-desktop/src/app/state/pet_tests.rs::sync_task_pet_from_runtime_adds_updates_completes_and_sorts_items`；A1…A15→B1…B15，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_none_keeps_controls`|4|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::mini_companion_none_keeps_controls`; `crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_none_keeps_controls`；A1可见性留宿主B1；A2尺寸/A3折叠/A4形态→task-board B1/B2/B3。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_scale_rejects_non_finite_and_bounds_values`|5|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::mini_companion_scale_rejects_non_finite_and_bounds_values`；A1…A5→B1…B5，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_stack_prioritizes_running_tasks`|3|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::mini_companion_stack_prioritizes_running_tasks`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_dismiss_updates_top_card`|4|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::mini_companion_dismiss_updates_top_card`；A1…A4→B1…B4，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_scaled_bounds_include_controls`|2|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::mini_companion_scaled_bounds_include_controls`；A1…A2→B1…B2，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_size_preview_preserves_bottom_edge_and_card_dimensions`|3|`crates/libs/desktop-task-board/src/host_module_39_tests.rs::mini_companion_size_preview_preserves_bottom_edge_and_card_dimensions`；A1…A3→B1…B3，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/presentation_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/presentation_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs::test_module_is_wired`|1|`crates/libs/desktop-settings-mcp/src/host_module_39_infrastructure_tests.rs::test_module_is_wired`；A1→B1；module_path后缀仅调整为实际领域模块名。|
|`crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs::mcp_settings_default_opens_servers_with_empty_runtime_state`|14|`crates/libs/desktop-settings-mcp/src/host_module_39_infrastructure_tests.rs::mcp_settings_default_opens_servers_with_empty_runtime_state`；A1…A14→B1…B14，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/settings/provider_model_tests.rs::test_module_is_wired`|1|`crates/libs/desktop-settings-providers/src/host_module_39_provider_model_tests.rs::test_module_is_wired`；A1→B1；module_path后缀仅调整为实际领域模块名。|
|`crates/vw-desktop/src/app/state/settings/provider_model_tests.rs::route_classification_and_heartbeat_defaults_are_ready_to_edit`|6|`crates/libs/desktop-settings-providers/src/host_module_39_provider_model_tests.rs::route_classification_and_heartbeat_defaults_are_ready_to_edit`；A1…A6→B1…B6，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/settings/runtime_tests.rs::test_module_is_wired`|1|`crates/libs/desktop-settings-runtime/src/host_module_39_runtime_tests.rs::test_module_is_wired`；A1→B1；module_path后缀仅调整为实际领域模块名。|
|`crates/vw-desktop/src/app/state/settings/runtime_tests.rs::gateway_client_agents_coordination_and_cost_defaults_are_stable`|11|`crates/libs/desktop-settings-runtime/src/host_module_39_runtime_tests.rs::gateway_client_agents_coordination_and_cost_defaults_are_stable`；A1…A11→B1…B11，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/settings_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/settings_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/state/workspace_tests.rs::test_module_is_wired`|1|`crates/vw-desktop/src/app/state/workspace_tests.rs::test_module_is_wired`；A1…A1→B1…B1，预期与判断顺序保留。|
|`crates/vw-desktop/src/app/ui_types_tests.rs::global_messages_exclude_removed_design_domain`|1|`crates/vw-desktop/src/app/ui_types_tests.rs::global_messages_exclude_removed_design_domain`；A1…A1→B1…B1，预期与判断顺序保留。|

LSP原用例的editor refresh调用从纯格式测试中移除：原有/新建editor language的全部断言由宿主 `desktop_gettext_area_16_locale_event_updates_existing_and_new_editors` 原位保留。宠物纯摘要夹具将 ChatMessage 转为领域实际消费的 (assistant, content)，宿主runtime联动用例仍用真实ChatRole。新增 session_title 用例另外固定首匹配、空标题不回落project、缺少session才用project的原有次序。

## App字段逐项审计

App只组合既有领域状态/系统资源。别名通过任务归属文件中的pub use和泛型绑定解析；下表的领域状态所有权不等于把状态实现留在宿主。`app_state.rs`与app_init/new仍是组合根，不整体迁移。

|字段|实际声明类型|宿主保留职责|
|---|---|
|`git`|`desktop_git_ui::state::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`usage_center`|`crate::app::views::usage::center::state::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`voice`|`crate::app::message::voice::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`session_ui_projection`|`crate::app::session::ui_projection::SessionUiProjection`|根事件/时间/焦点/视图投影在多个领域间分派；具体状态类型、布局或绘制逻辑由领域实现。|
|`startup_state`|`NativeStartupState`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`startup_started_at`|`std::time::Instant`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`startup_configuration`|`crate::app::impls::StartupConfiguration`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`navigation_workspace`|`desktop_navigation::panel::work_workspace::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`navigation_mode`|`desktop_navigation::panel::workspace_mode::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`layout_drag`|`desktop_navigation::view_layout::drag::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`popover_visibility`|`desktop_chat_input::popovers::Visibility`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`navigation_panels`|`desktop_navigation::panel::settings::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`usage_panel`|`desktop_usage::panel::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`update_panel`|`desktop_update::panel::PanelState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`navigation_tabs`|`desktop_navigation::panel::tabs::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`companion_navigation`|`desktop_navigation::panel::companion::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`web_navigation`|`desktop_apps_ui::web::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`external_navigation`|`desktop_navigation::panel::external::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`navigation_sidebar`|`desktop_navigation::panel::work_sidebar::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`work_browser`|`WorkBrowserState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`work_browser_controller`|`crate::app::webview_runtime::WorkBrowserController`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`workspace_state`|`desktop_workspace::project_state::State<crate::app::Message>`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`project_input`|`desktop_workspace::basic::InputState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`project_find`|`desktop_workspace::project_message::find::FindState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`session_state`|`desktop_session::session::state::State<desktop_chat_view::stream_render::StreamRenderState, crate::app::session::view_cache::SessionViewSnapshot>`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`acp_settings`|`AcpSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`mcp_settings`|`McpSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`chat_view`|`desktop_chat_view::view_snapshot::SessionViewSnapshot< ActiveSessionViewState, ChatPanelMode, >`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`chat_interaction`|`desktop_chat_view::interaction::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`todo_state`|`desktop_chat_view::todo::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`window_size`|`(f32, f32)`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`mobile_layout`|`MobileLayoutState`|根事件/时间/焦点/视图投影在多个领域间分派；具体状态类型、布局或绘制逻辑由领域实现。|
|`fullscreen_layout_settling`|`bool`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`startup_resize_checked`|`bool`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`window_position`|`(f32, f32)`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`main_window_id`|`Option<iced::window::Id>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`released_main_windows`|`Vec<iced::window::Id>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`task_pet_window_id`|`Option<iced::window::Id>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`shutdown_requested`|`bool`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`desktop_preferences_persistence`|`crate::app::config::DesktopPreferencesPersistence`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`general_settings`|`desktop_settings_appearance::general_state::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`web_install`|`crate::app::web_install::State`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`desktop_update`|`crate::app::desktop_update::DesktopUpdate`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`open_external_platform`|`Option<RuntimePlatform>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`file_manager`|`desktop_workspace::file_manager::FileManagerState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`status_animation_frame`|`usize`|根事件/时间/焦点/视图投影在多个领域间分派；具体状态类型、布局或绘制逻辑由领域实现。|
|`model_popover_hover`|`Option<ModelPopoverHover>`|根事件/时间/焦点/视图投影在多个领域间分派；具体状态类型、布局或绘制逻辑由领域实现。|
|`search`|`desktop_search::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`project_editor`|`desktop_workspace::project_message::edit::Editor`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`terminal`|`TerminalState`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`terminal_config_persistence`|`crate::app::message::terminal::config::ConfigPersistence`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`terminals_by_project`|`HashMap<String, TerminalState>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`appearance`|`desktop_settings_appearance::theme_state::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`input_feedback`|`desktop_i18n::input::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`lifecycle`|`desktop_platform::lifecycle::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`preview_state`|`desktop_editor::preview::state::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`clipboard_state`|`desktop_navigation::clipboard::Feedback`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`pet_state`|`desktop_task_board::pet_state::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`task_pet_size_pending`|`Option<crate::app::config::PreferencesScope>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`file_search_scroll_id`|`Id`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`slash_palette_scroll_id`|`Id`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`archived_chats`|`ArchivedChatsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`worktree_settings`|`WorktreeSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`provider_settings`|`ProviderSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`model_settings`|`ModelSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`embedding_routes_settings`|`EmbeddingRoutesSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`model_routes_settings`|`ModelRoutesSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`query_classification_settings`|`QueryClassificationSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`heartbeat_settings`|`HeartbeatSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`cron_settings`|`CronSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`scheduler_settings`|`SchedulerSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`hooks_settings`|`HooksSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`runtime_settings`|`RuntimeSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`skills_settings`|`SkillsSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`web_search_settings`|`WebSearchSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`browser_settings`|`BrowserSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`computer_control_settings`|`ComputerControlSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`http_request_settings`|`HttpRequestSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`gateway_settings`|`GatewaySettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`apps_catalog`|`AppsCatalogState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`agent_apps_auth`|`super::agent_apps_auth::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`app_providers`|`super::app_providers::list::ListState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`gateway_client_settings`|`GatewayClientSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`agents_settings`|`AgentsSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`cost_settings`|`CostSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`memory_settings`|`MemorySettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`channels_settings`|`ChannelsSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`reliability_settings`|`ReliabilitySettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`multimodal_settings`|`MultimodalSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`security_settings`|`SecuritySettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`autonomy_settings`|`AutonomySettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`observability_settings`|`ObservabilitySettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`storage_settings`|`StorageSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`proxy_settings`|`ProxySettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`tunnel_settings`|`TunnelSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`themes_settings`|`ThemesSettingsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`file_tree_interaction`|`desktop_workspace::interaction::InteractionState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`input_suggestions`|`desktop_chat_input::input_panel::suggestions::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`lsp_events`|`Option<std::sync::mpsc::Receiver<LspEvent>>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`lsp_event_sender`|`Option<std::sync::mpsc::Sender<LspEvent>>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`lsp_manager`|`Option<LspServiceManager>`|进程/Iced资源与宿主生命周期、原生窗口/配置回执作用域装配；领域不拥有进程或其它域资源。|
|`focus_area`|`FocusArea`|根事件/时间/焦点/视图投影在多个领域间分派；具体状态类型、布局或绘制逻辑由领域实现。|
|`knowledge`|`KnowledgeUiState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`workflow_state`|`crate::apps::workflow::state::WorkflowState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`cursor_position`|`Point`|根事件/时间/焦点/视图投影在多个领域间分派；具体状态类型、布局或绘制逻辑由领域实现。|
|`spinner_frame`|`usize`|根事件/时间/焦点/视图投影在多个领域间分派；具体状态类型、布局或绘制逻辑由领域实现。|
|`notification_state`|`desktop_navigation::message::notification::OwnedState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`toast_state`|`desktop_ui_controls::toast::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`account`|`AccountState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`expert_hub`|`crate::app::skill_hub::ExpertHubState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`shiju_apps`|`crate::app::shiju_apps::ShijuAppsState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`workflow_manager`|`crate::app::views::workflow_manager::WorkflowManagerState`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|
|`task_board`|`desktop_task_board::state::State`|组合根唯一持有领域状态句柄，供不同路由/视图借用；构造与状态实现已在声明类型所属库（别名见逐文件去向）。|

## 逐文件去向与splitlines

|来源|计划基线|实施前3c878ce29|当前宿主|迁出/保留事实|
|---|---:|---:|---:|---|
|`crates/vw-desktop/src/app/i18n/apps.rs`|21|21|21|只保留全局App到领域的局部借用、locale跨域广播或稳定兼容重导出；调用链见上文。|
|`crates/vw-desktop/src/app/i18n/apps_tests.rs`|21|21|0|旧文件与挂载删除；宏或领域断言去向见完整映射。|
|`crates/vw-desktop/src/app/i18n/editor.rs`|38|38|19|只保留全局App到领域的局部借用、locale跨域广播或稳定兼容重导出；调用链见上文。|
|`crates/vw-desktop/src/app/i18n/gettext_area_10_tests.rs`|34|34|20|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_11_tests.rs`|19|19|20|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs`|77|77|46|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs`|176|176|168|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs`|106|106|51|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs`|28|28|28|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs`|75|75|34|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs`|78|78|0|旧文件与挂载删除；宏或领域断言去向见完整映射。|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs`|157|157|99|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs`|176|176|42|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs`|50|50|0|旧文件与挂载删除；宏或领域断言去向见完整映射。|
|`crates/vw-desktop/src/app/i18n/macro.rs`|6|6|0|旧文件与挂载删除；宏或领域断言去向见完整映射。|
|`crates/vw-desktop/src/app/i18n/mod.rs`|40|40|30|只保留全局App到领域的局部借用、locale跨域广播或稳定兼容重导出；调用链见上文。|
|`crates/vw-desktop/src/app/i18n/other.rs`|20|19|12|只保留全局App到领域的局部借用、locale跨域广播或稳定兼容重导出；调用链见上文。|
|`crates/vw-desktop/src/app/i18n/project_tests.rs`|28|28|28|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs`|222|222|115|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/settings_runtime_tests.rs`|67|67|67|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/i18n/tasks.rs`|25|25|1|只保留全局App到领域的局部借用、locale跨域广播或稳定兼容重导出；调用链见上文。|
|`crates/vw-desktop/src/app/i18n/tasks_tests.rs`|16|16|0|旧文件与挂载删除；宏或领域断言去向见完整映射。|
|`crates/vw-desktop/src/app/models.rs`|18|18|18|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_chat_view, session_ui_types|
|`crates/vw-desktop/src/app/models_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/account.rs`|2|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_account_ui|
|`crates/vw-desktop/src/app/state/agent.rs`|65|39|39|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_chat_tools, desktop_chat_view, desktop_session, desktop_settings_agents|
|`crates/vw-desktop/src/app/state/agent_apps.rs`|88|71|71|catalog泛型实参绑定及多域invalidate/start/finish协调，逐字段职责见上文。|
|`crates/vw-desktop/src/app/state/agent_apps_auth.rs`|5|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps, desktop_settings_account|
|`crates/vw-desktop/src/app/state/app_providers/agy.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/claude.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/codex.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/desktop.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/dsh.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/edit.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/endpoints.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/env.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/gemini.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/grok.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/hermes.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/icons.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/kimi.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/leave.rs`|2|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/list.rs`|17|17|17|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/mcode.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/mod.rs`|40|40|40|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：|
|`crates/vw-desktop/src/app/state/app_providers/mode.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/model_catalog.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/omp.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/openclaw.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/opencode.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/paths.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/pi.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/preview.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/snippets.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_providers/universal.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_provider_apps|
|`crates/vw-desktop/src/app/state/app_state/app_aux.rs`|12|12|12|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_apps_ui, desktop_navigation, desktop_ui_controls, desktop_web_runtime, desktop_workspace|
|`crates/vw-desktop/src/app/state/app_state/app_aux_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state/chrome_fields_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state/core_fields_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state/fields_navigation_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state/fields_task_board_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state/fields_workspace_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state/git_fields_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state/settings_preview_fields_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state/workspace_task_fields_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/app_state_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/knowledge.rs`|2|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_expert_hub|
|`crates/vw-desktop/src/app/state/pet.rs`|241|237|210|原生窗口资源及runtime/session/chat/workspace→pet输入装配；全部局部派生/交互调用task-board。|
|`crates/vw-desktop/src/app/state/pet_tests.rs`|292|292|139|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/plugin_install.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_skills|
|`crates/vw-desktop/src/app/state/presentation.rs`|15|19|19|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_git_ui, desktop_navigation|
|`crates/vw-desktop/src/app/state/presentation_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/project_profiles.rs`|4|4|4|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_workspace|
|`crates/vw-desktop/src/app/state/runtime_tools.rs`|4|4|4|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_chat_tools, desktop_session|
|`crates/vw-desktop/src/app/state/settings/acp_diagnostic.rs`|2|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_agents|
|`crates/vw-desktop/src/app/state/settings/agents.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_agents|
|`crates/vw-desktop/src/app/state/settings/aggregation_omo.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_apps_ui|
|`crates/vw-desktop/src/app/state/settings/archived_chats.rs`|1|1|1|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_sessions|
|`crates/vw-desktop/src/app/state/settings/channels.rs`|22|22|7|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_channels|
|`crates/vw-desktop/src/app/state/settings/computer_control.rs`|2|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_security|
|`crates/vw-desktop/src/app/state/settings/cron_frequency.rs`|2|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_cron|
|`crates/vw-desktop/src/app/state/settings/cron_history.rs`|2|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_cron|
|`crates/vw-desktop/src/app/state/settings/cron_schedule_draft.rs`|2|2|2|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_cron|
|`crates/vw-desktop/src/app/state/settings/cron_templates.rs`|3|3|3|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_cron|
|`crates/vw-desktop/src/app/state/settings/infrastructure.rs`|78|44|42|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_agents, desktop_settings_appearance, desktop_settings_mcp, desktop_settings_memory, desktop_settings_network, desktop_settings_runtime, desktop_settings_security|
|`crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs`|31|31|0|旧文件与挂载删除；宏或领域断言去向见完整映射。|
|`crates/vw-desktop/src/app/state/settings/provider_model.rs`|42|40|38|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_cron, desktop_settings_providers, desktop_settings_runtime|
|`crates/vw-desktop/src/app/state/settings/provider_model_tests.rs`|24|24|0|旧文件与挂载删除；宏或领域断言去向见完整映射。|
|`crates/vw-desktop/src/app/state/settings/runtime.rs`|62|62|26|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_memory, desktop_settings_network, desktop_settings_runtime, desktop_settings_skills|
|`crates/vw-desktop/src/app/state/settings/runtime_tests.rs`|29|29|0|旧文件与挂载删除；宏或领域断言去向见完整映射。|
|`crates/vw-desktop/src/app/state/settings/worktrees.rs`|4|4|4|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_workspace|
|`crates/vw-desktop/src/app/state/settings.rs`|54|54|54|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_settings_shell|
|`crates/vw-desktop/src/app/state/settings_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/state/workspace.rs`|52|52|52|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_file_tree, desktop_git_ui, desktop_task_board, desktop_usage, desktop_workspace|
|`crates/vw-desktop/src/app/state/workspace_tests.rs`|9|9|9|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|
|`crates/vw-desktop/src/app/ui/chat.rs`|6|6|6|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：desktop_chat_view|
|`crates/vw-desktop/src/app/ui/mod.rs`|5|5|5|稳定兼容路径/父模块注册/跨域泛型实参绑定；无自有局部业务实现。领域：|
|`crates/vw-desktop/src/app/ui_types.rs`|115|115|115|Message为跨域启动/全局路由/剪贴板效果总线；FocusArea/SettingsTab/Screen等重导出navigation，DiffTheme重导出git-ui，ChatMarkdownSelection重导出chat-view。|
|`crates/vw-desktop/src/app/ui_types_tests.rs`|4|4|4|仅保留上表列明的模块注册或真实跨域契约；局部夹具已按断言映射迁出。|

归属文件合计：实施前2899行，当前1902行，宿主净减少997行。计划baseline合计2982行；此前其它任务已改变部分文件，因此不将其减量算作本任务。
按固定baseline production_candidate口径：实施前1063行，当前915行，候选净减少148行。该标记含注释/空行/测试注册，并非纯函数体行数；测试减量另由上表明确区分。

## 静态验收与以后验证入口

- 本次仅源码读取、路径/cfg/mod注册检查、逐断言映射、独立rustfmt解析及本任务diff --check；未运行Cargo/测试/绘制。
- 新领域模块无crate::app引用、无include!/path反向加载宿主；唯一path聚合指向本库测试文件。源码扫描include_str全部解析到现存文件。
- 保留三语言、双主题和真实renderer/layout/draw/screenshot源码；未新增中文MONOSPACE、控件风格、滚动条或布局行为。
- 现有工作区其它任务变更排除在本提交之外；共享lib只暂存本任务注册块，共享app_basic的render_input_error改接已随并行提交73030690b落盘，本次只提交对应领域实现并核对该调用链，不覆盖其余共享文件变化。

以下仅保存，须用户另行授权后执行，不能据此记录通过数：

- `python3 scripts/cargo_slot.py cargo test -p desktop-account-ui --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-chat-tools --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-editor --lib host_module_14_tests::host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-expert-hub --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-i18n --lib host_module_39_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-agents --lib host_module_16_tests::host_module_39_render -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-agents --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-appearance --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-channels --lib host_module_39_render -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-mcp --lib host_module_39_infrastructure_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-mcp --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-network --lib host_module_21_tests::host_module_39_render -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-network --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-providers --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-providers --lib host_module_39_provider_model_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-runtime --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-runtime --lib host_module_39_runtime_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-skills --lib host_module_23_tests::host_module_39_render -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-skill-hub --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib host_module_39_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-task-model --lib host_module_39_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-ui-controls --lib host_module_39_locale_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-workflow-ui --lib host_module_39_locale_tests -- --test-threads=1`
