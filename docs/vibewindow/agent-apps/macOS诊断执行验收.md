# macOS 诊断执行与取消验收（任务 49）

2026-11-12，Asia/Shanghai；macOS 27.0（26A428）/ Darwin arm64，ShiJu 各 crate 0.6.0，MCP 协议夹具 2025-03-26。直接在原仓库执行，没有创建工作树；plan.md 与上游只读。

## 已核实的依赖与真实链路

任务 1–3 的《高可用诊断契约》、来源映射与母计划执行分线已核对。直接依赖分别见 [41 候选 SDK](diagnostics/endpoint-candidates-sdk-41.md)、[47 MCP 桌面](diagnostics/desktop-mcp-47.md)、[45 模型桌面](diagnostics/desktop-model-45.md)、[46 端点测速](diagnostics/desktop-endpoint-selection-46.md)。上述生产接口均已存在，不把其文档中的历史回归状态当作新的实现前置。

本任务仅新增独立测试 `crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics_acceptance_tests.rs` 及父模块测试注册，不重建调度、授权、凭据或探测器。

- `diagnostic_macos_jobs_acceptance` 用实际 TCP/HTTP 运行生产 `agent_apps_diagnostics::router_with_policy`，注入真实 `GlobalDatabase`、`DiagnosticJobs` 与 `FailoverRuntime`，经 `GatewayClient::diagnostic_endpoint_candidates/start/status/cancel` 执行。测速使用主机候选 API 签发的 ID。
- 覆盖可达性、测速、模型、流式、MCP Streamable HTTP、MCP stdio 六条路径。供应商、模型、MCP 目录、连接修订及授权指纹均通过真实存储和原授权实现解析。生产领域结构与事务没有替代。
- 网络夹具实际解析 HTTP 正文，返回最小 Anthropic JSON、合法 SSE 或 MCP initialize 响应；挂起时直到真实 TCP EOF 才报告断开。stdio 使用 `/bin/sh` 和真实 `/bin/sleep` 子进程，记录请求和父子 PID，验证取消前存活与取消后退出。
- 每类检查终态、1/1 完成进度、取消前 running 与 0 完成进度、取消终态、重复取消、持久检查点。拒绝跨项目/主机/应用及无效令牌；取消拒绝外项目。
- 取消后同时占满三个真实 host 许可，第四项保持 queued，再取消排队及运行项，验证实际请求和 EOF。HTTP 与 stdio 的 `tools/call` 均为零。
- 向同一运行时健康注册表种入真实生产失败，使熔断保持打开；诊断前后以相同观察时间比对快照及准入，避免把时间流逝当状态变化。结束后受控推进 Tokio 时钟 601 秒，经 SDK 检查过期 GET 与 POST cancel 均为 404 `not_found`。
- `diagnostic_macos_jobs_acceptance_gateway` 使用现有隔离网关子进程入口，核对真实 bootstrap 注入和 SDK 启动/查询/取消链路；观察挂起请求的 TCP EOF，退出时 kill/wait 网关子进程。

以上均为**受控本地服务证据**，不是外部真实供应商通过。生产 API 不接受调用方指定证据源，执行器仍按生产规则签发 `RealService`；本记录明确标明测试端点实际由受控服务提供。未使用真实账号、未启动桌面；原生窗口和真实供应商由用户按母计划人工验收。

## 验证资源与命令

所有阶段以原子 mkdir 动态领取仓库共享槽位，优先热槽位 2，忙时遍历 1–20；前两轮实际领取槽位 **3**，第三轮、首轮 Clippy 与第四轮聚焦动态领取槽位 **2**。两个构建变量在每阶段同设为对应绝对路径：

```text
# 最终通过阶段（动态槽位2）
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
# 前两轮（动态槽位3）
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=1
CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_CODEGEN_UNITS=256
CARGO_PROFILE_TEST_LTO=off
CARGO_PROFILE_TEST_OPT_LEVEL=0
```

格式化使用 `cargo fmt --all -- --emit stdout`（后续加 `--config skip_children=true`）退出 0，只用 `rustfmt --edition 2024` 回写本任务测试文件，避免覆盖并行改动。不执行全量单元测试、wasm、Makefile，不改变 Cargo 配置或 features。

| 检查 | 确切命令 | 退出码 / 有效数量 | 结果 |
| --- | --- | --- | --- |
| 首轮接线 | `cargo test -p vw-agent --lib diagnostic_macos_jobs_acceptance -- --nocapture` | 101 / 0 | SDK 引用 McpAcpUsageQuery/McpAcpUsage 缺失，尚未运行测试；不计行为红灯。 |
| 第二轮 | 同上 | 101 / 0 | 同一外部类型缺失；复核源码已有类型，槽位编译产物未反映该文件当前内容。仅 touch 该源文件刷新 mtime，内容未改动，触发必要重编译，不删除缓存。 |
| 第三轮 | 同上 | 101 / 0 | 缓存刷新后类型依赖正常编译；同批其他会话新增 `app_routing/timeouts_deadline_tests.rs:22` 对不实现 Debug 的 RouteError 使用 unwrap，E0277 阻断 lib test。所有者随后将其改为显式错误处理；本任务未接管该文件。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | **0 / 不执行测试** | 槽位2，6m08s，全 workspace 全 targets 全 features 通过；本任务源码 SHA-256 为 `26273a4dff4ca177fc0b7779e7932f3b39b4844df4310e6475e54b79e5d5195d`。 |
| 第四轮有效聚焦验收 | `cargo test -p vw-agent --lib diagnostic_macos_jobs_acceptance -- --nocapture` | **0 / 2 项** | 槽位2；2 passed、0 failed、12426 filtered；构建6m35s、行为0.79s。上述六路径、许可、进程、健康、过期与独立网关断言全部通过。 |

首次有效运行即通过，未取得缺失行为红灯；本卡仅新增综合验收测试与证据，无需生产修复。前三轮编译失败以及同期反馈的四处本任务强类型/错误转换问题均不计行为红灯。调度已明确确认本综合回归验收卡可采用真实基线绿灯完成，不人为回退生产代码或伪造历史。

本轮实际输出记录：

```text
controlled stdio before cancel: leader=13244, child=13245, alive
controlled stdio after cancel: leader=13244, child=13245, exited
controlled acceptance: six successes, six active cancellations, queued cancellation,
three permits reusable, tools/call=0, health unchanged, expired GET/POST=404
standalone gateway: SDK POST=202, GET running=200, cancel=200, cancelled=200,
actual TCP EOF, gateway reaped
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 12426 filtered out
```

链接器输出既有 `__eh_frame section too large` 非致命提示，测试仍退出0；没有降低检查要求。两项测试均在同次聚焦输出中匹配，不再为缩窄过滤重复编译。

日志位于 `/tmp/shiju-task49-test-{1,2,3,4}.log` 与 `/tmp/shiju-task49-clippy-1.log`；关键结果已保留在本文。源码在全仓Clippy与有效聚焦验收间保持同一SHA-256。各阶段结束仅清理本任务的槽位锁，保留缓存；其余会话源码和暂存不纳入本任务提交。状态：已完成。推荐下一步：按母计划完成真实账号与原生窗口人工验收。
