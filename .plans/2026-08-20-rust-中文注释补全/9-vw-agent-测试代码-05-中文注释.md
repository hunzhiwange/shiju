# 任务 9: 补充 vw-agent 测试代码第 5 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 97 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-agent 的 97 个文件（生产 0 个、测试 97 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-agent/src/plugins/cli_tests.rs`
- 修改：`crates/vw-agent/src/plugins/hook_handler_tests.rs`
- 修改：`crates/vw-agent/src/plugins/hook_trust_tests.rs`
- 修改：`crates/vw-agent/src/plugins/hooks_tests.rs`
- 修改：`crates/vw-agent/src/plugins/install_tests.rs`
- 修改：`crates/vw-agent/src/plugins/loader_tests.rs`
- 修改：`crates/vw-agent/src/plugins/manifest_tests.rs`
- 修改：`crates/vw-agent/src/plugins/mcp_tests.rs`
- 修改：`crates/vw-agent/src/plugins/skills_tests.rs`
- 修改：`crates/vw-agent/src/project/icon_discovery_tests.rs`
- 修改：`crates/vw-agent/src/project/instance_tests.rs`
- 修改：`crates/vw-agent/src/project/migration_tests.rs`
- 修改：`crates/vw-agent/src/project/state_tests.rs`
- 修改：`crates/vw-agent/src/project/store/postgres_tests.rs`
- 修改：`crates/vw-agent/src/project/store_tests.rs`
- 修改：`crates/vw-agent/src/project/tests.rs`
- 修改：`crates/vw-agent/src/project/vcs_tests.rs`
- 修改：`crates/vw-agent/src/provider/auth_tests.rs`
- 修改：`crates/vw-agent/src/provider/error_tests.rs`
- 修改：`crates/vw-agent/src/provider/models_tests.rs`
- 修改：`crates/vw-agent/src/provider/provider_tests.rs`
- 修改：`crates/vw-agent/src/provider/tests.rs`
- 修改：`crates/vw-agent/src/provider/transform_tests.rs`
- 修改：`crates/vw-agent/src/providers/tests.rs`
- 修改：`crates/vw-agent/src/pty/pty_tests.rs`
- 修改：`crates/vw-agent/src/question/mod_tests.rs`
- 修改：`crates/vw-agent/src/scheduler/tests.rs`
- 修改：`crates/vw-agent/src/security/audit_tests.rs`
- 修改：`crates/vw-agent/src/security/detect/tests.rs`
- 修改：`crates/vw-agent/src/security/gateway_skey_store/postgres_tests.rs`
- 修改：`crates/vw-agent/src/security/gateway_skey_store_tests.rs`
- 修改：`crates/vw-agent/src/security/leak_detector_tests.rs`
- 修改：`crates/vw-agent/src/security/pairing_tests.rs`
- 修改：`crates/vw-agent/src/security/policy/action_tracker_tests.rs`
- 修改：`crates/vw-agent/src/security/policy/allowlist_tests.rs`
- 修改：`crates/vw-agent/src/security/policy/path_utils_tests.rs`
- 修改：`crates/vw-agent/src/security/policy/risk_tests.rs`
- 修改：`crates/vw-agent/src/security/policy/shell_lexer_tests.rs`
- 修改：`crates/vw-agent/src/security/policy/shell_redirect_tests.rs`
- 修改：`crates/vw-agent/src/security/policy/tests.rs`
- 修改：`crates/vw-agent/src/security/policy/types_tests.rs`
- 修改：`crates/vw-agent/src/security/prompt_guard_tests.rs`
- 修改：`crates/vw-agent/src/security/secrets_tests.rs`
- 修改：`crates/vw-agent/src/security/semantic_guard_tests.rs`
- 修改：`crates/vw-agent/src/security/syscall_anomaly_tests.rs`
- 修改：`crates/vw-agent/src/security/tests.rs`
- 修改：`crates/vw-agent/src/security/traits_tests.rs`
- 修改：`crates/vw-agent/src/service/common_tests.rs`
- 修改：`crates/vw-agent/src/service/linux_tests.rs`
- 修改：`crates/vw-agent/src/service/macos_tests.rs`
- 修改：`crates/vw-agent/src/service/mod_tests.rs`
- 修改：`crates/vw-agent/src/service/openrc_tests.rs`
- 修改：`crates/vw-agent/src/service/windows_tests.rs`
- 修改：`crates/vw-agent/src/session/capability_policy_tests.rs`
- 修改：`crates/vw-agent/src/session/child_agent/coordinator_tests.rs`
- 修改：`crates/vw-agent/src/session/child_agent_tests.rs`
- 修改：`crates/vw-agent/src/session/claude_protocol_parity_tests.rs`
- 修改：`crates/vw-agent/src/session/compaction_tests.rs`
- 修改：`crates/vw-agent/src/session/context_injection_tests.rs`
- 修改：`crates/vw-agent/src/session/instruction_cache_tests.rs`
- 修改：`crates/vw-agent/src/session/instruction_discovery_tests.rs`
- 修改：`crates/vw-agent/src/session/instruction_include_tests.rs`
- 修改：`crates/vw-agent/src/session/instruction_rules_tests.rs`
- 修改：`crates/vw-agent/src/session/instruction_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/config_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/replay_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/request_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/session_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/tests.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/updates_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/aisdk/convert_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/aisdk/driver_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/aisdk/error_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/aisdk/request_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/aisdk/stream_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/aisdk/util_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/logging_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/messages_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/model_selector_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/options_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/prompt_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/stream_tests.rs`
- 修改：`crates/vw-agent/src/session/llm/types_tests.rs`
- 修改：`crates/vw-agent/src/session/llm_raw_tests.rs`
- 修改：`crates/vw-agent/src/session/message_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/artifacts_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/context_governance_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/helpers_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/llm_messages_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/planning_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/prefetch_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/prompting_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/runner_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/todo_updates_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/todos_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/tools_exec_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/turn_policy_tests.rs`
- 测试：无新增测试；使用现有编译面并由最终审计任务统一运行 Clippy。

## 上下文

- 涉及区域：plugins、project、provider、providers、pty、question、scheduler、security、service、session。
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/9-vw-agent-测试代码-05-中文注释.md"
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

