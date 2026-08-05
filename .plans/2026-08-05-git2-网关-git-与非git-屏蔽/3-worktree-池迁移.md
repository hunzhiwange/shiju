# 任务 3: Worktree 池迁移

批次：【批次 2】依赖批次：批次 1

状态：未开始

目的：将任务 worktree 池的发现、创建、checkout、reset、clean、状态与回收全部迁移到 git2。

来源任务：无

预计会话范围：仅处理 `TaskWorktreePoolService` 及其独立测试；不迁移普通 worktree 模块、远程 fetch、submodule 或桌面批量管理。

## 文件

- 修改：`crates/vw-agent/src/task_worktree_pool/mod.rs`
- 修改：`crates/vw-agent/src/task_worktree_pool/tests.rs`

## 上下文

- 当前池通过 `git worktree list/add`、`reset --hard`、`clean -fd`、`checkout -B` 与 `status` 工作。
- git2 支持 worktree add/list、branch、reset、checkout 和 statuses；clean 需仅删除未跟踪且未忽略的仓库内路径。
- git2 Repository 不跨 await 保存；所有阻塞操作须在 `spawn_blocking` 内按路径重开。

## 测试计划

- 行为：无 Git PATH 时，池可完成 snapshot、maintain、acquire、重复 acquire、recycle，并拒绝空仓库和跨 worktree 分支冲突。
- 失败验证测试：新增 `task_worktree_pool_git2_lifecycle_works_without_git_executable` 与 `task_worktree_pool_requires_initial_commit_for_unborn_repo`。
- 失败验证命令：`cargo test -p vw-agent task_worktree_pool_git2_lifecycle_works_without_git_executable`
- 预期失败原因：当前 `run_git` 无法启动外部 Git。
- 通过验证命令：`cargo test -p vw-agent task_worktree_pool_git2_ && cargo test -p vw-agent task_worktree_pool_requires_initial_commit_for_unborn_repo`
- 模拟策略：fixture 全部改用 git2 创建仓库、分支、提交与工作树；环境锁下移除 Git PATH，使用真实文件系统验证 clean。

## 步骤

1. 先编写完整池生命周期与空仓库失败测试。
2. 运行失败验证并确认外部 Git 缺失导致池失败。
3. 用 git2 枚举 linked worktree、解析分支与基线提交，并用 `WorktreeAddOptions` 创建 slot。
4. 用 branch/reference、hard reset、forced checkout 与安全未跟踪清理实现 prepare/recycle；保留忽略文件和仓库边界。
5. 删除 `GitOutput`、`run_git*` 与 porcelain 解析等失去调用者的 CLI 辅助代码。
6. 运行聚焦测试、既有池测试过滤器和全目标 Clippy。

## 验证

- 运行：`cargo test -p vw-agent task_worktree_pool_git2_ && cargo test -p vw-agent task_worktree_pool_requires_initial_commit_for_unborn_repo && cargo clippy --all-targets --all-features -- -D warnings`
- 预期：池生命周期在无 Git PATH 下通过；空仓库返回明确初始提交错误；Clippy 退出码为 0。
- 所需证据：实现前外部 Git 启动失败；实现后 slot 路径、分支、状态、清理和幂等断言通过；`rg -n 'git_std_command|run_git_dynamic' crates/vw-agent/src/task_worktree_pool` 无匹配。

## 完成

完成时 `TaskWorktreePoolService` 不启动 Git 子进程，安全保持原租约/回收语义，并对非 Git、空仓库和不健康 slot 返回明确错误。
