# 任务 6: 非 Git 桌面屏蔽

批次：【批次 3】依赖批次：批次 1、批次 2

状态：未开始

目的：让桌面端缓存项目 Git 能力，并对非 Git 项目停止 worktree 请求和自动执行入口。

来源任务：无

预计会话范围：只改项目加载状态、任务看板调度与 worktree 控件；任务看板的创建、编辑和规划功能继续可用。

## 文件

- 修改：`crates/vw-desktop/src/app/state/app_state.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/new.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/new_tests.rs`
- 修改：`crates/vw-desktop/src/app/projects.rs`
- 修改：`crates/vw-desktop/src/app/projects_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/project/session/open.rs`
- 修改：`crates/vw-desktop/src/app/message/project/session/open_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/task_board/helpers/execution.rs`
- 修改：`crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/task_board/update/execution.rs`
- 修改：`crates/vw-desktop/src/app/message/task_board/update/execution_tests.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/control.rs`
- 修改：`crates/vw-desktop/src/app/views/task_board/control_tests.rs`

## 上下文

- `LoadedProjectInfo.info.vcs` 已包含网关 `ProjectDto.git.is_repo`，但当前 handler 丢弃该能力。
- snapshot 只检查 `project_path`，所以非 Git 项目每 5/15 秒请求并记录 400。
- scheduler 的本地执行强制租赁 worktree；非 Git 下必须拒绝启动，而不是退回项目根目录。

## 测试计划

- 行为：非 Git 项目不请求 snapshot/maintain/acquire/merge，不启动执行器，不显示 worktree 控件；Git 项目行为不变，项目切换会清空能力和旧快照。
- 失败验证测试：新增 `non_git_project_skips_worktree_snapshot_refresh`、`non_git_project_rejects_task_execution_with_toast` 与 `non_git_project_hides_worktree_controls`。
- 失败验证命令：`cargo test -p vw-desktop non_git_project_skips_worktree_snapshot_refresh`
- 预期失败原因：当前只要有 `project_path` 就启动 snapshot 请求并允许 `StartExecution`。
- 通过验证命令：`cargo test -p vw-desktop non_git_project_`
- 模拟策略：直接构造真实 App 状态和 iced Task 消息，不模拟 Git；网关项目映射测试提供 `is_repo` 真/假 DTO。

## 步骤

1. 编写非 Git snapshot、执行器和控件失败测试，并补充 Git 项目回归断言。
2. 运行失败验证，确认加载标记或执行状态被错误开启。
3. 在 App 中增加明确的当前项目 Git 能力字段，初始化/切换时为 false，`ProjectInfoLoaded` 后按 `info.vcs` 设置。
4. 在 snapshot/maintenance/manual merge/recycle 与 `StartExecution`、`ExecuteTask` 入口统一守卫；反馈使用 toast。
5. worktree 控件仅对 Git 项目渲染；非 Git 保留任务规划界面，不自动初始化仓库。
6. 处理在项目切换后迟到的 snapshot 响应，确保不会恢复旧状态。
7. 运行聚焦桌面测试与全目标 Clippy。

## 验证

- 运行：`cargo test -p vw-desktop non_git_project_ && cargo clippy --all-targets --all-features -- -D warnings`
- 预期：所有非 Git 测试通过；Git 回归断言通过；不再产生 task-pool snapshot 400；Clippy 退出码为 0。
- 所需证据：实现前请求/执行错误状态；实现后 iced Task 不包含网关调用、执行器保持停止、toast/控件断言和 Clippy 成功输出。

## 完成

完成时非 Git 项目默认屏蔽所有依赖 worktree 的任务执行能力，不污染日志、不修改客户目录，且 Git 项目流程保持可用。
