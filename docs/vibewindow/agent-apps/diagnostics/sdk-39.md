# 诊断客户端 SDK（任务 39）

2026-11-12，macOS 27.0 / arm64，vw-gateway-client 与 vw-api-types 0.6.0。直接在当前仓库执行，没有创建工作树或修改只读 plan.md。HTTP 为本地受控协议服务，不代表真实供应商、模型或账号验收。

## 生产接口与范围

直接依赖任务 38 已提交 `9cb5aaa84`，正式证据见 [查询与取消 API](read-cancel-api-38.md)。核对任务 1–3 的 [领域合同](../高可用诊断契约.md)、来源映射及现有生产代码：workspace 已注册 vw-agent-apps；诊断 DTO 位于 `vw-api-types/src/agent_apps_diagnostics.rs`；`bootstrap.rs` 注册三条诊断路由；`agent_apps_diagnostics.rs` 已消费授权、执行主机、真实资源和 `DiagnosticJobs`。SDK 没有新增权限门槛、探测执行、凭据存储或本地状态机。

`crates/vw-gateway-client/src/client/agent_apps_diagnostics_api.rs` 新增：

- `GatewayClient::diagnostic_start(scope, request, token)`：POST `/v1/agent-apps/diagnostics/jobs`，返回服务器 `DiagnosticJobIdentity`，支持 Reachability / Speed / Model / Streaming / Mcp 五种现有启动 DTO。
- `diagnostic_status(scope, task_id, token)`：GET `/v1/agent-apps/diagnostics/jobs/{id}`。
- `diagnostic_cancel(scope, task_id, token)`：POST `/v1/agent-apps/diagnostics/jobs/{id}/cancel`。

复用 `DiagnosticScopeQuery` 的 projectId / targetHostId / appId、`apply_request_context` 的租户与网关认证、现有应用成员令牌头及 `AgentAppsSettingsError`。错误保留 HTTP 状态和完整结构化错误体，包括关联 ID；解码和传输错误保持区分。任务 ID 作为独立 URL 段编码；空 ID 和点路径拒绝。令牌头标记 sensitive，不记录载荷。

原生接口复用 `dynamic_host_client`，禁止重定向和自动重试。SDK 返回网关取消快照，`cancelling` 仍需轮询；不会把它改成 `cancelled`，重复取消保持服务器终态与已有结果。未引入依赖或修改 Cargo 配置；父模块只追加必要注册。MCP 专用的带 generation 检查 SDK 保持原所有者，本接口消费统一诊断合同。

## 验证配置与命令

原子 `mkdir target/rust-slot-locks/slot-2.lock` 领取实际槽位 **2**，没有移除其他锁。验证命令继承：

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=1
CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_CODEGEN_UNITS=256
CARGO_PROFILE_TEST_LTO=off
CARGO_PROFILE_TEST_OPT_LEVEL=0
```

按用户“格式化仅回写自己负责文件”的最新要求，使用 `rustfmt --edition 2024 crates/vw-gateway-client/src/client/agent_apps_diagnostics_api.rs crates/vw-gateway-client/src/client/agent_apps_diagnostics_api_tests.rs`（退出 0），避免全仓格式化覆盖其他并行任务。未运行全量测试、wasm 或 Makefile。

| 命令 | 退出码 / 有效测试 | 证据 |
| --- | --- | --- |
| `cargo test -p vw-gateway-client --lib diagnostic_sdk_preserves_job_identity -- --nocapture`（初版普通 HTTP 客户端） | 101 / 2 | 1 passed、1 failed、446 filtered；真实 307 导致 POST 重放，实际请求数 2、预期 1；编译成功后行为断言失败 |
| 同一聚焦命令（禁重定向/重试，补充断线夹具） | 0 / 2 | 2 passed、0 failed、446 filtered，0.74 秒；构建 13.86 秒 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 101 / 不适用 | SDK 库通过检查；无关新增 `crates/vw-desktop/src/app/message/settings/mcp_management_form.rs:293` 的 `Task::perform` future 非 Send，`save` 持有非 Sync 的 `&Draft`（163 行）；未修改该文件 |

红灯原文：

```text
assertion `left == right` failed: diagnostic submission must not replay on redirect or failed acceptance
  left: 2
 right: 1
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 446 filtered out
```

独立测试位于 `client/agent_apps_diagnostics_api_tests.rs`，显式注册在 `client/mod.rs`。主用例实际观察五种 POST 的 202 身份、逐种取消中/已取消/重复取消的 200 快照，以及请求方法、完整路径、UTF-8/斜线/百分号编码、三项作用域、成员令牌、Bearer 和租户头。状态、进度、受控结果及关联 ID 精确保留；401/403/404/429/503 在启动、查询、取消三个方法上保留原错误体。307 与完整 POST 后断线分别只有一个实际请求，服务线程均 join 确认退出。另一个用例覆盖畸形成功响应、非 JSON 错误和非法任务路径。绿灯总计观察 39 次真实本地 HTTP 请求，全部使用动态端口。

全仓 Clippy 缺少通过证据，故任务保留为“已完成，待回归”。推荐 MCP 表单所有者修复其 Send 错误后复跑全仓 Clippy；取得全部证据后再删除任务卡。其余工作区改动与本任务无关，均排除提交。

补充局部检查：`cargo clippy -p vw-gateway-client --all-targets --all-features -- -D warnings` 退出 **0**，52.61 秒；该检查覆盖 SDK 库与测试目标，不替代上述全仓回归。最终审查新模块、独立测试、父模块五行注册及本记录；`git diff --cached --check` 退出 0。验证子进程均结束后释放自己的槽位2锁，保留缓存。
