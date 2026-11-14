# 外部会话领域合同

2026-11-14，子任务 1。此文定义后续实现边界，不代表 API、SDK、运行或 macOS 验收通过。本轮按用户要求只编写实现及必要合同，不运行 Cargo、单元测试、实机测试和最终回归，不自动提交。验证留待统一进行。

## 来源与实际底座

固定来源 cc-switch 4.0.25，提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。源仓 HEAD 与此一致。优先读取仓内 `crates/vw-agent-apps/upstream/`，它不属于正式运行模块树。来源 `commands/session_manager.rs` 的十个命令均在 `src-tauri/src/lib.rs` 注册，不将词法命中等同事件。

母任务 1/2/3/4 的编号卡当前已不存在，不能仅由缺卡推断全部验收。实际已有 `vw-agent-apps` 正式包，包含 apps/providers/auth 等模块；母任务 13 卡仍为“未开始”。当前正式包尚无 sessions 模块。本计划不依赖提示词领域实现，不把母任务13未开始当作会话读写阻塞。

现有复用点：

- `vw-agent/src/gateway/api/handlers/agent_apps/access.rs::authorize_digest`：管理员策略、凭据摘要、租户与目标主机 grant、可信主机状态；请求头或 host 字符串不能自行建立权限。现有 `Grant` 没有会话读取、删除、恢复或维护权限，须由本领域 API 任务增加独立授权，不借 provider_config_read/write 或 launch 放大权限。
- `vw-agent-apps/src/apps/discovery/read.rs::LocalReadEnvironment`、`locate_cli`：可信主机 home/search_paths/applications 与 CLI 定位；不要由请求传入 HOME 或任意可执行文件。
- `vw-agent-apps/src/providers/paths.rs`：已有配置路径边界和读取；会话任务复用适用的可信配置来源，不能直接开放用户路径。
- `vw-agent-apps/src/providers/live/`、`codex_login.rs` 和 agent 的 provider 发布入口已有原生配置事务路径；任务56接线前仍须核对其精确事务语义，不能假定覆盖历史迁移。不得在会话领域另造供应商配置系统。
- [任务与错误合同](../../vibewindow/agent-apps/任务事件与错误契约.md)：现有 SSE 不保证持久重放；事件只能提示刷新，可查询状态才是真相。

## 请求、响应与身份

共享 DTO 放 `vw-api-types`；业务在 `vw-agent-apps/src/sessions/`；认证、可信主机解析及存储适配在网关；SDK 为桌面唯一业务入口。URL 和最终类型名由任务61–72按真实调用者固定，下表是语义合同，不宣称端点已注册。

所有请求绑定 ProjectScope（内含 TenantScope）、target_host 和应用实例。服务端重新解析成员、主机与允许根目录。会话身份包含应用、原生 session ID、受控来源身份；与识矩/ACP SessionId 分离，不能跨引擎续聊。公开 sourcePath/命令只作显示，不能作为任意读取、执行授权。

| 操作及唯一实现任务 | 请求语义 | 响应与失败语义 |
| --- | --- | --- |
| 列表/搜索 30、61 | 应用/项目过滤、元数据查询、页大小、受控游标 | 稳定排序含唯一 tie-breaker，游标绑定作用域/主机/过滤/排序/快照；过期明确 cursor_expired，不能 offset 冒充稳定分页 |
| 消息 62、分块63 | 受控会话身份、读取/取消身份 | 结构化消息、轮次；分块 header→messages→done，错误或取消不得发完成成功；迟到块不得写入新阅读上下文 |
| 长正文64、图片65 | 同一会话返回的 ContentRef/ImageRef、受限 offset/limit | 服务端复验归属、指纹/引用有效性，越界/陈旧/类型不支持明确失败；图片仅允许受限本地引用，不下载任意 URL |
| 删除66 | 会话身份列表、明确确认 | 每项结果独立；数据库事务与文件边界检查，成功后清缓存；不支持项明确拒绝，不把部分成功写成全成功 |
| 恢复67 | 会话身份、白名单终端、受控参数 | 服务端依据已验证记录重建 argv/cwd；不接受任意 command/custom shell；Hermes/OpenClaw/DSH 不伪造恢复 |
| 维护68、69 | Codex 选项、服务端备份/任务身份 | 返回任务身份、阶段、数量和最终结果；备份账本约束迁移/幂等/回滚，不接受任意备份路径 |
| 压缩70 | 目标实例、开关 | 原生 Codex 配置读取/保存后重读，返回磁盘字节；非后台转码服务 |
| Markdown72 | 受控会话身份 | 服务端生成正文；桌面复制或用户选定文件保存，远端网关不弹本地文件选择器 |

消息保持来源 camelCase、块 type 的 snake_case 语义；有 blocks 时省略重复 content，content 由 blocks 投影；redacted 思考不尝试解密。时间单位沿用毫秒。详细字段由任务2核对来源 model.rs 和 src/types.ts 固定，不复制 Tauri 类型。

## 执行与错误边界

文件、数据库、CLI、迁移在授权目标主机执行；远端不支持或不可达必须明确失败，不回退桌面本机。Finder 仅同机且确认归属的已存在位置可显示，远端显示来源位置提示；剪贴板与保存对话框为桌面动作。

公共错误用稳定码、安全说明、关联 ID；invalid_input/not_found/unsupported/forbidden/conflict/cursor_expired/source_changed/storage_unavailable/host_unreachable 按真实场景映射。跨作用域身份不得泄露资源是否存在。不得把坏数据库、解析失败或扫描失败静默降为空成功；原始 SQL、文件内容、绝对敏感路径、凭据与错误链不直接公开或日志记录。

长任务 queued/running/cancelling，终态 succeeded/failed/cancelled/interrupted。取消受理不等于已停止；资源清理后才 cancelled，提交成功后不被迟到取消推翻。身份绑定租户、项目、主机、提交主体，查询/取消重复鉴权；重启未完成记录 interrupted，断线先查询而非盲重发。现有 SSE 不承担分块消息或可靠任务日志的隐含恢复协议。

## 来源差异与安全收敛

来源 launch_session_terminal 接受 renderer 任意 shell 字符串，基于可信 renderer 的豁免不适用于网关。新链只从受控身份重建命令；custom 配置必须显式授权并限制模板，不能借本迁移默认扩大能力。AGY 的 key_resume_target 是账号/供应商受控对接点，密钥不得进入会话 DTO。

Windows/Linux/WSL：保留来源，不在本轮验收范围；hermes_wsl.rs 与 reader.py 不能据 macOS 核对提升为已验证。真实终端和实际字形渲染全部留待后续统一验证。
