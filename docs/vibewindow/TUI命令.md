# TUI 斜杠命令能力矩阵

## 判定口径

- 核对日期：2026-08-24。
- `已实现` 只表示命令已进入 `commands.rs` 的活动目录，能经 `CommandRoute` 到达 `app.rs` handler，并有真实 Gateway 调用或本地动作与错误分支。
- `动态来源` 只表示 Gateway 枚举出的已启用 Skill；当前唯一动态形式是 `/skill:<stable-id>`。
- `后端阻塞` 表示没有稳定、安全且已接线的调用契约；`显式不适用` 表示品牌、账号或宿主专属能力。
- 模型、面板或 Gateway client 方法存在，不等于 TUI 命令已实现。

## 活动命令目录

| 命令 | 结论 | 活动调用链 | 失败语义 |
|---|---|---|---|
| `/help` | 已实现 | `CommandRoute::Help` → 动态目录帮助 | 未知命令显式报错 |
| `/status` | 已实现 | `CommandRoute::Status` → `command_status_text` | 缺失会话显示为未创建 |
| `/exit` | 已实现 | `CommandRoute::Exit` → `RunnerControl::Quit` | 无隐藏副作用 |
| `/resume` | 已实现 | 会话选择 → `session_ui_load` | 无候选或加载失败写入 transcript |
| `/continue` | 已实现 | 最近非归档会话 → `session_ui_load` | 无候选显式失败 |
| `/new` | 已实现 | `session_create` → canonical snapshot | 创建失败不切换本地状态 |
| `/rename <title>` | 已实现 | `session_rename` → snapshot reload | 空标题、Gateway 失败显式报错 |
| `/fork [message-id]` | 已实现 | `session_fork` → 切换新会话 | 锚点必须来自当前持久化快照 |
| `/reset <message-id>`、`/rewind <message-id>` | 已实现 | 选择锚点 → 类型确认 → `session_reset` | 忙碌、外部 ID、失败均不改本地快照 |
| `/compact <message-id>`、`/summarize <message-id>` | 已实现 | 选择锚点 → 类型确认 → `session_summarize` | 只接受持久化消息 ID |
| `/archive` | 已实现 | 类型确认 → `session_archive` | 活动流或排队输入存在时拒绝 |
| `/model [id]` | 已实现 | provider/model 能力目录 → `SessionOptions` | 只接受 Gateway 声明的模型 |
| `/effort [level]` | 已实现 | 能力矩阵 → `SessionOptions` | provider 未声明的强度被拒绝 |
| `/plan`、`/plan on|off` | 已实现 | 权限选择 → `PermissionMode` | 未声明模式或危险升级被拒绝/确认 |
| `/plan <request>` | 已实现 | `StartPlanRun` → task-plan-run 可恢复 SSE | 断线、阻塞、取消进入统一终态 |
| `/goal <objective>` | 已实现 | `StartGoalRun` → task-plan-run 可恢复 SSE | 空目标拒绝；停止须匹配 owned request/task ID |
| `/permissions [mode]` | 已实现 | 能力矩阵 → 会话权限设置 | bypass/full access 要求类型确认 |
| `/settings` | 已实现 | 会话设置面板 → 下一请求生效 | 不支持项显示原因，不伪装全局保存 |
| `/skills` | 已实现 | `skills_get` → 只读目录 | 单来源失败隔离并脱敏 |
| `/skills enable|disable|delete <id>` | 已实现 | stable ID + 确认 → Skill mutation API | 非目录 ID、内建或不可变项拒绝 |
| `/plugins` | 已实现 | `plugins_get` → 只读目录 | 单来源失败隔离并脱敏 |
| `/plugins enable|disable|delete <id>` | 已实现 | stable ID + 确认 → Plugin mutation API | 非目录 ID 拒绝；删除为破坏性确认 |
| `/mcp` | 已实现 | `mcp_inspect` → 只读服务目录 | 连接失败仅影响 MCP 来源 |
| `/memory` | 已实现 | `memory_query_get` → 只读记忆目录 | 不提供未经契约支持的写入 |
| `/workflows` | 已实现 | `workflow_applications_list` → 只读目录 | 当前不注册 workflow 执行命令 |
| `/schedule` | 已实现 | `cron_jobs_get` → 只读目录 | 当前不提供启停、编辑或删除 |
| `/agents` | 已实现 | scoped scheduler workers → 只读目录 | 项目 scope 不可解析时显示来源错误 |
| `/tasks` | 已实现 | scoped scheduler status → 只读目录 | 不从本地状态猜测任务归属 |
| `/tasks cancel <id>` | 已实现 | owned snapshot + 确认 → scheduler cancel | 仅当前项目、canonical 可取消状态可执行 |

## 动态命名空间

| 命令形式 | 结论 | 活动调用链 | 边界 |
|---|---|---|---|
| `/skill:<stable-id> [arguments]` | 动态来源 | 合并后的已启用 Skill → prompt expansion → 普通 chat stream | 只接受 Gateway 目录稳定 ID；禁用或无 prompt 的条目拒绝 |
| `/mcp__<server>__<prompt>` | 后端阻塞 | 无活动路由 | `mcp_inspect` 目前只读，不把 prompt 元数据伪装为可执行命令 |
| `/workflow:<id>` | 后端阻塞 | 无活动路由 | workflow 列表目前只读 |
| 任意裸 `/<skill-name>` | 后端阻塞 | 无活动路由 | 避免与稳定内建命令冲突；使用 `/skill:<id>` |

## 未注册的官方或旧矩阵命令

以下命令不在活动目录。直接输入会返回 unknown command，不会退化为普通聊天提示。

| 分类 | 命令 | 结论 | 原因 |
|---|---|---|---|
| 缺少 TUI 契约 | `/add-dir`、`/batch`、`/btw`、`/copy`、`/debug`、`/fast`、`/keybindings`、`/passes`、`/scroll-speed`、`/stop`、`/subtask`、`/voice` | 后端阻塞 | 无稳定调用、授权或归属契约 |
| 存在相邻模型但未接线 | `/background`、`/bg`、`/cd`、`/clear`、`/config`、`/context`、`/cost`、`/diff`、`/doctor`、`/export`、`/files`、`/hooks`、`/project`、`/reload-plugins`、`/reload-skills`、`/stats`、`/theme`、`/usage` | 后端阻塞 | 文件或旧路径存在，但不在最终命令目录与 handler 调用链 |
| 动态技能名称 | `/autofix-pr`、`/code-review`、`/dataviz`、`/deep-research`、`/focus`、`/init`、`/review`、`/security-review`、`/simplify`、`/ultraplan`、`/ultrareview`、`/verify`、`/web-setup` | 动态来源 | 仅当 Gateway 返回对应稳定 Skill ID 时通过 `/skill:<id>` 使用 |
| Anthropic 品牌/账号 | `/advisor`、`/bug`、`/share`、`/claude-api`、`/feedback`、`/insights`、`/login`、`/logout`、`/privacy-settings`、`/release-notes`、`/upgrade`、`/usage-credits` | 显式不适用 | VibeWindow 使用独立 Gateway 与认证体系 |
| 外部产品集成 | `/chrome`、`/design-login`、`/design-sync`、`/desktop`、`/app`、`/ide`、`/install-github-app`、`/install-slack-app`、`/mobile`、`/ios`、`/android`、`/radio`、`/remote-control`、`/remote-env`、`/teleport`、`/terminal-setup`、`/web-setup` | 显式不适用 | 依赖未授权的外部宿主或账号表面 |
| Claude Code 运行时专属 | `/heapdump`、`/powerup`、`/recap`、`/sandbox`、`/setup-bedrock`、`/setup-vertex`、`/statusline`、`/stickers`、`/tui`、`/vim` | 显式不适用 | 不复刻 Node 进程、发行、营销或宿主配置表面 |

## 审计结论

- 活动目录包含 25 个稳定命令；另有 `/rewind`、`/summarize` 两个仅解析别名。
- 只有 `/skill:<stable-id>` 是活动动态来源；MCP 与 Workflow 当前均为只读目录。
- 所有管理写操作要求 Gateway 目录稳定 ID；危险设置、删除、会话回退和任务取消均有明确确认或归属检查。
