# 任务5：消息渲染静态交付

本轮未运行任何 Cargo 命令、测试、绘制或实机验证。以下为源码静态证据，不表示编译或行为通过。

## 生产入口与依赖

- `message_view::bubble` 拥有完整消息气泡编排、复制身份、右键消息身份、用户折叠、附件分支、权限徽标、耗时、消息表面及 footer。宿主 `message_view` / `message_view_with_runtime` 直接调用此入口。
- `assistant_render` 拥有完整思考/工具/探索/命令/正文编排及待授权工具高亮、分组与回退；复用已有 `assistant_body_sequence`，没有平行算法。
- `markdown_body` 拥有全部 Markdown 正文编排、单编辑器回退、选择感知块和表格绘制；`render_support` 组合现有 text、think_block、markdown_code 与错误卡入口，映射局部 Action。
- `workspace_links` 拥有索引路径可见性、绝对路径与点击效果；使用 desktop-file-tree 的真实 FileTreeNode。
- `parse::assistant_speech_text` 拥有语音正文提取；`recovery` 拥有重试通知文案、有效请求判定和绘制。
- `view_snapshot::MessageViewSnapshot` 重新导出借用快照。既有编辑器、缓存、权限 DTO、会话状态直接借用其真实局部类型；不复制 App，不引入 Deref、path/include 或反向依赖。
- 真实调用链：chat panel 行渲染 → 宿主薄入口 → bubble → assistant_render → render_support → 既有领域组件。局部 Action 通过 snapshot.emit 映射回宿主 Message。
- 工具卡/命令摘要/探索摘要/工具编辑器由已有工具领域拥有，窄 ToolViews 端口调用真实入口；附件端口仍调用 desktop-chat-input 的真实解析与预览。desktop-chat-input 已依赖本库，因此禁止从本库反向依赖它；没有新增循环。
- 新增依赖仅为实际使用的 desktop-file-tree、directories 6、gateway-chat-stream 及测试用 iced_runtime；已有锁文件仅补本领域依赖边。

## 宿主逐项残留

| 文件/函数或模块 | 保留原因 |
| --- | --- |
| message_view.rs 的 message_view、message_view_with_runtime、chat_message_action_text | 构造局部快照并转发；前两者读取当前运行时生命周期作为输入 |
| message_view.rs 的 recovery_notice_text、recovery_notice_view | 取当前运行时请求与恢复事件并传入领域入口；无本地判定/绘制算法 |
| message_view.rs 的 should_use_user_message_editor（cfg test） | 邻接任务现有宿主契约的类型适配，直接调用领域函数 |
| snapshot.rs::snapshot | 借用宿主持有的领域状态，组合跨域工具、附件和链接端口；Fork 映射读取宿主会话锚点 |
| snapshot.rs::map_text/map_think | 将局部 UI 事件映射到宿主已有消息 |
| markdown_links.rs::WorkspaceLinks::new | 读取导航项目、会话目录和文件树域缓存，装配真实局部输入；其余方法和 map 仅映射跨域打开效果 |
| markdown_selection.rs::SelectionMessage impl | 原生上下文菜单/复制/选区事件映射到宿主路由 |
| parse.rs、render_cache.rs、styles.rs 和主文件 re-export | 稳定既有直接调用者路径，仅重导出领域类型/函数 |
| host_contract_tests.rs | 附件跨域装配契约、选区到宿主剪贴板快捷键的路由契约；无领域绘制夹具 |

原 assistant_body、assistant_error、markdown、markdown_code、markdown_rich、text、think_block、tool_summaries 源文件及所有旧测试挂载已移除。旧 styles 的思考状态包装与 render_cache 的 App 参数适配已移除；测试性能计数器随真实缓存入口归库。

## 断言映射

整组测试注册于 lib.rs 的 `host_module_5_tests`。RenderFixture 仅持有聊天局部状态与 ChatUi 所需数据；完成文本同步调用真实 ChatUi，选区更新调用真实 update_markdown_selection。工具卡使用真实 desktop-chat-tools 实现；不实现分组、解析、选择或绘制的测试算法。原生剪贴板完成/过期作为 fixture 边界反馈，截图和点击测试验证真实领域控件。附件测试继续位于宿主边界，避免伪造附件解析。

旧六组测试共 74 处 assert/eq/ne 宏；73 处留在领域测试，选区快捷键输出的 1 处位于宿主契约。原附件构造无 assert，原样留作宿主装配契约。中文、字体加载、Light/Dark、真实 draw、screenshot、鼠标布局与事件循环均保留源码，未执行。

| 旧测试 | 新归属 |
| --- | --- |
| assistant_body_tests::visible_non_explore_tool_count_skips_hidden_duplicates_and_post_explore_file_lists | desktop-chat-view::host_module_5_tests::assistant_body_tests::visible_non_explore_tool_count_skips_hidden_duplicates_and_post_explore_file_lists |
| assistant_body_tests::visible_text_closes_pending_explore_group_before_file_list_tool | desktop-chat-view::host_module_5_tests::assistant_body_tests::visible_text_closes_pending_explore_group_before_file_list_tool |
| assistant_body_tests::think_block_closes_pending_explore_group_before_file_list_tool_count | desktop-chat-view::host_module_5_tests::assistant_body_tests::think_block_closes_pending_explore_group_before_file_list_tool_count |
| assistant_body_tests::cached_tool_metadata_drives_dedup_and_count_without_reparsing_raw_blocks | desktop-chat-view::host_module_5_tests::assistant_body_tests::cached_tool_metadata_drives_dedup_and_count_without_reparsing_raw_blocks |
| assistant_body_tests::render_assistant_body_builds_plain_text_body | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_builds_plain_text_body |
| assistant_body_tests::completed_assistant_body_prefers_markdown_layout | desktop-chat-view::host_module_5_tests::assistant_body_tests::completed_assistant_body_prefers_markdown_layout |
| assistant_body_tests::special_text_block_prefers_markdown_layout | desktop-chat-view::host_module_5_tests::assistant_body_tests::special_text_block_prefers_markdown_layout |
| assistant_body_tests::render_assistant_body_builds_api_error_card_for_special_text_block | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_builds_api_error_card_for_special_text_block |
| assistant_body_tests::streaming_body_keeps_text_above_explore_as_thinking_resumes | desktop-chat-view::host_module_5_tests::assistant_body_tests::streaming_body_keeps_text_above_explore_as_thinking_resumes |
| assistant_body_tests::render_assistant_body_handles_special_text_think_and_tool_blocks | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_handles_special_text_think_and_tool_blocks |
| assistant_body_tests::render_assistant_body_groups_explore_tools_before_visible_text | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_groups_explore_tools_before_visible_text |
| assistant_body_tests::render_assistant_body_groups_adjacent_commands_into_one_top_level_summary | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_groups_adjacent_commands_into_one_top_level_summary |
| assistant_body_tests::render_assistant_body_keeps_commands_grouped_across_hidden_reasoning | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_keeps_commands_grouped_across_hidden_reasoning |
| assistant_body_tests::render_assistant_body_flushes_command_groups_at_visible_boundaries | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_flushes_command_groups_at_visible_boundaries |
| assistant_body_tests::render_assistant_body_keeps_pending_command_outside_summary_group | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_keeps_pending_command_outside_summary_group |
| assistant_body_tests::render_assistant_body_builds_api_error_card_without_special_blocks | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_builds_api_error_card_without_special_blocks |
| assistant_body_tests::render_assistant_body_falls_back_for_unknown_tool_block | desktop-chat-view::host_module_5_tests::assistant_body_tests::render_assistant_body_falls_back_for_unknown_tool_block |
| assistant_body_tests::streaming_special_text_ignores_stale_editor | desktop-chat-view::host_module_5_tests::assistant_body_tests::streaming_special_text_ignores_stale_editor |
| assistant_body_tests::streaming_body_keeps_completed_text_and_think_editors_visible | desktop-chat-view::host_module_5_tests::assistant_body_tests::streaming_body_keeps_completed_text_and_think_editors_visible |
| assistant_error_tests::assistant_api_error_view_rejects_invalid_or_unrelated_json | desktop-chat-view::host_module_5_tests::assistant_error_tests::assistant_api_error_view_rejects_invalid_or_unrelated_json |
| assistant_error_tests::assistant_api_error_view_accepts_retryable_error_with_url | desktop-chat-view::host_module_5_tests::assistant_error_tests::assistant_api_error_view_accepts_retryable_error_with_url |
| assistant_error_tests::assistant_api_error_view_accepts_non_retryable_special_text_error | desktop-chat-view::host_module_5_tests::assistant_error_tests::assistant_api_error_view_accepts_non_retryable_special_text_error |
| assistant_error_tests::assistant_api_error_view_accepts_error_without_retryability_hint | desktop-chat-view::host_module_5_tests::assistant_error_tests::assistant_api_error_view_accepts_error_without_retryability_hint |
| markdown_tests::markdown_visual_contract_preserves_readable_block_hierarchy | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_visual_contract_preserves_readable_block_hierarchy |
| markdown_tests::formatted_markdown_uses_one_editor_for_cross_line_selection | desktop-chat-view::host_module_5_tests::markdown_tests::formatted_markdown_uses_one_editor_for_cross_line_selection |
| markdown_tests::link_click_maps_to_external_open | desktop-chat-view::host_module_5_tests::markdown_tests::link_click_maps_to_external_open |
| markdown_tests::markdown_body_view_builds_every_block_kind_with_editor_and_text_fallbacks | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_body_view_builds_every_block_kind_with_editor_and_text_fallbacks |
| markdown_tests::table_link_renders_and_emits_open_message_in_both_themes | desktop-chat-view::host_module_5_tests::markdown_tests::table_link_renders_and_emits_open_message_in_both_themes |
| markdown_tests::markdown_code_copy_renders_feedback_and_restores_in_both_themes | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_code_copy_renders_feedback_and_restores_in_both_themes |
| markdown_tests::markdown_table_fills_available_width_and_overflows_only_when_needed | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_table_fills_available_width_and_overflows_only_when_needed |
| markdown_tests::markdown_keeps_code_fragments_and_identifiers_unlinked | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_keeps_code_fragments_and_identifiers_unlinked |
| markdown_tests::markdown_auto_links_indexed_paths_and_keeps_explicit_links | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_auto_links_indexed_paths_and_keeps_explicit_links |
| markdown_tests::markdown_renders_padded_inline_code_in_both_themes | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_renders_padded_inline_code_in_both_themes |
| markdown_tests::markdown_file_link_click_resolves_the_workspace_path | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_file_link_click_resolves_the_workspace_path |
| markdown_tests::markdown_code_payload_preserves_shorter_fences_and_internal_whitespace | desktop-chat-view::host_module_5_tests::markdown_tests::markdown_code_payload_preserves_shorter_fences_and_internal_whitespace |
| message_view_tests::message_view_does_not_hash_full_content | desktop-chat-view::host_module_5_tests::message_view_tests::message_view_does_not_hash_full_content |
| message_view_tests::message_view_actions_carry_identity_without_message_text | desktop-chat-view::host_module_5_tests::message_view_tests::message_view_actions_carry_identity_without_message_text |
| message_view_tests::message_view_builds_completed_processing_label | desktop-chat-view::host_module_5_tests::message_view_tests::message_view_builds_completed_processing_label |
| message_view_tests::message_view_hides_open_processing_label | desktop-chat-view::host_module_5_tests::message_view_tests::message_view_hides_open_processing_label |
| message_view_tests::message_view_builds_user_message_with_meta_and_branch_actions | desktop-chat-view::host_module_5_tests::message_view_tests::message_view_builds_user_message_with_meta_and_branch_actions |
| message_view_tests::message_view_builds_attachment_card_for_user_message | 宿主 host_contract_tests::message_view_builds_attachment_card_for_user_message |
| message_view_tests::message_view_builds_tool_role_as_assistant_like_body | desktop-chat-view::host_module_5_tests::message_view_tests::message_view_builds_tool_role_as_assistant_like_body |
| message_view_tests::message_view_builds_streaming_assistant_from_last_chat_message | desktop-chat-view::host_module_5_tests::message_view_tests::message_view_builds_streaming_assistant_from_last_chat_message |
| think_block_tests::think_block_view_builds_collapsed_and_empty_expanded_states | desktop-chat-view::host_module_5_tests::think_block_tests::think_block_view_builds_collapsed_and_empty_expanded_states |
| think_block_tests::think_block_view_uses_editor_for_completed_short_content | desktop-chat-view::host_module_5_tests::think_block_tests::think_block_view_uses_editor_for_completed_short_content |
| think_block_tests::think_block_view_uses_text_fallback_for_completed_short_content_without_editor | desktop-chat-view::host_module_5_tests::think_block_tests::think_block_view_uses_text_fallback_for_completed_short_content_without_editor |
| think_block_tests::think_block_view_uses_segmented_body_for_long_completed_content | desktop-chat-view::host_module_5_tests::think_block_tests::think_block_view_uses_segmented_body_for_long_completed_content |
| think_block_tests::think_block_view_builds_running_body | desktop-chat-view::host_module_5_tests::think_block_tests::think_block_view_builds_running_body |
| think_block_tests::think_block_running_header_builds_chinese_across_animation_frames | desktop-chat-view::host_module_5_tests::think_block_tests::think_block_running_header_builds_chinese_across_animation_frames |
| markdown_selection_tests::markdown_drag_selects_wrapped_text_table_cells_and_code_in_both_directions | desktop-chat-view::host_module_5_tests::markdown_selection_tests::markdown_drag_selects_wrapped_text_table_cells_and_code_in_both_directions；快捷键输出断言另见宿主 markdown_selected_text_routes_to_clipboard |
| markdown_selection_tests::markdown_link_click_without_drag_still_opens_the_target | desktop-chat-view::host_module_5_tests::markdown_selection_tests::markdown_link_click_without_drag_still_opens_the_target |

与既有 desktop_split_05_tests 的对照：既有 message_policies 覆盖的是解析/分组策略；本次完整迁移实际 widget 构造、树形、布局和交互绘制证据，测试名无重叠，不删除策略断言。没有在宿主保留同套领域夹具。

## 行数（Python pathlib.splitlines）

基线来源 3050 行；现存来源 216 行；本任务新建宿主文件 191 行；真实宿主净减 2643 行。生产候选基线 1536 行；现存候选 216 行；新增宿主生产 143 行；生产净减 1177 行。新增宿主测试单独计入总量，不当作生产减少。

| 来源 | 基线行数 | 当前行数 | 归属 |
| --- | ---: | ---: | --- |
| assistant_body.rs | 400 | 0 | 领域库；旧实现删除 |
| assistant_body_tests.rs | 524 | 0 | 领域库；旧实现删除 |
| assistant_error.rs | 39 | 0 | 领域库；旧实现删除 |
| assistant_error_tests.rs | 51 | 0 | 领域库；旧实现删除 |
| markdown.rs | 120 | 0 | 领域库；旧实现删除 |
| markdown_code.rs | 9 | 0 | 领域库；旧实现删除 |
| markdown_links.rs | 96 | 43 | 宿主装配/重导出 |
| markdown_rich.rs | 77 | 0 | 领域库；旧实现删除 |
| markdown_selection.rs | 25 | 22 | 宿主装配/重导出 |
| markdown_selection_tests.rs | 151 | 0 | 领域库；旧实现删除 |
| markdown_tests.rs | 469 | 0 | 领域库；旧实现删除 |
| message_view_tests.rs | 221 | 0 | 领域库；旧实现删除 |
| parse.rs | 37 | 1 | 宿主装配/重导出 |
| render_cache.rs | 30 | 4 | 宿主装配/重导出 |
| styles.rs | 5 | 1 | 宿主装配/重导出 |
| text.rs | 23 | 0 | 领域库；旧实现删除 |
| think_block.rs | 41 | 0 | 领域库；旧实现删除 |
| think_block_tests.rs | 98 | 0 | 领域库；旧实现删除 |
| tool_summaries.rs | 1 | 0 | 领域库；旧实现删除 |
| message_view.rs | 633 | 145 | 宿主装配/重导出 |

## 静态检查与后续授权验证

仅运行独立 rustfmt 与限定本任务路径 git diff --check；检查了所有 mod/cfg 注册、直接调用者及依赖方向。全工作区存在其它任务变更，未纳入本提交。本轮没有构建/测试结论，生命周期、类型检查、原生字体整形、截图及实际鼠标交互仍未验证。

以后仅在用户授权时：`python3 scripts/cargo_slot.py cargo test -p desktop-chat-view --lib host_module_5_tests -- --test-threads=1`；宿主跨域契约使用 `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::components::chat_panel::message_view::host_contract_tests -- --test-threads=1`；宿主目标检查使用 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。本轮严禁执行这些命令。
