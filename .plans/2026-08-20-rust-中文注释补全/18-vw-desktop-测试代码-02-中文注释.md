# 任务 18: 补充 vw-desktop 测试代码第 2 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 100 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-desktop 的 100 个文件（生产 0 个、测试 100 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/header_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/markers_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/selection_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/text_diff_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/filter_options_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/header_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/ops_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/ui/disabled_buttons_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/ui/glyph_buttons_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/ui/icon_buttons_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/ui/shared_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/ui_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/utils_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_mention_highlighter_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/agent_sessions_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/drop_area_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/file_references_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/file_search_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/icons_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/input_editor_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/model_popover_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/queue_panel_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/quick_messages_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/reasoning_selector_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/send_controls_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/task_mode_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/todo_panel_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/usage_button_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/usage_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/markdown_editor_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/model_hover_tooltip_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/notification_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/number_input_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/overlays/above/overlay_element_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/overlays/above/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/overlays/above/widget_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/overlays/inline_right_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/overlays/left_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/overlays/side_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/tabs_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/view/content_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/view/header_tabs_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/view/menus_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/view/settings_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/view/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/preview_panel/widgets_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/search_panel_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/status_animation_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/svg_path_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/svg_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_acp_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/basic_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/identity_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/shared_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/skills_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/tools_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_autonomy_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_browser_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_channels/basic_channels_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_channels/common_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_channels/enterprise_channels_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_channels/extended_channels_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_channels_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_common/help_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_common/icons_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_common/panels_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_common/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_common/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_common/theme_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_common/utils_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_cron_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_dialogue_flow_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_editor_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_embedding_routes_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_gateway_client_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_gateway_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_general_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_heartbeat_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_help_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_hooks_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_http_request_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_mcp_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_memory_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_model_routes_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_models_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_multimodal_tests.rs`
- 测试：无新增测试；使用现有编译面并由最终审计任务统一运行 Clippy。

## 上下文

- 涉及区域：app。
- 参考 `crates/vw-acp/src/agent_registry.rs`：模块级说明职责和边界，公开 API 说明契约，复杂逻辑只解释原因。
- 每个目标文件当前均未命中“注释行包含中文”的审计；不要用无意义占位注释只为通过正则。
- 普通模块可用 `//!`；由 `include!` 拼接的文件必须结合包含位置选择普通 `//` 或合法外部文档注释。

## 验证计划

- 测试驱动开发不适用：本任务只添加注释，不改变可执行行为或测试行为。
- 覆盖检查：从本任务文件提取全部修改路径，逐个确认至少存在一条含中文的 Rust 注释。
- 差异检查：运行 `git diff --check`，并人工确认目标差异只包含准确注释及其必要空行。
- 回归策略：本任务不运行 Cargo，避免批次 1 并行任务争用构建资源；全仓库 Clippy 由批次 2 统一执行。

## 步骤

1. 逐个读取目标文件、模块声明、相邻测试与必要调用者，确认真实职责。
2. 为普通模块补充简洁中文模块文档；为公开类型、字段和函数补充契约说明。
3. 仅在状态转换、安全边界、回退或兼容逻辑不直观时添加行内中文注释。
4. 测试文件说明覆盖的行为与 fixture 意图，不复述测试名和断言。
5. 运行覆盖检查与差异检查，修正遗漏、误导说明和非注释变更。

## 验证

- 运行：
  ```zsh
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/18-vw-desktop-测试代码-02-中文注释.md"
  comment_audit_failed=0
  while IFS= read -r rust_file; do
    if ! grep -Eq '//.*[一-龥]|^[[:space:]]*(/\*+|\*)[^[:cntrl:]]*[一-龥]' "$rust_file"; then
      print -r -- "$rust_file"
      comment_audit_failed=1
    fi
  done < <(sed -n 's/^- 修改：`\([^`]*\.rs\)`$/\1/p' "$task_plan_file")
  test "$comment_audit_failed" -eq 0
  sed -n 's/^- 修改：`\([^`]*\.rs\)`$/\1/p' "$task_plan_file" |
    tr '\n' '\0' |
    xargs -0 git diff --check --
  ```
- 预期：覆盖检查不输出路径，两条检查均以退出码 0 结束。
- 所需证据：100 个目标全部命中中文注释；`git diff --check` 成功；人工差异审查确认没有行为、字符串、断言或结构变更。

## 完成

仅当清单中的 100 个文件全部获得与实际语义一致的中文注释、聚焦验证通过且差异仅含注释及必要空行时，才可标记为 `已完成`。

