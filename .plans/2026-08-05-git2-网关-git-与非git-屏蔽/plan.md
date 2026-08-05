# Git2 网关 Git 与非 Git 屏蔽计划

目标：让项目识别、任务 worktree 池、网关提交与合并不依赖外部 Git，并对非 Git 项目默认屏蔽相关能力。

范围：以 `git2` 取代项目发现、分支/状态元数据、任务 worktree 池、`/v1/git/commit` 与 `/v1/git/merge` 的外部 Git 子进程；桌面端保存项目 Git 能力并停止非 Git 项目的池轮询、维护、执行和合并入口；保留网关对直接非法请求的明确拒绝。

范围外：不自动执行 `git init` 或创建初始提交；不迁移 clone、fetch、pull、push、submodule、LFS、skill 安装及普通 worktree 管理中的远程能力；不承诺本阶段完全删除系统 Git 依赖。

假设：`vw-agent` 与 `vw-desktop` 已启用 vendored `git2 0.21.0`；任务池只管理 `slot-*` 目录；非 Git 项目仍可使用任务看板做规划，但不能启动依赖隔离 worktree 的自动执行。

## 设计决策

- 选择 git2 本地后端；相比捆绑 Git 可减少发行物，且比混合回退更能暴露遗漏路径。
- 非 Git 不自动初始化，避免提交密钥、构建产物或改写客户目录；只显示不可用状态。
- 空 Git 仓库保留 Git 身份，但任务池返回“需要初始提交”；不把它误报为非 Git。
- git2 创建用户提交时不执行 Git hooks；需要 hooks 的仓库仍属于外部 Git 边界。
- 通用 `/v1/git/command`、远程与 submodule 操作暂留，不能据此宣称产品已完全不需要 Git。

## 架构说明

- 新增窄范围仓库发现模块，领域写操作仍留在 project、task pool 和 gateway handler 内。
- git2 对象不跨 `await` 或共享锁保存；阻塞文件操作通过 `spawn_blocking` 重开仓库。
- 先由网关 `ProjectDto.git.is_repo` 判定能力，桌面端缓存该值；服务端继续安全默认拒绝。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
- 每个任务完成后执行仓库要求的全目标 Clippy；不执行全量测试、格式检查或 Makefile 命令。
