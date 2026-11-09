# Codex 会话实现对照

依据本机 `/Users/xiangmin/worker/code/codex-main` 的 Rust 源码与测试。该目录是源码快照，未包含 Git 元数据。这里记录的是公开源码中的核心、App Server 和 TUI 行为。

## Codex 源码确认的行为

| 环节 | 实现与证据 |
| --- | --- |
| 创建会话 | `thread/start` 返回可用身份，预览为空，`name` 为 `null`。常规会话即使不是 `ephemeral`，启动时也不创建 rollout 文件。App Server 有明确的协议和文件断言。[thread_start.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/app-server/tests/suite/v2/thread_start.rs:390) |
| 首次写历史 | `record_user_prompt_and_emit_turn_item` 录入用户项，发出 `ItemCompleted(UserMessage)`，随后将 rollout 落盘。这一步可以早于首个助手返回。带继承历史的子会话也有在接收初始输入前持久化的路径。[session/mod.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/core/src/session/mod.rs:5626) · [spawn.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/core/src/agent/control/spawn.rs:829) |
| 自动命名 | 本地 TUI 从用户项完成事件取文本，在未命名时启动自动命名。按会话及用途去重，异步运行隐藏的 `ephemeral` 会话，禁用工具与环境能力。服务端拥有设置的模式会跳过这条 TUI 生成路径。[thread_routing.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/app/thread_routing.rs:2104) · [thread_title.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/app/thread_title.rs:64) · [temporary_structured_request.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/temporary_structured_request.rs:140) |
| 回写和展示 | 收到有效结构化标题后，先读取目标会话，确认尚未命名再保存。手工改名会取消生成，迟到结果按原会话处理。恢复列表优先显示正式标题，标题缺失时显示预览。Codex 的读取再写入为两个操作，ShiJu 的条件写入放在存储边界保护并发。[event_dispatch.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/app/event_dispatch.rs:3109) · [thread_title_tests.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/app/thread_title_tests.rs:118) · [resume_picker.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/resume_picker.rs:1282) |
| 子智能体 | 子会话由运行时标记为 `ThreadSource::Subagent`，`ThreadSpawn` 源保存父会话及角色信息。普通历史默认筛选交互式来源，Agents Overview 仍可选择子线程。[spawn.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/core/src/agent/control/spawn.rs:702) · [filters.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/app-server/src/filters.rs:6) · [agents_overview_view.rs](/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/app/agents_overview_view.rs:402) |

## ShiJu 实施约定

1. **先有稳定 ID，后有用户历史。** 桌面与 TUI 的空白页只保留草稿。实际提交后，按需为工具、审批和子任务保留受作用域保护的运行记录。请求失败、超时、取消或连接中断时，继续使用原 ID。若服务端可能已成功提交，优先用同一 SSE 请求重放和会话读取对账。
2. **首轮成功落盘才发布。** 首次有效助手终态与所需消息、轨迹都提交成功后，普通历史才可见。工具运行中、等待用户、模型失败和诊断记录都不触发。子任务保留必要的内部持久化和运行状态。
3. **预览和正式标题分离。** 首个成功轮的用户文本可作有界显示预览。普通会话的正式标题保持未设置，直到隔离的命名请求返回有效结果或用户明确改名。若命名失败，仍显示预览；附件轮用现有本地化占位。用户或工具明确提供的任务标签可在成功后用于命名，首轮前只显示于任务卡。
4. **一个命名所有者。** 由共享会话生命周期和服务端处理自动命名，桌面和 TUI 消费结果。复用已有内部 `session::title` 能力，核查 ACP 分支不复用或改写主会话上下文。生成不占用正式会话、不写历史、不带工具，按作用域与原会话去重，并对手工改名、删除和迟到响应做条件写入。
5. **由来源决定子会话投影。** 复用来源分类过滤普通历史。ShiJu 对 AgentTool 和工作流子会话禁用点击、键盘及直接 UI 路由，保留状态、详情和停止。普通分叉及明确用户可见的 SessionTool 任务保持可操作。

| 验收时点 | 历史列表 | 会话标题 |
| --- | --- | --- |
| 启动、新建空白页、首次请求失败或取消 | 无新增条目 | 未设置，无标题请求 |
| 同 ID 首次成功提交，命名进行中 | 单一条目，显示成功轮预览 | 未设置 |
| 命名失败、超时或结果无效 | 仍显示同一预览 | 未设置 |
| 命名成功或手工改名 | 更新原条目 | 有效名称，手工名称优先 |

ShiJu 当前提前写入的位置位于网关 `ensure_stream_agent_session`、桌面 `start` 和 TUI `restore_or_create_session_state`。实现沿用现有协议、会话存储及标题生成器，优先复用已有预览数据和工作区作用域。[网关 stream.rs](/Users/xiangmin/worker/code/shiju/crates/vw-agent/src/gateway/api/handlers/session/stream.rs:1268) · [桌面 session.rs](/Users/xiangmin/worker/code/shiju/crates/vw-desktop/src/app/message/chat/session.rs:1788) · [TUI app.rs](/Users/xiangmin/worker/code/shiju/crates/vw-cli/src/cli/tui/app.rs:3588) · [title.rs](/Users/xiangmin/worker/code/shiju/crates/vw-agent/src/session/title.rs:63)
