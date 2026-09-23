# Thread 管理与项目展示计划

目标：让可见 thread 在当前项目中可发现、可安全管理，并以与 Codex 一致的命令名操作和置顶。

范围：修复父 thread 归属、重启恢复、当前项目展示与来源文案；模型工具精确命名为 `create_thread`、`fork_thread`、`list_threads`、`read_thread`、`send_message_to_thread`、`handoff_thread`、`set_thread_title`、`set_thread_pinned`、`set_thread_archived`、`wait_threads`。新增持久化置顶状态和 UI 排序。

范围外：Codex 源码修改、跨主机迁移、无运行时支持的工作树交接、完整复制 Codex 项目/权限/环境参数、仓库重品牌。

假设：现有持久化记录已含 `project_id`、`directory`、`parent_id`、归档时间和可见任务 slug；`SessionTool` 是旧工具入口，工作侧栏目前按根记录过滤。

## 设计决策

- 模型工具名、参数名、返回字段、用户可见文案统一使用 thread；新实现模块命名为 `thread_tool`。现有 `session` 存储模块只作为内部数据适配层，不作为对外别名。
- 完成后从模型工具目录移除 `SessionTool`；不能留下同义命令或 action 包装入口。
- 只展示带可见任务标记的子 thread；内部 subagent 与 plan-run 子记录继续隐藏。
- 项目归属按持久化项目 ID 与目录核验；父子操作还须核验 `parent_id`。
- `set_thread_pinned` 使用布尔 `pinned`，持久化到项目作用域，置顶优先于普通 thread 排序；取消置顶后恢复普通排序。
- Codex 的命令名称保持逐字一致；参数仅开放 VibeWindow 已验证支持的子集，不支持的参数明确报错。
- 缺少真实交接基础时，`handoff_thread` 保留精确命令名并返回明确不支持错误。

## 架构说明

- 新 `thread_tool` 负责对外命令，现有 `session_tool`、`task_session`、`runtime/lifecycle` 为迁移来源；`toolset` 注册精确命令名，项目侧栏和消息流负责展示。
- `session::fork` 和网关 patch 可复用，但必须经过租户、项目及权限边界。

## 开发策略

- 对会改变行为的代码，使用失败验证、通过验证、重构。
- 实施前先执行各任务文件中的聚焦失败验证命令，确认失败原因。
- 每个编号任务只改其单一行为范围；同一 Cargo 验证资源按批次串行。
- 不运行全量测试或格式检查；代码任务按仓库要求执行 Clippy。

## 来源边界

- Codex app-server 的 `ThreadStartParams` 含 `model`、`cwd`、`project_id`；`ThreadForkParams` 支持源 thread 和历史截点；`ThreadListParams` 支持项目、归档、父子与分页过滤；`ThreadReadParams` 为 `thread_id` 和 `include_turns`。
- Codex app-server 的归档和反归档是两个操作；本计划遵循用户指定的 `set_thread_archived` 命令名，以布尔 `archived` 表示两种状态。置顶与 handoff 没有同名 app-server 协议实现，不推定其内部参数。
- VibeWindow 创建流程已写入父会话所属项目。工作侧栏过滤全部带 `parent_id` 的会话是已确认的展示缺口；仍须用端到端测试核对项目 ID、目录与刷新事件。
