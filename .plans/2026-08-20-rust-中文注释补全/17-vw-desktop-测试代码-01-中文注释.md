# 任务 17: 补充 vw-desktop 测试代码第 1 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 101 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-desktop 的 101 个文件（生产 0 个、测试 101 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-desktop/src/app/account/models_tests.rs`
- 修改：`crates/vw-desktop/src/app/account/store_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/icon_actions_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/icon_apps_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/icon_canvas_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/icon_files_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/icon_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/icon_ui_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/images_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/named_icons_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/provider_icons_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/svg_icons_tests.rs`
- 修改：`crates/vw-desktop/src/app/assets/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/about_modal_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/account_status_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/animated_text_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/empty_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/header_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_measure_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/assistant_body_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/assistant_error_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/message_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/parse_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/render_cache_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/text_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/think_block_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/tool_summaries_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tool_names_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tool_selector_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/advanced_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_preview_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/bash_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/changes_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/command_summary_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/config_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/diff_utils_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/explore_summary_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/file_link_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/files_view/common_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/files_view/list_ui_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/files_view/parse_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/files_view/write_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/files_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/git_diff_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/lsp_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/mcp_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/question_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/read_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/skill_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/text_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/todo_views_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/tool_detail_dialog_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/tool_parse_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/tool_permission_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/tool_renderer_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/types_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/web_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tools/workflow_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/trajectory/details_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/trajectory/json_tree_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/trajectory/ledger_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/trajectory/projection_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/trajectory/timeline_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/utils/app_state_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/utils/path_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/utils/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/utils/text_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/utils/theme_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/utils/time_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/utils/ui_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/code_review_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/color_picker/conversion_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/color_picker/hsv_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/color_picker/images_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/color_picker/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/color_picker/pickers_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/color_picker/render_full_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/color_picker/render_mini_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/editor_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/editor_toolbar_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/file_tree/find_results_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/file_tree/icons_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/file_tree/menu_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/file_tree/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/file_tree/tree_list_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/file_tree/widgets_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/comment_editor_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/context_menu_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/added_deleted_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/gaps_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/hunk_ops/deletes_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/hunk_ops/inserts_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/hunk_ops/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/hunk_ops/replaces_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/hunk_ops/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/standard_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/start_end_tests.rs`
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/17-vw-desktop-测试代码-01-中文注释.md"
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
- 所需证据：101 个目标全部命中中文注释；`git diff --check` 成功；人工差异审查确认没有行为、字符串、断言或结构变更。

## 完成

仅当清单中的 101 个文件全部获得与实际语义一致的中文注释、聚焦验证通过且差异仅含注释及必要空行时，才可标记为 `已完成`。

