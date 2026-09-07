# 任务 1: 锁定已开始会话的 ACP 后端

批次：【批次 1】 无

状态：未开始

目的：建立统一的“会话已开始”判定，并在消息处理边界拒绝修改已开始会话的 ACP 后端。

来源任务：无

预计会话范围：只涉及桌面端会话状态判定、ACP 选择分支及其单元测试；不改输入面板布局和底层 ACP 请求协议，适合一个聚焦会话完成。

## 文件

- 修改：`crates/vw-desktop/src/app/session/runtime.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/session.rs`
- 测试：`crates/vw-desktop/src/app/session/runtime_tests.rs`
- 测试：`crates/vw-desktop/src/app/message/chat/session_tests.rs`

## 上下文

- `ChatMessage::AcpAgentSelected` 当前同时更新 `SessionRuntimeState::acp_agent`、`App::acp_agent` 并持久化项目偏好。
- 判定必须覆盖已有 `app.chat`、`active_agent_request` 和非空 `queue`，且空白新会话保持可选。
- 拒绝切换时不得修改会话状态、应用默认值或持久化偏好，并通过现有 `show_warning_toast` 返回反馈。

## 测试计划

- 行为：空会话允许选择 ACP；已有消息、活动请求或排队请求的会话均保持原 ACP 不变。
- 失败验证测试：在 `runtime_tests.rs` 新增会话开始判定测试，在 `session_tests.rs` 新增 `acp_agent_selection_rejects_backend_switch_after_session_started`。
- 失败验证命令：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 预期失败原因：测试引用尚未实现的统一判定，或现有选择分支仍会覆盖已开始会话的 ACP 状态。
- 通过验证命令：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 模拟策略：直接构造真实 `App`、会话消息、队列项和活动请求状态；不模拟持久化客户端，只以返回任务与内存状态证明拒绝发生在持久化调用之前。

## 步骤

1. 编写覆盖空会话、已有消息、活动请求和排队请求的失败验证测试。
2. 运行失败验证命令并确认预期失败。
3. 在会话运行时增加最小统一判定，并在 `AcpAgentSelected` 分支最前方拒绝已开始会话的选择事件。
4. 运行通过验证命令，审查空会话原有选择与偏好同步路径保持不变。
5. 仅在检查保持通过时整理命名和重复判定。

## 验证

- 运行：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 预期：新增测试目标和生产代码全部编译，Clippy 退出码为 0 且无警告。
- 所需证据：实现前命令因缺失行为或符号失败；实现后退出码 0；差异显示拒绝分支发生在任何状态写入和持久化之前；空会话允许选择的既有断言仍保留。

## 完成

完成时应明确报告 `session/runtime.rs` 的会话开始判定、`message/chat/session.rs` 的 ACP 锁定分支及聚焦 Clippy 结果。仓库协议禁止默认执行 `cargo test`，因此用全目标 Clippy 编译全部测试代码，并保留行为测试供后续授权回归。
