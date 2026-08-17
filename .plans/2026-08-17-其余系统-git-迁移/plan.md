# 其余系统 Git 迁移计划

目标：将 vw-agent 余下两处生产系统 Git 调用迁为 git2。

范围：`memory/paths.rs` 的公共 Git 目录发现，以及 `tools/exit_worktree` 的脏工作树检测与 porcelain v1 输出；补充真实 git2 仓库测试。

范围外：测试 fixture 中的 Git CLI、其他 crate、工作树移除逻辑和 API schema 变更。

假设：vw-agent 已启用 git2；`git_repository::discover` 可提供规范化 `common_dir`；退出工具的 `status` JSON 字段需保持 porcelain v1 兼容。

## 设计决策

- memory 复用 `git_repository::discover`，避免重复仓库发现逻辑。
- exit_worktree 以 `StatusOptions` 读取状态，并在本模块显式生成当前测试覆盖的 porcelain v1 文本。

## 架构说明

- 相邻 git2 模式位于 `git_repository.rs`、`task_worktree_pool/mod.rs` 和 `git_operations/mod.rs`。
- 非 Git 目录必须继续返回清晰的状态读取错误，强制删除语义不变。

## 开发策略

- 对两个行为分别先写失败测试，再实施最小 git2 迁移。
- 使用 `git2::Repository::init`、index、commit 与 worktree API 建立真实仓库，不依赖系统 git。
- 每项聚焦测试通过后，执行 `cargo clippy --all-targets --all-features -- -D warnings`。
