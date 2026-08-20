# 任务 11: 补充 vw-agent 测试代码第 7 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 96 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-agent 的 96 个文件（生产 0 个、测试 96 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-agent/src/tools/file_edit/utils_tests.rs`
- 修改：`crates/vw-agent/src/tools/git_operations/tests.rs`
- 修改：`crates/vw-agent/src/tools/glob_search/tests.rs`
- 修改：`crates/vw-agent/src/tools/hooks/tests.rs`
- 修改：`crates/vw-agent/src/tools/http_headers_tests.rs`
- 修改：`crates/vw-agent/src/tools/mcp_inspect_tests.rs`
- 修改：`crates/vw-agent/src/tools/mcp_status/tests.rs`
- 修改：`crates/vw-agent/src/tools/mcp_tool_tests.rs`
- 修改：`crates/vw-agent/src/tools/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/powershell/tests.rs`
- 修改：`crates/vw-agent/src/tools/read_state/tests.rs`
- 修改：`crates/vw-agent/src/tools/registry/tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/ast/command_info/ast_walk_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/ast/command_info/scan_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/ast/command_info/tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/ast/command_info/wrappers_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/path/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/permissions/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/readonly/flag_validator_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/readonly/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/sandbox/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/sandbox/policy_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/backslash_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/brace_expansion_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/comment_quote_desync_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/control_chars_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/dangerous_vars_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/empty_command_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/git_commit_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/hash_comment_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/heredoc_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/ifs_injection_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/jq_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/malformed_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/metacharacters_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/newline_injection_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/obfuscated_flags_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/proc_environ_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/quoted_newline_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/redirection_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/substitution_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/unicode_whitespace_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/security/validators/zsh_dangerous_tests.rs`
- 修改：`crates/vw-agent/src/tools/shell/sed/validation_tests.rs`
- 修改：`crates/vw-agent/src/tools/skill/invocation_tests.rs`
- 修改：`crates/vw-agent/src/tools/skill/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/sleep/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/todo/normalize_tests.rs`
- 修改：`crates/vw-agent/src/tools/todo/schema_tests.rs`
- 修改：`crates/vw-agent/src/tools/todo/store_tests.rs`
- 修改：`crates/vw-agent/src/tools/todo/tests.rs`
- 修改：`crates/vw-agent/src/tools/toolset/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/traits/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/url_validation/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/web_fetch/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/web_search_tool/mod_tests.rs`
- 修改：`crates/vw-agent/src/tools/websearch/mod_tests.rs`
- 修改：`crates/vw-agent/src/util/abort_tests.rs`
- 修改：`crates/vw-agent/src/util/archive_tests.rs`
- 修改：`crates/vw-agent/src/util/color_tests.rs`
- 修改：`crates/vw-agent/src/util/context_tests.rs`
- 修改：`crates/vw-agent/src/util/defer_tests.rs`
- 修改：`crates/vw-agent/src/util/eventloop_tests.rs`
- 修改：`crates/vw-agent/src/util/filesystem_tests.rs`
- 修改：`crates/vw-agent/src/util/fn_tests.rs`
- 修改：`crates/vw-agent/src/util/iife_tests.rs`
- 修改：`crates/vw-agent/src/util/keybind_tests.rs`
- 修改：`crates/vw-agent/src/util/lazy_tests.rs`
- 修改：`crates/vw-agent/src/util/locale_tests.rs`
- 修改：`crates/vw-agent/src/util/lock_tests.rs`
- 修改：`crates/vw-agent/src/util/log_tests.rs`
- 修改：`crates/vw-agent/src/util/proxied_tests.rs`
- 修改：`crates/vw-agent/src/util/queue_tests.rs`
- 修改：`crates/vw-agent/src/util/rpc_tests.rs`
- 修改：`crates/vw-agent/src/util/scrap_tests.rs`
- 修改：`crates/vw-agent/src/util/signal_tests.rs`
- 修改：`crates/vw-agent/src/util/task_set_tests.rs`
- 修改：`crates/vw-agent/src/util/tests.rs`
- 修改：`crates/vw-agent/src/util/timeout_tests.rs`
- 修改：`crates/vw-agent/src/util/token_tests.rs`
- 修改：`crates/vw-agent/src/util/wildcard_tests.rs`
- 修改：`crates/vw-agent/src/workflow/code_runner_http_tests.rs`
- 修改：`crates/vw-agent/src/workflow/code_runner_tests.rs`
- 修改：`crates/vw-agent/src/workflow/conditions_tests.rs`
- 修改：`crates/vw-agent/src/workflow/model_tests.rs`
- 修改：`crates/vw-agent/src/workflow/template_tests.rs`
- 修改：`crates/vw-agent/src/workflow/variables_tests.rs`
- 修改：`crates/vw-agent/src/worktree/tests.rs`
- 修改：`crates/vw-agent/src/worktree/worktree_event_tests.rs`
- 修改：`crates/vw-agent/src/worktree/worktree_naming_tests.rs`
- 修改：`crates/vw-agent/src/worktree/worktree_native_tests.rs`
- 修改：`crates/vw-agent/src/worktree/worktree_ops_tests.rs`
- 修改：`crates/vw-agent/src/worktree/worktree_types_tests.rs`
- 测试：无新增测试；使用现有编译面并由最终审计任务统一运行 Clippy。

## 上下文

- 涉及区域：tools、util、workflow、worktree。
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/11-vw-agent-测试代码-07-中文注释.md"
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
- 所需证据：96 个目标全部命中中文注释；`git diff --check` 成功；人工差异审查确认没有行为、字符串、断言或结构变更。

## 完成

仅当清单中的 96 个文件全部获得与实际语义一致的中文注释、聚焦验证通过且差异仅含注释及必要空行时，才可标记为 `已完成`。

