# 任务 17: 覆盖 Session 核心生命周期

批次：【批次 17】 依赖批次 16

状态：未开始

目的：提供会话列表、创建、状态、详情、更新、删除、子会话、todo、fork、reset、diff、summarize 和标题生成。

来源任务：无

预计会话范围：只处理会话核心资源和生命周期，不处理消息 part、UI 快照、归档或 scope；一个聚焦会话可验证完整 core 路由矩阵。

## 文件

- 修改：`crates/vw-gateway-client/src/client/session_api.rs`
- 测试：`crates/vw-gateway-client/src/client/session_api_tests.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/session.rs`
- 测试：`crates/vw-cli/src/handlers/api/session_tests.rs`

## 上下文

- 所有业务请求经 `GatewayClient`，复杂 body 从共享 JSON 输入读取，输出走共享 renderer。
- 该接口族需顺序注册到 `handlers/api/mod.rs`，不得修改 `plan.md`。
- 保留当前工作树中与本任务无关的 SHIJU 和桌面改动。
- `session_api.rs` 已封装大部分生命周期方法和 `directory_query`。
- create/update/delete/todo/fork/reset/summarize/title 有写入或模型成本，必须确认；删除/reset 明确显示 session_id。

## 测试计划

- 行为：Session 核心命令保留 directory、消息锚点和真实返回，危险操作无确认时零请求。
- 失败验证测试：`crates/vw-cli/src/handlers/api/session_tests.rs` 中的 `api_session_commands_cover_core_lifecycle_routes`
- 失败验证命令：`cargo test -p vw-cli api_session_commands_cover_core_lifecycle_routes --lib`
- 预期失败原因：现有 Session client 只由 UI 调用，远程 CLI 尚不存在。
- 通过验证命令：`cargo test -p vw-cli api_session_commands_cover_core_lifecycle_routes --lib`
- 模拟策略：本地 HTTP 服务器返回最小 session DTO、todo、diff 和 title；不模拟持久化或模型。

## 步骤

1. 先写核心生命周期每个逻辑操作的失败路由测试。
2. 补齐 session client 缺失方法并复用已有 body/query 类型。
3. 实现 `api session` 核心子命令，ID 与 directory 分开处理。
4. 所有写入和成本操作确认，读取保持 JSON。
5. 运行客户端与 CLI 聚焦测试及 clippy。

## 验证

- 运行：`cargo test -p vw-cli api_session_commands_cover_core_lifecycle_routes --lib`
- 运行：`cargo test -p vw-gateway-client session_client_uses_core_lifecycle_routes --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：核心路由、query、body、确认和错误传播通过，clippy 通过。
- 所需证据：失败时无命令；通过后核心路由矩阵及零请求拒绝断言成功；测试 0 失败；clippy 0 警告。

## 完成

`api session` 的 core 命令覆盖会话核心生命周期，危险动作目标明确。

