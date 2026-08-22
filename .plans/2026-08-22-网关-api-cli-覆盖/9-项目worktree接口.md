# 任务 9: 覆盖项目与 Worktree 接口

批次：【批次 9】 依赖批次 8

状态：未开始

目的：为项目解析、详情、变更记录和 worktree 生命周期提供远程 CLI。

来源任务：无

预计会话范围：覆盖 projects list/resolve/detail/update/change-records 与 worktree list/create/detail/delete/reset，一个接口族内只有一个主要项目资源验证故事。

## 文件

- 修改：`crates/vw-gateway-client/src/client/project_api.rs`
- 测试：`crates/vw-gateway-client/src/client/project_api_tests.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/project.rs`
- 测试：`crates/vw-cli/src/handlers/api/project_tests.rs`

## 上下文

- 所有业务请求经 `GatewayClient`，CLI 只负责参数、输入、确认和输出。
- 该任务完成时须在中央 `ApiCommands` 注册一次；因共享注册文件，依赖前一批次完成。
- 当前工作树的无关改动必须原样保留。
- `project_api.rs` 已覆盖多数项目/worktree 路径，应补缺不重写。
- update/create/delete/reset 有副作用；project_id 与 worktree_id 必须分别编码，不能接受路径穿越。

## 测试计划

- 行为：项目和 worktree 子命令使用正确资源 ID、directory 与确认语义，删除/reset 未确认时零请求。
- 失败验证测试：`crates/vw-cli/src/handlers/api/project_tests.rs` 中的 `api_project_commands_cover_project_and_worktree_lifecycle`
- 失败验证命令：`cargo test -p vw-cli api_project_commands_cover_project_and_worktree_lifecycle --lib`
- 预期失败原因：尚无 `api project` 命令映射现有 GatewayClient 能力。
- 通过验证命令：`cargo test -p vw-cli api_project_commands_cover_project_and_worktree_lifecycle --lib`
- 模拟策略：本地 HTTP 服务器验证资源路径、查询和 body，不模拟项目存储或 Git worktree。

## 步骤

1. 先写十个逻辑操作的表驱动失败测试。
2. 补齐 GatewayClient 中缺失的 change-record/detail 方法并复用现有类型。
3. 实现 `api project` 子命令，复杂写入从 JSON 输入读取。
4. update/create/delete/reset 走共享确认，错误保持明确。
5. 运行客户端与 CLI 聚焦测试和 clippy。

## 验证

- 运行：`cargo test -p vw-cli api_project_commands_cover_project_and_worktree_lifecycle --lib`
- 运行：`cargo test -p vw-gateway-client project_api_uses_project_worktree_routes --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：全部逻辑路由、ID 编码和确认断言通过，clippy 通过。
- 所需证据：失败时命令缺失；通过后路由矩阵成功、危险分支零请求；测试 0 失败；clippy 0 警告。

## 完成

`api project` 覆盖项目与 worktree 的完整用户生命周期。

