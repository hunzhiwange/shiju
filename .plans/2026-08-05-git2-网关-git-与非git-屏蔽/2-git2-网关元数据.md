# 任务 2: Git2 网关元数据

批次：【批次 2】依赖批次：批次 1

状态：未开始

目的：让项目状态、变更统计、当前分支与 worktree DTO 完全通过 git2 读取。

来源任务：无

预计会话范围：只迁移只读元数据路径，保持现有 DTO 和状态映射，不处理提交、checkout、合并或池生命周期。

## 文件

- 修改：`crates/vw-agent/src/file/git_status.rs`
- 修改：`crates/vw-agent/src/file/git_status_tests.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/project.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/project_tests.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/instance.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/instance_tests.rs`

## 上下文

- `file::status_git` 当前执行三次 Git CLI；项目和实例 handler 仍用 `rev-parse`。
- `collect_project_change_records` 已使用 git2，可沿用其 diff/status 模式。
- 必须保留 Unicode、空文件、未终止行、删除与二进制统计的现有行为。

## 测试计划

- 行为：没有 Git 可执行文件时，`ProjectDto.git`、当前分支、worktree 分支与变更列表仍准确。
- 失败验证测试：新增 `gateway_git_metadata_works_without_git_executable`，并扩展现有 `status_git_*` 用例以使用 git2 fixture。
- 失败验证命令：`cargo test -p vw-agent gateway_git_metadata_works_without_git_executable`
- 预期失败原因：handler 的 `git_stdout`、`branch_for_worktree` 与 `status_git` 无法启动 Git。
- 通过验证命令：`cargo test -p vw-agent gateway_git_metadata_works_without_git_executable && cargo test -p vw-agent status_git_`
- 模拟策略：使用真实临时仓库和真实 git2 diff/status；只隔离进程 PATH。

## 步骤

1. 编写无 Git PATH 的网关元数据失败测试，并保留原状态统计断言。
2. 运行失败验证命令，确认当前分支或变更状态缺失。
3. 用 git2 statuses、tree/workdir diff 与 HEAD shorthand 替代外部命令。
4. 保持非 Git 与缺失路径返回空元数据，不扩大错误面。
5. 运行聚焦测试、相关状态测试与全目标 Clippy。

## 验证

- 运行：`cargo test -p vw-agent gateway_git_metadata_works_without_git_executable && cargo test -p vw-agent status_git_ && cargo clippy --all-targets --all-features -- -D warnings`
- 预期：元数据和状态测试通过；Clippy 退出码为 0；目标文件不再引用 `git_std_command`。
- 所需证据：实现前失败输出、实现后测试数量与退出码、`rg -n 'git_std_command' crates/vw-agent/src/file/git_status.rs crates/vw-agent/src/gateway/api/handlers/project.rs crates/vw-agent/src/gateway/api/handlers/instance.rs` 无匹配。

## 完成

完成时网关项目 DTO、实例 VCS 和文件状态均由 git2 读取，行为测试覆盖无 Git PATH，且不改变公开 DTO。
