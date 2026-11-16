# 任务38：部分实施证据

尚未完成，保留任务卡。静态交付不表示编译或行为通过。

## 已实施调用链

- top_bar::view → desktop_navigation::components::top_bar::toolbar(Sections) → view；领域固定11分区顺序，宿主仅投影数据和映射不同子域消息。
- with_account_details → desktop_account_ui::account_details::with_account_details → details/settings_modal_overlay；关闭消息和弹窗包装属于账户领域。
- overlays 通过一次公开重导出到 desktop-ui-controls，移除6个重复转发子文件；无 path/include 反向引用。
- Git浮层6个原构造用例迁到desktop-git-ui，使用已有Fixture/ViewContext及真实评论/丢弃视图。
- 权限弹窗6个原用例整组迁到desktop-chat-view，仅持有请求与队列；滚动测试仍走真实Iced布局、缓存与Operation路径。原用例没有消息断言，统一映射到单位类型保留原测试语义。
- LSP展示4个原用例迁到desktop-navigation，使用State、ServerProgress和真实lsp_tab_content；宿主保留管理插件的跨设置消息映射契约。

## 未完成的设计缺口

- about_modal_ui_refresh_41_tests、toast_ui_refresh_33_tests、chat_fork_ui_refresh_32_tests 混合多个领域的视图、App::update、隔离子进程和截图夹具，仍需拆成领域交互与最小宿主效果契约；本次原样保留，不能称为必要最小宿主边界。
- install_cli_modal_tests、task_pet_tests、top_bar_tests、project_edit/general_tab_tests 的领域绘制与状态夹具仍依赖App，需要迁入各实际所属库；不能以已有库用例代替逐断言迁移。
- question/rename/聚合模块的哨兵仍在宿主，需核实对应既有迁移覆盖再移除冗余挂载。
- 当前不删除任务卡，不标待回归；缺口来自尚未完成的夹具和接口拆分，与禁测无关。

## 逐文件归属与行数（splitlines）

| 原路径 | 固定基线 | 本次前 | 当前 | 去向/残留 |
|---|---:|---:|---:|---|
| `crates/vw-desktop/src/app/components/about_modal.rs` | 13 | 13 | 13 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs` | 584 | 584 | 584 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/components/color_picker/mod.rs` | 46 | 46 | 46 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/components/install_cli_modal.rs` | 88 | 88 | 88 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs` | 173 | 173 | 173 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/components/install_web_modal.rs` | 10 | 10 | 10 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/components/model_hover_tooltip.rs` | 5 | 5 | 5 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/components/notification.rs` | 15 | 15 | 15 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/components/overlays/above/mod.rs` | 1 | 1 | 0 | desktop-ui-controls::overlays；宿主仅公开兼容路径 |
| `crates/vw-desktop/src/app/components/overlays/below.rs` | 1 | 1 | 0 | desktop-ui-controls::overlays；宿主仅公开兼容路径 |
| `crates/vw-desktop/src/app/components/overlays/inline_right.rs` | 1 | 1 | 0 | desktop-ui-controls::overlays；宿主仅公开兼容路径 |
| `crates/vw-desktop/src/app/components/overlays/left.rs` | 1 | 1 | 0 | desktop-ui-controls::overlays；宿主仅公开兼容路径 |
| `crates/vw-desktop/src/app/components/overlays/mod.rs` | 28 | 28 | 2 | desktop-ui-controls::overlays；宿主仅公开兼容路径 |
| `crates/vw-desktop/src/app/components/overlays/point_below.rs` | 1 | 1 | 0 | desktop-ui-controls::overlays；宿主仅公开兼容路径 |
| `crates/vw-desktop/src/app/components/overlays/side.rs` | 1 | 1 | 0 | desktop-ui-controls::overlays；宿主仅公开兼容路径 |
| `crates/vw-desktop/src/app/components/task_pet.rs` | 52 | 52 | 52 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/components/task_pet_tests.rs` | 230 | 230 | 230 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/components/toast.rs` | 31 | 31 | 31 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/components/toast_tests.rs` | 35 | 35 | 35 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs` | 706 | 706 | 706 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/components/top_bar/controls.rs` | 13 | 13 | 13 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar/drag.rs` | 3 | 3 | 3 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar/external.rs` | 6 | 6 | 6 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar/gateway.rs` | 24 | 24 | 22 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs` | 122 | 122 | 38 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar/mapping.rs` | 51 | 51 | 51 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar/menu_actions.rs` | 1 | 1 | 1 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar/menus.rs` | 59 | 59 | 59 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar/workspace_mode.rs` | 12 | 12 | 12 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar.rs` | 34 | 34 | 48 | desktop-navigation；保留跨域消息/资源投影；gateway四个展示测试迁出，其余宿主测试待继续拆分 |
| `crates/vw-desktop/src/app/components/top_bar_tests.rs` | 77 | 77 | 77 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/impls/app_view_modals/account_details.rs` | 8 | 8 | 13 | 账户弹窗包装迁入desktop-account-ui；宿主仅输入投影与消息映射 |
| `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork.rs` | 18 | 18 | 18 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs` | 1240 | 1240 | 1240 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/impls/app_view_modals/chat_reset.rs` | 11 | 11 | 11 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/file_tree_delete.rs` | 11 | 11 | 11 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/git_diff.rs` | 13 | 13 | 9 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs` | 69 | 69 | 0 | 迁入 desktop-git-ui/src/host_module_38_tests.rs，原测试名称及断言保留 |
| `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal.rs` | 18 | 18 | 14 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs` | 175 | 175 | 0 | 迁入 desktop-chat-view/src/host_module_38_tests.rs，原测试名称及断言保留 |
| `crates/vw-desktop/src/app/impls/app_view_modals/project_edit/general_tab.rs` | 3 | 3 | 3 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/project_edit/general_tab_tests.rs` | 62 | 62 | 62 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/impls/app_view_modals/project_edit/mod.rs` | 92 | 92 | 92 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/question_modal.rs` | 17 | 17 | 17 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs` | 6 | 6 | 6 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/impls/app_view_modals/recent_project_remove.rs` | 11 | 11 | 11 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/rename.rs` | 16 | 16 | 16 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals/rename_tests.rs` | 6 | 6 | 6 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |
| `crates/vw-desktop/src/app/impls/app_view_modals.rs` | 35 | 35 | 35 | 已有领域调用边界保留；仍须完成与其关联的宿主测试拆分后复核最小性 |
| `crates/vw-desktop/src/app/impls/app_view_modals_tests.rs` | 8 | 8 | 8 | 仍在宿主，未完成领域夹具拆分/断言归属核对 |

本任务独占来源：本次前 4243 行，当前 3892 行，本次宿主净减少 351 行。固定基线包含前序任务已迁出实现，不将其算成本次贡献。

## 全部原测试入口映射（未删除断言）

- `crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs::ui_refresh_41_layout_and_interaction` → `crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs::ui_refresh_41_layout_and_interaction（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs::ui_refresh_41_preserves_values_and_disabled_states` → `crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs::ui_refresh_41_preserves_values_and_disabled_states（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs::app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes` → `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs::app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs::settings_tabs_peripheral_appearance_cli_commands` → `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs::settings_tabs_peripheral_appearance_cli_commands（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs::update_modals_hide_explanations_and_draw_hover_help_in_both_themes` → `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs::update_modals_hide_explanations_and_draw_hover_help_in_both_themes（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/task_pet_tests.rs::mini_companion_toolbar_actions_do_not_start_drag` → `crates/vw-desktop/src/app/components/task_pet_tests.rs::mini_companion_toolbar_actions_do_not_start_drag（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/task_pet_tests.rs::mini_companion_none_renders_toolbar` → `crates/vw-desktop/src/app/components/task_pet_tests.rs::mini_companion_none_renders_toolbar（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/task_pet_tests.rs::mini_companion_stack_has_bounded_depth` → `crates/vw-desktop/src/app/components/task_pet_tests.rs::mini_companion_stack_has_bounded_depth（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/task_pet_tests.rs::mini_companion_expanded_cards_keep_actions_and_scroll` → `crates/vw-desktop/src/app/components/task_pet_tests.rs::mini_companion_expanded_cards_keep_actions_and_scroll（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/toast_tests.rs::cache_sync_pending_retry_exposes_retry_action_in_toast` → `crates/vw-desktop/src/app/components/toast_tests.rs::cache_sync_pending_retry_exposes_retry_action_in_toast（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs::toast_text_wraps_as_needed_and_leaves_room_for_retry` → `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs::toast_text_wraps_as_needed_and_leaves_room_for_retry（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs::ui_refresh_33_layout_and_interaction` → `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs::ui_refresh_33_layout_and_interaction（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs::ui_refresh_33_confirmation_preserves_messages` → `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs::ui_refresh_33_confirmation_preserves_messages（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs::ui_refresh_33_preserves_values_and_disabled_states` → `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs::ui_refresh_33_preserves_values_and_disabled_states（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::manage_plugins_opens_the_standalone_agent_plugin_tab` → `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::manage_plugins_opens_the_standalone_agent_plugin_tab（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::lsp_tab_content_reports_disabled_state_first` → `crates/libs/desktop-navigation/src/host_module_38_tests.rs::lsp_tab_content_reports_disabled_state_first`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::lsp_tab_content_reports_empty_status_when_no_servers_exist` → `crates/libs/desktop-navigation/src/host_module_38_tests.rs::lsp_tab_content_reports_empty_status_when_no_servers_exist`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::lsp_tab_content_deduplicates_keys_and_appends_status` → `crates/libs/desktop-navigation/src/host_module_38_tests.rs::lsp_tab_content_deduplicates_keys_and_appends_status`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::lsp_tab_content_formats_progress_detail` → `crates/libs/desktop-navigation/src/host_module_38_tests.rs::lsp_tab_content_formats_progress_detail`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::gateway_tests_keep_public_view_messages_reachable` → `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::gateway_tests_keep_public_view_messages_reachable（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar_tests.rs::view_builds_single_row_with_all_toolbar_sections` → `crates/vw-desktop/src/app/components/top_bar_tests.rs::view_builds_single_row_with_all_toolbar_sections（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar_tests.rs::work_top_bar_keeps_all_global_menus_and_project_tools` → `crates/vw-desktop/src/app/components/top_bar_tests.rs::work_top_bar_keeps_all_global_menus_and_project_tools（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/components/top_bar_tests.rs::workspace_mode_selector_keeps_code_controls` → `crates/vw-desktop/src/app/components/top_bar_tests.rs::workspace_mode_selector_keeps_code_controls（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::fork_desktop_clicks_send_frozen_turn_and_open_gateway_history` → `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::fork_desktop_clicks_send_frozen_turn_and_open_gateway_history（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::fork_desktop_layout_and_interaction` → `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::fork_desktop_layout_and_interaction（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_preserves_values_and_disabled_states` → `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_preserves_values_and_disabled_states（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_escape_cancels_only_the_visible_modal` → `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_escape_cancels_only_the_visible_modal（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::file_tree_delete_confirmation_draws_and_routes_actions` → `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::file_tree_delete_confirmation_draws_and_routes_actions（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_invalid_file_name_feedback_keeps_inline_editor` → `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_invalid_file_name_feedback_keeps_inline_editor（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::todo_continuation_status_draws_centered_in_both_themes` → `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::todo_continuation_status_draws_centered_in_both_themes（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::fork_selection_reopen_gateway_reads_worktree_authority` → `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::fork_selection_reopen_gateway_reads_worktree_authority（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs::with_git_diff_comment_returns_root_when_comment_modal_is_absent` → `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_comment_returns_root_when_comment_modal_is_absent`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs::with_git_diff_comment_layers_modal_when_comment_draft_exists` → `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_comment_layers_modal_when_comment_draft_exists`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs::with_git_diff_overlays_builds_empty_overlay_layer_when_modals_are_absent` → `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_overlays_builds_empty_overlay_layer_when_modals_are_absent`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs::with_git_diff_overlays_builds_comment_overlay_without_discard_modal` → `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_overlays_builds_comment_overlay_without_discard_modal`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs::with_git_diff_overlays_builds_discard_overlay_without_comment_modal` → `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_overlays_builds_discard_overlay_without_comment_modal`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs::with_git_diff_overlays_layers_comment_and_discard_modals_together` → `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_overlays_layers_comment_and_discard_modals_together`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs::permission_and_question_modals_preserve_scroll_position` → `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::permission_and_question_modals_preserve_scroll_position`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs::with_permission_modal_builds_single_request_dialog_when_queue_is_empty` → `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_builds_single_request_dialog_when_queue_is_empty`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs::with_permission_modal_builds_single_and_multi_request_dialogs` → `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_builds_single_and_multi_request_dialogs`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs::with_permission_modal_builds_navigation_for_middle_and_last_requests` → `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_builds_navigation_for_middle_and_last_requests`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs::with_permission_modal_falls_back_to_first_position_for_request_outside_queue` → `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_falls_back_to_first_position_for_request_outside_queue`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs::with_permission_modal_limits_long_pending_request_list` → `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_limits_long_pending_request_list`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/project_edit/general_tab_tests.rs::project_edit_general_form_controls_render_and_update_draft` → `crates/vw-desktop/src/app/impls/app_view_modals/project_edit/general_tab_tests.rs::project_edit_general_form_controls_render_and_update_draft（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs::question_modal_tests_module_is_wired` → `crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs::question_modal_tests_module_is_wired（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals/rename_tests.rs::rename_tests_module_is_wired` → `crates/vw-desktop/src/app/impls/app_view_modals/rename_tests.rs::rename_tests_module_is_wired（宿主保留，未完成最小边界审计）`；原断言原样保留。
- `crates/vw-desktop/src/app/impls/app_view_modals_tests.rs::app_view_modals_tests_module_is_wired` → `crates/vw-desktop/src/app/impls/app_view_modals_tests.rs::app_view_modals_tests_module_is_wired（宿主保留，未完成最小边界审计）`；原断言原样保留。

## 静态验证与以后命令

- git diff --check 已执行；未执行 Cargo、测试、截图或实机。
- 新测试模块均由各库 lib.rs 的 cfg(test) 正向注册，无宿主源码引用。
- 以下仅保存，须用户另行授权后执行：
  - `python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_38_tests -- --test-threads=1`
  - `python3 scripts/cargo_slot.py cargo test -p desktop-git-ui --lib host_module_38_tests -- --test-threads=1`
  - `python3 scripts/cargo_slot.py cargo test -p desktop-chat-view --lib host_module_38_tests -- --test-threads=1`
