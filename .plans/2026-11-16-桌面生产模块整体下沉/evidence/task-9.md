# 任务9 静态交付证据

本轮未执行任何 Cargo 命令、单元测试、实机绘制或回归；以下是源码审查证据，不能视为编译或行为验证通过。

## 生产归属与真实调用链

`project::layout::build_base_layout` 只提取 App 中跨域内容及字段，调用 `desktop_workspace::views::project::layout::code_workspace`，后者拥有主区容器、工具叠层、侧栏/悬浮覆盖层、圆角及底部组合。展开侧栏时仍不创建悬浮会话内容。工作模式继续接入既有 `layout::work::build_work_layout`。

`layout::work_sidebar::view` → 领域 `current_project_card` / `view`；当前项目名称回退、菜单、刷新按钮与局部事件均由领域构建。宿主只映射局部事件和提供运行时状态、已知目录、聊天压缩状态。`task_button` 与宿主映射契约使用同一 `session_context_menu`，不再保留测试专用右键/覆盖层算法。

过滤、分页、置顶排序、草稿行、项目行、拖拽、布局与暗黑样式仍复用既有 views/project 实现，没有平行算法。状态仍复用既有导航/workspace 类型与 App 持有的领域模型缓存；不新增配置、状态机或反向 App 依赖。已有局部消息按现有 map 分支装配至宿主 Project/View/Chat/TaskBoard/SkillHub 等系统。本次无行为或权限变更。

## 断言归属

下表逐用例记录迁移，断言和事件坐标保持原值；仅将 App 初始化/字段输入改为局部领域夹具，消息匹配改为对应局部消息。中文项目网格保留字体加载、Light/Dark、UserInterface.update/draw/screenshot 和点击/悬停坐标。不删除或去重旧断言；先前 desktop_split_19 与领域相邻测试保持独立，它们覆盖尺寸边界、样式、图标等，未用文件名推定等价。

| 原用例 | 新归属 | 断言数 |
| --- | --- | --- |
| `code_projects_rail_renders_titles_and_opens_the_four_column_grid` | `host_module_9_tests::projects_grid::code_projects_rail_renders_titles_and_opens_the_four_column_grid` | 7 |
| `session_items_build_loading_empty_and_load_button_states` | `host_module_9_tests::sessions_panel::session_items_build_loading_empty_and_load_button_states` | 0 |
| `session_tree_projects_only_main_tasks_regardless_of_expansion` | `host_module_9_tests::sessions_panel::session_tree_projects_only_main_tasks_regardless_of_expansion` | 5 |
| `session_items_filters_archived_and_shows_load_more` | `host_module_9_tests::sessions_panel::session_items_filters_archived_and_shows_load_more` | 0 |
| `session_items_reports_empty_when_only_archived_sessions_exist` | `host_module_9_tests::sessions_panel::session_items_reports_empty_when_only_archived_sessions_exist` | 0 |
| `session_items_builds_missing_summary_menu_default_anchor_and_no_load_more` | `host_module_9_tests::sessions_panel::session_items_builds_missing_summary_menu_default_anchor_and_no_load_more` | 0 |
| `project_panel_builds_empty_and_selected_project_states` | `host_module_9_tests::sessions_panel::project_panel_builds_empty_and_selected_project_states` | 0 |
| `project_panel_uses_path_fallback_for_unknown_or_blank_recent_names` | `host_module_9_tests::sessions_panel::project_panel_uses_path_fallback_for_unknown_or_blank_recent_names` | 0 |
| `work_sidebar_hides_archived_task_and_shows_restored_task` | `host_module_9_tests::sidebar_model::work_sidebar_hides_archived_task_and_shows_restored_task` | 2 |
| `work_sidebar_shows_visible_task_in_own_project` | `host_module_9_tests::sidebar_model::work_sidebar_shows_visible_task_in_own_project` | 3 |
| `work_sidebar_builds_workbuddy_sections_from_real_projects` | `host_module_9_tests::sidebar_model::work_sidebar_builds_workbuddy_sections_from_real_projects` | 20 |
| `work_sidebar_paginates_tasks_and_spaces_from_ten_in_steps_of_five` | `host_module_9_tests::sidebar_model::work_sidebar_paginates_tasks_and_spaces_from_ten_in_steps_of_five` | 6 |
| `work_sidebar_paginates_expanded_space_tasks_from_twenty` | `host_module_9_tests::sidebar_model::work_sidebar_paginates_expanded_space_tasks_from_twenty` | 7 |
| `work_sidebar_filters_sessions_by_project_and_update_time` | `host_module_9_tests::sidebar_model::work_sidebar_filters_sessions_by_project_and_update_time` | 6 |
| `test_module_is_wired` | `host_module_9_tests::sidebar_tests::test_module_is_wired` | 1 |
| `sidebar_group_expanded_builds_resize_handle` | `host_module_9_tests::sidebar_tests::sidebar_group_expanded_builds_resize_handle` | 0 |
| `sidebar_group_collapsed_hides_resize_handle` | `host_module_9_tests::sidebar_tests::sidebar_group_collapsed_hides_resize_handle` | 1 |
| `hover_overlay_layout_without_hover_uses_plain_sidebar` | `host_module_9_tests::sidebar_tests::hover_overlay_layout_without_hover_uses_plain_sidebar` | 0 |
| `hover_overlay_layout_with_hover_builds_sessions_overlay` | `host_module_9_tests::sidebar_tests::hover_overlay_layout_with_hover_builds_sessions_overlay` | 0 |
| `hover_overlay_layout_with_hover_clamps_negative_panel_width` | `host_module_9_tests::sidebar_tests::hover_overlay_layout_with_hover_clamps_negative_panel_width` | 0 |
| `sidebar_group_collapsed_empty_projects` | `host_module_9_tests::sidebar_tests::sidebar_group_collapsed_empty_projects` | 1 |
| `hover_overlay_layout_with_hover_zero_session_panel_scale` | `host_module_9_tests::sidebar_tests::hover_overlay_layout_with_hover_zero_session_panel_scale` | 0 |
| `hover_overlay_layout_with_hover_zero_spacing` | `host_module_9_tests::sidebar_tests::hover_overlay_layout_with_hover_zero_spacing` | 0 |
| `hover_overlay_layout_without_hover_zero_spacing` | `host_module_9_tests::sidebar_tests::hover_overlay_layout_without_hover_zero_spacing` | 0 |
| `hover_overlay_layout_with_hover_equal_panel_and_rail_widths` | `host_module_9_tests::sidebar_tests::hover_overlay_layout_with_hover_equal_panel_and_rail_widths` | 0 |
| `work_sidebar_replaces_search_row_with_compact_header` | `host_module_9_tests::sidebar_view::work_sidebar_replaces_search_row_with_compact_header` | 3 |
| `work_sidebar_uses_resizable_width_from_app_state` | `host_module_9_tests::sidebar_view::work_sidebar_uses_resizable_width_from_app_state` | 1 |

## 宿主残留逐文件职责审计

生产函数保留原因如下（re-export 与 mod 注册仅维持真实调用入口）：

- `components/projects_list.rs`：`map`。从 SessionRuntimeState 请求/队列/未读状态投影注意力输入，并映射项目/应用中心跨域事件。
- `components/session_row.rs`：`status_for_session`, `running_draft_sessions`, `row_input`, `session_row`。请求/ACP 绑定、会话预览、领域元数据与时钟投影；把领域行事件映射至宿主路由。
- `components/sessions_panel.rs`：`map`, `input`, `project_sessions_panel`。跨域请求状态与草稿投影、宿主事件映射；测试适配器仅装配相同领域实现。
- `components/new_session.rs`：`input`, `map`, `button_input`。工作树删除/重置错误由宿主本地化，局部事件接至 Project 操作。
- `layout.rs`：`build_base_layout`, `drag_badge_layer`。提取聊天、文件树、工具叠层和终端系统资源，交给领域 code_workspace 组合。
- `layout/bottom.rs`：`bottom_panel`, `fullscreen_terminal`。终端真实内容及拖动消息装配。
- `layout/chat.rs`：`chat_area`。真实聊天、输入面板及皮肤资源装配。
- `layout/chrome.rs`：`right_column_chrome`。宿主 WorkspaceMode 到领域布尔布局参数的适配。
- `layout/drag.rs`：`drag_badge_layer`。文件树拖动状态及顶栏/页签高度装配。
- `layout/main.rs`：`main_area`。文件树、预览与聊天内容及跨域分割拖动消息装配。
- `layout/sessions_panel.rs`：`project_sessions_panel_container`。实际项目会话内容装配至领域容器。
- `layout/sidebar.rs`：`workspace_rail`, `sidebar_group`。设置、外部会话路由与项目内容装配；尺寸与覆盖层算法均归领域。
- `layout/work.rs`：`build_work_layout`。聊天/资源/工具叠层/终端真实内容和跨域拖动事件装配。
- `layout/work_resources.rs`：`body_kind`, `mode`, `overview_model`, `event`, `view`。Git 仓库探测、浏览器、文件树、预览真实子域内容装配。
- `layout/work_sidebar.rs`：`map`, `cached_model`, `model`, `input`, `view`, `refresh_all_message`, `project_navigation_message`, `project_create_message`, `refresh_all_control`, `task_pool_control`, `expert_hub_button`, `shiju_apps_button`, `shiju_skills_button`, `task_session_context_menu`。运行时请求/队列状态、缓存生命周期、已知目录及压缩状态投影；局部消息映射。

宿主测试保留用例逐项清单；保留的是使用真实 App 的跨域装配契约，领域夹具不反向借用 App：

- `projects_list_tests.rs::projects_list_builds_empty_recent_projects`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `projects_list_tests.rs::projects_list_builds_with_hover_meta_and_project_attention`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `projects_list_tests.rs::projects_list_falls_back_to_current_sessions_for_attention`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `projects_list_tests.rs::projects_list_builds_with_queue_attention`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `projects_list_tests.rs::projects_list_uses_empty_path_when_edit_has_no_matching_recent_project`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `projects_list_tests.rs::projects_list_builds_without_attention_when_runtime_state_is_missing`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `projects_list_tests.rs::projects_list_does_not_use_current_sessions_for_unselected_project`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `projects_list_tests.rs::projects_list_ignores_recent_project_meta_for_other_paths`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `session_row_tests.rs::desktop_first_response_chinese_preview_and_title_draw_in_both_themes`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `session_row_tests.rs::running_draft_title_uses_first_non_empty_input_line`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `sessions_panel_tests.rs::session_items_builds_runtime_status_variants`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `sessions_panel_tests.rs::plan_run_live_children_integrate_with_session_visibility`：宿主运行态/预览/最近项目元数据/规划执行状态投影和路由。
- `bottom_tests.rs::bottom_panel_builds_terminal_when_visible`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_resources_tests.rs::work_resource_panel_routes_overview_workspace_files_and_browser`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_resources_tests.rs::work_resource_files_reuse_existing_file_manager_without_mutating_code_flags`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_resources_tests.rs::work_resource_panel_draws_all_modes_in_dark_and_light_themes`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::set_thread_pinned_round_trip_and_project_order`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_shows_running_draft_until_history_is_available`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_projects_runtime_status_for_main_tasks_only`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_task_context_menu_routes_session_id_and_pointer_position`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::shared_session_row_draws_right_details_and_routes_hover_actions`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::shared_session_row_renders_pin_without_hover_in_both_themes`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_navigation_routes_project_actions`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_filters_sessions_by_runtime_status`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_routes_shiju_entries_and_expert_hub_separately`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_refresh_message_deduplicates_recent_projects`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_refreshes_sessions_for_every_space`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_current_project_above_new_task_has_refresh_and_tools_buttons`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_task_pool_control_reuses_code_mode_entry`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_collapsed_keeps_session_list_and_one_new_task`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_draws_dark_chinese_without_monospace_panic`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::work_sidebar_projection_reuses_scroll_snapshot_and_updates_after_session_change`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::browser_and_request_polling_reuse_sidebar_without_hiding_metadata_updates`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_sidebar_tests.rs::sidebar_navigation_hover_and_menus_reuse_session_projection`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_layout_keeps_single_chat_surface_when_code_panels_are_enabled`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_layout_renders_matching_backgrounds_in_dark_and_light_themes`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_layout_uses_fixed_work_sidebar_when_code_sidebar_state_changes`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_resource_panel_split_ratio_and_fullscreen_have_independent_layouts`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_resource_fullscreen_hides_sidebar_and_restores_right_half`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_half_fullscreen_keeps_sidebar_and_selects_one_surface`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_layout_chat_fullscreen_hides_sidebar_and_resources`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_layout_hidden_resources_leave_chat_as_the_only_right_surface`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::work_layout_drag_handles_emit_sidebar_and_chat_split_messages`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::terminal_modes_layout_allocates_one_bottom_panel_in_both_themes_and_modes`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::terminal_modes_work_fullscreen_temporarily_hides_terminal_and_restores_layout`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `work_tests.rs::terminal_fullscreen_toolbar_round_trips_workspace_layout`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::test_module_is_wired`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::base_layout_builds_with_settings_sidebar_and_bottom_panel`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::base_layout_places_terminal_across_the_full_bottom_edge`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::base_layout_builds_without_settings_sidebar_or_bottom_panel`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::base_layout_builds_hovered_recent_project_overlay`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::base_layout_clamps_sidebar_overlay_width_inputs`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::base_layout_builds_chat_fullscreen_without_bottom_panel`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::base_layout_builds_git_fullscreen_without_bottom_panel`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::base_layout_builds_with_tool_detail_overlay`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::work_layout_code_builder_still_accepts_code_panel_state`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::drag_badge_layer_builds_empty_layer`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。
- `layout_tests.rs::drag_badge_layer_builds_active_badge`：真实聊天/终端/工具/文件/资源内容的装配和消息映射。

## 基线同口径统计

pathlib.read_text().splitlines()，含空行和注释；按 baseline.files[].task == 9，生产候选仍用固定布尔分类。新增宿主替代文件：无。

来源总量 3884 → 3152，净减少 732 行；生产候选 683 → 629，净减少 54 行。测试减少单列，不冒充生产减少。领域新增夹具只包含对应 Input 所需字段，不复制 App/AppState。

| 来源 | 基线 | 当前 |
| --- | ---: | ---: |
| `crates/vw-desktop/src/app/views/project/components/mod.rs` | 21 | 21 |
| `crates/vw-desktop/src/app/views/project/components/new_session.rs` | 41 | 41 |
| `crates/vw-desktop/src/app/views/project/components/projects_list.rs` | 39 | 39 |
| `crates/vw-desktop/src/app/views/project/components/projects_list_tests.rs` | 318 | 204 |
| `crates/vw-desktop/src/app/views/project/components/session_row.rs` | 80 | 80 |
| `crates/vw-desktop/src/app/views/project/components/session_row_tests.rs` | 97 | 97 |
| `crates/vw-desktop/src/app/views/project/components/sessions_panel.rs` | 58 | 58 |
| `crates/vw-desktop/src/app/views/project/components/sessions_panel_tests.rs` | 392 | 282 |
| `crates/vw-desktop/src/app/views/project/handles.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/views/project/layout/bottom.rs` | 7 | 7 |
| `crates/vw-desktop/src/app/views/project/layout/bottom_tests.rs` | 9 | 9 |
| `crates/vw-desktop/src/app/views/project/layout/chat.rs` | 6 | 6 |
| `crates/vw-desktop/src/app/views/project/layout/chrome.rs` | 5 | 5 |
| `crates/vw-desktop/src/app/views/project/layout/drag.rs` | 8 | 8 |
| `crates/vw-desktop/src/app/views/project/layout/main.rs` | 15 | 15 |
| `crates/vw-desktop/src/app/views/project/layout/sessions_panel.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/views/project/layout/sidebar.rs` | 14 | 12 |
| `crates/vw-desktop/src/app/views/project/layout/sidebar_tests.rs` | 157 | 0 |
| `crates/vw-desktop/src/app/views/project/layout/work.rs` | 13 | 13 |
| `crates/vw-desktop/src/app/views/project/layout/work_resources.rs` | 24 | 24 |
| `crates/vw-desktop/src/app/views/project/layout/work_resources_tests.rs` | 112 | 112 |
| `crates/vw-desktop/src/app/views/project/layout/work_sidebar.rs` | 208 | 162 |
| `crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs` | 1350 | 1053 |
| `crates/vw-desktop/src/app/views/project/layout/work_tests.rs` | 564 | 564 |
| `crates/vw-desktop/src/app/views/project/layout.rs` | 135 | 129 |
| `crates/vw-desktop/src/app/views/project/layout_tests.rs` | 202 | 202 |
| `crates/vw-desktop/src/app/views/project/styles.rs` | 3 | 3 |
| `crates/vw-desktop/src/app/views/project/utils.rs` | 1 | 1 |

## 静态检查与未验证风险

已审查 lib.rs 的 cfg(test) 注册、旧 sidebar_tests 挂载移除、迁移用例旧位置删除、App → domain → 局部事件 → App 消息的调用链。领域无宿主 path/include/App 引用。新增 iced_runtime 仅为保留原 UserInterface 绘制源码的 dev-dependency，Cargo.lock 仅增加 desktop-workspace 对已存在包的引用；不改根清单。修改文件执行独立 rustfmt（skip_children），并执行 git diff --check。

以后只有用户另行授权才可执行：`python3 scripts/cargo_slot.py cargo test -p desktop-workspace --lib host_module_9_tests -- --test-threads=1`；宿主 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。宿主保留契约仍按实际完整模块过滤执行。未验证：Rust 类型检查、链接、字体整形/真实绘制、事件命中及系统资源初始化。无成功通过数声明。
