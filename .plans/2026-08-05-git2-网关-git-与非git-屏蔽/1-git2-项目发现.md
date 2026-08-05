# 任务 1: Git2 项目发现

批次：【批次 1】依赖批次：无

状态：未开始

目的：用 git2 发现仓库、稳定项目 ID 与当前分支，使已提交及空 Git 仓库在没有 Git 可执行文件时仍被识别。

来源任务：无

预计会话范围：聚焦项目发现与 VCS 分支跟踪，不迁移状态、提交、合并或 worktree 写操作；新增共享模块只提供本任务已有调用者使用的发现能力。

## 文件

- 新建：`crates/vw-agent/src/git_repository.rs`
- 新建：`crates/vw-agent/src/git_repository_tests.rs`
- 修改：`crates/vw-agent/src/lib.rs`
- 修改：`crates/vw-agent/src/project/core.rs`
- 修改：`crates/vw-agent/src/project/core_tests.rs`
- 修改：`crates/vw-agent/src/project/vcs.rs`
- 修改：`crates/vw-agent/src/project/vcs_tests.rs`

## 上下文

- `project::from_directory` 当前依赖 `.git` 文件探测、`git --version`、`rev-list` 与 `rev-parse`。
- git2 提供 `Repository::discover`、`workdir`、`commondir`、`head` 与 revwalk。
- 空仓库须写入稳定缓存 ID，避免首个提交后项目 ID 漂移；禁止调用 `Repository::init`。

## 测试计划

- 行为：PATH 中没有 Git 时，已提交仓库、linked worktree 与空仓库仍映射为 `Vcs::Git`，且空仓库再次发现保持同一 ID。
- 失败验证测试：在 `project/core_tests.rs` 新增 `git2_project_discovery_works_without_git_executable`；在 `project/vcs_tests.rs` 新增 `git2_branch_tracking_works_without_git_executable`。
- 失败验证命令：`cargo test -p vw-agent git2_project_discovery_works_without_git_executable`
- 预期失败原因：当前 `git_available()` 返回 false，项目被写成 `vcs: None`。
- 通过验证命令：`cargo test -p vw-agent git2_project_discovery_works_without_git_executable && cargo test -p vw-agent git2_branch_tracking_works_without_git_executable`
- 模拟策略：使用 git2 创建真实临时仓库、提交和 linked worktree；用仓库全局环境锁临时移除 Git PATH，不模拟 Repository。

## 步骤

1. 编写失败测试，覆盖已提交仓库、空仓库、linked worktree 和无 Git PATH。
2. 运行失败验证命令并确认 VCS 判定失败。
3. 新增只读 git2 发现函数，并替换 `git_available`、`roots_from_git`、`show_toplevel`、`git_common_dir_root` 与分支子进程。
4. 保持已有根提交 ID 规则；空仓库使用路径 ID 并写入 common git dir 的 `vibewindow` 缓存。
5. 运行聚焦测试与全目标 Clippy，仅在测试保持通过时整理重复代码。

## 验证

- 运行：`cargo test -p vw-agent git2_project_discovery_works_without_git_executable && cargo test -p vw-agent git2_branch_tracking_works_without_git_executable && cargo clippy --all-targets --all-features -- -D warnings`
- 预期：两个聚焦测试通过；Clippy 退出码为 0；生产路径不再通过外部 Git 判定仓库或当前分支。
- 所需证据：失败测试在实现前复现 `vcs: None`；实现后测试与 Clippy 均成功；差异中不存在自动初始化调用。

## 完成

完成时应存在 `git_repository.rs` 的只读发现 API，`project/core.rs` 与 `project/vcs.rs` 不再启动 Git 子进程，空仓库身份稳定，且上述验证证据齐全。
