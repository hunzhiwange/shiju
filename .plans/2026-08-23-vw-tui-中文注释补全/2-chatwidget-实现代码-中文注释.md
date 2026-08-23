# 任务 2: 补充 chatwidget 实现代码的中文注释

批次：【批次 1】 无

状态：未开始

目的：为 chatwidget 的实现文件补充准确中文注释，并完整保留原有英文注释。

来源任务：无

预计会话范围：本任务只覆盖主聊天界面的状态、事件路由、工具生命周期、设置与插件等实现文件；文件清单与其它任务互斥。 共 65 个文件，改动类型统一为只添加注释，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-tui/src/chatwidget/command_lifecycle.rs`
- 修改：`crates/vw-tui/src/chatwidget/connector_mentions.rs`
- 修改：`crates/vw-tui/src/chatwidget/connectors.rs`
- 修改：`crates/vw-tui/src/chatwidget/constructor.rs`
- 修改：`crates/vw-tui/src/chatwidget/exec_state.rs`
- 修改：`crates/vw-tui/src/chatwidget/goal_menu.rs`
- 修改：`crates/vw-tui/src/chatwidget/goal_status.rs`
- 修改：`crates/vw-tui/src/chatwidget/hook_lifecycle.rs`
- 修改：`crates/vw-tui/src/chatwidget/hooks.rs`
- 修改：`crates/vw-tui/src/chatwidget/ide_context.rs`
- 修改：`crates/vw-tui/src/chatwidget/input_flow.rs`
- 修改：`crates/vw-tui/src/chatwidget/input_queue.rs`
- 修改：`crates/vw-tui/src/chatwidget/input_restore.rs`
- 修改：`crates/vw-tui/src/chatwidget/input_submission.rs`
- 修改：`crates/vw-tui/src/chatwidget/interaction.rs`
- 修改：`crates/vw-tui/src/chatwidget/interrupts.rs`
- 修改：`crates/vw-tui/src/chatwidget/keymap_picker.rs`
- 修改：`crates/vw-tui/src/chatwidget/mcp_startup.rs`
- 修改：`crates/vw-tui/src/chatwidget/misalignment_policy.rs`
- 修改：`crates/vw-tui/src/chatwidget/model_popups.rs`
- 修改：`crates/vw-tui/src/chatwidget/notifications.rs`
- 修改：`crates/vw-tui/src/chatwidget/permission_popups.rs`
- 修改：`crates/vw-tui/src/chatwidget/permission_shortcuts.rs`
- 修改：`crates/vw-tui/src/chatwidget/permissions_menu.rs`
- 修改：`crates/vw-tui/src/chatwidget/pets.rs`
- 修改：`crates/vw-tui/src/chatwidget/plan_implementation.rs`
- 修改：`crates/vw-tui/src/chatwidget/plugin_catalog.rs`
- 修改：`crates/vw-tui/src/chatwidget/plugins.rs`
- 修改：`crates/vw-tui/src/chatwidget/protocol.rs`
- 修改：`crates/vw-tui/src/chatwidget/protocol_requests.rs`
- 修改：`crates/vw-tui/src/chatwidget/rate_limits.rs`
- 修改：`crates/vw-tui/src/chatwidget/reasoning_shortcuts.rs`
- 修改：`crates/vw-tui/src/chatwidget/rendering.rs`
- 修改：`crates/vw-tui/src/chatwidget/replay.rs`
- 修改：`crates/vw-tui/src/chatwidget/reset_credits.rs`
- 修改：`crates/vw-tui/src/chatwidget/review.rs`
- 修改：`crates/vw-tui/src/chatwidget/review_popups.rs`
- 修改：`crates/vw-tui/src/chatwidget/safety_buffering.rs`
- 修改：`crates/vw-tui/src/chatwidget/service_tiers.rs`
- 修改：`crates/vw-tui/src/chatwidget/session_flow.rs`
- 修改：`crates/vw-tui/src/chatwidget/session_header.rs`
- 修改：`crates/vw-tui/src/chatwidget/settings.rs`
- 修改：`crates/vw-tui/src/chatwidget/settings_popups.rs`
- 修改：`crates/vw-tui/src/chatwidget/side.rs`
- 修改：`crates/vw-tui/src/chatwidget/skills.rs`
- 修改：`crates/vw-tui/src/chatwidget/slash_dispatch.rs`
- 修改：`crates/vw-tui/src/chatwidget/status_controls.rs`
- 修改：`crates/vw-tui/src/chatwidget/status_state.rs`
- 修改：`crates/vw-tui/src/chatwidget/status_surfaces.rs`
- 修改：`crates/vw-tui/src/chatwidget/streaming.rs`
- 修改：`crates/vw-tui/src/chatwidget/thread_usage.rs`
- 修改：`crates/vw-tui/src/chatwidget/tokens.rs`
- 修改：`crates/vw-tui/src/chatwidget/tokens/chart.rs`
- 修改：`crates/vw-tui/src/chatwidget/tokens/chart/palette.rs`
- 修改：`crates/vw-tui/src/chatwidget/tool_lifecycle.rs`
- 修改：`crates/vw-tui/src/chatwidget/tool_requests.rs`
- 修改：`crates/vw-tui/src/chatwidget/transcript.rs`
- 修改：`crates/vw-tui/src/chatwidget/transcript_export.rs`
- 修改：`crates/vw-tui/src/chatwidget/turn_lifecycle.rs`
- 修改：`crates/vw-tui/src/chatwidget/turn_runtime.rs`
- 修改：`crates/vw-tui/src/chatwidget/usage.rs`
- 修改：`crates/vw-tui/src/chatwidget/user_messages.rs`
- 修改：`crates/vw-tui/src/chatwidget/warnings.rs`
- 修改：`crates/vw-tui/src/chatwidget/windows_sandbox_prompts.rs`
- 修改：`crates/vw-tui/src/chatwidget/working_directory.rs`
- 测试：无新增测试；使用静态审计，并由最终任务统一运行 Clippy。

## 上下文

- `ChatWidget` 只拥有展示状态、历史、活动单元、输入队列与覆盖层；运行时请求和协议生命周期在 crate 外部。注释不得把展示层描述成业务执行层。
- 禁止每个文件只加一行概述：每个原有英文注释块都要相邻补充语义等价的中文；同时覆盖模块职责、关键类型与字段、跨模块 API、复杂状态/回退/安全/性能原因。
- 测试文件应覆盖每个独立测试或紧密相关的参数化测试组，说明场景、触发条件与预期；辅助 fixture 和非显然构造也要说明用途。
- 中文注释行数按文件规模设最低护栏：不超过 40 行至少 2 行，41–120 行至少 4 行，121–300 行至少 8 行，超过 300 行至少 12 行；达到数量仍必须通过语义审阅。
- 原有英文注释是必须保留的源码资产；在其前后补充中文，不删除、不翻译替换、不改写英文原文。
- 普通模块优先使用 `//!`；公开项使用 `///`；复杂状态、安全边界、回退或兼容原因使用克制的 `//`。

## 验证计划

- 测试驱动开发不适用：本任务只添加注释，不改变可执行行为、用户字符串、断言或快照。
- 分层覆盖检查：逐文件确认每个英文注释块已有中文对应，模块/API/复杂逻辑/测试行为组均按适用项覆盖；不得以文件级单行概述代替。
- 密度护栏：按文件有效行数检查最低中文注释行数；该检查只拦截明显敷衍，不能替代逐语义单元人工审阅。
- 保留检查：目标文件的 Git 差异不得出现被删除的非空源码行，从而证明原有英文注释和代码均未删除或替换。
- 差异检查：运行 `git diff --check`；人工确认新增内容只有中文注释及必要空行，并逐条核对注释与源码语义。
- 回归策略：本任务不运行 Cargo；批次 2 统一执行仓库要求的 Clippy。

## 步骤

1. 逐个读取目标文件、模块声明、相邻测试与必要调用者，确认真实职责和现有英文说明。
2. 逐个英文注释块相邻补充语义等价的中文说明，原英文内容与顺序保持不变。
3. 继续覆盖未被英文注释解释的模块职责、关键类型/字段、跨模块 API 及复杂状态、回退、安全、性能原因。
4. 测试文件按独立测试或紧密相关测试组说明场景、触发条件与预期，不复述断言。
5. 运行密度护栏、原行零删除和差异检查，再逐文件审阅语义覆盖清单；数量达标但语义遗漏仍不得完成。

## 验证

- 运行：
  ```zsh
  task_plan_file=".plans/2026-08-23-vw-tui-中文注释补全/2-chatwidget-实现代码-中文注释.md"
  comment_audit_failed=0
  while IFS= read -r rust_file; do
    source_lines="$(wc -l < "$rust_file" | tr -d ' ')"
    chinese_comment_lines="$(
      rg -n '//.*[一-龥]|^[[:space:]]*(/\*+|\*)[^[:cntrl:]]*[一-龥]' "$rust_file" |
        wc -l |
        tr -d ' '
    )"
    if (( source_lines <= 40 )); then
      minimum_comments=2
    elif (( source_lines <= 120 )); then
      minimum_comments=4
    elif (( source_lines <= 300 )); then
      minimum_comments=8
    else
      minimum_comments=12
    fi
    if (( chinese_comment_lines < minimum_comments )); then
      print -r -- "$rust_file: $chinese_comment_lines/$minimum_comments"
      comment_audit_failed=1
    fi
  done < <(sed -n 's/^- 修改：`\([^`]*\.rs\)`$/\1/p' "$task_plan_file")
  test "$comment_audit_failed" -eq 0

  removed_nonempty_lines="$(
    sed -n 's/^- 修改：`\([^`]*\.rs\)`$/\1/p' "$task_plan_file" |
      tr '\n' '\0' |
      xargs -0 git diff --unified=0 -- |
      rg '^-[^-].+' || true
  )"
  test -z "$removed_nonempty_lines"

  sed -n 's/^- 修改：`\([^`]*\.rs\)`$/\1/p' "$task_plan_file" |
    tr '\n' '\0' |
    xargs -0 git diff --check --
  ```
- 预期：密度护栏不输出路径；删除行检查为空；`git diff --check` 成功；所有命令退出码为 0。
- 所需证据：65 个目标均达到密度护栏，并通过英文注释块配对与逐语义单元审阅；无非空源码删除行；差异检查成功；人工审查确认原英文注释完整且没有非注释行为变更。

## 完成

仅当清单中的 65 个文件均完成英文注释块配对及模块/API/复杂逻辑或测试行为组的适用覆盖、所有原有英文注释原样保留、聚焦验证通过且差异仅含新增中文注释及必要空行时，才可标记为 `已完成`。
