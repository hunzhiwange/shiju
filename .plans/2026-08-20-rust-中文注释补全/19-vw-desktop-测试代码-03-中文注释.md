# 任务 19: 补充 vw-desktop 测试代码第 3 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 100 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-desktop 的 100 个文件（生产 0 个、测试 100 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-desktop/src/app/components/system_settings_network_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_observability_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_projects_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_providers/catalog_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_providers/connect_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_providers/connected_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_providers/custom_provider_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_proxy_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_query_classification_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_quick_messages_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_reliability_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_runtime_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_scheduler_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_search_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_security_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_skills/browser_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_skills/catalog_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_skills/help_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_skills/plugins_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_skills_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_staff/teams_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_staff/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_staff/workers_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_storage_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_themes/editor_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_tunnel_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_web_search_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/tab_bar_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/task_pet_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/terminal_panel_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/text_editor_context_menu_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/text_editor_scroll_panel_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/toast_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/top_bar/controls_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/top_bar/drag_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/top_bar/external_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/top_bar/menus_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/top_bar/widgets_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/top_bar/workspace_mode_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/top_bar_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/widgets_tests.rs`
- 修改：`crates/vw-desktop/src/app/config_agent_tests.rs`
- 修改：`crates/vw-desktop/src/app/config_cli_bootstrap_tests.rs`
- 修改：`crates/vw-desktop/src/app/config_cron_jobs_tests.rs`
- 修改：`crates/vw-desktop/src/app/config_desktop_tests.rs`
- 修改：`crates/vw-desktop/src/app/config_gateway_tests.rs`
- 修改：`crates/vw-desktop/src/app/config_system_settings_tests.rs`
- 修改：`crates/vw-desktop/src/app/config_tests.rs`
- 修改：`crates/vw-desktop/src/app/git_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/agent_stream_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_basic_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/external_apps_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/new/load_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/new/settings_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/new/state_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/new_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/settings_builders_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/startup_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_subscription_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_update_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/account_details_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/chat_reset_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/project_edit/common_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/project_edit/general_tab_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/project_edit/launch_tab_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/project_edit/refresh_tab_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/project_edit/scheduling_tab_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/project_edit/tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/recent_project_remove_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals/rename_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_modals_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_status_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/impls/tests.rs`
- 修改：`crates/vw-desktop/src/app/lsp/manager_tests.rs`
- 修改：`crates/vw-desktop/src/app/lsp/tests.rs`
- 修改：`crates/vw-desktop/src/app/message/account_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/context_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/input/clipboard_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/input/context_menus_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/input/input_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/input/tool_detail_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/input_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/processing_timing_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/stream/permissions_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/stream/questions_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/stream/steps_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/stream/tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/stream/todos_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/trajectory_tests.rs`
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/19-vw-desktop-测试代码-03-中文注释.md"
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

