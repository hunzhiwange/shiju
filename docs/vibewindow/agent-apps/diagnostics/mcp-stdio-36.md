# 任务 36：MCP stdio 握手与进程回收

## 实施边界

在原工作区执行；未创建工作树。任务 35 的已提交依赖为 `06e95ff46`；任务 1–3 的来源、诊断和高可用契约已核对，`vw-agent-apps` 已注册 workspace。复用 `agent_apps/mcp/network_probe.rs` 的短期初始化路径、目录配置解析与共享诊断调度，不新建 gateway 第二实现，不修改生产 MCP 连接。后续 API 接线使用 `network_probe::start_handshakes`。

真实路径：`crates/vw-agent/src/agent_apps/mcp/{network_probe,connectivity,probe_jobs}.rs`，独立测试为 `stdio_probe_tests.rs`、`probe_jobs_tests.rs` 与 `connectivity_tests.rs`。进程组及 wait 复用 `tools/mcp_common/wire_stdio.rs::ProbeChild`。

## 授权边界

stdio 握手只接受目录记录 ID；命令、参数、env 和 cwd 来自当前获准目录修订，探测入口不接受任意命令。新增主机策略许可 `stdio_sha256` 与 `stdio_workspace`：前者是 `serde_json::to_vec(McpConnectionDefinition)` 的 SHA256 小写十六进制，包含命令、参数、环境和工作目录定义；后者是管理员提供的可信绝对工作区。stdio 许可不填写 `endpoint_sha256`。既有 HTTP/SSE 许可继续使用 URL 摘要，不能启动 stdio。

凭据仍须逐引用授权并由执行主机的账号存储解析。stdio 许可摘要覆盖凭据引用；不会把账号令牌写进策略或证据。显式配置的 PATH 优先，否则复用 `vw_shared::shell` 的安全 PATH 补全，不执行登录 shell。相对 cwd 仍由既有 `runtime_server` 以可信工作区为基准解析。

握手只初始化，不发现或调用工具。取消、超时、初始化失败和正常完成均在归还调度许可前终止自己的私有进程组，并 wait 回收主进程；测试通过系统 PID 存在性检查实际观察后代退出。常规目录连接检查仍保留既有工具发现行为与成功时的正常关闭机会。

## 验证记录

状态：**已完成，待回归**。修复后的测试未执行成功，不能声称已验证进程回收；保留编号任务卡。

环境：2026-11-11，macOS 27.0（26A428）arm64，rustc 1.99.1，rmcp 0.8.5；夹具协议版本 2025-03-26。仅本地真实 stdio/HTTP/SSE 协议夹具，不是实际供应商或原生窗口验收。

### 命令、退出码与数量

聚焦命令：

```bash
cargo test -p vw-agent --lib diagnostic_mcp_stdio_cancellation_reaps_process_tree -- --nocapture
```

| 阶段 | 槽位 | 退出码 | 有效测试数量与结果 |
| --- | --- | --- | --- |
| 原实现红灯 | 1 | 101 | 1 项执行，0 通过、1 失败；断言 `timed-out stdio probe left a live process tree`，10.07 秒，SQLite。观察的是实际 OS PID 存在性，失败分支杀掉暴露的残留进程。 |
| 首次绿灯构建 | 1 | 101 | 0 项执行；无关 `vw-api-types/src/agent_apps/mod.rs:93` 注册 aggregation 时文件尚未写出，稍后文件出现。 |
| 第二次绿灯构建 | 3 | 101 | 0 项执行；无关 `providers/live/project_binding` 尚未就绪，以及 `check_pending_project` 缺失；随后已出现对应文件。 |
| 最终绿灯构建 | 7 | 101 | 0 项执行；`crates/vw-agent-apps/src/aggregation/claude_publish.rs:11` 引用私有 `providers::projection`（E0603）。 |

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

首次 Clippy 在槽位 1 退出 101：同时发现本任务夹具错误使用 `AuthService::Codex`，已修正为真实的 `CodexOauth`。其余错误包括外部聚合授权可见性、`Pending.project_preimage` 测试初始化缺失、桌面 `Provider.protocols` 缺失及外部测试警告，未修改这些文件。

修正后再次执行同一 Clippy 命令，槽位 7，退出 101；最终仅报告上述 `claude_publish.rs:11` 的私有模块 E0603，阻断 `vw-agent-apps` lib/lib test，未完成本任务的编译检查。Clippy 不执行测试，有效测试数量为 0。

`cargo fmt --all -- --emit stdout` 退出 0，仅将格式化结果写回本任务文件；修正枚举后再次运行也退出 0。共享授权文件后续出现的其他会话改动未写回或暂存。`git diff --check` 与提交前 `git diff --cached --check` 退出 0。

已有正常 stdio 回归命令 `cargo test -p vw-agent --lib cc_mcp_t30_stdio_discovers_tools_using_args_env_and_cwd -- --nocapture`，以及任务 35 的 `cargo test -p vw-agent --lib diagnostic_mcp_http_handshake_is_scoped -- --nocapture`，因最终聚焦构建失败而未运行，不记为通过。

### 构建资源与隔离

实际使用：

- 槽位 1：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 槽位 3：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
- 槽位 7：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7`。

均按 1–20 原子 mkdir 顺序领取，锁内记录任务、会话与 PID，只释放自己的锁，保留编译缓存。

红灯阶段虽已设置绝对 `CARGO_TARGET_DIR`，但仓库既有 `.cargo/config.toml` 的 `build.build-dir = "target"` 令中间产物及测试执行路径仍为共享 `target/debug/deps`；该次红灯证明原行为缺陷，但不代表完整构建隔离。发现后，后续每次验证均同时设置 `CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`，确保中间产物也进入领取槽位；未修改仓库配置。回归时须保留该环境设置。

### 回归范围

先由对应任务修复共享编译错误，再按编号任务卡领取新槽位，运行上述聚焦测试、两项相邻过滤命令及规定 Clippy。新增 stdio 握手夹具覆盖正常、缺命令、挂起、取消、超时、cwd/env/PATH、未授权记录/摘要/凭据与不发送 tools/list；目录批次夹具另检查取消后可复用并发名额且不影响其他任务。当前只有红灯运行证据，修复后的真实退出及成功数量仍待回归。
