# 端点候选客户端 SDK（任务 41）

2026-11-12，macOS 27.0 / Darwin arm64，vw-gateway-client 与 vw-api-types 0.6.0。直接在当前仓库执行，未创建工作树；只读 plan.md 未修改。

## 依赖与实现

核实任务 39 提交 `2b5362db5`、任务 40 提交 `3619f2233` 及其正式证据 [SDK](sdk-39.md)、[候选 API](endpoint-candidates-api-40.md)，并核对任务 1–3 的来源映射与领域合同。实际 API 已通过授权处理器读取供应商记录、应用预设和自定义端点，再调用领域 `diagnostics::endpoints::collect`；本任务不另建收集、授权、存储或执行器。

在既有 `crates/vw-gateway-client/src/client/agent_apps_diagnostics_api.rs` 增加 `GatewayClient::diagnostic_endpoint_candidates(scope, query, token)`，复用现有父模块和独立测试模块注册，无需改共享入口。使用公开 `DiagnosticEndpointCandidatesQuery`、`DiagnosticEndpointCandidatesResponse` 和任务 6 的候选/来源类型，不新增 DTO。

调用 `POST /v1/agent-apps/diagnostics/endpoint-candidates/query`，URL 查询只带 projectId、targetHostId、appId；JSON 正文带 providerId、draftUrls、presetName。响应中的候选顺序、主机签发 ID、显示 URL 和 draft/saved_config/preset/custom 来源原样保留。认证沿用 sensitive 的 x-agent-apps-token、网关 Bearer 与租户请求上下文；403 保留状态及完整结构化错误（含关联 ID）。

将既有内部请求函数的固定根从 diagnostics/jobs 提升到 diagnostics，各调用者显式追加 jobs 或 endpoint-candidates/query，继续共用禁重定向、禁重试客户端与错误处理。SDK 只有一次 HTTP 发送，没有自动测速、保存、配置更新或启动作业调用。没有新增依赖或更改 Cargo 配置。

## 验证

原子 mkdir 优先领取空闲热缓存槽位 **2**，验证阶段记录本会话持有进程。所有 Cargo 命令继承以下环境；未使用其他构建目录，未运行全量单元测试、wasm 或 Makefile。

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

独立测试 `client/agent_apps_diagnostics_api_tests.rs::diagnostic_endpoints_sdk_uses_scoped_body` 使用动态端口的真实本地 TCP/HTTP 服务，受控响应，不代表实际供应商验收。服务解析完整请求和正文，逐条记录所有连接；在首请求后继续观察 350ms，200、403 各观察到恰好一次请求，然后 join 确认线程退出。断言覆盖 POST 完整路径（仅有三项作用域）、包含特殊字符的供应商与草稿正文、预设选择、成员认证、Bearer、租户头、四来源/候选 ID/URL 保留和 403 原错误，无额外探测、保存或重试请求。

| 命令 | 退出码 / 有效测试 | 结果 |
| --- | --- | --- |
| `cargo test -p vw-gateway-client --lib diagnostic_endpoints_sdk_uses_scoped_body -- --nocapture`（最小 SDK 接线） | 101 / 1 | 编译成功；0 passed、1 failed、460 filtered；路径行为断言失败，实际仍是 `/diagnostics/jobs`，要求 `/diagnostics/endpoint-candidates/query` |
| 同一命令（正确候选路径） | 0 / 1 | 1 passed、0 failed、460 filtered；构建 15.92s，执行 0.72s |
| `cargo test -p vw-gateway-client --lib diagnostic_sdk_preserves_job_identity -- --nocapture` | 0 / 2 | 2 passed、0 failed、459 filtered，执行 0.74s；共享函数调整后，原启动/查询/取消及不重放合同通过 |
| `cargo clippy -p vw-gateway-client --all-targets --all-features -- -D warnings` | 0 / 不适用 | SDK 库与全部测试目标通过，32.35s；不替代全仓检查 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 101 / 不适用 | 无关桌面模块 `crates/vw-desktop/src/app/message/agent_apps_diagnostics.rs:24–28` 的 Phase::Handshake/Succeeded/Failed/Cancelled 未构造，触发 dead_code；SDK 检查已通过，无本任务报错 |

红灯原文：

```text
assertion `left == right` failed: candidate query must use its read-only endpoint with only scope in the URL
left: POST /v1/agent-apps/diagnostics/jobs?projectId=...&targetHostId=...&appId=claude HTTP/1.1
right: POST /v1/agent-apps/diagnostics/endpoint-candidates/query?projectId=...&targetHostId=...&appId=claude HTTP/1.1
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 460 filtered out
```

`cargo fmt --all -- --emit stdout --config skip_children=true` 退出 0，只输出格式化结果；随后 `rustfmt --edition 2024` 仅回写本任务两个 Rust 文件，遵守并行会话不得覆盖其他文件的要求。差异审查仅包含新增方法、共享路径最小调整和独立行为测试；其他会话变更及暂存保持原状。

全仓 Clippy 阻断时，该桌面文件为其他会话新增、未跟踪文件；未修改它或放宽 lint。任务状态为 **已完成，待回归**，保留编号卡；推荐其所有者完成接线后复跑全仓 Clippy，再删除任务卡。

所有验证进程结束后释放自己的 slot-2.lock，保留缓存。两个 Rust 文件通过检查后未再变更；SHA-256 分别为 `b7c10dbe4cc8a234209045dbbed9c337b99de7c1d64a746ab9abcb285d06cdaf` 和 `92cc20a9b3c6c6aedcbd2190ec92293912e28e33bdb8d2ee496769cb8b5d6ab3`。
