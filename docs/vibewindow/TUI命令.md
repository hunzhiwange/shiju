# TUI 斜杠命令能力矩阵

## 判定口径

- 核对日期：2026-08-24。
- `已实现` 只表示命令已进入 `commands.rs` 的活动目录或活动动态路由，能经 `CommandRoute` 到达 `app.rs` handler，并有真实 Gateway 调用或本地动作与错误分支。
- `动态来源` 必须经过当前 Gateway 快照中的稳定 ID 校验；仅有 DTO、旧 widget、测试 helper 或 Gateway client 方法不算接入。
- `部分接入` 表示只开放了该能力中安全、稳定的子集；`范围外` 与 `显式不适用` 不得写成已实现。

## 活动命令目录

`COMMANDS` 当前注册 30 个稳定命令：

| 命令 | 结论 | 活动调用链 | 失败语义 |
|---|---|---|---|
| `/help` | 已实现 | `CommandRoute::Help` → 动态目录帮助 | 未知命令显式报错 |
| `/status` | 已实现 | `CommandRoute::Status` → `command_status_text` | 缺失会话显示为未创建 |
| `/doctor` | 已实现 | `RunDoctor` → `doctor_get` → 受限诊断摘要 | 迟到目录结果丢弃；错误和诊断文本脱敏、限长 |
| `/exit` | 已实现 | `CommandRoute::Exit` → `RunnerControl::Quit` | 无隐藏副作用 |
| `/diff` | 已实现 | `ShowDiff` → 当前会话 `session_diff` → 结构化文件 diff | 无会话或 Gateway 失败写入 transcript；迟到会话结果丢弃 |
| `/files [relative-path]` | 已实现（只读） | `OpenWorkspaceFiles` → `directory_file_list`；选择文件 → `file_read_in_directory` | 只接受当前 Gateway 目录内相对路径；响应 root/path、UTF-8 与文本内容不匹配时拒绝 |
| `/hooks`、`/hooks audit` | 已实现 | `hooks_get` → Hook 目录；`hook_audit_get` → 受限审计摘要 | 迟到目录结果丢弃；畸形、过量和敏感文本受限处理 |
| `/hooks trust|revoke <plugin-id> <hook-hash>` | 已实现 | 当前目录 Hook 快照 + 类型确认 → `hook_trust`/`hook_revoke` → 刷新目录与审计 | 仅接受当前快照中的精确稳定凭据；并发 mutation、状态漂移或响应不匹配时拒绝 |
| `/cd <path>` | 已实现 | `project_resolve(create_if_missing=false)` → 临时 runtime rebind → 会话、能力、集成和编排完整 bootstrap → 原子提交 | 活动 stream、排队输入、并发切换或 memory mutation 时拒绝；失败保留旧目录并恢复命令草稿 |
| `/resume` | 已实现 | 会话选择 → `session_ui_load` | 无候选或加载失败写入 transcript |
| `/continue` | 已实现 | 最近非归档会话 → `session_ui_load` | 无候选显式失败 |
| `/new` | 已实现 | `session_create` → canonical snapshot | 创建失败不切换本地状态 |
| `/rename <title>` | 已实现 | `session_rename` → snapshot reload | 空标题、Gateway 失败显式报错 |
| `/fork [message-id]` | 已实现 | `session_fork` → 切换新会话 | 锚点必须来自当前持久化快照 |
| `/reset <message-id>`、`/rewind <message-id>` | 已实现；后者为解析别名 | 选择锚点 → 类型确认 → `session_reset` | 忙碌、外部 ID、失败均不改本地快照 |
| `/compact <message-id>`、`/summarize <message-id>` | 已实现；后者为解析别名 | 选择锚点 → 类型确认 → `session_summarize` | 只接受持久化消息 ID |
| `/archive` | 已实现 | 类型确认 → `session_archive` | 活动流或排队输入存在时拒绝 |
| `/model [id]` | 已实现 | provider/model 能力目录 → `SessionOptions` | 只接受 Gateway 声明的模型 |
| `/effort [level]` | 已实现 | 能力矩阵 → `SessionOptions` | provider 未声明的强度被拒绝 |
| `/plan`、`/plan on|off` | 已实现 | 权限选择 → `PermissionMode` | 未声明模式或危险升级被拒绝/确认 |
| `/plan <request>` | 已实现 | `StartPlanRun` → task-plan-run 可恢复 SSE | 断线、阻塞、取消进入统一终态 |
| `/goal <objective>` | 已实现 | `StartGoalRun` → task-plan-run 可恢复 SSE | 空目标拒绝；停止须匹配 owned request/task ID |
| `/permissions [mode]` | 已实现 | 能力矩阵 → 会话权限设置 | bypass/full access 要求类型确认 |
| `/settings` | 已实现 | 会话设置面板 → 下一请求生效 | 不支持项显示原因，不伪装全局保存 |
| `/skills`、`/skills enable|disable|delete <id>` | 已实现 | `skills_get` → stable ID prompt/toggle/delete | 非目录 ID、内建或不可变项拒绝；删除确认 |
| `/plugins`、`/plugins enable|disable|delete <id>` | 已实现 | `plugins_get` → stable ID toggle/delete | 非目录 ID 拒绝；删除确认 |
| `/mcp` | 已实现（只读） | `mcp_inspect` → 服务目录 | 单来源失败隔离；不伪造 prompt/resource/tool 执行 |
| `/memory [query]` | 已实现 | `memory_query_get` → 当前目录查询快照 | mutation 期间拒绝查询；迟到目录或查询结果丢弃 |
| `/memory remember <key> <content>` | 已实现 | 类型确认 → `memory_store(category=core)` → 按原查询刷新 | 空 key/content、并发 mutation 或目录切换时拒绝 |
| `/memory forget <key>` | 已实现 | 当前目录查询快照 + 类型确认 → `memory_delete` → 刷新 | 只删除当前快照中的精确 key；快照过期或外部 key 拒绝 |
| `/workflows` | 已实现 | `workflow_applications_list` → 选择 stable UUID → `workflow_application_get` | UUID 必须来自当前 Workflow 快照 |
| `/schedule`、`/schedule enable|disable|delete <owned-job-id>` | 已实现 | `cron_jobs_get`；选择详情加载 job/runs；确认后启停或删除并刷新 | 详情与 mutation 只接受当前 Cron 快照拥有的 ID |
| `/agents` | 已实现（只读） | scoped scheduler workers → 只读目录 | 项目 scope 不可解析时显示来源错误 |
| `/tasks`、`/tasks start|retry|pause|resume|cancel|archive <owned-task-id>` | 已实现 | scoped task 快照；选择详情加载 graph/log/artifact；owned snapshot + 确认 → `agent_scheduler_command` → 刷新编排快照 | 仅当前项目 canonical ID 可读详情或控制；动作必须符合 canonical 状态机 |

## 动态命名空间

| 命令形式 | 结论 | 活动调用链 | 边界 |
|---|---|---|---|
| `/skill:<stable-id> [arguments]` | 动态来源 | 合并后的已启用 Skill → prompt expansion → 普通 chat stream | 只接受 Gateway 目录稳定 ID；禁用或无 prompt 的条目拒绝 |
| `/workflow:<uuid> <query>` | 动态来源 | 当前 Workflow 快照 stable UUID → workflow resumable SSE → pause overlay/resume | 要求当前会话、非空 query 和 owned UUID；pause token 不进入 UI 状态 |
| `/mcp__<server>__<prompt>` | 部分接入（执行未接入） | 无活动执行路由 | `mcp_inspect` 目前只读，不把 prompt 元数据伪装为可执行命令 |
| 任意裸 `/<skill-name>` | 未接入 | 无活动路由 | 避免与稳定内建命令冲突；使用 `/skill:<id>` |

## 非斜杠活动入口

| 入口 | 结论 | 活动调用链 | 边界 |
|---|---|---|---|
| composer 中的 `@query` | 已实现 | `FileSearchQueryChanged` → `file_find_in_directory` → 文件选择/附件 | 仅文件名、当前目录相对路径、最多 20 项；迟到 query 结果丢弃 |
| chat/tool 产生的结构化文件变化 | 已实现 | Gateway typed stream → `V3DisplayEvent::FileDiff` | 这是流内变更展示；`/diff` 另行读取当前会话 canonical diff |

## 未注册的官方或旧矩阵命令

以下命令不在活动目录。直接输入会返回 unknown command，不会退化为普通聊天提示。

| 分类 | 命令 | 结论 | 原因 |
|---|---|---|---|
| 缺少 TUI 契约 | `/add-dir`、`/batch`、`/btw`、`/copy`、`/debug`、`/fast`、`/keybindings`、`/passes`、`/scroll-speed`、`/stop`、`/subtask`、`/voice` | 未接入 | 无稳定调用、授权或归属契约 |
| 存在相邻模型但未接线 | `/background`、`/bg`、`/clear`、`/config`、`/context`、`/cost`、`/export`、`/project`、`/reload-plugins`、`/reload-skills`、`/stats`、`/theme`、`/usage` | 未接入 | 文件或旧路径存在，但不在最终命令目录与 handler 调用链 |
| 动态技能名称 | `/autofix-pr`、`/code-review`、`/dataviz`、`/deep-research`、`/focus`、`/init`、`/review`、`/security-review`、`/simplify`、`/ultraplan`、`/ultrareview`、`/verify`、`/web-setup` | 动态来源 | 仅当 Gateway 返回对应稳定 Skill ID 时通过 `/skill:<id>` 使用 |
| Anthropic 品牌/账号 | `/advisor`、`/bug`、`/share`、`/claude-api`、`/feedback`、`/insights`、`/login`、`/logout`、`/privacy-settings`、`/release-notes`、`/upgrade`、`/usage-credits` | 显式不适用 | VibeWindow 使用独立 Gateway 与认证体系 |
| 外部产品集成 | `/chrome`、`/design-login`、`/design-sync`、`/desktop`、`/app`、`/ide`、`/install-github-app`、`/install-slack-app`、`/mobile`、`/ios`、`/android`、`/radio`、`/remote-control`、`/remote-env`、`/teleport`、`/terminal-setup`、`/web-setup` | 显式不适用 | 依赖未授权的外部宿主或账号表面 |
| Claude Code 运行时专属 | `/heapdump`、`/powerup`、`/recap`、`/sandbox`、`/setup-bedrock`、`/setup-vertex`、`/statusline`、`/stickers`、`/tui`、`/vim` | 显式不适用 | 不复刻 Node 进程、发行、营销或宿主配置表面 |

## 审计结论

- 活动目录包含 30 个稳定命令；另有 `/rewind`、`/summarize` 两个仅解析别名。
- `/skill:<stable-id>` 与 `/workflow:<uuid>` 是活动动态来源；MCP prompt/resource/tool 执行仍未接入。
- 文件管理只读；Hook、Memory、Cron、Scheduler、集成删除及会话破坏性操作均要求稳定快照、精确 ID 和确认。
