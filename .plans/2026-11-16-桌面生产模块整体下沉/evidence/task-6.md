# 任务6 静态交付进展（未完成）

本次未编译、未运行测试、未运行绘制。任务卡保留，不能据此启动整体完成判定。

## 已落实生产接线

- `chat_panel::view → desktop_chat_view::panel::conversation_window`：局部借用输入负责虚拟窗口切片、cache回退、metadata、修订键、heavy可见性、MeasuredChatMessage及滚动容器。宿主闭包装配消息snapshot并映射滚动事件。
- `chat_panel_context_menu_overlay → panel::context_menu_overlay`：真实单层菜单布局移入领域库。
- `input::apply_chat_message_measurements → scroll_update::apply_targeted_message_measurements`：代际/修订/身份校验及索引投影移入领域；宿主同步兼容字段。
- `tools::render_shared_tool_view → desktop_chat_tools::tools::tool_renderer::render_shared_tool_view`：原分派回退顺序移入库，宿主逐种装配既有跨域视图。
- App原本已经持有领域库的virtualizer/cache/height index类型；本次没有重复状态或反向依赖。

## 已迁测试逐用例映射

所有下列用例保留原名、断言、中文/主题/事件与真实draw/screenshot源码；输入由局部fixture构造，消息渲染共享任务5的RenderFixture，不复制App。
- 原 `global_status_tests.rs::thinking_pulse_shapes_all_animation_frames_with_fixed_layout` → `host_module_6_tests::global_status_tests::thinking_pulse_shapes_all_animation_frames_with_fixed_layout`。
- 原 `global_status_tests.rs::global_status_spinner_rotates_at_text_height` → `host_module_6_tests::global_status_tests::global_status_spinner_rotates_at_text_height`。
- 原 `global_status_tests.rs::global_status_view_builds_for_hidden_and_visible_states` → `host_module_6_tests::global_status_tests::global_status_view_builds_for_hidden_and_visible_states`。
- 原 `global_status_tests.rs::global_status_detail_fills_remaining_width` → `host_module_6_tests::global_status_tests::global_status_detail_fills_remaining_width`。
- 原 `global_status_tests.rs::global_status_keeps_height_during_request_state_transitions` → `host_module_6_tests::global_status_tests::global_status_keeps_height_during_request_state_transitions`。
- 原 `header_tests.rs::active_header_app` → `host_module_6_tests::header_tests::active_header_app`。
- 原 `header_tests.rs::half_fullscreen_control_is_available_in_code_and_work_modes` → `host_module_6_tests::header_tests::half_fullscreen_control_is_available_in_code_and_work_modes`。
- 原 `header_tests.rs::active_session_directory_returns_none_without_matching_session` → `host_module_6_tests::header_tests::active_session_directory_returns_none_without_matching_session`。
- 原 `header_tests.rs::active_session_directory_returns_active_session_directory` → `host_module_6_tests::header_tests::active_session_directory_returns_active_session_directory`。
- 原 `header_tests.rs::workspace_badge_state_labels_primary_named_and_fallback_workspaces` → `host_module_6_tests::header_tests::workspace_badge_state_labels_primary_named_and_fallback_workspaces`。
- 原 `header_tests.rs::workspace_badge_element_builds_only_when_active_session_has_directory` → `host_module_6_tests::header_tests::workspace_badge_element_builds_only_when_active_session_has_directory`。
- 原 `header_tests.rs::session_status_defaults_when_no_active_session` → `host_module_6_tests::header_tests::session_status_defaults_when_no_active_session`。
- 原 `header_tests.rs::session_status_defaults_runtime_flags_when_active_session_has_no_runtime_state` → `host_module_6_tests::header_tests::session_status_defaults_runtime_flags_when_active_session_has_no_runtime_state`。
- 原 `header_tests.rs::session_status_reads_runtime_flags_queue_and_animation_color` → `host_module_6_tests::header_tests::session_status_reads_runtime_flags_queue_and_animation_color`。
- 原 `header_tests.rs::session_actions_elements_build_for_empty_and_active_session` → `host_module_6_tests::header_tests::session_actions_elements_build_for_empty_and_active_session`。
- 原 `header_tests.rs::session_actions_popover_contains_compact_and_existing_actions` → `host_module_6_tests::header_tests::session_actions_popover_contains_compact_and_existing_actions`。
- 原 `header_tests.rs::header_icon_button_builds_tooltip_button` → `host_module_6_tests::header_tests::header_icon_button_builds_tooltip_button`。
- 原 `header_tests.rs::chat_header_view_builds_default_session_variants` → `host_module_6_tests::header_tests::chat_header_view_builds_default_session_variants`。
- 原 `header_tests.rs::chat_header_places_mode_control_before_title` → `host_module_6_tests::header_tests::chat_header_places_mode_control_before_title`。
- 原 `header_tests.rs::chat_header_view_keeps_container_shape_across_fullscreen_modes` → `host_module_6_tests::header_tests::chat_header_view_keeps_container_shape_across_fullscreen_modes`。
- 原 `header_tests.rs::return_to_parent_session_pressed_is_none_without_parent` → `host_module_6_tests::header_tests::return_to_parent_session_pressed_is_none_without_parent`。
- 原 `header_tests.rs::return_to_parent_session_pressed_emits_open_parent_when_parent_id_present` → `host_module_6_tests::header_tests::return_to_parent_session_pressed_emits_open_parent_when_parent_id_present`。
- 原 `header_tests.rs::return_to_parent_session_pressed_uses_project_sessions_cache_when_needed` → `host_module_6_tests::header_tests::return_to_parent_session_pressed_uses_project_sessions_cache_when_needed`。
- 原 `header_tests.rs::return_to_parent_session_pressed_ignores_blank_parent_id` → `host_module_6_tests::header_tests::return_to_parent_session_pressed_ignores_blank_parent_id`。
- 原 `header_tests.rs::chat_header_view_builds_with_return_to_parent_control` → `host_module_6_tests::header_tests::chat_header_view_builds_with_return_to_parent_control`。
- 原 `header_tests.rs::header_session_menu_exposes_compact_current_session_action` → `host_module_6_tests::header_tests::header_session_menu_exposes_compact_current_session_action`。
- 原 `layout_tests.rs::chat_message_column_fills_available_width` → `host_module_6_tests::layout_tests::chat_message_column_fills_available_width`。
- 原 `layout_tests.rs::conversation_shell_uses_shared_reading_column_width` → `host_module_6_tests::layout_tests::conversation_shell_uses_shared_reading_column_width`。
- 原 `layout_tests.rs::message_column_bounds` → `host_module_6_tests::layout_tests::message_column_bounds`。
- 原 `layout_tests.rs::virtual_content_layout_matches_height_index_at_all_scroll_positions` → `host_module_6_tests::layout_tests::virtual_content_layout_matches_height_index_at_all_scroll_positions`。
- 原 `message_measure_tests.rs::chat_message` → `host_module_6_tests::message_measure_tests::chat_message`。
- 原 `message_measure_tests.rs::chat_message_measure_ignores_unchanged_heights` → `host_module_6_tests::message_measure_tests::chat_message_measure_ignores_unchanged_heights`。
- 原 `message_measure_tests.rs::chat_message_measure_updates_changed_heights_without_touching_copy_caches` → `host_module_6_tests::message_measure_tests::chat_message_measure_updates_changed_heights_without_touching_copy_caches`。
- 原 `message_view_tests.rs::user_message_falls_back_to_source_text_while_selectable_editor_is_empty` → `host_module_6_tests::message_view_tests::user_message_falls_back_to_source_text_while_selectable_editor_is_empty`。
- 原 `message_view_tests.rs::selectable_user_message_fills_available_width` → `host_module_6_tests::message_view_tests::selectable_user_message_fills_available_width`。
- 原 `message_view_tests.rs::message_visual_contract_keeps_assistant_flat_and_user_full_width` → `host_module_6_tests::message_view_tests::message_visual_contract_keeps_assistant_flat_and_user_full_width`。
- 原 `message_view_tests.rs::message_visual_contract_renders_complex_content_in_light_and_dark_themes` → `host_module_6_tests::message_view_tests::message_visual_contract_renders_complex_content_in_light_and_dark_themes`。
- 原 `message_view_tests.rs::message_view_builds_user_message_with_metadata_and_attachment` → `host_module_6_tests::message_view_tests::message_view_builds_user_message_with_metadata_and_attachment`。
- 原 `message_view_tests.rs::message_view_builds_assistant_special_blocks_without_metadata` → `host_module_6_tests::message_view_tests::message_view_builds_assistant_special_blocks_without_metadata`。
- 原 `message_view_tests.rs::message_view_builds_tool_role_with_metadata` → `host_module_6_tests::message_view_tests::message_view_builds_tool_role_with_metadata`。
- 原 `performance_tests.rs::chat_performance_acceptance_long_session_is_bounded` → `host_module_6_tests::performance_tests::chat_performance_acceptance_long_session_is_bounded`。
- 原 `performance_tests.rs::chat_performance_acceptance_stream_is_incremental` → `host_module_6_tests::performance_tests::chat_performance_acceptance_stream_is_incremental`。
- 原 `performance_tests.rs::chat_performance_acceptance_persistence_is_incremental` → `host_module_6_tests::performance_tests::chat_performance_acceptance_persistence_is_incremental`。
- 原 `performance_tests.rs::chat_performance_acceptance_real_renderer_preserves_content` → `host_module_6_tests::performance_tests::chat_performance_acceptance_real_renderer_preserves_content`。
- 原 `selectable_text_tests.rs::selectable_text_policy_uses_preview_for_unsafe_text` → `host_module_6_tests::selectable_text_tests::selectable_text_policy_uses_preview_for_unsafe_text`。
- 原 `selectable_text_tests.rs::selectable_text_reads_all_aux_editors` → `host_module_6_tests::selectable_text_tests::selectable_text_reads_all_aux_editors`。
- 原 `selectable_text_tests.rs::selectable_text_reads_markdown_block_editor_selection` → `host_module_6_tests::selectable_text_tests::selectable_text_reads_markdown_block_editor_selection`。

`header_tests::return_to_parent_session_pressed_emits_open_parent_when_parent_id_present` 的真实project update/open断言仍在宿主；库测试保留父会话导航结果断言。性能中涉及prepare_session_open与原生probe的整合断言仍在宿主，未删除。

## 未完成项（不可标记任务完成）

- 工具卡片 advanced/apply_patch/text/tool_renderer 的领域渲染断言仍需从宿主完整迁入领域库，并保留真实diff/dialog注入。
- tool_selector_tests 尚需拆分局部SelectorInput与宿主runtime/catalog消息映射；不可把当前整个App夹具视为最终薄契约。
- mod_tests 仍包含较大App视觉场景夹具，必须继续拆出领域draw/screenshot与局部状态测试；目前只迁出三个纯布局用例。
- empty_tests、utils/app_state_tests及其宿主装配依赖需要逐断言核对，不应仅以重导出判断整体完成。
- 仍需逐函数核对全部来源残留职责与前轮host_split断言去重，当前清单仅记录文件现状，不能代替该审计。

## baseline口径统计

本任务尚无新建宿主文件。下表使用pathlib.read_text().splitlines()；基线到当前包含前置任务对本区域的变化，不能全归因本次提交。
- 基线总计 4449；当前 3228；宿主净减少 1221 行。
- 生产候选基线 1274；当前 1224。

| 来源 | 基线 | 当前 |
|---|---:|---:|
| `crates/vw-desktop/src/app/components/chat_panel/chunk_loader.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/chunk_loader_tests.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/empty.rs` | 30 | 30 |
| `crates/vw-desktop/src/app/components/chat_panel/empty_tests.rs` | 33 | 33 |
| `crates/vw-desktop/src/app/components/chat_panel/global_status.rs` | 12 | 12 |
| `crates/vw-desktop/src/app/components/chat_panel/global_status_tests.rs` | 197 | 0 |
| `crates/vw-desktop/src/app/components/chat_panel/header.rs` | 81 | 81 |
| `crates/vw-desktop/src/app/components/chat_panel/header_tests.rs` | 355 | 55 |
| `crates/vw-desktop/src/app/components/chat_panel/height_index.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/message_measure.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/message_measure_tests.rs` | 68 | 0 |
| `crates/vw-desktop/src/app/components/chat_panel/message_view_tests.rs` | 322 | 0 |
| `crates/vw-desktop/src/app/components/chat_panel/mod.rs` | 393 | 361 |
| `crates/vw-desktop/src/app/components/chat_panel/mod_tests.rs` | 525 | 449 |
| `crates/vw-desktop/src/app/components/chat_panel/performance.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/performance_tests.rs` | 315 | 191 |
| `crates/vw-desktop/src/app/components/chat_panel/selectable_text.rs` | 11 | 11 |
| `crates/vw-desktop/src/app/components/chat_panel/selectable_text_tests.rs` | 84 | 0 |
| `crates/vw-desktop/src/app/components/chat_panel/tool_names.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/tool_selector.rs` | 33 | 33 |
| `crates/vw-desktop/src/app/components/chat_panel/tool_selector_tests.rs` | 381 | 381 |
| `crates/vw-desktop/src/app/components/chat_panel/tool_text_support.rs` | 32 | 32 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/advanced_view.rs` | 22 | 22 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/advanced_view_tests.rs` | 354 | 354 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_preview.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_view.rs` | 5 | 5 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_view_tests.rs` | 104 | 104 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/bash_view.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/card_adapter.rs` | 41 | 41 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/changes.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/command_summary_view.rs` | 5 | 5 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/config_view.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/diff_utils.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/explore_summary_view.rs` | 13 | 13 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/file_link.rs` | 8 | 8 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/files_view.rs` | 10 | 10 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/git_diff_view.rs` | 12 | 12 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/lsp_view.rs` | 7 | 7 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/mcp_view.rs` | 7 | 7 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/question_view.rs` | 7 | 7 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/read_view.rs` | 7 | 7 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/skill_view.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/text_view.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/text_view_tests.rs` | 59 | 59 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/todo_parse.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/todo_views.rs` | 8 | 8 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_detail_dialog.rs` | 26 | 26 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_meta.rs` | 19 | 19 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_parse.rs` | 32 | 32 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_permission.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_renderer.rs` | 82 | 64 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_renderer_tests.rs` | 305 | 305 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/types.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/web_view.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/workflow_view.rs` | 5 | 5 |
| `crates/vw-desktop/src/app/components/chat_panel/tools.rs` | 155 | 155 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/details.rs` | 16 | 16 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/details_content.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/json_tree.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/ledger.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/mod.rs` | 74 | 74 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/model.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/projection.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/styles.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/timeline.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/app_state.rs` | 6 | 6 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/app_state_tests.rs` | 72 | 72 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/mod.rs` | 43 | 43 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/path.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/text.rs` | 2 | 2 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/theme.rs` | 1 | 1 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/time.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/ui.rs` | 12 | 12 |
| `crates/vw-desktop/src/app/components/chat_panel/virtualizer.rs` | 2 | 2 |

## 验证证据

限定修改路径 `git diff --check` 无错误；修改文件独立rustfmt语法解析无错误（skip_children=true），不是Rust类型检查。没有Cargo或测试执行结论。
以后另行授权时：`python3 scripts/cargo_slot.py cargo test -p desktop-chat-view --lib host_module_6_tests -- --test-threads=1`；宿主目标另按计划统一检查。
