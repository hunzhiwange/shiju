# Codex thread 参数与 VibeWindow 现状对照

来源：本地 `/Users/Shared/work/dir/data/codes/codex-main` 快照中的 `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`、`codex-rs/app-server/src/request_processors/thread_processor.rs`；VibeWindow 当前 `crates/vw-agent/src/tools/session_tool/mod.rs`。Codex Desktop 一方工具包装层未在该源码快照中找到，同名参数不能据此声称已证实。

| 用户列出的工具 | Codex 本地源码可核实的接口 | VibeWindow 当前能力 | 计划边界 |
| --- | --- | --- | --- |
| `create_thread` | `ThreadStartParams` 有 `model`、`cwd`、`project_id`、权限和环境等字段；项目 ID 需存在 | `SessionTool.create` 接收 `title`、`prompt`，继承父会话目录和项目 | 显式返回 `thread_id`、`project_id`、目录；跨项目与环境参数无真实支持时拒绝 |
| `fork_thread` | `ThreadForkParams` 有源 `thread_id`、`last_turn_id`、`before_turn_id`、`cwd` 等；Codex fork 继承项目 ID | `session::fork` 可复制消息至指定 `message_id`，默认建根会话 | 建立用户可见 fork 入口并校验项目、历史截点；不宣称支持 turn 截点 |
| `list_threads` | `ThreadListParams` 有 `cursor`、`limit`、`archived`、`project_id`、`cwd`、`parent_thread_id` 等 | `SessionTool.list` 只查进程内当前父会话任务 | 先支持当前项目及当前父会话范围，返回真实持久化记录；未支持的过滤明确拒绝 |
| `read_thread` | `ThreadReadParams` 有 `thread_id`、`include_turns` | `SessionTool.read` 按字符上限返回消息 | 统一父会话归属与读取上限 |
| `send_message_to_thread` | Codex app-server 使用 turn 输入流程；同名 Desktop 工具包装未找到 | `SessionTool.send` 可给已登记子任务续发 | 保留忙碌拒绝、模型和权限边界；仅操作当前父会话的可见任务 |
| `handoff_thread` | 本地 app-server 无同名协议操作 | 无跨主机任务迁移基础 | 明确返回不支持，不以 fork 或新会话伪装 handoff |
| `set_thread_title` | `ThreadSetNameParams` 为 `thread_id`、`name` | 会话更新及网关 patch 已可改标题 | 增加受作用域约束的工具入口 |
| `set_thread_pinned` | 本地 app-server 无同名协议操作；此名称由用户指定 | 目前没有持久化置顶状态 | 新增项目作用域持久化置顶记录；输入 `thread_id`、`pinned`，返回真实状态，`list_threads` 含置顶状态，两个项目列表置顶优先排序 |
| `set_thread_archived` | `ThreadArchiveParams` 与 `ThreadUnarchiveParams` 分开 | 会话归档时间及网关 patch 已存在 | 工具参数显式布尔 `archived`，更新持久化状态与列表展示 |

命名约束：对模型注册表中本表的每个命令都必须使用表中逐字相同的 snake_case 名称；`set_thread_title` 输入 `thread_id`、`title`，`set_thread_archived` 输入 `thread_id`、`archived`，`set_thread_pinned` 输入 `thread_id`、`pinned`。新增参数及结果统一写 `thread_id`、`project_id`，UI 文案统一称 thread/任务。不得继续向模型暴露 `SessionTool` 或同义 action 别名；现有 `session` 存储函数仅供内部适配。

`wait_threads` 也按 Codex 桌面工具名称单独注册。精确命名不表示可无条件复制 Codex 参数：本地快照缺少某些桌面工具包装实现，未核实或无法执行的字段应明确拒绝。

项目展示证据：`session/runtime/lifecycle.rs` 在当前 `SessionScope` 内创建子会话，`session::create_next` 写入当前 `project_id`；`views/project/layout/work_sidebar.rs` 的 `collect_top_level_sessions` 排除所有非根会话，未按 `Info::is_visible_task()` 放行。`sessions_panel.rs` 已放行可见任务，内部 subagent 必须继续隐藏。
