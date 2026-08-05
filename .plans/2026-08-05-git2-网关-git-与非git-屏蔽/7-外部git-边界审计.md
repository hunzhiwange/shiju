# 任务 7: 外部 Git 边界审计

批次：【批次 4】依赖批次：批次 1、批次 2、批次 3

状态：未开始

目的：证明本计划覆盖的核心路径不再启动外部 Git，并明确仍依赖 Git CLI 的范围。

来源任务：无

预计会话范围：只做集成回归、失去调用者的代码清理和模块边界文档；不顺手迁移远程、submodule、skill 安装或普通 worktree 管理。

## 文件

- 修改：`crates/vw-agent/src/task_worktree_pool/mod.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/git.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/git_tests.rs`
- 修改：`crates/vw-agent/src/project/core.rs`
- 修改：`crates/vw-agent/src/project/vcs.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/project.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/instance.rs`
- 修改：`crates/vw-agent/src/file/git_status.rs`

## 上下文

- 本计划只承诺项目发现/元数据、任务池、网关 commit/merge 无外部 Git。
- `/v1/git/command`、普通 worktree 的 fetch/submodule、skill clone/pull、snapshot 和 CLI 状态仍可能使用 Git。
- 审计不得删除仍有真实调用者的通用命令协议，也不得宣称全产品无 Git 依赖。

## 验证计划

- 行为：清空 Git PATH 后，从项目 resolve 到 task pool、commit、merge 的核心集成流程通过；目标生产文件没有 Git 子进程调用。
- 集成验证：新增 `gateway_git2_core_flow_works_without_git_executable`，串联项目解析、提交和合并；复用任务 3 的池生命周期测试。
- 验证命令：`cargo test -p vw-agent gateway_git2_core_flow_works_without_git_executable && cargo test -p vw-agent task_worktree_pool_git2_lifecycle_works_without_git_executable`
- 适用说明：本任务只做审计、集成回归、死代码清理和文档收口，没有新的行为实现，因此不要求人为制造失败测试。
- 真实依赖策略：使用真实 git2 仓库和真实网关 handler；只隔离 PATH，不 mock 项目、index、worktree 或提交图。

## 步骤

1. 编写无 Git PATH 的核心集成测试，并记录首次运行结果。
2. 对计划范围内文件运行精确文本审计，定位残余 `git_std_command`、`Command::new("git")` 和通用 run_git helper。
3. 仅删除已无调用者的残余代码；仍属范围外的调用保留并在模块文档中明确边界。
4. 复查错误语义：非 Git、空仓库、签名缺失、worktree 冲突与 merge 冲突均为明确拒绝。
5. 运行集成测试、池测试和全目标 Clippy；审查差异未触及范围外模块。

## 验证

- 运行：`rg -n 'git_std_command|Command::new\\(\"git\"\\)|run_git_dynamic' crates/vw-agent/src/task_worktree_pool crates/vw-agent/src/project/core.rs crates/vw-agent/src/project/vcs.rs crates/vw-agent/src/file/git_status.rs crates/vw-agent/src/gateway/api/handlers/git.rs crates/vw-agent/src/gateway/api/handlers/project.rs crates/vw-agent/src/gateway/api/handlers/instance.rs`
- 运行：`cargo test -p vw-agent gateway_git2_core_flow_works_without_git_executable && cargo test -p vw-agent task_worktree_pool_git2_lifecycle_works_without_git_executable && cargo clippy --all-targets --all-features -- -D warnings`
- 预期：文本审计无匹配；两项集成测试和 Clippy 退出码为 0；范围外依赖在模块文档中准确列出。
- 所需证据：审计命令零匹配、测试名称与成功输出、Clippy 成功输出、差异清单未包含范围外模块。

## 完成

完成时可准确声明“核心本地任务 Git 路径无需系统 Git”，同时明确通用命令、远程与 submodule 能力仍需后续独立计划。
