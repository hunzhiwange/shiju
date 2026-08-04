# TUI 斜杠命令能力矩阵

## 基线与判定规则

- 官方基线：[Claude Code interactive mode commands](https://code.claude.com/docs/en/commands) 与 [Claude Code CLI reference](https://code.claude.com/docs/en/cli-usage)。
- 核对日期：2026-08-02。
- 本文逐行记录官方 Commands 页面当前列出的命令；同一官方表格项的别名合并在一行。
- `已实现` 表示当前 TUI 已有真实处理路径；`动态来源` 表示只有网关实际返回对应 Skill、Plugin、MCP prompt 或 Workflow 时才注册；`显式不适用` 表示 Anthropic 品牌、账号或平台专属表面；`后端阻塞` 表示缺少稳定、安全的等价调用契约。
- “可列出”不等于“可执行”。没有可靠副作用 API 的命令不会以提示词、空操作或成功 toast 伪装实现。

## 官方命令逐项映射

| 命令/别名 | 官方用途摘要 | VibeWindow 结论 | API 或本地符号 | 目标任务或理由 |
|---|---|---|---|---|
| `/add-dir` | 将额外目录加入当前会话工作区 | 后端阻塞 | `CompletionCatalog::added_directories`、workspace canonicalization | 命令可发现并明确拒绝；网关尚无可持久化目录授权契约 |
| `/advisor` | 打开 Claude 产品顾问能力 | 显式不适用 | 无 | Anthropic 品牌服务，不属于 VibeWindow 网关 |
| `/agents` | 查看和管理 Agent | 已实现 | `agent_scheduler_api.rs`、`runtime/agents.rs` | 展示真实 worker/team；无可证明归属的修改操作保持只读 |
| `/autofix-pr` | 自动修复 PR 问题 | 动态来源 | Skill/Plugin command merge | 任务 8；仅在网关返回已启用技能时出现 |
| `/background`, `/bg` | 管理后台任务 | 已实现 | `agent_scheduler_api.rs`、`runtime/tasks.rs` | 作为 `/tasks` 别名展示真实调度任务 |
| `/batch` | 批量并行执行工作项 | 后端阻塞 | `agent_scheduler_api.rs` 仅有结构化 task spec | 缺少从自由文本安全生成批任务的稳定契约 |
| `/branch` | 从当前对话创建分支 | 已实现 | `GatewayClient::session_fork` | 使用真实会话 fork 接口 |
| `/btw` | 在不打断主对话时提出旁路问题 | 后端阻塞 | 无独立旁路 turn API | 不能把普通消息伪装成隔离的旁路上下文 |
| `/bug`, `/share` | 将当前会话提交给 Claude Code 团队 | 显式不适用 | 无 | Anthropic 反馈服务 |
| `/cd` | 切换当前工作目录 | 已实现 | `GatewayClient::project_resolve`、`GatewayUiRuntime` | canonicalize、项目解析、会话切换失败回滚 |
| `/chrome` | 配置 Claude in Chrome | 显式不适用 | 无 | Anthropic Chrome 集成 |
| `/claude-api` | 加载 Claude API 开发技能 | 动态来源 | Skill command merge | 任务 8；不内置 Anthropic 专属技能 |
| `/clear`, `/reset`, `/new` | 开始新的空会话 | 已实现 | `TuiSlashCommandKind::Clear` | 当前内置；任务 5 补齐确认与事务语义 |
| `/code-review` | 审查代码变更 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/color` | 设置会话颜色 | 已实现 | `SettingKey::Theme` | 作为 `/theme` 别名映射为 TUI 主题/颜色设置 |
| `/compact` | 压缩会话上下文 | 已实现 | `GatewayClient::session_summarize`、`SessionCommand::Compact` | 使用持久化消息锚点并要求确认 |
| `/config`, `/settings` | 打开配置界面 | 已实现 | `GatewayClient::config_get/config_patch`、`SettingsPanel` | 真实配置面板与项目偏好保存 |
| `/context` | 查看上下文占用 | 已实现 | `InspectCommand::Context` | 只读报告 |
| `/copy` | 复制最近回复 | 后端阻塞 | 无跨平台剪贴板 runtime 契约 | 不引入平台副作用或伪成功 |
| `/cost` | 查看当前会话成本与用量 | 已实现 | `InspectCommand::Cost`、session usage metadata | 只展示 provider 实际返回的数据 |
| `/dataviz` | 运行数据可视化技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/debug` | 切换或查看调试日志 | 后端阻塞 | 无会话级 debug API | 不允许命令绕过日志安全策略 |
| `/deep-research` | 运行深度研究技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/design-login` | 登录 Anthropic 设计集成 | 显式不适用 | 无 | Anthropic 账号与设计服务 |
| `/design-sync` | 同步设计上下文 | 显式不适用 | 无 | Anthropic 设计服务 |
| `/desktop`, `/app` | 在 Claude Desktop 中打开会话 | 显式不适用 | 无 | Anthropic Desktop 产品入口 |
| `/diff` | 查看当前会话代码变更 | 已实现 | `GatewayClient::session_diff`、`InspectCommand::Diff` | 真实会话 diff 报告 |
| `/doctor`, `/checkup` | 诊断安装、配置和集成 | 已实现 | `IntegrationCommand::Doctor`、只读 gateway probes | 来源失败隔离并脱敏展示 |
| `/effort` | 调整推理强度 | 已实现 | `SettingKey::Effort`、`SessionOptions` | 转发到 Gateway 请求 |
| `/exit`, `/quit` | 退出交互会话 | 已实现 | `TuiSlashCommandKind::Exit` | 当前内置 |
| `/export` | 导出当前对话 | 已实现 | `InspectCommand::Export`、`export_transcript` | 安全导出可见 transcript |
| `/fast` | 切换快速模式 | 后端阻塞 | `SettingKey::Fast`、`SessionOptions` | 命令可发现并明确拒绝；Gateway stream 契约未提供快速模式 |
| `/feedback` | 向 Anthropic 提交反馈 | 显式不适用 | 无 | Anthropic 反馈服务 |
| `/fewer-permission-prompts` | 调整权限提示策略 | 已实现 | `SettingKey::Permissions`、`PermissionMode` | 作为 `/permissions` 别名；保持默认拒绝与服务端授权边界 |
| `/focus` | 运行聚焦工作技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/fork` | 分叉当前会话 | 后端阻塞 | `CommandRegistry` unavailable entry | Claude 后台 fork 缺少契约；明确引导使用 `/branch` |
| `/goal` | 创建持续目标循环 | 后端阻塞 | 无 goal lifecycle API | 不把普通任务伪装为持续目标 |
| `/heapdump` | 生成 Claude Code 进程堆转储 | 显式不适用 | 无 | Anthropic/Node 进程诊断面，不属于 Rust TUI 用户命令 |
| `/help` | 显示可用命令帮助 | 已实现 | `TuiSlashCommandKind::Help` | 当前内置；任务 4 改为注册表动态帮助 |
| `/hooks` | 查看 Hook 配置 | 已实现 | `IntegrationCommand::Hooks`、config API | 只读展示，不执行任意本地脚本 |
| `/ide` | 管理 IDE 集成 | 显式不适用 | 无 | 当前 TUI 没有 IDE 宿主契约 |
| `/init` | 初始化项目 Claude 指令 | 动态来源 | Skill/Workflow command merge | 任务 8/10；仅在真实动态源提供时注册 |
| `/insights` | 生成 Claude Code 使用洞察 | 显式不适用 | 无 | 依赖 Anthropic 产品遥测 |
| `/install-github-app` | 安装 Claude GitHub App | 显式不适用 | 无 | Anthropic GitHub App 安装流 |
| `/install-slack-app` | 安装 Claude Slack App | 显式不适用 | 无 | Anthropic Slack App 安装流 |
| `/keybindings` | 打开快捷键配置 | 后端阻塞 | `keybindings.rs` 只有本地配置模型 | 本计划不新增可变快捷键写入面 |
| `/login` | 登录 Claude 账号 | 显式不适用 | 无 | VibeWindow 网关独立认证，不复刻 Claude 登录 |
| `/logout` | 退出 Claude 账号 | 显式不适用 | 无 | VibeWindow 网关独立认证 |
| `/loop`, `/proactive` | 周期性重复提示 | 动态来源 | Schedule/Skill command merge | 任务 8/11；仅接受网关返回的结构化计划 |
| `/mcp` | 查看 MCP 服务器、工具和资源 | 已实现 | `GatewayClient::mcp_inspect`、`IntegrationCommand::Mcp` | 展示真实服务器、工具、资源和 prompt 元数据 |
| `/memory` | 查看项目与用户记忆 | 已实现 | `IntegrationCommand::Memory`、config/project preference API | 只读展示真实记忆来源 |
| `/mobile`, `/ios`, `/android` | 连接 Claude 移动端 | 显式不适用 | 无 | Anthropic 移动端服务 |
| `/model` | 选择会话模型 | 已实现 | `TuiSlashCommandKind::Model` | 当前内置；任务 6 改为真实 provider/model 面板 |
| `/passes` | 配置反复审查轮次 | 后端阻塞 | 无 review-pass API | 不能用客户端循环伪造服务端审查语义 |
| `/permissions`, `/allowed-tools` | 查看和修改权限 | 已实现 | `GatewayClient::permission_pending_list/reply`、`PermissionMode` | 真实待处理权限与明确回复 |
| `/plan` | 切换计划模式 | 已实现 | `PermissionMode::Plan`、`SessionOptions` | 限制允许工具为计划待办能力 |
| `/plugin` | 管理插件 | 已实现 | `GatewayClient::plugins_get/plugin_enable/plugin_delete` | `/plugins` 别名；删除使用带类型确认 |
| `/powerup` | 启用 Claude Code 增强能力 | 显式不适用 | 无 | Anthropic 分发与账号能力 |
| `/pr-comments` | 读取 PR 评论（官方标记已移除） | 显式不适用 | 无 | 保留审计行但不注册已移除命令 |
| `/privacy-settings` | 打开 Claude 隐私设置 | 显式不适用 | 无 | Anthropic 账号隐私服务 |
| `/radio` | 连接 Claude Radio | 显式不适用 | 无 | Anthropic 专属媒体服务 |
| `/recap` | 查看 Claude Code 产品回顾 | 显式不适用 | 无 | Anthropic 产品内容 |
| `/release-notes` | 查看 Claude Code 发布说明 | 显式不适用 | 无 | 不以 Anthropic 发布通道作为 VibeWindow 功能 |
| `/reload-plugins` | 重新加载插件 | 已实现 | `GatewayClient::plugins_get`、registry merge | 刷新网关真值 |
| `/reload-skills` | 重新加载技能 | 已实现 | `GatewayClient::skills_get`、registry merge | 刷新网关真值 |
| `/remote-control`, `/rc` | 允许远端控制本地 Claude Code | 显式不适用 | 无 | 不扩大远程控制权限 |
| `/remote-env` | 配置远程执行环境 | 显式不适用 | 无 | 当前 TUI 无远程环境授权契约 |
| `/rename` | 重命名当前会话 | 已实现 | `GatewayClient::session_update`、`SessionCommand::Rename` | 真实会话更新 |
| `/resume`, `/continue` | 恢复历史会话 | 已实现 | `TuiSlashCommandKind::Resume` | 当前内置；任务 5 接入真实会话列表与事务恢复 |
| `/review` | 运行变更审查技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/rewind`, `/checkpoint`, `/undo` | 回退到历史消息锚点 | 已实现 | `GatewayClient::session_reset`、`SessionCommand::Reset` | 持久化消息选择与带类型确认 |
| `/run` | 运行选定 Skill 或 Workflow | 动态来源 | Skill/Workflow command merge | 任务 8/10；只能执行网关返回的稳定 ID |
| `/run-skill-generator` | 创建新的技能 | 动态来源 | `GatewayClient::skill_create`、Skill command merge | 任务 8；仅由受信任动态技能暴露 |
| `/sandbox` | 配置 Claude Code OS 沙箱 | 显式不适用 | 无 | VibeWindow 不允许 TUI 降低宿主沙箱策略 |
| `/schedule`, `/routines` | 查看和管理定时任务 | 已实现 | `cron_api.rs` | 真实列表、历史、启停与带类型删除确认 |
| `/scroll-speed` | 设置终端滚动速度 | 后端阻塞 | 本地 controller 常量 | 本计划不增加全局终端偏好写入 |
| `/security-review` | 运行安全审查技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/setup-bedrock` | 配置 Amazon Bedrock | 后端阻塞 | provider/config API 缺少专用安全向导 | 不通过命令收集或回显云密钥 |
| `/setup-vertex` | 配置 Google Vertex AI | 后端阻塞 | provider/config API 缺少专用安全向导 | 不通过命令收集或回显云密钥 |
| `/simplify` | 运行代码简化技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/skills` | 查看和管理技能 | 已实现 | `GatewayClient::skills_get`、`IntegrationCommand::Skills` | 真实技能列表与安全 prompt expansion |
| `/stats` | 查看会话统计 | 已实现 | `InspectCommand::Stats` | 只读报告 |
| `/status` | 查看当前会话、模型和连接状态 | 已实现 | `InspectCommand::Status` | 忙碌期间仍可立即执行 |
| `/statusline` | 配置 Claude Code 状态栏 | 显式不适用 | 无 | VibeWindow footer 是内建 TUI 布局，不执行外部 statusline 命令 |
| `/stickers` | 订购或查看 Claude 贴纸 | 显式不适用 | 无 | Anthropic 营销服务 |
| `/stop` | 停止后台任务 | 后端阻塞 | `agent_scheduler_api.rs`、`runtime/tasks.rs` | 命令可发现并明确拒绝；列表未提供可证明归属且可停止的任务 |
| `/subtask` | 创建隔离子任务 | 后端阻塞 | scheduler create API 需要结构化 `spec_uri/meta_uri` | 任务 9 保留明确拒绝，不从自由文本伪造结构化任务 |
| `/tasks`, `/bashes` | 查看后台 shell 与任务 | 已实现 | `agent_scheduler_api.rs`、`UiTaskOverlay` | 真实调度任务只读面板，忙碌期间立即执行 |
| `/team-onboarding` | 运行团队上手引导 | 动态来源 | Skill/Workflow command merge | 任务 8/10；随真实组织工作流注册 |
| `/teleport`, `/tp` | 将会话传送到远程/网页环境 | 显式不适用 | 无 | Anthropic 远程会话服务 |
| `/terminal-setup` | 安装终端键绑定 | 显式不适用 | 无 | 不从 TUI 修改用户终端配置 |
| `/theme` | 选择 TUI 主题 | 已实现 | `SettingKey::Theme`、`SettingsPanel` | 运行时主题切换并适配暗黑主题 |
| `/tui` | 切换到 Claude Code TUI | 显式不适用 | 无 | 用户已处于 VibeWindow TUI |
| `/ultraplan` | 运行超深度计划技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/ultrareview` | 运行超深度审查技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/upgrade` | 升级 Claude Code 套餐或程序 | 显式不适用 | 无 | Anthropic 订阅与发行服务 |
| `/usage` | 查看计划用量 | 已实现 | `InspectCommand::Cost`、`InspectCommand::Stats` | 只显示网关实际提供的用量，忙碌期间立即执行 |
| `/usage-credits` | 查看或购买 Claude 额度 | 显式不适用 | 无 | Anthropic 计费服务 |
| `/verify` | 运行验证技能 | 动态来源 | Skill/Plugin command merge | 任务 8；随真实技能注册 |
| `/vim` | 切换 Vim 输入（官方标记已移除） | 显式不适用 | `SettingKey::Vim` 仅保留本地快捷方式 | 保留审计行但不注册已移除命令 |
| `/voice` | 启用语音输入 | 显式不适用 | 无 | 当前网关无音频输入契约 |
| `/web-setup` | 配置 Claude Web 集成 | 动态来源 | Skill/Workflow command merge | 任务 8/10；仅在真实动态源提供时注册 |
| `/workflows` | 查看可用工作流 | 已实现 | `GatewayClient::workflow_application_list` | 真实应用列表与稳定 UUID 动态命令 |

## 动态命名空间

| 命令形式 | 用途 | VibeWindow 结论 | API 或本地符号 | 目标任务或理由 |
|---|---|---|---|---|
| `/mcp__<server>__<prompt>` | 执行 MCP 服务声明的 prompt | 动态来源 | `GatewayClient::mcp_inspect`、`CommandAction::PromptExpansion` | 任务 8；服务失败只影响该来源 |
| `/<skill-name>` | 执行已启用 Skill 或 Plugin 提供的 Skill | 动态来源 | `GatewayClient::skills_get/plugins_get` | 任务 8；稳定 ID、来源去重、禁止本地副作用 |
| `/workflow:<name>` | 执行 VibeWindow Workflow | 动态来源 | `workflow_api.rs` SSE run | 任务 10；VibeWindow 扩展 |

## VibeWindow 扩展

| 命令/别名 | 用途 | VibeWindow 结论 | API 或本地符号 | 目标任务或理由 |
|---|---|---|---|---|
| `/project` | 解析并切换网关项目上下文 | 已实现 | `GatewayClient::project_list/project_resolve` | 展示当前 Gateway 项目上下文 |
| `/files` | 查看会话关联文件 | 已实现 | `InspectCommand::Files` | 只读报告 |
| `/cron` | 管理网关 Cron 任务与历史 | 已实现 | `cron_api.rs` | `/schedule` 别名 |

## 覆盖审计

- 所有结论均由最终注册表、处理器、Gateway runtime 与回归测试核对；无计划态占位。
- 动态命令只接受网关枚举出的稳定标识；动态来源不能注册本地文件、进程或外部应用副作用。
- 后端阻塞项必须在命令面或直调时返回原因，不隐藏、不降级为普通提示词。
- 显式不适用项不会进入可执行注册表；已移除的官方命令仅保留审计记录。
