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

原生 Gateway/ACP backend 的后台宿主在未指定 `bash.timeout`、未收到显式 stop/会话销毁/应用关闭且无较短外层整轮预算时，没有 120 秒默认终止；可跨多次短等待继续读取。以上短回归证明计时器彼此独立，**不冒充已运行二十分钟**；真实时长证据由批次 7 获得。

渠道默认总预算最多 300 × 4 = 1200 秒，还包括模型与工具开销，因此不能承诺默认渠道完整跑完一个二十分钟命令。需要在现有 `channels_config.message_timeout_secs` 配置入口提供足够预算（总预算须覆盖命令和其他步骤）；`max_iterations` 缩放最多为 4，不会因命令持续输出而延长。设置了较短的 `bash.timeout` 或远端 ACP 自身预算时，较短期限仍有效。

`stop_channel_execution` 的纯排空截止用 Tokio 可控时间验证；OS 子进程存活、输出和取消使用真实握手与有界等待，没有用 advance 模拟二十分钟进程运行。
