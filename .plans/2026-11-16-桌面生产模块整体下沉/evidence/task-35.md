# 任务35：账号认证与用量面板

静态交付；未编译、未测试、未运行实机或回归。

## 真实调用链与领域归属

- account_status → desktop-account-ui::view；account::update → desktop-account-ui::update → desktop-account 仓库与凭据。宿主服务入口仅提供配置目录、网关与跨域效果。
- apps_usage_cards::{update,read_linked_providers,scope} → desktop-usage::card_update。Cards（含设置编辑器）、作用域校验、绑定身份检查、票据和代际过滤、单项/批量请求、联动同步均由领域持有。宿主 Input 只借用项目、认证及供应商局部数据，不传 App。
- usage::center::update → desktop-usage::center::update。筛选、范围、Tick/Close、刷新及子面板分派均由领域处理。宿主只选择身份、屏幕可见性、网关工厂及反馈展示。删除原子面板 update/load/reset/activity 冗余入口。
- 删除 apps_usage_details 空挂载；保留其渲染夹具供 file_tree 跨域组合测试使用。
- account UI 渲染夹具使用 AccountState、真实 details/menu/quota 与统一模态覆盖层；应用生命周期夹具使用真实 AccountRepository/ShijuAppsState，仅网关端口离线。没有复制 App 或引用宿主源码。

## 静态检查

- 对新增文件和修改入口运行独立 rustfmt（skip_children=true）；未执行 Cargo。
- 新依赖 desktop-usage → desktop-provider-apps 使用已有泛型状态；按清单路径静态追踪，无返回 desktop-usage 的依赖路径。
- 领域新增模块未出现 crate::app、path/include 加载宿主。原账户 UI 和应用生命周期测试旧挂载已移除；新模块均以 cfg(test) 注册。
- 本任务路径 git diff --check 已检查；工作区并行任务的其它路径检查结果不归本任务。

## 逐文件与函数残留

| 原路径 | baseline / 当前 splitlines | 去向及必要宿主边界 |
|---|---:|---|
| `crates/vw-desktop/src/app/account/mod.rs` | 38 / 38 | desktop-account 仓库重导出；load/save/store_path 仅装配系统配置目录；函数：load_repository, save_repository, store_path |
| `crates/vw-desktop/src/app/components/account_status.rs` | 35 / 28 | 已有领域视图入口；只保留宿主当前会话投影和全局消息/导航组合；函数：map, trigger_label, view, monthly_quota |
| `crates/vw-desktop/src/app/components/account_status_ui_refresh_31_tests.rs` | 794 / 0 | 已删除；实现/断言迁领域，旧挂载清除 |
| `crates/vw-desktop/src/app/components/apps_usage_cards/batch.rs` | 69 / 0 | 已删除；实现/断言迁领域，旧挂载清除 |
| `crates/vw-desktop/src/app/components/apps_usage_cards.rs` | 257 / 116 | desktop-usage::card_update；仅 Input 投影、Element/Task 全局消息映射及 toast 装配；函数：map_view, provider_view, account_view, input, run, update, read_linked_providers, message_event, scope, current_scope |
| `crates/vw-desktop/src/app/components/apps_usage_cards_tests.rs` | 40 / 40 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：gateway, request, respond |
| `crates/vw-desktop/src/app/components/apps_usage_details.rs` | 1 / 0 | 已删除；实现/断言迁领域，旧挂载清除 |
| `crates/vw-desktop/src/app/components/apps_usage_details_tests.rs` | 66 / 66 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：traverse, text, new, bounds, send, renderer |
| `crates/vw-desktop/src/app/components/apps_usage_settings.rs` | 28 / 28 | desktop-usage；供应商选择及设置模态跨域装配、反馈映射；函数：send, map, update, overlay |
| `crates/vw-desktop/src/app/components/system_settings_auth/binding_modal_tests.rs` | 226 / 226 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：prepare, drive, serve, gateway, binding_case, auth6_ui_binding_commits_expected_scoped_result, auth6_ui_binding_rejects_invalid_or_stale_input |
| `crates/vw-desktop/src/app/components/system_settings_auth.rs` | 89 / 89 | desktop-settings-account；认证与用量跨域投影，映射宿主认证/Usage 消息；函数：state, project, view, overlay, map |
| `crates/vw-desktop/src/app/message/account.rs` | 89 / 89 | 领域状态借用与服务端口；模型、技能、toast 跨域效果匹配；函数：services, load_repository, update, model_validation_feedback |
| `crates/vw-desktop/src/app/message/account_shiju_skill.rs` | 71 / 71 | 账户完成结果 → expert_hub 安装状态/通知，属于真实跨域效果；函数：start, finished |
| `crates/vw-desktop/src/app/message/account_tests.rs` | 411 / 378 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：repository_with_account, repository_with_profile, native_shiju_skill_restored_account_schedules_preparation, native_shiju_skill_anonymous_restore_still_schedules_detection, native_shiju_skill_login_during_preparation_retries_after_stale_result, account_me, account_messages_use_gateway_login_and_validation_tasks, account_profile_edit_messages_use_gateway_and_ignore_late_results, account_password_edit_validates_redacts_and_ignores_late_results, account_change_invalidates_skill_hub_state, successful_login_and_switch_schedule_shiju_key_sync, sign_out_and_delete_do_not_sync_shiju_key, shiju_key_sync_failure_keeps_account_active, monthly_quota_ignores_old_account_results_and_allows_retry |
| `crates/vw-desktop/src/app/message/shiju_apps.rs` | 89 / 89 | desktop-apps-ui / desktop-app-launch；仅标签页、系统 Child 所有权、helper 路径和通知效果；函数：update, open_shiju_apps_tab, launch, set_launch_error |
| `crates/vw-desktop/src/app/message/shiju_apps_tests.rs` | 480 / 43 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：repository, shiju_apps_open_loads_mine_through_gateway |
| `crates/vw-desktop/src/app/shiju_apps/mod.rs` | 15 / 15 | desktop-apps-ui / desktop-app-launch；仅标签页、系统 Child 所有权、helper 路径和通知效果；函数：launch_shiju_app |
| `crates/vw-desktop/src/app/views/apps/modals.rs` | 63 / 63 | desktop-apps-ui；宿主富帮助、CopyCode 与全局书签消息/模态组合投影；函数：message, input, add_web_bookmark_modal, edit_web_bookmark_modal, active_modal |
| `crates/vw-desktop/src/app/views/apps/modals_tests.rs` | 409 / 409 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：new_app, active_modal_returns_none_without_modal_state, active_modal_prefers_edit_modal_over_add_modal, active_modal_returns_add_bookmark_close_message, add_web_bookmark_modal_builds_for_incomplete_input, add_web_bookmark_modal_builds_for_valid_input, edit_web_bookmark_modal_builds_for_selected_bookmark, edit_web_bookmark_modal_uses_safe_index_fallback_without_selection, run_in_fixture, help_rollout_04_bookmarks_render_contextual_help, traverse, text, settings_help_gap_18_cookie_example_pins_and_copies_static_text |
| `crates/vw-desktop/src/app/views/apps/tiles.rs` | 32 / 32 | desktop-apps-ui；宿主富帮助、CopyCode 与全局书签消息/模态组合投影；函数：map, render_tiles_grid, render_header |
| `crates/vw-desktop/src/app/views/apps/ui.rs` | 3 / 3 | desktop-apps-ui；宿主富帮助、CopyCode 与全局书签消息/模态组合投影 |
| `crates/vw-desktop/src/app/views/apps.rs` | 78 / 78 | desktop-apps-ui；宿主富帮助、CopyCode 与全局书签消息/模态组合投影；函数：view |
| `crates/vw-desktop/src/app/views/apps_tests.rs` | 40 / 40 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：root_child_count, view_without_active_modal_returns_scrollable_base_content, view_with_add_bookmark_modal_stacks_overlay_above_base_content, view_with_edit_bookmark_modal_stacks_overlay_above_base_content, view_with_both_modal_flags_keeps_single_modal_stack |
| `crates/vw-desktop/src/app/views/usage/center/charts.rs` | 13 / 12 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：map, view |
| `crates/vw-desktop/src/app/views/usage/center/display.rs` | 22 / 28 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：map, overlay, amount, status |
| `crates/vw-desktop/src/app/views/usage/center/groups.rs` | 13 / 11 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：map, view |
| `crates/vw-desktop/src/app/views/usage/center/logs.rs` | 17 / 11 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：map, view |
| `crates/vw-desktop/src/app/views/usage/center/mod.rs` | 80 / 63 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：identity, update, view |
| `crates/vw-desktop/src/app/views/usage/center/models.rs` | 13 / 10 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：message, overlay |
| `crates/vw-desktop/src/app/views/usage/center/overview.rs` | 2 / 2 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：view |
| `crates/vw-desktop/src/app/views/usage/center/pricing.rs` | 20 / 22 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：map, overlay |
| `crates/vw-desktop/src/app/views/usage/center/scan.rs` | 21 / 21 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配；函数：map, overlay |
| `crates/vw-desktop/src/app/views/usage/center/state.rs` | 1 / 1 | desktop-usage::center；仅 identity、状态重导出、子视图/overlay 全局消息映射和屏幕/反馈装配 |
| `crates/vw-desktop/src/app/views/usage/data.rs` | 12 / 12 | desktop-usage::data；从宿主活动会话及项目缓存选择元数据；函数：from_app |
| `crates/vw-desktop/src/app/views/usage/data_tests.rs` | 89 / 89 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：session_info, step, from_app_prefers_loaded_session_metadata_and_latest_step_usage, from_app_falls_back_to_project_metadata_for_active_session |
| `crates/vw-desktop/src/app/views/usage/layout.rs` | 3 / 3 | 已有领域视图入口；只保留宿主当前会话投影和全局消息/导航组合；函数：build_usage_view |
| `crates/vw-desktop/src/app/views/usage/mod.rs` | 108 / 108 | 已有领域视图入口；只保留宿主当前会话投影和全局消息/导航组合；函数：view, external_usage_entry, view_input, map_message |
| `crates/vw-desktop/src/app/views/usage/raw_modal.rs` | 3 / 3 | 已有领域视图入口；只保留宿主当前会话投影和全局消息/导航组合；函数：view |
| `crates/vw-desktop/src/app/views/usage/tests.rs` | 102 / 102 | 保留真实宿主组合契约：全局消息映射、通知/标签页、跨域结果及活动会话投影；未删除剩余断言；函数：child_count, usage_data_counts_tool_messages_separately, usage_view_constructs_from_app_state, usage_data_exposes_active_steps_and_raw_calls |

上述独占文件 baseline 3942 行，当前 2439 行。含测试和空行，不将总代码减少等同于生产减少。新增领域文件为 card_update、center/update 及两库 host_module_35_tests；测试单独 cfg 注册。

## 全部旧用例与断言映射

同名迁移均保留各断言及场景顺序；下表按实际旧源码逐用例统计 assert 宏数（不将动态迭代次数当作通过数）。账户 UI 的宿主消息 None 过滤删除，因为局部 Message 无 None；网关环境自检留给宿主服务测试。应用末例的 children.is_empty 改为“不产生 Launch 效果”，表达领域授权边界；真实 Child 所有权仍留宿主 launch。

| 原用例 | assert 宏数 | 当前归属 |
|---|---:|---|
| `ui_refresh_31_layout_and_interaction` | 18 | `desktop-account-ui::host_module_35_tests::ui_refresh_31_layout_and_interaction` |
| `ui_refresh_31_login_menu_preserves_account_actions` | 21 | `desktop-account-ui::host_module_35_tests::ui_refresh_31_login_menu_preserves_account_actions` |
| `ui_refresh_31_preserves_values_and_disabled_states` | 25 | `desktop-account-ui::host_module_35_tests::ui_refresh_31_preserves_values_and_disabled_states` |
| `monthly_quota_renders_ring_and_shows_details_on_hover` | 7 | `desktop-account-ui::host_module_35_tests::monthly_quota_renders_ring_and_shows_details_on_hover` |
| `auth6_ui_binding_commits_expected_scoped_result` | 0 | `crates/vw-desktop/src/app/components/system_settings_auth/binding_modal_tests.rs（宿主跨域契约）` |
| `auth6_ui_binding_rejects_invalid_or_stale_input` | 2 | `crates/vw-desktop/src/app/components/system_settings_auth/binding_modal_tests.rs（宿主跨域契约）` |
| `native_shiju_skill_restored_account_schedules_preparation` | 2 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `native_shiju_skill_anonymous_restore_still_schedules_detection` | 2 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `native_shiju_skill_login_during_preparation_retries_after_stale_result` | 6 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `account_messages_use_gateway_login_and_validation_tasks` | 0 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `account_profile_edit_messages_use_gateway_and_ignore_late_results` | 10 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `account_password_edit_validates_redacts_and_ignores_late_results` | 9 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `late_account_responses_do_not_replace_newer_operation` | 5 | `desktop-account-ui::host_module_35_tests::late_account_responses_do_not_replace_newer_operation` |
| `account_change_invalidates_skill_hub_state` | 3 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `successful_login_and_switch_schedule_shiju_key_sync` | 3 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `sign_out_and_delete_do_not_sync_shiju_key` | 2 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `shiju_key_sync_failure_keeps_account_active` | 4 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `monthly_quota_ignores_old_account_results_and_allows_retry` | 11 | `crates/vw-desktop/src/app/message/account_tests.rs（宿主跨域契约）` |
| `shiju_apps_open_loads_mine_through_gateway` | 7 | `crates/vw-desktop/src/app/message/shiju_apps_tests.rs（宿主跨域契约）` |
| `shiju_apps_tabs_ignore_stale_or_wrong_account_results` | 5 | `desktop-apps-ui::host_module_35_tests::shiju_apps_tabs_ignore_stale_or_wrong_account_results` |
| `open_shiju_app_checks_binding_before_launch` | 2 | `desktop-apps-ui::host_module_35_tests::open_shiju_app_checks_binding_before_launch` |
| `configured_binding_continues_existing_launch` | 1 | `desktop-apps-ui::host_module_35_tests::configured_binding_continues_existing_launch` |
| `unconfigured_binding_selects_first_enabled_key` | 5 | `desktop-apps-ui::host_module_35_tests::unconfigured_binding_selects_first_enabled_key` |
| `unconfigured_binding_rejects_read_only_or_missing_enabled_keys` | 5 | `desktop-apps-ui::host_module_35_tests::unconfigured_binding_rejects_read_only_or_missing_enabled_keys` |
| `binding_save_rejects_missing_or_disabled_selection` | 7 | `desktop-apps-ui::host_module_35_tests::binding_save_rejects_missing_or_disabled_selection` |
| `binding_save_success_continues_launch_and_ignores_stale_results` | 4 | `desktop-apps-ui::host_module_35_tests::binding_save_success_continues_launch_and_ignores_stale_results` |
| `binding_failures_preserve_selection_and_close_detail_cancels_stale_work` | 5 | `desktop-apps-ui::host_module_35_tests::binding_failures_preserve_selection_and_close_detail_cancels_stale_work` |
| `stale_or_wrong_account_launch_is_ignored` | 2 | `desktop-apps-ui::host_module_35_tests::stale_or_wrong_account_launch_is_ignored` |
| `active_modal_returns_none_without_modal_state` | 1 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `active_modal_prefers_edit_modal_over_add_modal` | 1 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `active_modal_returns_add_bookmark_close_message` | 1 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `add_web_bookmark_modal_builds_for_incomplete_input` | 1 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `add_web_bookmark_modal_builds_for_valid_input` | 1 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `edit_web_bookmark_modal_builds_for_selected_bookmark` | 1 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `edit_web_bookmark_modal_uses_safe_index_fallback_without_selection` | 2 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `help_rollout_04_bookmarks_render_contextual_help` | 2 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `settings_help_gap_18_cookie_example_pins_and_copies_static_text` | 21 | `crates/vw-desktop/src/app/views/apps/modals_tests.rs（宿主跨域契约）` |
| `view_without_active_modal_returns_scrollable_base_content` | 1 | `crates/vw-desktop/src/app/views/apps_tests.rs（宿主跨域契约）` |
| `view_with_add_bookmark_modal_stacks_overlay_above_base_content` | 1 | `crates/vw-desktop/src/app/views/apps_tests.rs（宿主跨域契约）` |
| `view_with_edit_bookmark_modal_stacks_overlay_above_base_content` | 1 | `crates/vw-desktop/src/app/views/apps_tests.rs（宿主跨域契约）` |
| `view_with_both_modal_flags_keeps_single_modal_stack` | 1 | `crates/vw-desktop/src/app/views/apps_tests.rs（宿主跨域契约）` |
| `from_app_prefers_loaded_session_metadata_and_latest_step_usage` | 4 | `crates/vw-desktop/src/app/views/usage/data_tests.rs（宿主跨域契约）` |
| `from_app_falls_back_to_project_metadata_for_active_session` | 2 | `crates/vw-desktop/src/app/views/usage/data_tests.rs（宿主跨域契约）` |
| `usage_data_counts_tool_messages_separately` | 5 | `crates/vw-desktop/src/app/views/usage/tests.rs（宿主跨域契约）` |
| `usage_view_constructs_from_app_state` | 1 | `crates/vw-desktop/src/app/views/usage/tests.rs（宿主跨域契约）` |
| `usage_data_exposes_active_steps_and_raw_calls` | 4 | `crates/vw-desktop/src/app/views/usage/tests.rs（宿主跨域契约）` |

## 以后用户明确要求时的验证命令（本轮未运行）

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-account-ui --lib host_module_35_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib host_module_35_tests -- --test-threads=1
```

风险：当前仅源代码/依赖/调用链审查，不能据此承诺编译成功、有效运行数、字体无 panic 或事件绘制通过。
