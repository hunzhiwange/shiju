# 任务 18: 覆盖 Session 内容与 UI 状态

批次：【批次 18】 依赖批次 17

状态：未开始

目的：提供消息读取/part 修改、UI 快照、预览、归档、scope、path 与 any-ID 查询。

来源任务：无

预计会话范围：覆盖 session message、message part、ui-previews、archived、scope、UI save/load、preview/path/any；不重复核心生命周期。

## 文件

- 修改：`crates/vw-gateway-client/src/client/session_api.rs`
- 测试：`crates/vw-gateway-client/src/client/session_api_tests.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/session_content.rs`
- 测试：`crates/vw-cli/src/handlers/api/session_content_tests.rs`

## 上下文

- 所有业务请求经 `GatewayClient`，复杂 body 从共享 JSON 输入读取，输出走共享 renderer。
- 该接口族需顺序注册到 `handlers/api/mod.rs`，不得修改 `plan.md`。
- 保留当前工作树中与本任务无关的 SHIJU 和桌面改动。
- 消息 part PATCH/DELETE、archived/scope/UI PUT 都会改变状态，必须确认。
- `SessionUiLoadResult` 含原始 LLM 数据映射；默认输出不得意外扩展或泄露只读 raw 字段。

## 测试计划

- 行为：Session 内容/UI 命令使用正确复合 ID 和查询，敏感 raw 数据只在明确命令下输出，写操作受确认。
- 失败验证测试：`crates/vw-cli/src/handlers/api/session_content_tests.rs` 中的 `api_session_content_commands_cover_message_ui_archive_and_scope_routes`
- 失败验证命令：`cargo test -p vw-cli api_session_content_commands_cover_message_ui_archive_and_scope_routes --lib`
- 预期失败原因：CLI 无内容/UI 接口族，现有 client 的部分方法也未暴露脚本化输出。
- 通过验证命令：`cargo test -p vw-cli api_session_content_commands_cover_message_ui_archive_and_scope_routes --lib`
- 模拟策略：本地服务器验证复合路径、404 与 body；不模拟 UI 渲染。

## 步骤

1. 先写 message、part、UI、preview、archive、scope、path、any 的失败测试。
2. 补齐 client 缺失方法并保持 404 语义。
3. 实现 `api session-content`，raw LLM 仅由明确的 raw 操作输出。
4. PATCH/DELETE/PUT 统一确认并显示 session/message/part 目标。
5. 运行聚焦测试与 clippy。

## 验证

- 运行：`cargo test -p vw-cli api_session_content_commands_cover_message_ui_archive_and_scope_routes --lib`
- 运行：`cargo test -p vw-gateway-client session_client_uses_content_and_ui_routes --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：复合路由、404、raw 边界与确认断言通过，clippy 通过。
- 所需证据：失败时命令缺失；通过后路由矩阵、零请求拒绝和 raw 输出隔离成功；测试 0 失败；clippy 0 警告。

## 完成

`api session-content` 覆盖消息与 UI 状态，原始模型载荷不会被默认扩散。

