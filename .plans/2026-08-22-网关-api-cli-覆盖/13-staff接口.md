# 任务 13: 覆盖 Staff 编排接口

批次：【批次 13】 依赖批次 12

状态：未开始

目的：提供能力、编排、投影、命令和自动排程计划的远程 CLI。

来源任务：无

预计会话范围：覆盖 staff stage/project/workflow capabilities、orchestration、projection、commands 和 auto-schedule-plan；staff IPC messages 明确排除。

## 文件

- 修改：`crates/vw-gateway-client/src/client/staff_api.rs`
- 测试：`crates/vw-gateway-client/src/client/staff_api_tests.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/staff.rs`
- 测试：`crates/vw-cli/src/handlers/api/staff_tests.rs`

## 上下文

- 所有业务请求经 `GatewayClient`，CLI 只负责参数、输入、确认和输出。
- 该任务完成时须在中央 `ApiCommands` 注册一次；因共享注册文件，依赖前一批次完成。
- 当前工作树的无关改动必须原样保留。
- 现有 staff client 已包含 project/workflow 路径构造。
- PUT capabilities/orchestration、commands 和 auto-schedule-plan 会改变调度状态；IPC 路由只供内部员工进程协议。

## 测试计划

- 行为：Staff CLI 区分 project/workflow 层级、正确编码 ID、保护写操作并拒绝 IPC 注入。
- 失败验证测试：`crates/vw-cli/src/handlers/api/staff_tests.rs` 中的 `api_staff_commands_cover_capability_orchestration_and_projection_routes`
- 失败验证命令：`cargo test -p vw-cli api_staff_commands_cover_capability_orchestration_and_projection_routes --lib`
- 预期失败原因：Staff client 尚无 CLI 命令表面和内部 IPC 分类。
- 通过验证命令：`cargo test -p vw-cli api_staff_commands_cover_capability_orchestration_and_projection_routes --lib`
- 模拟策略：本地 HTTP 服务器验证层级路径与 body；不模拟 staff 状态机。

## 步骤

1. 先写 stage、project、workflow、projection、command、auto-plan 和 IPC 排除测试。
2. 补齐 client 中缺失路径并复用现有 DTO。
3. 实现 `api staff` 子命令，清楚区分 project_id/workflow_id。
4. 所有 PUT/POST 编排动作确认；IPC 直接明确拒绝。
5. 运行聚焦测试与 clippy。

## 验证

- 运行：`cargo test -p vw-cli api_staff_commands_cover_capability_orchestration_and_projection_routes --lib`
- 运行：`cargo test -p vw-gateway-client staff_client_uses_workflow_orchestration_paths --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：层级路由、编码、确认和 IPC 排除断言通过，clippy 通过。
- 所需证据：失败时无命令；通过后用户面路由完整、内部 IPC 零请求；测试 0 失败；clippy 0 警告。

## 完成

`api staff` 提供明确的能力与编排管理面，不开放内部 IPC 生产者协议。

