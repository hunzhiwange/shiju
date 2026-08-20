# 任务 10: 补充 vw-agent 测试代码第 6 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 97 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-agent 的 97 个文件（生产 0 个、测试 97 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-agent/src/session/processor/types_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/utils/docs_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/utils/file_links_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/utils/message_format_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/utils/output_compaction_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/utils/session_ingest_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/utils/tool_parsing_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/utils_tests.rs`
- 修改：`crates/vw-agent/src/session/prompt_tests.rs`
- 修改：`crates/vw-agent/src/session/retry_tests.rs`
- 修改：`crates/vw-agent/src/session/revert_tests.rs`
- 修改：`crates/vw-agent/src/session/runtime/lifecycle_tests.rs`
- 修改：`crates/vw-agent/src/session/runtime/prompt_tests.rs`
- 修改：`crates/vw-agent/src/session/session_tests.rs`
- 修改：`crates/vw-agent/src/session/status_tests.rs`
- 修改：`crates/vw-agent/src/session/store/postgres_tests.rs`
- 修改：`crates/vw-agent/src/session/store_tests.rs`
- 修改：`crates/vw-agent/src/session/summary_tests.rs`
- 修改：`crates/vw-agent/src/session/system_tests.rs`
- 修改：`crates/vw-agent/src/session/title_tests.rs`
- 修改：`crates/vw-agent/src/session/todo_tests.rs`
- 修改：`crates/vw-agent/src/session/ui_config_tests.rs`
- 修改：`crates/vw-agent/src/session/ui_store_tests.rs`
- 修改：`crates/vw-agent/src/shiju/account_tests.rs`
- 修改：`crates/vw-agent/src/shiju/apps_tests.rs`
- 修改：`crates/vw-agent/src/skill/audit_tests.rs`
- 修改：`crates/vw-agent/src/skill/constants_tests.rs`
- 修改：`crates/vw-agent/src/skill/discovery_tests.rs`
- 修改：`crates/vw-agent/src/skill/install_tests.rs`
- 修改：`crates/vw-agent/src/skill/loader_tests.rs`
- 修改：`crates/vw-agent/src/skill/open_skills_tests.rs`
- 修改：`crates/vw-agent/src/skill/policy_tests.rs`
- 修改：`crates/vw-agent/src/skill/prompt_tests.rs`
- 修改：`crates/vw-agent/src/skill/skill_tests.rs`
- 修改：`crates/vw-agent/src/skill/source_tests.rs`
- 修改：`crates/vw-agent/src/skill/symlink_tests/tests.rs`
- 修改：`crates/vw-agent/src/skill/types_tests.rs`
- 修改：`crates/vw-agent/src/skills/audit/manifest_tests.rs`
- 修改：`crates/vw-agent/src/skills/audit/markdown_tests.rs`
- 修改：`crates/vw-agent/src/skills/audit/report_tests.rs`
- 修改：`crates/vw-agent/src/skills/audit/risk_tests.rs`
- 修改：`crates/vw-agent/src/skills/audit/scan_tests.rs`
- 修改：`crates/vw-agent/src/skills/audit/support_tests.rs`
- 修改：`crates/vw-agent/src/skills/cli_tests.rs`
- 修改：`crates/vw-agent/src/skills/discovery_tests.rs`
- 修改：`crates/vw-agent/src/skills/init_tests.rs`
- 修改：`crates/vw-agent/src/skills/installer_tests.rs`
- 修改：`crates/vw-agent/src/skills/invoked_tests.rs`
- 修改：`crates/vw-agent/src/skills/listing_tests.rs`
- 修改：`crates/vw-agent/src/skills/loader_tests.rs`
- 修改：`crates/vw-agent/src/skills/marketplace/install_tests.rs`
- 修改：`crates/vw-agent/src/skills/marketplace/provider_tests.rs`
- 修改：`crates/vw-agent/src/skills/marketplace/skillhub_tests.rs`
- 修改：`crates/vw-agent/src/skills/mod_tests.rs`
- 修改：`crates/vw-agent/src/skills/open_skills_tests.rs`
- 修改：`crates/vw-agent/src/skills/policy_tests.rs`
- 修改：`crates/vw-agent/src/skills/prompt_tests.rs`
- 修改：`crates/vw-agent/src/skills/session_state_tests.rs`
- 修改：`crates/vw-agent/src/skills/shiju_tests.rs`
- 修改：`crates/vw-agent/src/skills/source_tests.rs`
- 修改：`crates/vw-agent/src/skills/types_tests.rs`
- 修改：`crates/vw-agent/src/snapshot/common_tests.rs`
- 修改：`crates/vw-agent/src/snapshot/native_tests.rs`
- 修改：`crates/vw-agent/src/snapshot/wasm_tests.rs`
- 修改：`crates/vw-agent/src/storage/tests.rs`
- 修改：`crates/vw-agent/src/task_lifecycle/builtin_tests.rs`
- 修改：`crates/vw-agent/src/task_lifecycle/registry_tests.rs`
- 修改：`crates/vw-agent/src/task_lifecycle/store/postgres_tests.rs`
- 修改：`crates/vw-agent/src/task_lifecycle/store_tests.rs`
- 修改：`crates/vw-agent/src/task_plan_run/control_tests.rs`
- 修改：`crates/vw-agent/src/task_plan_run/model_tests.rs`
- 修改：`crates/vw-agent/src/task_plan_run/schedule_tests.rs`
- 修改：`crates/vw-agent/src/test_capabilities_tests.rs`
- 修改：`crates/vw-agent/src/tests.rs`
- 修改：`crates/vw-agent/src/tools/agent_tool/tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/actions_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/agent_browser_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/backend_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/computer_use/config_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/computer_use/execution_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/computer_use/response_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/computer_use/tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/computer_use/validation_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/helpers_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/native_backend/actions_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/native_backend/session_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/native_backend/snapshot_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/native_backend/tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/native_backend/webdriver_tests.rs`
- 修改：`crates/vw-agent/src/tools/browser/tool_tests.rs`
- 修改：`crates/vw-agent/src/tools/context/tests.rs`
- 修改：`crates/vw-agent/src/tools/decision/tests.rs`
- 修改：`crates/vw-agent/src/tools/delegate/agentic_tests.rs`
- 修改：`crates/vw-agent/src/tools/delegate/execution_tests.rs`
- 修改：`crates/vw-agent/src/tools/delegate/support_tests.rs`
- 修改：`crates/vw-agent/src/tools/delegated_tools_tests.rs`
- 修改：`crates/vw-agent/src/tools/file_edit/types_tests.rs`
- 测试：无新增测试；使用现有编译面并由最终审计任务统一运行 Clippy。

## 上下文

- 涉及区域：session、shiju、skill、skills、snapshot、storage、task_lifecycle、task_plan_run、tools、根模块。
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/10-vw-agent-测试代码-06-中文注释.md"
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
- 所需证据：97 个目标全部命中中文注释；`git diff --check` 成功；人工差异审查确认没有行为、字符串、断言或结构变更。

## 完成

仅当清单中的 97 个文件全部获得与实际语义一致的中文注释、聚焦验证通过且差异仅含注释及必要空行时，才可标记为 `已完成`。

