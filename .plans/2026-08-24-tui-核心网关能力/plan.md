# TUI 核心网关能力补齐计划

目标：补齐适合终端交互、Gateway 已有稳定契约但活动 TUI 尚无调用链的核心工作区与运维能力。

范围：接通工作目录切换、Gateway 诊断、只读文件浏览、Hook 目录与审计、Hook 信任管理、记忆查询与记忆写删；更新 TUI 能力矩阵。沿用 V3 纯数据边界、ActiveRunner 非阻塞宿主和稳定 ID 校验。

范围外：现有 `@` 文件搜索、结构化工具 diff 与待完成 `/diff` 计划；文件写删、全局配置、Provider 认证、知识库、PTY、APIQL、Shiju 账号/市场和桌面专属能力。

假设：当前未提交的 `tui-at-file-search` 与 `tui-file-diff` 改动是实施基线；Gateway client 已提供 `project_resolve`、文件只读、`doctor_get`、Hook 与 memory API；活动入口仍为 `V3GatewayApp`。

## 设计决策

- 只接入已有强契约且适合终端的能力；不把全部 Gateway 管理 API 暴露为 TUI 命令。
- 所有网络请求由 CLI runtime/runner 执行，`vw-tui` 只接收纯展示数据与稳定选择 ID。
- 目录切换、Hook 信任/撤销和记忆删除均先校验活动快照；破坏性操作必须确认。
- 文件能力先只读，路径由当前 Gateway 目录约束；不在 TUI 直接执行文件系统写操作。

## 架构说明

- `commands.rs` 负责稳定命令语法，`app.rs` 负责异步宿主与原子状态提交，`runtime/*` 隔离 Gateway DTO。
- 复用现有 selection/confirm overlay；错误进入 transcript，迟到结果不得覆盖新目录或新请求。
- 当前 287 个 GatewayClient 异步方法中仅约 40 个名称出现在 TUI 源码，不能以客户端方法存在代替活动可达证据。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
- 仓库禁止主动执行 `cargo test`；用全量 clippy 编译所有新增测试并在交接中注明未运行测试。
