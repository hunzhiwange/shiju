# 任务 10: vw-tui 中文注释、英文保留与 Clippy 审计

批次：【批次 2】 依赖批次 1

状态：未开始

目的：统一证明 vw-tui 全部 Rust 文件已按语义单元完成中文说明、原英文注释逐块保留，并通过仓库 Clippy 门槛。

来源任务：无

预计会话范围：本任务不新增注释，只审计前九个互斥任务的合并结果；检查范围固定为 `crates/vw-tui` 的 374 个 Rust 文件。

## 文件

- 修改：无；若审计发现遗漏或删除，只记录阻塞证据并返回对应批次 1 任务修复，不在本任务越界修改。
- 测试：`crates/vw-tui` 全部 Rust 文件及 workspace 全特性编译面。

## 上下文

- 批次 1 应覆盖全部 374 个文件；`live_wrap.rs` 中“你/好”只是英文示例字符，不能视为已有中文说明。
- 原有英文注释必须全部保留；批次 1 采用只新增注释策略，因此最终差异不应包含任何被删除的非空源码行。
- 文件行数密度仅是最低护栏；最终必须逐文件审阅英文注释块配对，以及模块/API/复杂逻辑或测试行为组的适用覆盖。
- 仓库协议要求代码变更后运行 `cargo clippy --all-targets --all-features -- -D warnings`；不运行全量单元测试、fmt、wasm check 或 Makefile 命令。

## 验证计划

- 测试驱动开发不适用：本任务是只读审计。
- 分层覆盖：确认 374 个 Rust 文件均达到最低密度，并逐文件确认不是单行打卡；每个英文注释块都有相邻中文对应，模块/API/复杂逻辑或测试行为组均按适用项覆盖。
- 英文保留：确认 `git diff --unified=0 -- crates/vw-tui` 不含被删除的非空源码行，并逐文件审阅原英文注释相邻新增方式。
- 编译审计：运行 workspace 全目标、全特性 Clippy，并将无关既有失败与本次引入失败区分记录。

## 步骤

1. 确认批次 1 的九个任务均已完成并删除；若仍存在任务文件，则停止并记录依赖未满足。
2. 对 `crates/vw-tui` 全部 Rust 文件运行分层密度审计，输出低于最低护栏的路径。
3. 逐文件审查 Git 差异，确认每个英文注释块已有中文对应，适用语义单元均获说明，且不存在删除行、英文改写或非注释行为变更。
4. 运行 `git diff --check` 与仓库规定的 Clippy 命令。
5. 若失败来自本次改动，返回对应领域重新建立聚焦修复任务；若仅因无关既有改动失败，记录证据并标记 `已完成，待回归`。

## 验证

- 运行：
  ```zsh
  test "$(rg --files crates/vw-tui -g '*.rs' | wc -l | tr -d ' ')" -eq 374

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
  done < <(rg --files crates/vw-tui -g '*.rs' | sort)
  test "$comment_audit_failed" -eq 0

  removed_nonempty_lines="$(
    git diff --unified=0 -- crates/vw-tui |
      rg '^-[^-].+' || true
  )"
  test -z "$removed_nonempty_lines"

  git diff --check -- crates/vw-tui
  cargo clippy --all-targets --all-features -- -D warnings
  ```
- 预期：文件数为 374；密度护栏不输出路径；删除行检查为空；差异检查与 Clippy 均以退出码 0 结束。
- 所需证据：374/374 文件达到密度护栏；逐文件审阅确认英文注释块全部配对、适用语义单元全部覆盖且不存在单行打卡；无被删除的非空源码行；`git diff --check` 和 Clippy 成功；若为无关失败，保存明确的错误位置与归属证据。

## 完成

只有分层密度、逐语义单元审阅、英文逐块保留、差异和 Clippy 审计全部成功才能标记为 `已完成`；若 Clippy 仅被无关既有改动阻断，可记录完整证据并标记为 `已完成，待回归`。
