# 任务 15: 覆盖 Workflow 应用接口

批次：【批次 15】 依赖批次 14

状态：未开始

目的：提供 workflow 恢复、应用 CRUD 和应用聊天流的远程 CLI。

来源任务：无

预计会话范围：覆盖 workflow resume、applications list/create/detail/update/delete 与两种 chat-messages；只处理 workflow 领域流，不含通用 agent/chat。

## 文件

- 修改：`crates/vw-gateway-client/src/client/workflow_api.rs`
- 测试：`crates/vw-gateway-client/src/client/workflow_api_tests.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/workflow.rs`
- 测试：`crates/vw-cli/src/handlers/api/workflow_tests.rs`

## 上下文

- 所有业务请求经 `GatewayClient`，CLI 只负责参数、输入、确认和输出。
- 该任务完成时须在中央 `ApiCommands` 注册一次；因共享注册文件，依赖前一批次完成。
- 当前工作树的无关改动必须原样保留。
- `workflow_api.rs` 已有 SSE event 类型与 stream 方法，应复用真实终止/暂停/error 事件。
- create/update/delete/resume/chat 都有副作用或计算成本；流式输出必须为逐行 JSON，不缓存整流。

## 测试计划

- 行为：Workflow CRUD 使用正确路由，聊天流逐事件 JSONL 输出并保留终止语义，危险操作受确认。
- 失败验证测试：`crates/vw-cli/src/handlers/api/workflow_tests.rs` 中的 `api_workflow_commands_cover_application_crud_resume_and_stream`
- 失败验证命令：`cargo test -p vw-cli api_workflow_commands_cover_application_crud_resume_and_stream --lib`
- 预期失败原因：现有 workflow client 被桌面/TUI 使用，CLI 尚无应用管理和 JSONL 流出口。
- 通过验证命令：`cargo test -p vw-cli api_workflow_commands_cover_application_crud_resume_and_stream --lib`
- 模拟策略：本地 HTTP/SSE 服务器发送真实 framing 与终止事件；不模拟 workflow runner。

## 步骤

1. 先写 CRUD、resume、默认/UUID chat stream 的失败测试。
2. 补齐 client 缺失方法，保持现有 SSE 解析器。
3. 实现 `api workflow`，流式事件直接交给 JSONL writer。
4. 所有 POST/PUT/DELETE 和 chat 走确认；中断返回非零或明确停止结果。
5. 运行聚焦测试与 clippy。

## 验证

- 运行：`cargo test -p vw-cli api_workflow_commands_cover_application_crud_resume_and_stream --lib`
- 运行：`cargo test -p vw-gateway-client workflow_client_uses_application_and_stream_routes --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：CRUD path/body、SSE 顺序、终止和确认断言通过，clippy 通过。
- 所需证据：失败时命令缺失；通过后 HTTP 与 SSE 测试 0 失败、无事件丢失；clippy 0 警告。

## 完成

`api workflow` 覆盖应用生命周期及真实 JSONL 聊天流。

