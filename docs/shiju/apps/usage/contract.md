# 余额、套餐额度与查询脚本合同

本合同供任务 8–59 实施和验收。上游包装直接调用 Tauri，余额和套餐命令接收 URL 与密钥。ShiJu 以授权范围内的 ID 和不透明凭据引用发起查询，由执行端选取适配器和凭据。原始行为与实现、测试归属见[来源覆盖](coverage.md) · [上游查询包装](../../../../crates/vw-agent-apps/upstream/src/lib/api/subscription.ts#L8-L48) · [上游结果类型](../../../../crates/vw-agent-apps/upstream/src/types/subscription.ts#L1-L55)。

## 方案选择

| 方案 | 收益与成本 |
| --- | --- |
| 直接复用 Tauri 命令 | 可少移植一层，但必须携带 AppHandle、事件与应用状态，并改造接收原始密钥的调用边界。 |
| 全部改成脚本 | 易扩展模板，但会把 OAuth、账号轮换、签名和原生槽处理搬到脚本边界，增加运行时权限及解析成本。 |
| 原生适配器和隔离 QuickJS | 采用。官方订阅、余额和 Coding Plan 保留类型化的原生解析及签名。脚本在执行端受限运行，宿主验证网络请求和结果。每家适配器需要独立响应夹具，QuickJS 还要维护执行限额、取消、出站策略及返回值校验。 |

## 已有入口与接入边界

| 母任务 | 已核对入口 | 额度接入 |
| --- | --- | --- |
| 1 独立后端 | Cargo workspace 已注册 `vw-agent-apps`，网关依赖此包。[workspace](../../../../Cargo.toml#L9) · [原生模块](../../../../crates/vw-agent-apps/src/lib.rs) | 查询业务置于原生包，API/存储编排由网关持有，Iced 经 SDK 调用。 |
| 2 网关底座 | `tenant_scope_middleware` 装配 `TenantScope`（平台和公司）。`ProjectScope` 增加项目。项目 handler 在租户范围解析 ID。应用访问策略从 `x-agent-apps-token` 解析主体并按租户、目标主机和动作校验；策略提供本地绑定的 HOME、凭据根和主机状态。[中间件](../../../../crates/vw-agent/src/gateway/middleware.rs) · [作用域类型](../../../../crates/vw-api-types/src/id.rs) · [项目 handler](../../../../crates/vw-agent/src/gateway/api/handlers/project.rs) · [主机授权](../../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs) | 项目归属和额度动作授权需在任务 9、44–46 接入。现有主机策略的匹配维度为平台、公司、主机和动作，项目成员权限需要可信项目上下文。 |
| 2 设置与存储 | `GatewaySettingsStore::{get,upsert}` 复用 `GlobalDatabase` 的 SQLite/PostgreSQL。已交付 `GET/PATCH /v1/agent-apps/preferences?targetHostId=…`，使用独立用户和主机命名空间。[设置存储](../../../../crates/vw-agent/src/gateway/settings_store.rs) · [偏好路由](../../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/preferences.rs) | 任务 38 使用独立额度命名空间，存放脱敏设置与引用。存储键包含项目、主机、应用、供应商及适用的主体/账号，权限从网关验证结果取得。 |
| 2、3 任务与事件 | 母任务 2 交付共同行为契约。母任务 3 的真实进度/取消为 `GET/DELETE /v1/agent-apps/operations/{id}?targetHostId=…`，事件为 `GET /v1/agent-apps/operations/{id}/events?targetHostId=…`。该事件是最长 25 秒的 JSON 长轮询，响应前重新鉴权。共用的 `Operation` 提供 `snapshot`、`subscribe` 和 `cancel_and_wait`。SDK 已有 `agent_apps_operation`、`agent_apps_operation_events` 和 `agent_apps_operation_cancel` 及各自的 `_authorized` 版本。[底座任务契约](../../../vibewindow/agent-apps/任务事件与错误契约.md) · [生命周期路由](../../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs) · [Operation](../../../../crates/vw-agent-apps/src/apps/lifecycle/operations.rs) · [SDK](../../../../crates/vw-gateway-client/src/client/agent_apps_lifecycle_api.rs) | 现有 HTTP handler 的操作表和权限绑定安装类 `AppLifecycleAction`。任务 37、45、48 复用执行和事件机制及客户端恢复语义，扩展现有操作资源以承载额度任务，并为额度任务校验自己的项目、主体和动作。当前路由尚无法读取、订阅或取消额度任务，进程内状态也无法提供重启恢复。 |
| 4 供应商事务 | `GET /v1/app-providers/{app}/{id}` 返回脱敏摘要。执行端从 `AppProviderStore` 的 `app_providers.records` 读取配置。供应商绑定使用 `claude_account_binding`、`codex_account_binding`、`agy_account_binding` 和 `kimi_account_binding`。[供应商路由](../../../../crates/vw-agent/src/gateway/api/handlers/app_providers.rs) · [供应商存储](../../../../crates/vw-agent/src/app_providers.rs) · [绑定交接](../../agent-apps/provider-contracts.md) | 任务 9 在同一授权上下文内读取配置、绑定与凭据。供应商记录和绑定需要跟踪实际生效版本，供缓存与进行中请求失效。 |
| 5 模型目录 | `vw-agent-apps::models` 提供模型预设及元数据。[模型模块](../../../../crates/vw-agent-apps/src/models/mod.rs) | 供应商身份由已授权记录解析。额度类别由适配器能力和查询设置决定。 |
| 6 账号授权 | `GET /v1/agent-apps/auth/{accounts,status,bindings}` 使用 `host_id`、`service`，绑定读取可带 `account_id`。对应 SDK 为 `agent_apps_auth_accounts`、`agent_apps_auth_status`、`agent_apps_auth_bindings`。`AuthAccountStore` 用 `AuthAccountScope`（租户、主体、主机、服务）解析受管账号及 `agent_apps.auth_accounts.v2` 元数据；`codex_access_token`、`xai_access_token`、`copilot_session` 等在服务内取用凭据。Codex/xAI 已有私有轮换代次检查。[授权路由](../../../../crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_read.rs) · [SDK](../../../../crates/vw-gateway-client/src/client/agent_apps_auth_read.rs) · [账号存储](../../../../crates/vw-agent/src/agent_apps/auth_accounts.rs) · [凭据租约](../../../../crates/vw-agent-apps/src/auth/credential_lease.rs) | 任务 9 将已有账号身份和受管凭据接到额度解析。母任务 6 的 `AuthCredentialState` 是登录投影；额度五态由查询域分别计算。统一可比较的额度凭据版本及仅限查询的取用边界仍待接入。 |

以下为已有接口的调用形态，省略网关认证、作用域头和主体凭据。

```http
GET /v1/agent-apps/auth/status?host_id=mac-host&service=codex_oauth
GET /v1/agent-apps/auth/bindings?host_id=mac-host&service=codex_oauth&account_id=account-a
GET /v1/agent-apps/preferences?targetHostId=mac-host
GET /v1/app-providers/codex/provider-a
GET /v1/agent-apps/operations/op-a?targetHostId=mac-host
GET /v1/agent-apps/operations/op-a/events?targetHostId=mac-host
DELETE /v1/agent-apps/operations/op-a?targetHostId=mac-host
```

## 身份与凭据

服务端从认证上下文取得租户（平台、公司）和主体，核验请求选择的项目、目标主机、应用、供应商和账号。显式账号或绑定指向的账号须在同一主体、主机、服务内查验。失效显式绑定直接失败。仅在该适配器允许且无显式目标时解析托管默认号或目标主机独占的原生登录槽，缓存使用最终解析出的账号身份。绑定主机离线或执行通道不可用时返回明确错误。

请求只传资源 ID、允许的查询选项和不透明凭据引用。只在获得执行授权后，由指定主机解析引用、取用最小权限的短时凭据。已有长期凭据由受管存储持有，限制本次查询的取用生命周期，适配器仅使用上游支持的短时派生凭据。禁止客户端以供应商 token、路径或 URL 声明凭据或主机权限。密钥和刷新材料留在主机，公开 DTO、URL、日志、事件和 JS 异常只承载脱敏信息。脚本出站由宿主检查目的地、重定向和主机网络授权，脚本输出也要经过结果校验与脱敏。

## 查询结果、失败和缓存

| 类别 | 返回约束 |
| --- | --- |
| 官方订阅 | 保存真实档位、比例、重置时间、套餐标签和可用的超额明细。Credits、可用重置次数和估算值保留来源及单位。 |
| 余额 | 保留各钱包剩余金额、币种及可用的总额、赠金或充值明细，负值保留，缺失与零区分。 |
| Coding Plan | 保留多窗口用量、已用/总额/剩余或百分比及重置时间。智谱个人版仅在上游明确无套餐时按任务 16 回落余额，结果标明余额来源。 |
| 自定义脚本 | 宿主校验每项套餐、有效性、数量和单位，保留脚本来源。试运行使用独立草稿和结果，禁止保存设置、污染正式快照或发布正式刷新事件。 |

每次查询分别表达结果类别、来源、查询时间、成功或失败及凭据状态。凭据状态为 `valid`、`expired`、`refresh_pending`、`not_found`、`parse_error`。结果中的缺失值显式表达，估算、官方额度、余额及消费账单分别标识。公共错误使用稳定码、安全说明和关联 ID。非法输入、未认证、未授权、作用域内未找到、冲突、能力不支持和服务不可用分别沿用网关的 400、401、403、404、409、501、503 边界。上游拒绝与网关权限失败分开归类。

同一授权范围内共享缓存和进行中请求。键包含平台、公司、项目、主体、目标主机、应用、供应商、查询种类、解析出的账号、凭据版本与生效查询配置版本。同账号的多个入口解析到相同键时合并请求。传输超时、网络故障、HTTP 429 或明确可重试的上游 5xx 可保留旧成功值至距真实成功不足 10 分钟，保留原成功时间并标明陈旧及本次错误。鉴权拒绝、确定的凭据失效、缺配置、无法解析或确定业务失败立即清除对应旧值。版本变化、切绑、登出及撤销使旧值和在途结果失效。发布前再比较身份与版本，丢弃迟到结果。在任何缓存读写前检查权限与目标主机。读取失败时返回可区分的错误。

## 额度 API 与权限矩阵（待实现）

网关认证、可信租户和主体、项目归属、目标主机和应用访问是每行的前置条件。对账号和供应商同时核验资源归属。批量与自动调度逐项核验，跳过未获准项时不得透露其存在。授权必须覆盖实际副作用，账号读取、应用偏好写入和安装许可各自保持原范围。

| 方法与路径 | 语义 | 额外权限 |
| --- | --- | --- |
| GET `/v1/apps/usage/snapshot` | 读取当前脱敏快照，不触发出站查询 | 额度读取 |
| GET `/v1/apps/usage/templates` | 读取授权应用的模板元数据 | 额度读取 |
| GET `/v1/apps/usage/settings` | 读取设置与脱敏引用 | 额度设置读取 |
| PUT `/v1/apps/usage/settings` | 校验并保存查询设置 | 额度设置写入及引用使用权限 |
| POST `/v1/apps/usage/refresh` | 刷新一项，长任务返回不透明操作 ID | 额度刷新及对应凭据使用权限 |
| POST `/v1/apps/usage/refresh-all` | 对当前授权范围内的项进行有界批量刷新 | 批量刷新及逐项刷新、凭据使用权限 |
| POST `/v1/apps/usage/script-test` | 使用受控草稿在目标主机试运行 | 脚本试运行、对应凭据使用和出站权限 |
| GET 操作资源、GET 操作事件、DELETE 操作资源 | 目标路径沿用上节已列出的操作资源。断线先读同一 ID 的状态和对应快照，显式取消后等待清理终态 | 重新核验任务所属项目、主体、主机、应用、账号和原动作。状态和事件要求读取权，取消还需取消权限 |

额度请求示意，字段序列化由任务 8、44–46 落实。

```http
POST /v1/apps/usage/refresh
Content-Type: application/json

{"projectId":"project-a","targetHostId":"mac-host","appId":"codex","providerId":"provider-a","accountId":"account-a","kind":"official_subscription"}
```

操作 ID 由服务端生成并绑定完整授权范围。取消受理与已停止分别表达，重复取消按同一操作终态处理，已经提交的结果保留。断线重连从权威操作状态和快照恢复，禁止盲目重复发起刷新。任务 45 需明确重启中断、状态保留和事件恢复行为。事件只提示已确认的状态变化，携带最少身份字段、关联 ID、操作 ID 和版本，在投递前校验范围，SDK 再比对当前选择，漏收后重读快照。

## 实施交接

任务 8 定义类型，9 对齐项目、账号和凭据版本，33–37 实现隔离和取消，38–40 接入持久化、缓存和调度。任务 44–48 开始 API/SDK 实施前，应以本合同中的真实路由及相邻测试核验接入点，并补齐额度授权、操作归属和事件隔离。任务 54–58 分开验收 macOS 无窗口链路、真实账号和桌面窗口。本文证据限于固定来源和已有代码核对。
