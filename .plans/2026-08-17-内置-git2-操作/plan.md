# 内置 git2 操作计划

目标：将 GitOperationsTool 改为直接使用 git2，使该工具不依赖系统 Git 可执行文件。

范围：仅迁移 `crates/vw-agent/src/tools/git_operations/` 的八项公开操作及其测试；审计 vw-agent 的其余生产代码中仍启动系统 Git 的位置并报告，不在本计划中迁移它们。

范围外：不改变工具 schema、权限/限流语义、其他 Git API 或测试 fixture；不进行全仓库 Git CLI 消除。

假设：`vw-agent` 已声明启用 vendored-libgit2 的 git2 0.21.0；现有 JSON 输出字段为对外契约，应保持兼容。

## 设计决策

- 用 git2 的 `Repository`、`StatusOptions`、`Diff`、`Revwalk`、`Branch`、`Index`、`CheckoutBuilder` 与 stash API 直接实现允许的操作。
- 移除仅为 shell 调用服务的参数净化与字符串输出解析，改为对路径、分支、stash 索引施加等价的显式输入约束。
- 保留当前安全策略，仓库检测改用 `Repository::discover` 以支持子目录和 worktree。

## 架构说明

- 可参考既有 `file/git_status.rs`、`git_repository.rs` 与 `task_worktree_pool/mod.rs` 的 git2 使用模式。
- libgit2 与 CLI 在 hook、配置和输出细节上并不完全等价；实现应仅承诺现有 schema 表达的行为。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 先用 git2 在临时目录创建真实仓库和提交，使关键测试在没有 `git` 二进制时仍然可运行。
- 完成后运行目标测试与 `cargo clippy --all-targets --all-features -- -D warnings`；不运行仓库禁止的全量测试或格式化检查。
