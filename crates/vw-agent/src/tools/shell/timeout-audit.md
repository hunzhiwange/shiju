# 原生长命令超时边界审计

审计日期：2026-10-26。范围为原生 bash 会话；独立 PowerShell 和远端 ACP 代理自己的命令实现不在此保证内。

| 层与实际符号 | 默认值 / 来源 | 计时区间、到期行为及证据 |
| --- | --- | --- |
| `tools/shell/mod.rs::parse_yield_time_ms`、`session_tools.rs::ShellTool::call_session` / `BashWaitTool::call` | 10000 ms，允许 250–30000 ms | 每次响应等待；返回 running、句柄和游标。不是硬执行预算。真实进程测试 `shell_outer_timeout_yield_does_not_cancel_process` 经同步工具入口连续两次 yield 后仍 running。 |
| `tools/shell/mod.rs::parse_timeout_ms` → `processes.rs::start_with_timeout` | `timeout` 可选，单位 ms；省略为 None | 从 spawn 起算、不因输出续期的硬执行预算；到期为 timed_out，终止进程树并保留尾部输出。原生会话没有默认 120 秒硬终止。 |
| `tools/toolset/mod.rs::execute_tool_call` → `block_on` | 同步桥使用临时 Tokio runtime；已有 runtime 时另起线程 | 调用结束销毁桥；子进程由 `ShellProcesses::shared` 的独立线程/runtime 持有。上述真实工具入口测试覆盖 registry/runtime 重建。 |
| `session/processor/runner.rs::run` → `llm_runner::run_llm_step_with_retry` → `tools_exec::run_tool_and_record_with_protocol` | 模型步骤与工具执行分阶段 | 模型步骤收集结束后才执行工具；后台进程不依赖当前模型流。工具结果保留结构化 running/终态信息。 |
| `session/processor/llm_runner.rs::collect_llm_step_from_stream` / `llm_step_idle_timeout` | 默认 90 秒；`acp_close_client_after_request=true` 时 900 秒 | 仅等待模型事件，事件重置 idle 计时；到期中止模型流、报“模型响应超时”。不会给宿主进程设置执行期限。`shell_outer_timeout_model_idle_does_not_cover_background_runtime` 让真实进程握手就绪，再触发模型 idle，确认仍 running；最终 shutdown 回收。 |
| `gateway/mod.rs::request_timeout_middleware` / `request_timeout_for_path` | 普通路由 30 秒；聊天/工作流/任务计划 SSE 路由 3600 秒 | `timeout(next.run(req))` 覆盖响应生成，到 Response 返回即结束，不覆盖后续流式响应体的完整生命周期。`gateway/bootstrap.rs` 安装该 middleware；原生聊天经 `gateway/api/handlers/session.rs` 的 `/v1/chat/stream`，不是普通 30 秒路径。因此不全局放宽 HTTP 超时。已有 `request_timeout_middleware_*` 和 `timeout_for_path_uses_extended_window_for_long_running_sse` 覆盖路由/截止选择。 |
| `gateway/api/handlers/session/stream.rs::collect_gateway_chat_stream*` → `runtime::start_prompt` | resumable stream store 管理连接与 stop | SSE 传输拒绝调用 `PromptCancelHandle::interrupt`，仅中断事件投递；显式 stop 调用 cancel，回收本会话宿主进程。正常提示完成不终止后台进程；重连不重新启动命令。 |
| `channels/manager/start.rs` → `channels/config.rs::channel_message_timeout_budget_secs` → `message.rs` → `message_execution.rs::run_message_execution` | `channels_config.message_timeout_secs` 默认 300 秒，生效基值至少 30 秒；总预算为基值 × clamp(max_iterations, 1, 4)，饱和乘法 | 这是实际渠道整轮 deadline，覆盖模型与工具，不随输出重置。到期取消 processor，按可信租户/规范工作区/会话回收进程树；取消前后各清理一次覆盖 spawn 竞争，处理器投递排空最多等待 5 秒（每次宿主清理另有 5 秒上限）。错误报告 `channel_message_timeout_budget`、budget_secs、elapsed_ms、channel_deadline 和回收结果；尾部仍可用原句柄通过 bash_wait 读取，不把敏感命令输出写进诊断日志。显式用户取消仍返回 Cancelled，清理失败显式报错。真实进程测试 `shell_outer_timeout_explicit_budget_is_reported` 验证取消终态和尾部保留；`shell_outer_timeout_user_stop_preserves_other_sessions` 验证会话隔离。 |
| `agent/loop_/core/timeouts.rs::{effective_message_timeout_secs,message_timeout_budget_secs}` | 同样的下限/缩放函数 | 该文件仅有导出、文档及单测引用；本渠道执行链实际使用 `channels/config.rs` 的函数。不能把旧 helper 当作额外生效期限。 |
| `acp/backend.rs::start_prompt_core` → `runtime::start_prompt` / `bridge_prompt` | 原生 ACP backend 未添加整轮 deadline | backend 复用原生 prompt，显式取消经过 `PromptCancelHandle` 回收，普通完成保活。作为远端 ACP 客户端使用时，还存在上面的 900 秒模型 idle 和远端自身限制；不保证所有 ACP 模式可静默等待 20 分钟。 |

## 二十分钟的适用边界

原生 Gateway/ACP backend 的后台宿主在未指定 `bash.timeout`、未收到显式 stop/会话销毁/应用关闭且无较短外层整轮预算时，没有 120 秒默认终止；可跨多次短等待继续读取。以上短回归证明计时器彼此独立；下文批次 7 记录独立运行的真实二十分钟证据。

渠道默认总预算最多 300 × 4 = 1200 秒，还包括模型与工具开销，因此不能承诺默认渠道完整跑完一个二十分钟命令。需要在现有 `channels_config.message_timeout_secs` 配置入口提供足够预算（总预算须覆盖命令和其他步骤）；`max_iterations` 缩放最多为 4，不会因命令持续输出而延长。设置了较短的 `bash.timeout` 或远端 ACP 自身预算时，较短期限仍有效。

`stop_channel_execution` 的纯排空截止用 Tokio 可控时间验证；OS 子进程存活、输出和取消使用真实握手与有界等待，没有用 advance 模拟二十分钟进程运行。

## 批次 7 真实入口与统一回归（2026-10-26）

新增 `long_running_tests.rs`，从同步 `execute_tool_call` 入口调用 `bash`、`bash_wait`、`bash_cancel`。每次调用重新创建可信上下文和工具注册，调用桥的 Tokio runtime 随调用销毁；没有修改生产权限、审批、路径、环境或退出语义。

旧版本 `8f86a6ce2` 的 `shell/mod.rs` 使用 `DEFAULT_TIMEOUT_MS = 120_000`、`cmd.kill_on_drop(true)` 与包裹 `cmd.output()` 的执行超时。本批次开始时前置实现已经具备会话能力，新增短测试首跑即通过（2 passed、1 ignored，退出 0），作为保护性回归；没有制造人为红灯，也没有将无关失败算作目标行为缺失。首次长测因测试把块游标误认为字节偏移而主动中断，不计为二十分钟证据；修正为验证游标单调与终态空续读后重新计时。

### Cargo 槽位与环境

使用原子 `mkdir` 动态领取，最终及中间产物均在槽位内（同时设置 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，避免仓库 `build.build-dir=target` 将中间产物写到共享根目录）。缓存保留，不删除其他会话的锁或进程。

- 主验证槽位 1：`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`。
- 补充诊断槽位 2：`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`。
- macOS 的 `/var` 临时路径别名导致 Shell 路径策略用例失败；规范化 `TMPDIR` 后完整 Shell 组通过。`/private/tmp` 虽能通过 Shell 组，但数据库测试拒绝该公共目录（`secure global database directory` / EPERM）；补充阶段采用当前用户临时目录的规范路径 `/private/var/folders/dt/hvzzy7ds3756n3cpg14j17200000gn/T`，没有放宽目录安全校验。

### 聚焦命令与有效数量

所有命令从仓库根执行、默认 features（Clippy 按要求启用全部 features）。下表只将实际报告通过的测试计入数量。

| 命令 | 退出码 | 有效结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 差异审查仅纳入本任务文件，既有工作区改动保持原样 |
| `cargo test -p vw-agent --lib shell_long_running_ -- --nocapture` | 0 | 2 passed、0 failed、1 ignored |
| `cargo test -p vw-agent --lib shell_long_running_real_entry_survives_twenty_minutes -- --ignored --nocapture` | 0 | 1 passed、0 failed；真实时长 1200.455 秒 |
| `cargo test -p vw-agent --lib tools::shell -- --nocapture` | 0 | 421 passed、0 failed、1 ignored；规范 TMPDIR 下复跑亦通过 |
| `cargo test -p vw-agent --lib tools::toolset -- --nocapture` | 101 | 34 项计划测试未完整执行；既有子代理默认配置断言失败后 panic runtime SIGABRT，不计作通过 |
| `cargo test -p vw-agent --lib shell_session_ -- --nocapture` | 0 | 16 passed、0 failed |
| `cargo test -p vw-agent --lib shell_outer_timeout_ -- --nocapture` | 0 | 原环境与用户规范临时目录均为 5 passed、0 failed；公共临时目录的失败不算通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 仅既有依赖 `block v0.1.6` 的 Cargo future-incompat 提示 |

toolset 既有失败单列，不修改或放宽断言：

1. `configured_subagent_opt_out_overrides_builtin_default` 期待 `main` 出现在可委派列表，但 `BUILTIN_AGENT_SPECS` 的 `main.allow_as_subagent=false`。槽位 2 原有二进制（`--list shell_long_running_` 为零项，证明不含本次测试）单独运行该用例，复现同一断言和 SIGABRT（进程退出 -6）。零项列表仅作基线身份核查，不作行为验收。
2. `full_access_context_disables_workspace_only_for_tool_security` 期待默认 `workspace_only=true`，而 `AutonomyConfig::default` 已是 false。
3. `full_access_context_allows_shell_paths_outside_workspace` 期待默认拒绝 `/etc/hosts`，与上述既有默认值不符。

后两项分别运行 `cargo test -p vw-agent --lib full_access_context_disables_workspace_only_for_tool_security -- --nocapture` 和 `cargo test -p vw-agent --lib full_access_context_allows_shell_paths_outside_workspace -- --nocapture`，各退出 101，复现原断言。排除这三项的补充命令为 `cargo test -p vw-agent --lib tools::toolset -- --nocapture --skip configured_subagent_opt_out_overrides_builtin_default --skip full_access_context_disables_workspace_only_for_tool_security --skip full_access_context_allows_shell_paths_outside_workspace`，退出 0，31 passed、0 failed。它补充其余工具注册/桥接覆盖，不替代完整 toolset 命令的失败记录。

取消测试中，父进程及真实 `sleep` 子进程经 PID 文件握手就绪；`bash_cancel` 返回 cancelled、exit_code=9，TERM 尾部保留，OS 对两个 PID 均返回 ESRCH。隔离测试验证外部会话、工作区和租户拿到同 ID 后，读取及取消均得到与未知 ID 相同的拒绝结果；原所有者仍 running，最后显式取消回收。默认 Shell 组还覆盖沙箱、路径、环境过滤、审批、输出及非零退出语义。

平台边界：仅本机 macOS 原生路径取得真实进程证据；新增测试按 `cfg(unix)` 注册，不宣称 Windows 或 Docker 环境通过。非 Unix 的托管进程树终止明确返回 unsupported；`docker` / `podman` / `ssh` runtime 无法保证远端进程树回收时明确拒绝，相关拒绝路径已在 Shell 组通过，未将拒绝测试描述成实际容器运行通过。

### 真实二十分钟证据

- macOS 原生进程 `83650506-fb69-4deb-8339-b65ea290726f`，脚本 PID 89855；未指定 `bash.timeout`，仅使用短响应等待及按游标轮询。
- 开始：2026-10-26 15:55:36.059949 +08:00；结束：2026-10-26 16:15:36.520731 +08:00。单调时钟总时长 **1200.455167750 秒**，首响应 **304.891958 毫秒**；测试报告 1 passed、0 failed，命令退出 0。
- 120 秒后仍 running，最后一次输出在 1200.443 秒返回 running；1200.455 秒进入 exited、exit_code=0。`marker:0` 至 `marker:40` 的完整顺序文本精确匹配，零丢失、零重复、truncated=false、dropped_bytes=0；终态 cursor=41 再读为空且游标保持 41。
- 每次游标：`0->1, 1->2, 2->3, 3->4, 4->5, 5->6, 6->7, 7->8, 8->9, 9->10, 10->11, 11->12, 12->13, 13->14, 14->15, 15->16, 16->17, 17->18, 18->19, 19->20, 20->21, 21->22, 22->23, 23->24, 24->25, 25->26, 26->27, 27->28, 28->29, 29->30, 30->31, 31->32, 32->33, 33->34, 34->35, 35->36, 36->37, 37->38, 38->39, 39->40, 40->41, 41->41`。游标计输出块，不能当作字节偏移。
- 原始输出：`/tmp/vw-task7-twenty-minutes.log`；主阶段命令退出记录 `/tmp/vw-task7-results.txt`；规范临时目录补充记录 `/tmp/vw-task7-extra-results.txt`。中断的首次长测单独保留在 `/tmp/vw-task7-interrupted-long.log`，不纳入通过证据。
- 主测试及脚本 PID 已消失，正常测试临时目录由 TempDir 回收；早先中断和既有 suite SIGABRT 遗留的本任务三组 fixture 经核对 cwd、脚本内容及 PID 后清理。槽位 1、2 均已释放，缓存保留。

本批次三个新增测试均取得真实通过证据，Clippy 通过；完整 toolset 仍有上述三项既有失败，因此任务记录保留为“已完成，待回归”。批次 5 的任务文件保持原样，留给原会话最终回归；不宣称整个计划统一回归已全部通过。
