# 工作区与应用专属设置前置契约

2026-11-14，任务 1 静态核对完成。范围为母任务 15 的前置合同与交叉归属；不实现运行行为。用户本轮要求不执行 Cargo、单元测试、实机测试或最终回归。本文的“存在”只表示源码入口存在，不表示编译、权限拒绝或 macOS 行为验证通过。

## 输入与证据等级

来源 cc-switch 4.0.25，`git rev-parse HEAD` 输出 `ca58c0313360eae7c851fbd7553c1614c7af03e7`（退出 0）。来源根为同级 `cc-switch`；下文来源路径均相对该根，Rust 简写相对 `src-tauri/src/`。目标是当前共享 checkout，已有其他会话未提交修改，不能把整个工作区视为已验收基线。

已读取本计划 plan.md、任务卡、来源映射.md、接口边界.md及母计划执行分线。历史交付入口：[底座交付映射](../../vibewindow/agent-apps/底座交付映射.md)、[供应商合同](provider-contracts.md)、[应用目录](task-01.md)、[应用 API](task-33.md)、[远程隔离](task-45.md)、[模型目录验收](../model-catalog/macos-acceptance.md)、[授权主机隔离](../../vibewindow/agent-apps/auth/回归账号权限与主机隔离.md)。历史证据仅引用，不重签其结论。母任务 7–14 的目录、提示词、会话等实现必须按直接消费接口逐项核对，不能以母任务编号推定完成。

## 实际合同与可复用边界

以下目标路径相对仓库根。

| 合同 | 实际文件 / 符号 | 消费与限制 |
| --- | --- | --- |
| 应用身份 | `crates/vw-api-types/src/agent_apps/catalog.rs::AppId`；`crates/vw-agent-apps/src/apps/catalog.rs::catalog` | OpenClaw、Hermes、Agy、Dsh、Mcode、Pi、Omp 使用已有 ID；MiniMax 是 mcode，不新造记忆应用 ID |
| 执行端路径 | `crates/vw-agent-apps/src/providers/paths.rs::read/execution_environment/open_beneath` | 授权 HOME 内路径；环境取执行进程，非请求环境；macOS 外明确 unsupported；目录/文件 Present、Absent、Inaccessible 分开，拒绝链接和父级逃逸。路径观察不授权写入 |
| 路径 API/SDK | `crates/vw-agent/src/gateway/api/handlers/app_providers/paths.rs::router_with_policy`；`crates/vw-gateway-client/src/client/app_providers/paths.rs` | `/v1/app-providers/{app}/config/paths` POST；目录 GET/PUT。目录偏好与真实读写解析必须一致，不自行增加客户端路径回退 |
| 主机授权 | `crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs::authorize_provider_config/authorize_provider_editor` | tenant、target_host_id、敏感 token 及管理员 policy；分别使用 ProviderConfigRead/Write。现有 wrapper 不接收 project，不能声称已有工作区项目级授权；工作区/记忆及敏感 env 读权限须在后续任务显式定义，不能用启动权限放行 |
| 主机分派 | 同文件 `authorize_scoped`，及 `agent_apps/mod.rs::provider_config_home/authorize_provider_config_access` | 授权后才取得 host.home；离线与 remote 不支持必须保留错误，不回退桌面。不能由此宣称存在任意远程执行协议 |
| 物理锁与单文件写 | `crates/vw-agent-apps/src/providers/live/write_guard.rs::lock_directory/AppWriteGuard::write/WriteError` | 锁含 AppId 与目录 dev/ino，跨租户别名不能绕过；进程内锁不约束外部应用或其他执行进程。descriptor 相对文件写，私有权限、暂存及 rename；父目录授权由调用者负责 |
| 持久事务 | `crates/vw-agent-apps/src/providers/live/commit.rs::ProviderTransactions::commit_checked` | 复用主机私有 pending、冲突和恢复；不是任意路径写工具，也不是多文件原子回滚承诺。工作区文件不应伪装供应商状态 |
| 两应用配置适配 | `crates/vw-agent-apps/src/providers/openclaw.rs`、`providers/hermes.rs` 的 `read/state/patch` 和 `snapshot/apply` | 已有配置锁与事务接点。现有 provider patch 不等于专属任意分节 API，新增 model/defaults 与 provider 写入必须共享物理锁，不另造配置所有者 |
| 错误 | `providers/live/write_guard.rs::WriteError` | Recovered、Conflict、InvalidResource、Io；SDK `AgentAppsReadError` 区分传输/HTTP/解码。不能用空对象替代缺文件、拒绝或损坏；现有状态码不等于已交付本领域稳定错误 DTO |
| 长任务 | `crates/vw-agent/src/gateway/api/handlers/app_providers/operations.rs::router_with_editor` | `/v1/app-providers/operations` 提交、GET/DELETE `/{id}`、事件入口为供应商任务所有。工作区搜索和 dashboard 不能直接冒用供应商动作；复用取消设计前须落实本领域动作与作用域，不承诺 SSE 持久重放 |
| 模型目录 | `crates/vw-agent-apps/src/models/preset_catalog.rs`、`preset_opencode_openclaw.rs`、`preset_hermes_mcode.rs`；`crates/vw-gateway-client/src/client/model_catalog_api.rs` | 母 5 保持预设/模型元数据所有权。OpenClaw provider/model 组合去重、名称回退、中文排序的专用投影尚未在本次核对中定位；这是消费模型选项前的缺项，不能在 Iced 临时另写业务排序 |

默认路径：OpenClaw `~/.openclaw/openclaw.json`；Hermes `~/.hermes/config.yaml`（HERMES_HOME）；Agy `~/.gemini/antigravity-cli/settings.json`；Dsh `~/.dsh/profiles/{web,desktop,headless,acp}/cordis.patch.yml`（DSH_HOME）；Mcode `~/.minimax/config.yaml`（MINIMAX_DATA_DIR/MAVIS_DATA_DIR）；Pi `~/.pi/agent/{models,settings}.json`（PI_CODING_AGENT_DIR）；Omp `~/.omp/agent/models.yml` 优先 models.yaml，config.yml 优先 config.yaml，另含 mcp.json。Omp profile/PI_CONFIG_DIR/XDG 数据路径按 `paths::read` 实际分支，不与 Pi 混用。`~` 始终是授权执行主机 HOME。专属工作区位置尚需任务 2 从 OpenClaw 配置实际解析，不能直接把配置目录当 workspace。

## 逐命令归属

以下逐行沿用只读来源映射，追加当前接点。API 前缀为 `/v1`；尚未实现的路径见接口边界.md，不能当当前可调用接口。

| 来源命令 | 唯一所有者 | 当前实现 / API、SDK、Iced 接点或缺项 |
| --- | --- | --- |
| `import_hermes_providers_from_live` | 母任务 4 供应商领域（导入/live provider 读取）；本计划任务 1 核对，设置 API/桌面复用 | `providers/hermes.rs::declarations/state`；`/app-providers/hermes/config/import`、`config/state`；SDK `hermes_import/hermes_state`；Iced `message/app_providers/hermes.rs`。state 为脱敏状态，不能冒充原始凭据片段读取 |
| `get_hermes_live_provider_ids` | 母任务 4 供应商领域（导入/live provider 读取）；本计划任务 1 核对，设置 API/桌面复用 | `providers/hermes.rs::declarations/state`；`/app-providers/hermes/config/import`、`config/state`；SDK `hermes_import/hermes_state`；Iced `message/app_providers/hermes.rs`。state 为脱敏状态，不能冒充原始凭据片段读取 |
| `get_hermes_live_provider` | 母任务 4 供应商领域（导入/live provider 读取）；本计划任务 1 核对，设置 API/桌面复用 | `providers/hermes.rs::declarations/state`；`/app-providers/hermes/config/import`、`config/state`；SDK `hermes_import/hermes_state`；Iced `message/app_providers/hermes.rs`。state 为脱敏状态，不能冒充原始凭据片段读取 |
| `get_hermes_model_config` | 任务 14（接入Hermes记忆与模型路由） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `get_hermes_memory` | 任务 11（实现Hermes双记忆文件） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `set_hermes_memory` | 任务 11（实现Hermes双记忆文件） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `get_hermes_memory_limits` | 任务 12（实现Hermes记忆限制与开关） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `set_hermes_memory_enabled` | 任务 12（实现Hermes记忆限制与开关） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `open_hermes_web_ui` | 任务 27（实现Hermes面板探测） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `launch_hermes_dashboard` | 任务 28（接入Hermes面板启动动作） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `import_openclaw_providers_from_live` | 母任务 4 供应商领域（导入/live provider 读取）；本计划任务 1 核对，设置 API/桌面复用 | `providers/openclaw.rs::declarations/state`；`/app-providers/openclaw/config/import`、`config/state`；SDK `openclaw_import/openclaw_state`；Iced `message/app_providers/openclaw.rs`。state 为脱敏状态，不能冒充原始凭据片段读取 |
| `get_openclaw_live_provider_ids` | 母任务 4 供应商领域（导入/live provider 读取）；本计划任务 1 核对，设置 API/桌面复用 | `providers/openclaw.rs::declarations/state`；`/app-providers/openclaw/config/import`、`config/state`；SDK `openclaw_import/openclaw_state`；Iced `message/app_providers/openclaw.rs`。state 为脱敏状态，不能冒充原始凭据片段读取 |
| `get_openclaw_live_provider` | 母任务 4 供应商领域（导入/live provider 读取）；本计划任务 1 核对，设置 API/桌面复用 | `providers/openclaw.rs::declarations/state`；`/app-providers/openclaw/config/import`、`config/state`；SDK `openclaw_import/openclaw_state`；Iced `message/app_providers/openclaw.rs`。state 为脱敏状态，不能冒充原始凭据片段读取 |
| `scan_openclaw_config_health` | 任务 21（实现OpenClaw配置健康扫描） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `get_openclaw_default_model` | 任务 16（实现OpenClaw默认模型分节） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `set_openclaw_default_model` | 任务 16（实现OpenClaw默认模型分节） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `get_openclaw_model_catalog` | 任务 17（实现OpenClaw模型白名单分节） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `set_openclaw_model_catalog` | 任务 17（实现OpenClaw模型白名单分节） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `get_openclaw_agents_defaults` | 任务 18（实现OpenClaw代理默认设置） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `set_openclaw_agents_defaults` | 任务 18（实现OpenClaw代理默认设置） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `get_openclaw_env` | 任务 19（实现OpenClaw环境设置） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `set_openclaw_env` | 任务 19（实现OpenClaw环境设置） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `get_openclaw_tools` | 任务 20（实现OpenClaw工具设置） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `set_openclaw_tools` | 任务 20（实现OpenClaw工具设置） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `list_daily_memory_files` | 任务 7（实现每日记忆列表元数据） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `read_daily_memory_file` | 任务 5（实现每日记忆读取与保存） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `write_daily_memory_file` | 任务 5（实现每日记忆读取与保存） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `search_daily_memory_files` | 任务 8（实现每日记忆中文全文搜索） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `delete_daily_memory_file` | 任务 6（实现每日记忆幂等删除） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `read_workspace_file` | 任务 4（实现白名单工作区原子读写） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `write_workspace_file` | 任务 4（实现白名单工作区原子读写） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |
| `open_workspace_directory` | 任务 26（实现工作区目录主机动作） | 本领域后续任务；本次未定位完整 API→SDK→Iced 闭环，不能宣称 macOS 行为或拒绝路径通过 |

## 交叉入口核对

| 来源文件与入口 | 唯一归属 / 当前接点 | 后续限制与证据缺项 |
| --- | --- | --- |
| `commands/agy_account.rs::agy_account_is_running/agy_account_quota`；agy_config.rs | 母 3/4/6/7；`providers/agy.rs`、`apps/lifecycle/`、`auth/`、`usage/` | is_running 是旧账号写回风险提示；Keychain/OAuth、绑定账户额度不能由通用配置页替代。本次未重验系统钥匙串/真实配额 |
| `commands/dsh.rs::dsh_account_status/sign_out/login_start/login_poll/login_cancel`；dsh_config.rs | 母 4/6；`providers/dsh.rs`，SDK `app_providers/dsh.rs` 与 `agent_apps_auth_flow.rs`、`agent_apps_auth_accounts.rs` | 配置与授权保持两条职责；退出和取消不得仅清空 UI；本次未验证五入口运行状态机 |
| Hermes MiniMax key；mcode_config.rs | 母 3/4/5/6；`providers/mcode.rs`、`providers/hermes/form.rs`、`models/preset_hermes_mcode.rs` | 使用现有原生 schema/凭据投影，不造 MiniMax 工作区；内建 key 与端点匹配需后续消费测试 |
| `commands/pi.rs::get_pi_current_state/update_pi_provider_usage_script/get_pi_session_discovery`；pi_config、services/pi_state | 母 4/7/13/14；`providers/pi.rs`，SDK `app_providers/pi.rs` 的 state、usage-script、form 接点 | current 不由新增成员决定；usage script 保存不等于执行证据；session discovery 和 prompt 文件闭环须由母 14/13 单独核实 |
| `commands/omp.rs::get_omp_current_state`；omp_config.rs | 母 4/13/14；`providers/omp.rs`、SDK `app_providers/omp.rs::omp_state` | explicit providers 与 builtin default 分开；源码来源允许 default 无显式节点。提示词/会话可达性未在本次得到运行证据 |
| 两应用 ProviderForm、headers、builtin ids、hooks | 母 4/5/6；`providers/openclaw_form.rs`、`providers/hermes/form.rs` 与对应 SDK/Iced message 文件 | 保留专属字段与秘密编辑意图；设置页复用，不再复制 provider 存储或配置写入器 |
| `pnpm-workspace.yaml`、Hermes 图标、三语言 workspace 手册 | 母 1、3/20、本计划文档 | workspace 清单非运行工作区；图标不产生业务能力；中文文档覆盖行为不等于已完成三语言翻译 |

## 后续编码准入

可直接复用已有 AppId、路径观察、主机授权接点和物理锁/事务实现；不能把它们自动扩展为工作区内容权限。优先补齐实际 workspace 路径、项目边界、敏感 env 授权、本领域任务取消动作，以及模型选项投影合同。此处记录的是尚需交付或核实的合同，不新增通用配置、trait、路由或绕过。

本任务仅完成静态核对与交接；依用户指令删除编号卡，不因未测试保留待回归卡。后续实现仍须拒绝没有实际合同的访问路径。未执行本任务卡的文档断言脚本、单元测试或 macOS 行为验收；未声称通过任何运行验证。
