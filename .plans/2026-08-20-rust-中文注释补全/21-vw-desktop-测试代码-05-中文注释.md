# 任务 21: 补充 vw-desktop 测试代码第 5 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 100 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-desktop 的 100 个文件（生产 0 个、测试 100 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-desktop/src/app/message/view/panel/usage_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/web_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/work_workspace_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/workspace_mode_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/view/tests.rs`
- 修改：`crates/vw-desktop/src/app/message/view/theme_tests.rs`
- 修改：`crates/vw-desktop/src/app/models_tests.rs`
- 修改：`crates/vw-desktop/src/app/native_menu/actions_tests.rs`
- 修改：`crates/vw-desktop/src/app/projects_tests.rs`
- 修改：`crates/vw-desktop/src/app/provider_tests.rs`
- 修改：`crates/vw-desktop/src/app/session/active_tests.rs`
- 修改：`crates/vw-desktop/src/app/session/agent_children_tests.rs`
- 修改：`crates/vw-desktop/src/app/session/chat_ui_tests.rs`
- 修改：`crates/vw-desktop/src/app/session/loading_tests.rs`
- 修改：`crates/vw-desktop/src/app/session/runtime_tests.rs`
- 修改：`crates/vw-desktop/src/app/session/view_cache_tests.rs`
- 修改：`crates/vw-desktop/src/app/session_gateway_tests.rs`
- 修改：`crates/vw-desktop/src/app/shiju_apps/launcher_tests.rs`
- 修改：`crates/vw-desktop/src/app/shiju_apps/state_tests.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/cache_sync_tests.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/client_tests.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/installer_tests.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/models_tests.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/package_version_cache_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/account_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/agent_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/app_aux_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/chrome_fields_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/core_fields_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/fields_navigation_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/fields_task_board_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/fields_workspace_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/git_fields_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/settings_preview_fields_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/workspace_task_fields_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/pet_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/presentation_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/agents_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/channels_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/provider_model_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/runtime_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/staff_resources_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/staff_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/workspace_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/backend_output_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/command_exec_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/git_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/process_utils_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/programs_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/state_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/worktree_admin_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/worktree_pool_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/staff/commands_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/staff/events_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/staff/models_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/staff/routing_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/staff/scheduling_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/staff/stage_capabilities_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/store/artifacts_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/store/operations_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/store/paths_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/store/persistence_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/store_tests.rs`
- 修改：`crates/vw-desktop/src/app/task/tests.rs`
- 修改：`crates/vw-desktop/src/app/task/workflow/registry_tests.rs`
- 修改：`crates/vw-desktop/src/app/terminal_tests.rs`
- 修改：`crates/vw-desktop/src/app/tests.rs`
- 修改：`crates/vw-desktop/src/app/theme_catalog/active_tests.rs`
- 修改：`crates/vw-desktop/src/app/theme_catalog/convert_tests.rs`
- 修改：`crates/vw-desktop/src/app/theme_catalog/model_tests.rs`
- 修改：`crates/vw-desktop/src/app/theme_catalog/overlay_tests.rs`
- 修改：`crates/vw-desktop/src/app/theme_catalog/pack_tests.rs`
- 修改：`crates/vw-desktop/src/app/theme_catalog/presets_assets_tests.rs`
- 修改：`crates/vw-desktop/src/app/theme_catalog/presets_tests.rs`
- 修改：`crates/vw-desktop/src/app/theme_catalog/resolve_tests.rs`
- 修改：`crates/vw-desktop/src/app/ui/chat_tests.rs`
- 修改：`crates/vw-desktop/src/app/ui/lsp_tests.rs`
- 修改：`crates/vw-desktop/src/app/ui_types_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/apps/modals_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/apps/tiles_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/apps/ui_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/apps_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/detail_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/install_guide_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/installed_skills_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/list_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/public_skill_detail_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/public_skills_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/shiju_apps_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/home/common_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/home/header_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/home/recent_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/home/tests.rs`
- 修改：`crates/vw-desktop/src/app/views/knowledge_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/project/components/new_session_tests.rs`
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/21-vw-desktop-测试代码-05-中文注释.md"
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

