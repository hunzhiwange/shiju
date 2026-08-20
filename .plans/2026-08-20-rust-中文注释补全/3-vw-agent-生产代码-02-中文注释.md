# 任务 3: 补充 vw-agent 生产代码第 2 组的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 55 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-agent 的 55 个文件（生产 55 个、测试 0 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-agent/src/channels/irc/mod.rs`
- 修改：`crates/vw-agent/src/channels/lark/token_test_support.rs`
- 修改：`crates/vw-agent/src/channels/lark/ws_test_support.rs`
- 修改：`crates/vw-agent/src/channels/manager/health.rs`
- 修改：`crates/vw-agent/src/channels/manager/listener.rs`
- 修改：`crates/vw-agent/src/channels/manager/telegram.rs`
- 修改：`crates/vw-agent/src/channels/slack/mod.rs`
- 修改：`crates/vw-agent/src/channels/wati/mod.rs`
- 修改：`crates/vw-agent/src/channels/whatsapp/mod.rs`
- 修改：`crates/vw-agent/src/channels/whatsapp_storage/app_sync_store.rs`
- 修改：`crates/vw-agent/src/channels/whatsapp_storage/device_store.rs`
- 修改：`crates/vw-agent/src/channels/whatsapp_storage/mod.rs`
- 修改：`crates/vw-agent/src/channels/whatsapp_storage/signal_store.rs`
- 修改：`crates/vw-agent/src/channels/whatsapp_web/mod.rs`
- 修改：`crates/vw-agent/src/config/schema/config_save.rs`
- 修改：`crates/vw-agent/src/config/store.rs`
- 修改：`crates/vw-agent/src/config/store/postgres.rs`
- 修改：`crates/vw-agent/src/config/traits.rs`
- 修改：`crates/vw-agent/src/cron/consolidation/mod.rs`
- 修改：`crates/vw-agent/src/cron/mod.rs`
- 修改：`crates/vw-agent/src/cron/store.rs`
- 修改：`crates/vw-agent/src/gateway/api/auth.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/apiql_connections.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/apiql_entities.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/apiql_executor.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/apiql_formatters.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/apiql_functions.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/apiql_transformers.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/apiql_virtuals.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/hooks.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/knowledge.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/mcp.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/memory.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/shiju_account.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/shiju_apps.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/shiju_skills.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/skill_market.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/task_lifecycle.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/workflow_store.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/workflow_store/postgres.rs`
- 修改：`crates/vw-agent/src/gateway/handlers.rs`
- 修改：`crates/vw-agent/src/gateway/mod.rs`
- 修改：`crates/vw-agent/src/gateway/node_control.rs`
- 修改：`crates/vw-agent/src/gateway/openai_compat/mod.rs`
- 修改：`crates/vw-agent/src/gateway/settings_store.rs`
- 修改：`crates/vw-agent/src/gateway/settings_store/postgres.rs`
- 修改：`crates/vw-agent/src/global_database.rs`
- 修改：`crates/vw-agent/src/global_database/config.rs`
- 修改：`crates/vw-agent/src/global_database/postgres.rs`
- 修改：`crates/vw-agent/src/global_database/test_support.rs`
- 修改：`crates/vw-agent/src/goals/engine/mod.rs`
- 修改：`crates/vw-agent/src/hooks/runtime.rs`
- 修改：`crates/vw-agent/src/integrations/mod.rs`
- 修改：`crates/vw-agent/src/knowledge/chunker.rs`
- 修改：`crates/vw-agent/src/knowledge/chunker/patterns.rs`
- 测试：无新增测试；使用现有编译面并由最终审计任务统一运行 Clippy。

## 上下文

- 涉及区域：channels、config、cron、gateway、global_database、goals、hooks、integrations、knowledge、根模块。
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/3-vw-agent-生产代码-02-中文注释.md"
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
- 所需证据：55 个目标全部命中中文注释；`git diff --check` 成功；人工差异审查确认没有行为、字符串、断言或结构变更。

## 完成

仅当清单中的 55 个文件全部获得与实际语义一致的中文注释、聚焦验证通过且差异仅含注释及必要空行时，才可标记为 `已完成`。

