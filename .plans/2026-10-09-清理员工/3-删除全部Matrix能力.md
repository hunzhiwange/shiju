# 任务 3: 删除全部 Matrix 能力

批次：【批次 3】 依赖批次 2

状态：未开始

目的：删除 Matrix 聊天、task-pool、调度运行时、配置、feature 和全部 matrix-sdk 依赖。

来源任务：无

预计会话范围：以 Matrix 协议为单一关注点跨 crate 删除完整垂直能力；不同时删除 Worker、Team 或 Staff，以便独立确认依赖树和配置面真正收缩。

## 文件

- 删除：`crates/vw-agent/src/channels/matrix/`
- 删除：`crates/vw-agent/src/agent_scheduler/adapters/matrix.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/adapters/matrix_tests.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/runner/matrix.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/runner/matrix_tests.rs`
- 删除：`crates/vw-agent/src/gateway/api/handlers/task_pool_matrix.rs`
- 删除：`crates/vw-agent/src/gateway/api/handlers/task_pool_matrix_tests.rs`
- 修改：`crates/vw-agent/Cargo.toml`
- 修改：`crates/vw-cli/Cargo.toml`
- 修改：`Cargo.lock`
- 修改：`crates/vw-config-types/src/channels/chat.rs`
- 修改：`crates/vw-config-types/src/channels/other.rs`
- 修改：`crates/vw-agent/src/channels/mod.rs`
- 修改：`crates/vw-agent/src/channels/manager/channels_config.rs`
- 修改：`crates/vw-agent/src/channels/manager/command.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/adapters/mod.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/runner/mod.rs`
- 修改：`crates/vw-agent/src/daemon/mod.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/mod.rs`
- 修改：`crates/vw-agent/src/gateway/api/mod.rs`
- 修改：`crates/vw-agent/src/gateway/bootstrap.rs`
- 修改：`crates/vw-agent/src/gateway/api/secrets.rs`
- 修改：`crates/vw-api-types/src/task.rs`
- 修改：`crates/vw-gateway-client/src/client/task_pool_api.rs`
- 修改：`crates/vw-cli/src/handlers/api/task_pool.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/channels.rs`
- 修改：`crates/vw-desktop/src/app/message/settings/channels/`
- 修改：`crates/vw-desktop/src/app/components/system_settings_channels/`
- 修改：`crates/vw-desktop/src/app/message/task_board/update.rs`
- 测试：上述模块的相邻 `*_tests.rs`

## 上下文

- `vw-agent` 通过默认启用的 `channel-matrix` feature 引入 `matrix-sdk 0.16.1`，继而拉入 base、common、crypto、sqlite、indexeddb 和 store-encryption。
- Matrix 同时存在于聊天通道、任务池事件、远程调度和桌面设置，必须沿配置—注册—运行—API—UI—依赖链完整删除。
- Rust 模块调用与路由影响用 RustCodeGraph 分析；Cargo feature、锁文件、文档和配置用 `rg` 核对；提交不得包含执行前已有改动。

## 测试计划

- 行为：workspace 不再包含 Matrix 配置、feature、代码路径或 `matrix-sdk-*` 依赖，其它聊天通道仍可编译。
- 失败验证测试：记录 Matrix 模块、配置、feature、路由和 Cargo.lock 依赖的现有命中。
- 失败验证命令：`rg -n "matrix-sdk|channel-matrix|MatrixConfig|channels/matrix|task_pool_matrix|MatrixAgentWorker|ConfiguredMatrixDispatch" Cargo.toml Cargo.lock crates --glob 'Cargo.toml' --glob '*.rs' --glob '*.md'`
- 预期失败原因：当前存在完整 Matrix 通道、调度、任务池与 SDK 依赖链。
- 通过验证命令：`cargo clippy --all-targets --all-features -- -D warnings`
- 模拟策略：不模拟网络；通过删除垂直能力、全 feature 编译和锁文件负向检查验证。

## 步骤

1. 按 `./skills/rustcodegraph/SKILL.md` 检查索引并分析 Matrix 模块、注册入口和调用影响。
2. 记录 Matrix 代码和依赖树的失败验证清单。
3. 删除 Matrix 聊天模块、通道注册、配置类型、密钥遮罩和桌面设置。
4. 删除 task-pool Matrix DTO、网关路由、客户端、CLI 与桌面发布/轮询逻辑。
5. 删除调度 Matrix 适配器、运行器、远程回执和专用 outbox 行为。
6. 删除 `channel-matrix` feature 与 `matrix-sdk` 直接依赖，更新默认 feature 和锁文件。
7. 清理 Matrix 测试、fixture、文档和示例，同时保留其它聊天通道。
8. 执行负向搜索与全 feature clippy。
9. 按 `./skills/git-commit/SKILL.md` 仅提交本任务删除和关联修复。

## 验证

- 运行：`! rg -n "matrix-sdk|channel-matrix|MatrixConfig|channels/matrix|task_pool_matrix|MatrixAgentWorker|ConfiguredMatrixDispatch" Cargo.toml Cargo.lock crates --glob 'Cargo.toml' --glob '*.rs' --glob '*.md' && cargo clippy --all-targets --all-features -- -D warnings`
- 预期：Matrix 能力和依赖无命中，clippy 退出码 0；Cargo.lock 不含任何 `matrix-sdk*` 包。
- 所需证据：删除文件清单、负向搜索结果、Cargo.lock 检查、全量 clippy 退出码、RustCodeGraph 影响分析记录、其它聊天通道审查记录、提交短 hash 与剩余变更清单。

## 完成

完成后不得保留 Matrix 空配置、禁用 feature、兼容错误或间接 SDK 依赖；其它通道不因删除 Matrix 而改变行为。
