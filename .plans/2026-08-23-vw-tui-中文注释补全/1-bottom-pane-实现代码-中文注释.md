# 任务 1: 补充 bottom_pane 实现代码的中文注释

批次：【批次 1】 无

状态：未开始

目的：为 bottom_pane 的实现文件补充准确中文注释，并完整保留原有英文注释。

来源任务：无

预计会话范围：本任务只覆盖输入编辑器、补全弹窗、选择器、审批与用户输入覆盖层等 bottom_pane 实现文件；文件清单与其它任务互斥。 共 63 个文件，改动类型统一为只添加注释，适合一个聚焦会话。

## 文件

- 修改：`crates/vw-tui/src/bottom_pane/action_required_title.rs`
- 修改：`crates/vw-tui/src/bottom_pane/app_link_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/apply_patch_header.rs`
- 修改：`crates/vw-tui/src/bottom_pane/approval_overlay.rs`
- 修改：`crates/vw-tui/src/bottom_pane/bottom_pane_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer/attachment_state.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer/completion_target.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer/draft_state.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer/footer_state.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer/history_search.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer/popup_state.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer/slash_input.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer_history.rs`
- 修改：`crates/vw-tui/src/bottom_pane/chat_composer_history/search_batch.rs`
- 修改：`crates/vw-tui/src/bottom_pane/command_popup.rs`
- 修改：`crates/vw-tui/src/bottom_pane/custom_prompt_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/effort_ignition.rs`
- 修改：`crates/vw-tui/src/bottom_pane/effort_ignition_styles.rs`
- 修改：`crates/vw-tui/src/bottom_pane/effort_status_line.rs`
- 修改：`crates/vw-tui/src/bottom_pane/experimental_features_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/feedback_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/file_search_popup.rs`
- 修改：`crates/vw-tui/src/bottom_pane/footer.rs`
- 修改：`crates/vw-tui/src/bottom_pane/hooks_browser_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/list_selection_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mcp_server_elicitation.rs`
- 修改：`crates/vw-tui/src/bottom_pane/memories_settings_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mentions_v2/candidate.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mentions_v2/filter.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mentions_v2/footer.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mentions_v2/mod.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mentions_v2/popup.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mentions_v2/render.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mentions_v2/search_catalog.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mentions_v2/search_mode.rs`
- 修改：`crates/vw-tui/src/bottom_pane/mod.rs`
- 修改：`crates/vw-tui/src/bottom_pane/multi_select_picker.rs`
- 修改：`crates/vw-tui/src/bottom_pane/paste_burst.rs`
- 修改：`crates/vw-tui/src/bottom_pane/pending_input_preview.rs`
- 修改：`crates/vw-tui/src/bottom_pane/pending_thread_approvals.rs`
- 修改：`crates/vw-tui/src/bottom_pane/popup_consts.rs`
- 修改：`crates/vw-tui/src/bottom_pane/prompt_args.rs`
- 修改：`crates/vw-tui/src/bottom_pane/request_user_input/layout.rs`
- 修改：`crates/vw-tui/src/bottom_pane/request_user_input/mod.rs`
- 修改：`crates/vw-tui/src/bottom_pane/request_user_input/render.rs`
- 修改：`crates/vw-tui/src/bottom_pane/scroll_state.rs`
- 修改：`crates/vw-tui/src/bottom_pane/selection_popup_common.rs`
- 修改：`crates/vw-tui/src/bottom_pane/selection_row_layout.rs`
- 修改：`crates/vw-tui/src/bottom_pane/selection_tabs.rs`
- 修改：`crates/vw-tui/src/bottom_pane/skill_popup.rs`
- 修改：`crates/vw-tui/src/bottom_pane/skills_toggle_view.rs`
- 修改：`crates/vw-tui/src/bottom_pane/slash_commands.rs`
- 修改：`crates/vw-tui/src/bottom_pane/startup.rs`
- 修改：`crates/vw-tui/src/bottom_pane/status_line_setup.rs`
- 修改：`crates/vw-tui/src/bottom_pane/status_line_style.rs`
- 修改：`crates/vw-tui/src/bottom_pane/status_surface_preview.rs`
- 修改：`crates/vw-tui/src/bottom_pane/textarea.rs`
- 修改：`crates/vw-tui/src/bottom_pane/textarea/vim.rs`
- 修改：`crates/vw-tui/src/bottom_pane/textarea/vim_commands.rs`
- 修改：`crates/vw-tui/src/bottom_pane/textarea/wrapping.rs`
- 修改：`crates/vw-tui/src/bottom_pane/title_setup.rs`
- 修改：`crates/vw-tui/src/bottom_pane/unified_exec_footer.rs`
- 测试：无新增测试；使用静态审计，并由最终任务统一运行 Clippy。

## 上下文

- 该目录受 `crates/vw-tui/src/bottom_pane/AGENTS.md` 约束；修改 `chat_composer.rs`、`paste_burst.rs` 及相关状态机时，中文说明必须与 Enter、新行、粘贴突发和 `disable_paste_burst` 的实际语义一致。
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
  task_plan_file=".plans/2026-08-23-vw-tui-中文注释补全/1-bottom-pane-实现代码-中文注释.md"
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
- 所需证据：63 个目标均达到密度护栏，并通过英文注释块配对与逐语义单元审阅；无非空源码删除行；差异检查成功；人工审查确认原英文注释完整且没有非注释行为变更。

## 完成

仅当清单中的 63 个文件均完成英文注释块配对及模块/API/复杂逻辑或测试行为组的适用覆盖、所有原有英文注释原样保留、聚焦验证通过且差异仅含新增中文注释及必要空行时，才可标记为 `已完成`。
