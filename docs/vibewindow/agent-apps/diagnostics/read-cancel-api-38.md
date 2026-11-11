# 诊断查询与取消 API（任务 38）

状态：已完成。2026-11-12，macOS 27.0，arm64；直接在原仓库执行，没有创建工作树，也未修改只读 plan.md。接口包为 vw-agent/vw-api-types 0.6.0，夹具使用本地 HTTP；没有真实供应商或模型版本验收。

## 实现与依赖

- 复用任务 29 的 `DiagnosticJobs::snapshot/cancel`（生命周期 `c9ebc4319`、取消与资源回收 `2b27eaacc`），以及任务 37 的诊断启动接口和执行主机注入（`c2beaee9f`、`51b934632`）。没有复制注册表或新建 SSE 框架。
- `gateway/api/handlers/agent_apps_diagnostics.rs` 提供 `GET /v1/agent-apps/diagnostics/jobs/{id}` 和 `POST /v1/agent-apps/diagnostics/jobs/{id}/cancel`，查询参数沿用 `projectId/targetHostId/appId`。`bootstrap.rs` 增补路由清单，复用已有 router 注册。
- `agent_apps/access.rs::authorize_diagnostic_job` 沿用现有供应商诊断或 MCP 诊断作用域授权，重新验证当前成员、租户和目标主机，再由注册表核对完整作用域与提交成员。不要求新增权限配置，不读取凭据或提交新探测。
- 成功返回 200 和类型化 `DiagnosticJobSnapshot`，取消受理可显示 `cancelling`，实际清理后才显示 `cancelled`。查询和取消均为 `Cache-Control: no-store`。同一已授权范围内的未知、过期和他人成员任务统一 404；无权选择的范围 403，无效成员令牌 401。错误使用现有脱敏错误体，诊断失败仅含阶段与错误码。

## 环境与命令

各阶段原子领取共享槽位，实际均为 **5**；基线结束后释放旧锁，再次原子领取 5，缓存保留。

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=1
CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_CODEGEN_UNITS=256
```

基线保持原测试优化配置。恢复实现的首次必需构建保留 `opt-level=1`、另设 `CARGO_PROFILE_TEST_LTO=off`；在无关聚合编译错误修复后，最终必要重跑另设 `CARGO_PROFILE_TEST_OPT_LEVEL=0`，默认 features 始终不变。本机 Cargo 官方 profiles 文档明确：`lto=false` 仍执行 thin local LTO，`lto="off"` 才完全关闭；没有修改 Cargo 配置文件。

| 检查 | 退出码 / 有效测试数 | 实际结果 |
| --- | --- | --- |
| `cargo fmt --all -- --emit stdout --config skip_children=true` | 0 | 只输出全仓格式化结果，随后仅回写负责文件；共享文件按合入锁写入，差异逐块审查 |
| `cargo test -p vw-agent --lib diagnostic_api_read_and_cancel_are_scope_bound -- --nocapture`（真实基线） | 101 / 1 | 缺少 GET 路由，同作用域读取断言实际 404、预期 200；1 failed、12337 filtered，0.08 秒 |
| 同一聚焦命令（恢复实现、LTO=off、测试 opt-level=0） | 0 / 1 | 1 passed、0 failed、12396 filtered，0.10 秒；构建 40m10s |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 最新源码完整复跑通过，dev profile [optimized]，5m18s；无错误或警告 |

基线断言原文：

```text
assertion `left == right` failed: same scope must read a live diagnostic job
  left: 404
 right: 200
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12337 filtered out
```

## 测试范围与证据边界

独立测试 `agent_apps_diagnostics_tests.rs::diagnostic_api_read_and_cancel_are_scope_bound` 使用真实注册表、检查点和私有授权策略、本地真实 HTTP 服务及受控探测取消边界；已验证部分结果与进度、两次实际 HTTP 请求及不重放、取消清理后的终态、成功/失败后的取消幂等、MCP 专属授权、权限撤销、未知/过期 ID，以及跨成员/项目/应用/主机/租户拒绝。结果标记为 `ControlledFixture`，不代表真实供应商、账号或人工窗口验收。

首次编译尝试退出 101、0 项测试，原因为无关 Chrome `Download.url` 旧依赖快照和 `chrome_tests.rs` 的错误类型不匹配，不计行为红灯。后续复核两者已由其他所有者修复，直接复用；本任务对这些 Chrome 文件的实际修改为 0，排除提交。曾启动的恢复实现流水线按后续要求中断为 143，仅停止自己的进程，再精确回退自身生产块以保住真实基线。

真实基线构建耗时 68m10s。前期采样位于 LLVM 内联优化；约一小时后重新采样已进入机器指令生成和目标文件写出，154→156 个目标文件、最近一分钟写出约 16 个，因此保留近链接阶段的构建，最终取得上述真实断言。基线另有既有 `__eh_frame` 超过 16MB 的链接警告，编译和链接成功，不是红灯原因。

最终聚焦执行实际观察两次本地 HTTP 请求，仅一个探测完成；取消后另一探测不恢复执行，轮询与取消不重放网络请求。HTTP 夹具服务任务在断言结束前 abort 并 await，确认取消退出。成功/失败后的取消不改变原终态，已取消任务重复取消保持终态与部分结果。MCP 专属授权可查询和取消自己的任务；撤销授权后返回 403。跨租户测试使用同时获准两个租户的同一成员，另一租户下读取与取消均 404。

恢复实现后的首次绿灯尝试还遇到无关聚合重启 DTO 旧快照、Pi 授权函数参数不一致、未用变量，退出 101、0 项测试；后续均由其他所有者补齐，本任务没有接管这些改动。首次完整 Clippy 退出 101，原因是桌面语音验收夹具缺少 tokio-tungstenite 声明；最新清单已由其他所有者补齐，绿灯后完整复跑退出 0。验证针对共享工作区当时源码；提交只包含诊断块，聚合、语音和 Chrome 等修复保留原归属。

全部验证阶段均仅释放自己的锁，保留槽位 5 缓存；未运行全量单元测试、wasm 构建、fmt check 或项目 Makefile 命令。共享诊断处理器中同期 MCP 提交逻辑及可见性调整保持原归属；本任务只提交读取/取消相关导入、路由与函数、授权适配、独立测试及本记录。
