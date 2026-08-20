# 任务 22: 补充 vw-desktop 测试代码第 6 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 100 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-desktop 的 100 个文件（生产 0 个、测试 100 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-desktop/src/app/views/project/components/projects_list_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/components/sessions_panel_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/handles_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/bottom_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/chat_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/chrome_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/drag_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/main_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/sessions_panel_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/sidebar_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/work_resources_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout/work_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/layout_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/utils_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/board_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/common_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/control/helpers_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/control/worktree_office/floor_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/control/worktree_office/room_scene_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/control/worktree_office/summary_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/control/worktree_office_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/control_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/drag_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/modals_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/panel/executor_selector_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/panel/logs_modal_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/panel/model_selector_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/panel/reasoning_selector_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/panel/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/panel/subtask_editor_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/panel/tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/staff_mode/assignment_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/staff_mode/feed_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/staff_mode/lanes_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/staff_mode/roster_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/staff_mode/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/staff_mode/tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/tests.rs`
- 修改：`crates/vw-desktop/src/app/views/usage/components_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/usage/data_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/usage/layout_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/usage/raw_modal_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/usage/session_menu_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/usage/steps_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/usage/styles_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/usage/utils_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/workflow_manager/capabilities_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/workflow_manager/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/webview_runtime_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/canvas/handles_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/canvas/mod_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/canvas/render_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/canvas/tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/message/app_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/message/canvas_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/message/document_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/message/mod_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/message/node_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/message/run_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/message/tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/mod_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/model/creation_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/model/defaults_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/model/dify_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/model/loader_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/model/registry_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/model/save_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/app_ui_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/canvas_ops_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/draft_build_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/ids_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/if_else_helpers_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/mod_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/node_editor_branches_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/node_editor_integrations_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/node_editor_start_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/node_validation_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/start_variable_drafts_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/start_variable_validation_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/variables_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/visual_sync_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/state/yaml_helpers_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/canvas_shell_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/header_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/if_else_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/start_inputs_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/start_meta_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/start_panel_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/styles_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/toolbar_tests.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/view/variables_tests.rs`
- 修改：`crates/vw-desktop/src/fonts_tests.rs`
- 修改：`crates/vw-desktop/src/legacy_task_board_switches_exact_tests.rs`
- 修改：`crates/vw-desktop/src/main_tests.rs`
- 修改：`crates/vw-desktop/src/tests.rs`
- 修改：`crates/vw-desktop/src/workflow_capability_panel_exact_tests.rs`
- 测试：无新增测试；使用现有编译面并由最终审计任务统一运行 Clippy。

## 上下文

- 涉及区域：app、apps、根模块。
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/22-vw-desktop-测试代码-06-中文注释.md"
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

