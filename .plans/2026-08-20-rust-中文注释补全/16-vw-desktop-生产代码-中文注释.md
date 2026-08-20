# 任务 16: 补充 vw-desktop 生产代码的中文注释

批次：【批次 1】 无

状态：未开始

目的：为清单中的 57 个 Rust 文件补充真实中文注释，不改变代码行为。

来源任务：无

预计会话范围：本任务仅覆盖 vw-desktop 的 57 个文件（生产 57 个、测试 0 个），改动类型统一为注释，文件清单与其它任务互斥，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-desktop/src/app/account/mod.rs`
- 修改：`crates/vw-desktop/src/app/account/models.rs`
- 修改：`crates/vw-desktop/src/app/account/store.rs`
- 修改：`crates/vw-desktop/src/app/components/account_status.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_measure.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/selectable_text.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/agent_sessions.rs`
- 修改：`crates/vw-desktop/src/app/components/svg_path.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_search.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_skills/plugins.rs`
- 修改：`crates/vw-desktop/src/app/config_cron_jobs.rs`
- 修改：`crates/vw-desktop/src/app/config_gateway.rs`
- 修改：`crates/vw-desktop/src/app/message/account.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/processing_timing.rs`
- 修改：`crates/vw-desktop/src/app/message/preview/tab.rs`
- 修改：`crates/vw-desktop/src/app/message/settings/general.rs`
- 修改：`crates/vw-desktop/src/app/message/shiju_apps.rs`
- 修改：`crates/vw-desktop/src/app/message/skill_hub.rs`
- 修改：`crates/vw-desktop/src/app/message/skill_hub/public.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/apps.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/settings.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/tabs.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/web.rs`
- 修改：`crates/vw-desktop/src/app/session/agent_children.rs`
- 修改：`crates/vw-desktop/src/app/shiju_apps/launcher.rs`
- 修改：`crates/vw-desktop/src/app/shiju_apps/mod.rs`
- 修改：`crates/vw-desktop/src/app/shiju_apps/state.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/cache_sync.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/client.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/installer.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/mod.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/models.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/package_version_cache.rs`
- 修改：`crates/vw-desktop/src/app/skill_hub/state.rs`
- 修改：`crates/vw-desktop/src/app/state/account.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/tool_fields.rs`
- 修改：`crates/vw-desktop/src/app/state/pet.rs`
- 修改：`crates/vw-desktop/src/app/task/store/artifacts.rs`
- 修改：`crates/vw-desktop/src/app/task/store/operations.rs`
- 修改：`crates/vw-desktop/src/app/task/store/paths.rs`
- 修改：`crates/vw-desktop/src/app/task/store/persistence.rs`
- 修改：`crates/vw-desktop/src/app/terminal.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/detail.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/install_guide.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/installed_skills.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/list.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/mod.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/public_skill_detail.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/public_skills.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/shiju_apps/detail.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/shiju_apps/mine.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/shiju_apps/mod.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/shiju_apps/store.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/shiju_apps/styles.rs`
- 修改：`crates/vw-desktop/src/app/views/expert_hub/styles.rs`
- 修改：`crates/vw-desktop/src/app/views/workflow_manager/mod.rs`
- 修改：`crates/vw-desktop/src/apps/workflow/message/run.rs`
- 测试：无新增测试；使用现有编译面并由最终审计任务统一运行 Clippy。

## 上下文

- 涉及区域：app、apps。
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
  task_plan_file=".plans/2026-08-20-rust-中文注释补全/16-vw-desktop-生产代码-中文注释.md"
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
- 所需证据：57 个目标全部命中中文注释；`git diff --check` 成功；人工差异审查确认没有行为、字符串、断言或结构变更。

## 完成

仅当清单中的 57 个文件全部获得与实际语义一致的中文注释、聚焦验证通过且差异仅含注释及必要空行时，才可标记为 `已完成`。

