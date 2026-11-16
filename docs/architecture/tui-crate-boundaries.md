# TUI crate 边界与静态收尾

任务 20：生产入口仅保留公共重导出；测试入口保留历史插入、surface 合约和上游快照矩阵。未编译、未测试、未执行实机或跨平台验证，待用户统一授权验收。Cargo.lock 未改动。计划引用的首轮 plan.md 已不存在，本次依据当前源码和第二阶段公共规则执行。

## 责任与依赖方向

contracts 拥有 action/event/diff/presentation 纯数据；text/input/theme/render/highlight/terminal/clipboard 提供基础能力；markdown/stream/cells 提供文本与展示；widgets 提供通用控件；composer/pickers/popups/approval/user-input/bottom-pane 提供交互；transcript/exec-state/chat-state 提供独立状态；chat 拥有 ChatWidget 及固有实现；onboarding/pager/terminal-lifecycle/surface 拥有各自入口。vw-tui 只直接依赖公开导出所需的库，Markdown、ratatui、crossterm 仅用于集成测试。不存在责任包反向依赖 vw-tui。

## 本次迁移与唯一所有者

| 原路径（crates/vw-tui/src） | 唯一所有者（crates/libs） | 调用接线 |
| --- | --- | --- |
| goal_display.rs | tui-cells/src/goal_display.rs | V3StepStatus 直接使用 contracts；step 格式化使用 cells 内模块 |
| ui_consts.rs | tui-widgets/src/ui_consts.rs | 布局常量保持 crate 可见性 |
| git_action_directives.rs | tui-markdown/src/git_action_directives.rs | 解析器供同包 copy_targets 使用，保持私有范围 |
| markdown.rs | tui-markdown/src/copy_targets.rs | CopyTarget、extract_copy_targets 唯一所有者；既有测试同步迁移 |
| status_indicator_widget.rs（前置已迁出） | tui-chat/src/status_indicator_widget.rs | chatwidget/status_surfaces.rs 直接调用 |
| vibe/mod.rs、vibe/surface.rs | tui-surface/src/vibe.rs | 重导出 contracts 和 surface，保持 vw_tui::vibe 路径 |
| 其余底层/交互兼容入口 | 各 tui-* 所有者 | 根 lib.rs 直接重导出，不再编译旧门面 |

目标文案与布局常量当前无生产调用者，迁移不新增调用行为。Git 解析及复制目标提取保留原行为。展示函数没有放入 contracts。所有责任包已为 workspace 成员，无需修改根清单。

## 测试与资源

目标文案与 Git 解析原内联测试分别移至独立 goal_display_tests.rs、git_action_directives_tests.rs；Markdown 复制测试移至同包。Git 指令命名快照移至 tui-markdown/src/snapshots，矩阵已有 markdown 目录收集逻辑，无需改计数。原快照 source 元数据保持不变。历史插入保持 insert_history::tests 模块名与快照名，仅在 cfg(test) 接线；集成测试继续使用 vw-tui 测试后端，surface 测试直接导入 widgets；矩阵入口字符串按薄门面更新。其余既有快照与 fixture 保留原路径；未启用源码已在后续清理中删除，见下方历史清单。

## 静态证据

已逐项审阅根模块声明、生产依赖、跨包导入、cfg(test) 和资源路径；执行指定的全 src 引用搜索（crate/super/include/path 等）。大量匹配属于下列未启用旧源码，不意味着生产模块遗漏。git diff --check 通过；静态结果不代表编译或行为验证。

## 已清理的未激活 Rust 源码

2026-11-16 按维护者要求删除以下 248 个未进入编译图的 Rust 文件（106287 行）。保留当前生产重导出、5 个已接线源码文件、全部快照和 fixture；不修改 tui-* 实现。下列清单作为历史删除记录保留，不再代表现存文件。

删除前版本：`324e666a4e90fb562dadfa84d7a42dd53698fe38`。可用 `git show <版本>:<仓库相对路径>` 查看，或从该版本恢复所需文件并重新适配接线。

以下路径相对 crates/vw-tui/src；仅列源码，快照等资源保留：

- `app_event.rs`
- `app_event_sender.rs`
- `approval_events.rs`
- `ascii_animation.rs`
- `auto_review_denials.rs`
- `bin/md-events.rs`
- `bottom_pane/app_link_view.rs`
- `bottom_pane/apply_patch_header.rs`
- `bottom_pane/chat_composer/attachment_state.rs`
- `bottom_pane/chat_composer/completion_target.rs`
- `bottom_pane/chat_composer/completion_target_tests.rs`
- `bottom_pane/chat_composer/draft_state.rs`
- `bottom_pane/chat_composer/history_search.rs`
- `bottom_pane/chat_composer/popup_state.rs`
- `bottom_pane/chat_composer/popup_state_tests.rs`
- `bottom_pane/chat_composer/slash_input.rs`
- `bottom_pane/chat_composer_effort_tests.rs`
- `bottom_pane/chat_composer_history/search_batch.rs`
- `bottom_pane/chat_composer_history/search_batch_tests.rs`
- `bottom_pane/custom_prompt_view.rs`
- `bottom_pane/custom_prompt_view_tests.rs`
- `bottom_pane/effort_ignition.rs`
- `bottom_pane/effort_ignition_styles.rs`
- `bottom_pane/effort_ignition_tests.rs`
- `bottom_pane/effort_status_line.rs`
- `bottom_pane/effort_status_line_tests.rs`
- `bottom_pane/experimental_features_view.rs`
- `bottom_pane/feedback_view.rs`
- `bottom_pane/hooks_browser_view.rs`
- `bottom_pane/mcp_server_elicitation.rs`
- `bottom_pane/memories_settings_view.rs`
- `bottom_pane/mentions_v2/candidate.rs`
- `bottom_pane/mentions_v2/filter.rs`
- `bottom_pane/mentions_v2/footer.rs`
- `bottom_pane/mentions_v2/mod.rs`
- `bottom_pane/mentions_v2/popup.rs`
- `bottom_pane/mentions_v2/render.rs`
- `bottom_pane/mentions_v2/search_catalog.rs`
- `bottom_pane/mentions_v2/search_mode.rs`
- `bottom_pane/mod.rs`
- `bottom_pane/multi_select_picker.rs`
- `bottom_pane/pending_thread_approvals.rs`
- `bottom_pane/popup_consts.rs`
- `bottom_pane/prompt_args.rs`
- `bottom_pane/request_user_input/render_tests.rs`
- `bottom_pane/skill_popup.rs`
- `bottom_pane/skills_toggle_view.rs`
- `bottom_pane/startup.rs`
- `bottom_pane/status_line_setup.rs`
- `bottom_pane/status_line_style.rs`
- `bottom_pane/status_surface_preview.rs`
- `bottom_pane/title_setup.rs`
- `bottom_pane/unified_exec_footer.rs`
- `branch_summary.rs`
- `chatwidget/connector_mentions.rs`
- `chatwidget/connectors.rs`
- `chatwidget/goal_menu.rs`
- `chatwidget/goal_status.rs`
- `chatwidget/hook_lifecycle.rs`
- `chatwidget/hooks.rs`
- `chatwidget/ide_context.rs`
- `chatwidget/input_flow.rs`
- `chatwidget/input_restore.rs`
- `chatwidget/interaction.rs`
- `chatwidget/keymap_picker.rs`
- `chatwidget/mcp_startup.rs`
- `chatwidget/misalignment_policy.rs`
- `chatwidget/model_popups.rs`
- `chatwidget/notifications.rs`
- `chatwidget/permission_popups.rs`
- `chatwidget/permission_shortcuts.rs`
- `chatwidget/permissions_menu.rs`
- `chatwidget/pets.rs`
- `chatwidget/pets_tests.rs`
- `chatwidget/plan_implementation.rs`
- `chatwidget/plugin_catalog.rs`
- `chatwidget/plugins.rs`
- `chatwidget/protocol.rs`
- `chatwidget/protocol_requests.rs`
- `chatwidget/rate_limits.rs`
- `chatwidget/reasoning_shortcuts.rs`
- `chatwidget/replay.rs`
- `chatwidget/reset_credits.rs`
- `chatwidget/review.rs`
- `chatwidget/review_popups.rs`
- `chatwidget/safety_buffering.rs`
- `chatwidget/service_tiers.rs`
- `chatwidget/session_flow.rs`
- `chatwidget/session_header.rs`
- `chatwidget/settings.rs`
- `chatwidget/settings_popups.rs`
- `chatwidget/side.rs`
- `chatwidget/skills.rs`
- `chatwidget/slash_dispatch.rs`
- `chatwidget/status_controls.rs`
- `chatwidget/tests/app_server.rs`
- `chatwidget/tests/approval_requests.rs`
- `chatwidget/tests/composer_submission.rs`
- `chatwidget/tests/config_errors_tests.rs`
- `chatwidget/tests/goal_menu.rs`
- `chatwidget/tests/goal_validation.rs`
- `chatwidget/tests/guardian.rs`
- `chatwidget/tests/helpers.rs`
- `chatwidget/tests/history_replay.rs`
- `chatwidget/tests/mcp_startup.rs`
- `chatwidget/tests/misalignment_policy_tests.rs`
- `chatwidget/tests/permission_shortcuts_tests.rs`
- `chatwidget/tests/permissions.rs`
- `chatwidget/tests/plan_mode.rs`
- `chatwidget/tests/plugin_catalog_tests.rs`
- `chatwidget/tests/popups_and_settings.rs`
- `chatwidget/tests/review_mode.rs`
- `chatwidget/tests/side.rs`
- `chatwidget/tests/slash_commands.rs`
- `chatwidget/tests/status_command_tests.rs`
- `chatwidget/tests/status_surface_previews.rs`
- `chatwidget/tests/terminal_title.rs`
- `chatwidget/tests/usage.rs`
- `chatwidget/tests.rs`
- `chatwidget/thread_usage.rs`
- `chatwidget/thread_usage_tests.rs`
- `chatwidget/tokens/chart/palette.rs`
- `chatwidget/tokens/chart/palette_tests.rs`
- `chatwidget/tokens/chart.rs`
- `chatwidget/tokens/chart_tests.rs`
- `chatwidget/tokens.rs`
- `chatwidget/tokens_tests.rs`
- `chatwidget/tool_requests.rs`
- `chatwidget/transcript_export.rs`
- `chatwidget/turn_runtime.rs`
- `chatwidget/usage.rs`
- `chatwidget/warnings.rs`
- `chatwidget/windows_sandbox_prompts.rs`
- `chatwidget/working_directory.rs`
- `chatwidget.rs`
- `clipboard_paste.rs`
- `collaboration_modes.rs`
- `custom_terminal.rs`
- `exec_cell/live_output.rs`
- `exec_cell/live_output_tests.rs`
- `exec_command.rs`
- `external_editor.rs`
- `external_editor_tests.rs`
- `file_search.rs`
- `frames.rs`
- `get_git_diff.rs`
- `goal_files.rs`
- `history_cell/approvals.rs`
- `history_cell/base.rs`
- `history_cell/hook_cell.rs`
- `history_cell/markdown_render_cache.rs`
- `history_cell/mcp.rs`
- `history_cell/mcp_result.rs`
- `history_cell/mcp_tests.rs`
- `history_cell/messages_tests.rs`
- `history_cell/notices.rs`
- `history_cell/plans.rs`
- `history_cell/plans_tests.rs`
- `history_cell/request_user_input.rs`
- `history_cell/search.rs`
- `history_cell/separators.rs`
- `ide_context/ipc.rs`
- `ide_context/prompt.rs`
- `ide_context/windows_pipe.rs`
- `ide_context/windows_pipe_tests.rs`
- `ide_context.rs`
- `inline_visualization/viewer.rs`
- `inline_visualization.rs`
- `inline_visualization_tests.rs`
- `keymap/bindings.rs`
- `keymap/chords.rs`
- `keymap/chords_tests.rs`
- `keymap.rs`
- `keymap_setup/actions.rs`
- `keymap_setup/capture.rs`
- `keymap_setup/capture_tests.rs`
- `keymap_setup/debug.rs`
- `keymap_setup/picker.rs`
- `keymap_setup.rs`
- `main.rs`
- `mention_codec.rs`
- `multi_agents.rs`
- `notifications/bel.rs`
- `notifications/mod.rs`
- `notifications/osc9.rs`
- `onboarding/keys.rs`
- `onboarding/mod.rs`
- `onboarding/onboarding_screen.rs`
- `onboarding/trust_directory.rs`
- `pager_overlay/scrolling_tests.rs`
- `pets/ambient.rs`
- `pets/asset_pack.rs`
- `pets/catalog.rs`
- `pets/frames.rs`
- `pets/image_protocol.rs`
- `pets/mod.rs`
- `pets/model.rs`
- `pets/picker.rs`
- `pets/preview.rs`
- `pets/sixel.rs`
- `public_widgets/composer_input.rs`
- `public_widgets/mod.rs`
- `render/line_utils.rs`
- `render/mod.rs`
- `resume_picker/archive.rs`
- `resume_picker/archive_tests.rs`
- `resume_picker/page_loading.rs`
- `resume_picker_transcript_preview.rs`
- `resume_picker_transcript_preview_tests.rs`
- `session_state.rs`
- `skills_helpers.rs`
- `slash_command.rs`
- `startup_error.rs`
- `status/account.rs`
- `status/helpers.rs`
- `status/rate_limits.rs`
- `status/remote_connection.rs`
- `status/tests.rs`
- `status/thread_usage.rs`
- `status/thread_usage_tests.rs`
- `terminal_hyperlinks.rs`
- `terminal_title.rs`
- `terminal_visualization_instructions.rs`
- `test_support.rs`
- `token_usage.rs`
- `tooltips.rs`
- `tui/event_stream.rs`
- `tui/frame_rate_limiter.rs`
- `tui/frame_requester.rs`
- `tui/history_tail.rs`
- `tui/history_tail_tests.rs`
- `tui/input_boundary.rs`
- `tui/job_control.rs`
- `tui/keyboard_modes.rs`
- `tui/screen_size.rs`
- `tui/screen_size_tests.rs`
- `tui/scrollback.rs`
- `tui/scrollback_tests.rs`
- `tui/terminal_stderr.rs`
- `tui/test_support.rs`
- `tui/windows_console.rs`
- `tui_startup_tests.rs`
- `version.rs`
- `vibe/mod.rs`
- `vibe/surface.rs`
- `workspace_command.rs`
- `workspace_messages.rs`
- `workspace_messages_tests.rs`

## 清理验证（2026-11-16）

- `git diff --check` 通过；仓库 Rust、Cargo、Python 与 shell 文件未发现跨目录引用被删源码的路径；保留入口的模块声明未指向被删文件。
- 静态资源盘点通过：689 个上游命名快照、50 个适配快照、其中 20 个迁移快照保留，无 `.new` 文件。这不是行为测试通过证据。
- 已通过槽位入口尝试 `cargo test -p vw-tui --lib upstream_snapshot_matrix`；编译被未修改的 `tui-input/src/textarea.rs:121` 既有 E0277 阻塞：`TextElement` 未实现 `Eq`。测试未运行，后续顺序命令 `cargo test -p vw-tui --lib insert_history::tests` 因前一步失败未执行。`tui-markdown` 同时报告既有 dead_code 警告。未扩大本次清理范围修复。
