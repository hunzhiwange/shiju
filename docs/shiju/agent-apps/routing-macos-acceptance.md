# 常驻路由 macOS 验收记录

## 任务 1　核对来源与冻结路由契约

2026-11-10，macOS 27.0（26A428），arm64。状态为**已完成（来源盘点与接口合同）**。任务 1 没有实施前置。母任务 8 的实现和实机验收由后续任务交付。

### 来源与分工

CC Switch 4.0.25，固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。以仓库 `crates/vw-agent-apps/upstream/` 快照为读取入口，以相邻仓库的固定 Git 对象校对。相邻工作树的 `package.json` 修改和 `demo/` 不参与取证。主责文件的逐项路径、SHA256 和责任任务见[来源映射](../../../.plans/2026-11-05-常驻路由与完整协议适配/来源映射.md)，任务顺序见[执行索引](../../../.plans/2026-11-05-常驻路由与完整协议适配/执行索引.md)。

- 75 个主责文件各自与来源清单、快照、固定 Git 对象三方 SHA256 一致。母清单能力 8 的文件与命令集合均与来源映射相同。两份来源命令文件的 39 个候选全部存在且实际注册，`commands/proxy.rs` 34 个，`commands/global_proxy.rs` 5 个。命令行号指属性或定义。下方逐项给出接收入口或唯一主责。
- `services/proxy.rs` 创建和替换 `ProxyServer`，`mode/controller.rs` 管客户端模式与文件事务。`forwarder.rs` 和模型发现 handler 消费适配器，`get_adapter_for_provider_type` 的枚举分支只提供类型映射，实际请求分支以 `get_adapter(app)`、格式判定及凭据注入为准。Tauri 注册、`lib/api/proxy.ts`、`lib/api/globalProxy.ts`、`hooks/useProxyStatus.ts` 与查询层互相对照。
- 相邻来源用例覆盖 WebSocket 426、浏览器拒绝、回环自转发、独立端点别名、图片与 Alpha 搜索、provider 推断和模式相关配置。`src-tauri/tests/proxy_commands.rs` 实际只回归计价来源读写，移交母 17，不能用作任务 10/12/86 的启停、参数或事务验证。Rust 内联来源用例迁移时放到独立测试文件。
- 执行索引 1–108 顺序及盘点时的交叉链接已检查。编号任务完成后按约定清理任务文件，按任务号在本验收记录查交付证据。

### 正式数据面路由

`proxy/server.rs::build_router` 注册 37 个路径模式。其中 34 个静态路径显式挂载 38 个方法，另有 3 个 Gemini `any`。下表按显式方法登记，Axum 自动 HEAD 属框架行为。花括号内表示同一组的可选前缀，`空` 表示无前缀。

| 来源方法与路径 | 落点 |
| --- | --- |
| GET `/health`、`/status` | 10、85，分别映射授权的管理健康和脱敏状态，由管理面提供 |
| POST `/v1/messages`、`/claude/v1/messages`、`/claude-desktop/v1/messages` | 35、38–43、46–50，Claude Desktop 独立应用身份 |
| GET `/claude-desktop/v1/models`、`/models`、`/v1/models` | 79，前者只发布 Desktop 映射，后两者为 Codex 模型发现 |
| POST `{空,/v1,/v1/v1,/codex/v1}/chat/completions` | 34 |
| POST `{空,/v1,/v1/v1,/codex/v1,/grokbuild/v1}/responses` | 36、51–60，Grok Build 使用独立选路身份 |
| GET `{空,/v1,/v1/v1,/codex/v1}/responses` | 36、95，WebSocket 握手返回 426 `websocket_not_supported`，验证 Codex 回退 HTTP/SSE。Grok Build 没有此 GET |
| POST `{空,/v1,/v1/v1,/codex/v1,/grokbuild/v1}/responses/compact` | 76–78，Grok Build 命名空间独立 |
| POST `{空,/v1,/v1/v1,/codex/v1}/alpha/search` | 81 |
| POST `{空,/v1,/v1/v1,/codex/v1}/images/generations` | 82 |
| POST `{空,/v1,/v1/v1,/codex/v1}/images/edits` | 83 |
| ANY `/v1beta/*path`、`/gemini/v1beta/*path`、`/gemini/v1/*path` | 37、80。ShiJu 在三前缀下仅放行 GET `/models`、`/models/{id}` 和 POST `/models/{id}:generateContent`、`:streamGenerateContent`、`:countTokens`，显式拒绝其他子路径或方法 |

数据面先校验来源、Host 和应用能力凭证，再解析正文或接触上游。数据面凭证由模式事务消费，绑定租户、项目、执行主机、应用、账号和代次，撤销须终止在途请求（8–9）。HTTP 错误沿用各协议格式（24）。Messages 没有正式 `count_tokens` 注册。来源 Gemini handler 直接转发方法与路径，ShiJu 的白名单只承诺表中的路径。来源的 200 MiB 设置、浏览器规则和连接退出策略需分别通过 6、9、18–23 的验证，额度不能从路由数推断。

### Provider 与协议

| 来源 ProviderType | 应用、认证与协议归属 |
| --- | --- |
| `Claude` | Claude Code / Desktop，Anthropic API key。Messages 原生（35），按档案格式桥接 Chat、Responses 或 Gemini（38–50） |
| `ClaudeAuth` | Claude Code / Desktop，配置选择 Bearer-only。Messages 与相同格式分支（31、35、38–50） |
| `Codex` | Codex / Grok Build 适配器，普通 API key 使用 Bearer 或目标协议所需认证。Chat、Responses 原生（34、36），Responses → Chat 或 Anthropic（51–60）。官方登录分支另走 72–73 |
| `Gemini` | Gemini CLI 原生，或 Claude Code / Desktop 的 `gemini_native` 档案。Google API key，Gemini 原生（37、80）及 Messages → Gemini（46–50） |
| `GeminiCli` | 同一 Gemini 协议族，OAuth Bearer 由授权账号提供（37、46–50、75、80） |
| `OpenRouter` | Claude 适配器，默认 Messages 兼容透传，显式格式才能选择对应桥接（31、35、38–50） |
| `GitHubCopilot` | 托管 OAuth 换账号专属 Copilot token。Claude 入口可按模型选 Chat 或 Responses，Codex 路径按来源模型和格式分流（38–43、51–54、71） |
| `CodexOAuth` | 托管账号凭据、官方 Codex 路径和账号隔离，Messages → Responses 或原生 Responses（36、41–43、72–73） |
| `XaiOAuth` | 托管账号凭据、Responses 路径，Messages → Responses 与原生 Responses 清理（36、41–43、63、74） |

任务 31 负责九类选择及认证边界。按入口协议与实际格式分流，分别验收请求、JSON、SSE 和拒绝路径。Gemini → Chat 是当前 ShiJu relay 的兼容路径，分配给 2/37 做保行为验证。OpenCode/OpenClaw/Hermes 在来源可选 Codex 适配器，但没有应用路由模式。Pi/Mcode/Kimi/Dsh/OMP/AGY 无来源路由适配器。矩阵只适用于已授权应用和目标组合。

### TS 显示、轮询和事件

| 来源规则或事件 | 迁移责任与证据要求 |
| --- | --- |
| `config/appConfig.tsx` 和 Rust `AppType` | Claude Code、Codex、Gemini、Grok Build 有应用路由参数与故障转移入口。Claude Desktop 也有路由模式，隐藏其应用路由参数与故障转移。Stack 仅 Claude Code、Codex、Claude Desktop，归母 10。其余九个应用隐藏模式入口。后端同样检查能力（12、86、88–89、母 9/10） |
| `utils/providerCapabilities.ts`、`presentation.ts` | 先识别官方账号和受支持的 Codex 接管身份。托管 OAuth 路由要求优先于可编辑的 `apiFormat`，Desktop 普通档案看 direct/proxy，Claude 看非 Anthropic/完整 URL，Codex/Grok 看 Chat、Anthropic、完整 URL 或 TOML `wire_api`。官方账号及 OMO 的连通检测与健康隐藏归母 9（86、91–92） |
| `ProviderForm.tsx`、`CodexFormFields.tsx`、`GrokBuildProviderForm.tsx`、`ClaudeDesktopProviderForm.tsx` | 通用表单对非官方 Claude/Codex 开启请求覆盖和 UA，Grok 有独立编辑和提交路径，Desktop 表单未接该编辑器。Claude 推理项限非官方且 Chat/Responses，Codex prompt cache 选项限非官方 Chat。按应用、档案和格式验证，不能从共用控件推出任意应用都可写（13、16、64、70、91） |
| `requestOverrides.ts`、`userAgent.ts` 及相邻测试 | 空值清空，覆盖必须为 JSON 对象，禁止顶层 `stream`，头名标准化后防重及拒绝受保护头，头值必须为字符串，拒绝除 TAB 外的控制字符，UA 保留合法 UTF-8。执行端验证先于存储/转发（13–17、91） |
| `query/proxy.ts`、`useProxyStatus.ts`、`RoutingSection.tsx` | 状态在运行时轮询，模式/Stack 按能力和面板启用。界面只消费 SDK。来源界面只收 1024–65535，任务 5 明确允许服务请求端口 0 并回读实际端口。客户端主机与监听独立，WSL 设置登记为范围外及未验证（5、10、86–92） |
| `provider-switched` | `proxy/failover_switch.rs:116` 发出 `appType/providerId/source=failover`，`lib/api/providers.ts` 监听。母 9 拥有故障转移通知，`commands/failover.rs:301` 同属母 9。项目应用与托盘发射分别由母 16 和 20 负责，85/92 仅消费已授权状态 |
| `proxy-official-warning` | 子计划来源映射漏列。实际由 `mode/controller.rs:854` 在提交模式后通过 `services/proxy.rs::emit` 发出，`App.tsx:608` 订阅。母 4 拥有模式事务，86/92 在授权响应或告警中保留等价可见语义。不得仅凭告警允许不受支持的接管 |
| 启动接入失败、请求终态 | 来源启动失败是 `take_startup_attach_failures` 取后清空，属于查询而非领域事件（7、92）。任务 85 新增一次脱敏终态事件，usage 交母 17 |

额外交叉来源 `codex_compaction.rs` → 76–78，`codex_oauth_auth.rs` 及 `browser.rs`、`xai_oauth_auth.rs`、`copilot_auth.rs` → 母 6 管账号，71–75 仅消费凭据与传输，`copilot_model_map.rs` → 71，`codex_quota_protection.rs` → 73 消费母 7 额度。模式、文件残留与事务属母 4，目录聚合与 daemon 属母 10，计价与用量属母 17。事件按[底座事件合同](../../vibewindow/agent-apps/任务事件与错误契约.md)发布，发出前做作用域隔离，重连以查询快照恢复。

### 底座核对

| 母任务 | 当前可复用的交付与剩余接线 |
| --- | --- |
| 1 | workspace 已注册 `vw-agent-apps`，upstream 跟踪 1,489 个文件，本项所需 75 个主责文件均匹配。结论仅覆盖本次来源 |
| 2 | [底座交付映射](../../vibewindow/agent-apps/底座交付映射.md)及 `gateway/api/handlers/agent_apps/access.rs` 已有 Gateway 认证、`TenantScope`、`x-agent-apps-token` 摘要及 host grant。现有 scoped grant 基础匹配 platform/company/host，常驻路由须另接真实项目成员和路由动作权限，不能拿主机授权代替项目授权。离线拒绝，remote 501，无客户端回退 |
| 3 | `vw-api-types::agent_apps::AppId`、`vw-agent-apps/src/apps/catalog.rs` 已有 14 个身份和执行主机目录，复用应用 ID。常驻路由能力仍以协议与模式白名单判定 |
| 4 | [供应商验收](provider-macos-acceptance.md)及 `app_providers/{mode_state,mode_apply,request_options}` 的 handler/SDK 已交付。事务存在 `RouteRuntime::prepare/publish` 注入点，现有 handler 只执行 Claude 路径，尚无生产常驻 `ProviderRouteRuntime` 注入。接入的五应用、批量恢复及连接地址重投影按 86–87 核验真实事务支持。请求选项当前 `NotApplied`，需 13/15/16 真正消费并核验 Grok 能力 |
| 5 | `gateway/api/handlers/model_catalog.rs`、`model_catalog_api.rs` 和供应商模型编辑已有目录与元数据。79/80 仍需把授权应用和已选目标投影到协议发现端点 |
| 6 | [账号接口契约](../../vibewindow/agent-apps/auth/接口契约.md)和 `auth/credential_file.rs` 已有账号作用域、绑定及 Copilot/xAI 等凭据租约。71–75 必须验证真实 provider/主机/账号租约接线，官方在线验收豁免不能替代路由实测 |
| 7 | `GET /v1/apps/usage/snapshot` 与 `apps_usage_snapshot` 已提供授权额度快照。未找到可直接执行常驻 Codex 请求准入的闸口，73 需确认安全消费底座快照或明确阻塞 |

当前 `vw-agent/src/model_relay/` 有 `transport.rs` 与协议实现供 2–3 检查复用。会话 relay 使用回环临时端口、4 MiB、120 秒及绑定身份。定时 `RelayBinding::new` 的上限是 900 秒，`for_prompt` 由 ACP 启动所有者释放时撤销。常驻能力由网关拥有独立作用域与代次。计划编写时的「workspace 未注册」以及「所有会话能力固定 TTL」已不符合当前源码。

### 控制 API 与 SDK 合同

新增接口以下表为准。路径前缀 **R** = `/v1/app-routing/hosts/{targetHostId}`，**A** = `R/projects/{projectId}/apps/{app}`。GET 无请求体，JSON DTO 为 camelCase 且拒绝未知字段。SDK 使用 `GatewayClient` 的认证和租户上下文，传递授权 token 与结构化错误。DTO 落在 `vw-api-types/src/app_routing/`，网关使用对应 handler 和有状态路由注册，Iced 通过 `vw-gateway-client/src/client/app_routing/`。复用行沿用现有 DTO/SDK 模块。新增入口和权限均为实施合同。

所有入口先认证成员，再按租户、资源和执行主机检查动作许可。**H读/H管** 是显式主机级路由读取/管理授权，生命周期、出口、监听配置影响同一主机上共用服务，项目成员权限不足以授权这些动作。**P读/P管** 另要求该项目的已验证成员与对应应用权限。只有 H管加非回环明确授权且数据面鉴权就绪才允许非回环监听。**诊断**同时要求已验证项目成员和显式主机诊断许可。复用供应商入口保留 `provider_config_read/write` 等已有要求。路由读取和写入带 `projectId`，作为现有 DTO 的兼容扩展。只对常驻路由操作要求该选择器，直连旧请求继续使用原权限合同。实际附接与目标发布前校验项目与路由权限，无法唯一证明项目时拒绝发布。凭据解析、文件、监听、任务执行均在获准的 macOS 执行主机。网关负责控制授权、错误投影，桌面无直连主机管理旁路。

新增接口返回 `ApiErrorBody`（`error.code/message/details?`，关联 ID 置于脱敏 details），保留底座认证状态和复用接口已有的状态语义。统一错误为 400 `invalid_input`、401 `unauthenticated`、403 `forbidden`、404 `not_found`、409 `conflict`、422 `unsupported_app`、429 `busy`、501 `unsupported_host`、503 `unavailable`。作用域无权时不透露其他资源存在性。下表补充各入口特有错误，且所有回复和错误均不返回秘密、原始 URL 凭据、用户路径或上游错误正文。

| 任务与来源命令 | HTTP 与请求 | 成功响应与特有失败 | 权限及 SDK |
| --- | --- | --- | --- |
| 10 `get_proxy_status`、`is_proxy_running` | GET `R/status` | 200 `RoutingStatus`（state、boundAddress、boundPort、startedAt，未绑定字段为 null）。state 为 stopped/starting/running/stopping/failed，running 从 state 派生 | H读 `app_routing_status` |
| 10 /health | GET `R/health` | 200 `{ready:true}`，503 `not_ready`。不返回地址或目标 | H读 `app_routing_health` |
| 10 `start_proxy_server` | POST `R/start`，空 JSON 对象 | 200 `RoutingStatus`，已启动返回当前状态，409 `address_in_use` 或 `transition_in_progress` | H管 `app_routing_start` |
| 10 `stop_proxy_server` | POST `R/stop`，空 JSON 对象 | 200 `RoutingStatus`，已停止幂等，409 `apps_attached` 或 `transition_in_progress` | H管 `app_routing_stop` |
| 12 `get_proxy_config`、`get_global_proxy_config` / `update_proxy_config`、`update_global_proxy_config` | GET / PUT `R/config`。PUT `{enabled,listenAddress,listenPort,loggingEnabled}` | 200 返回同形 `RoutingConfig`。IPv4/IPv6，端口 0 或 1024–65535。绑定/重投影失败返回 409 `config_not_applied` 并保持旧有效配置和监听 | H读/管 `app_routing_config` / `app_routing_set_config` |
| 12 `get_proxy_config_for_app` / `update_proxy_config_for_app` | GET / PUT `A/config`。PUT `{firstByteTimeoutSecs,idleTimeoutSecs,nonStreamingTimeoutSecs}`，分别 1–120、0–600、60–1200，0 仅表示 idle 禁用 | 200 同形 `AppRoutingConfig`，非法数值 400。enabled 从模式取得，重试、故障转移及熔断写入归母 9 | P读/管 `app_routing_app_config` / `app_routing_set_app_config` |
| 28 `get_global_proxy_url`、`get_upstream_proxy_status` / `set_global_proxy_url` | GET / PUT `R/egress`。PUT `{mode,proxyUrl?}`，mode 为 direct/system/custom，仅 custom 需写入 URL（http/https/socks5/socks5h） | 200 `{configuredMode,effectiveMode,scheme,hasCredentials}`，无代理时 scheme=null。400 非法代理，409 `egress_not_applied` 保持原出口。空配置映射为系统策略或直连须明确选 mode | H读/管 `app_routing_egress` / `app_routing_set_egress` |
| 29 `test_proxy_url` | POST `R/projects/{projectId}/egress/tests`，空 JSON 对象。用已配置出口到执行端固定 TLS 目标（来源候选中 `https://api.anthropic.com`），10 秒总时限，无调用者提供的目标或 URL | 202 `{operationId}`，409 `egress_not_configured`，429 达并发上限。真实连接结果见操作状态 | 诊断 `app_routing_egress_test_start` |
| 30 `scan_local_proxies` | POST `R/projects/{projectId}/apps/{app}/egress/scans`，空 JSON。仅固定八个 IPv4 回环候选，每端口至多 100ms，2 秒网络期限，排队后复核；GET/DELETE 同路径追加 `/{task}` 查询/取消 | 202 `DiagnosticJobIdentity`，类别 outbound_scan。状态保留进度及端口开放状态、来源协议候选；开放只代表 TCP 可连。429 达任务容量上限 | 诊断 `app_routing_start_outbound_scan` / `app_routing_outbound_scan_status` / `app_routing_cancel_outbound_scan` |
| 29–30 读取进度与取消 | GET / DELETE `R/projects/{projectId}/egress/operations/{operationId}` | GET 200 `{operationId,kind,state,completed,total,result?,errorCode?}`，状态 queued/running/completed/failed/cancelled/interrupted。测试结果含 latencyMs，扫描结果含候选端口及 scheme。DELETE 202 接受取消，终态重发 200 返回当前状态。不存在 404，过期 410 `operation_expired` | 诊断且匹配发起主体 `app_routing_egress_operation` / `app_routing_egress_cancel` |
| 85 请求状态 | GET `A/requests?limit=…`，1–100，默认 50 | 200 `{activeCount,requests}`，只含 requestId、state、startedAt、firstTokenMs、finishedAt、errorCode 等脱敏摘要，按最近开始时间排序。无活动返回空列表 | P读 `app_routing_requests` |
| 85 请求终态 | GET `A/requests/events`，SSE | 200 `request_finished`，一次终态含作用域、requestId、结果与时间，无 prompt/usage 载荷，usage 单次交母 17。重连先读快照 | P读 `app_routing_request_events` |
| 7、92 `take_startup_attach_failures` | GET `R/startup-failures` | 200 `{failures:[{projectId,app,code,occurredAt}]}`，当前启动代次的有界快照，只读不因另一桌面读取而丢失 | H读，逐项过滤项目权限，`app_routing_startup_failures` |
| 86 `get_proxy_takeover_status`、`get_app_mode`、`get_direct_provider`、`is_live_takeover_active` | 复用 GET `/v1/app-providers/{app}/mode/state?targetHostId=…&projectId=…`，按已授权项目读取，跨应用状态逐应用查询 | 200 `ProviderModeStatus`，从 mode/attached/directProvider 派生，routeRuntime 未就绪明示 unavailable | P读加供应商读取许可，`provider_mode_state` |
| 86 `set_proxy_takeover_for_app`、`set_proxy_route`、`switch_proxy_provider` | 复用 POST `/v1/app-providers/{app}/mode/apply`，`ModeApplyRequest {targetHostId,projectId,action}`，已有 `proxy {providerId}`、`direct`、`detach`。增加 `selectRoute {providerId}`，直连时只记目标，已附接时用同一事务切换。Stack 动作由母 10 的模式事务交接 | 200 `ModeApplyResult`（configurationChanged、attached、targetPublication），未就绪 503，事务冲突 409。待发布明确为 pending | P管加供应商编辑许可，`provider_mode_apply` |
| 86 `stop_proxy_with_restore`、`exit_proxy_apps_in_mode` | POST `R/restore-and-stop` 空对象 / POST `R/modes/exit` `{mode:"route" 或 "stack"}`。后者保留来源模式迁移的 Claude Code/Codex 行为 | 200 `{affectedApps,status}`，每个客户端写回由母 4 事务负责，预先校验全部受影响作用域。任何恢复失败 409 `restore_incomplete`，已安全提交的结果可查，监听保留 | H管且每个作用域有 P管及供应商编辑许可，`app_routing_restore_and_stop` / `app_routing_exit_mode` |
| 87 `get_proxy_client_host` / `set_proxy_client_host` | GET / PUT `R/client-host`，PUT `{host}`，null 或空值推导，IPv6 正规化，拒绝通配地址、端口和 URL | 200 `{host,effectiveHost}`。投影失败 409 `client_projection_failed`，可按原输入重试，监听独立 | H读/管，写入时还需所有受影响配置写权限，`app_routing_client_host` / `app_routing_set_client_host` |
| 13、16 请求覆盖和 UA | 复用 POST `/v1/app-providers/{app}/{id}/request-options/read` `RequestOptionsTarget`、PUT `/v1/app-providers/{app}/{id}/request-options` `RequestOptionsWrite`，路由作用域携带 `projectId` | 200 `RequestOptionsView` 脱敏结果。格式非法 400，应用/档案不支持 422，待接通时现有状态为 NotApplied | P读/管加供应商配置许可，`app_provider_request_options_read` / `app_provider_request_options_save` |
| 母 9 `get_provider_health`、`reset_circuit_breaker`、`get_circuit_breaker_config`、`update_circuit_breaker_config`、`get_circuit_breaker_stats` | 健康、熔断及故障转移由母 9 提供独立接口 | 母 8 交请求结果与授权目标消费 | 母 9 |
| 母 10 `get_proxy_stack`、`restart_codex_app_server_daemon`、`acknowledge_codex_stale_clients`、`set_proxy_stack_member`、`adopt_codex_stack_catalog` | Stack 名单、模型发布和 daemon 生命周期由母 10 提供独立接口 | 母 8 只接可信目标与模型目录。缺接口明确不可用 | 母 10 |
| 母 17 `get_pricing_model_source`、`set_pricing_model_source` | 计价模型来源由母 17 提供独立接口 | 母 8 交一次用量观察 | 母 17 |

任务 8 的签发与撤销是执行端附接事务内部动作，凭证只交给授权客户端配置写入，管理 API 只返回状态。29–30 的操作状态绑定发起主体、项目与主机，遵循底座任务协议。终态保留 10 分钟，网关重启后查询已接受且未完结的操作返回 interrupted。SDK 断开后查状态，取消成功与自然终态竞争由执行端裁决。85 事件与健康页均不提供上游凭据。任何管理端点均以领域 DTO 解析，命令候选仅用于追溯。

### 验证证据与后续记录

| 检查 | 退出码与结果 |
| --- | --- |
| 母清单、来源映射、固定 Git 对象和仓库快照按路径校验 SHA256 | 0，75/75 |
| 解析两份命令属性、行号和 `generate_handler!`，对照合同分配 | 0，39/39 |
| 解析 `build_router` 与 `ProviderType`，检查顺序及交叉链接 | 0，37/37 路径、38/38 显式方法、3/3 any 挂载、9/9 类型、108/108 任务 |
| 对照事件名与源行、前端订阅、TS 规则及额外交叉来源 | 0，2 个事件、母清单 6 处引用 |
| `git diff --check -- ".plans/2026-11-05-常驻路由与完整协议适配" "docs/shiju/agent-apps/routing-macos-acceptance.md"` | 0 |
| 文档单独暂存后 `git diff --cached --check -- docs/shiju/agent-apps/routing-macos-acceptance.md` | 0 |

## 后续 macOS 证据表

本项仅文档，无 TDD、有效行为测试、Cargo 槽位或构建目录。macOS 真实客户端、版本、上游账号、SDK 与 Iced 实际交互等待对应任务取证。Windows/Linux/WSL 范围外，未验证。

| 后续验收任务 | 来源及实现/API/SDK/UI | macOS、架构与客户端版本 | 红绿命令、有效测试数、退出码、槽位与 target 路径 | 实际交互 |
| --- | --- | --- | --- | --- |
| 93–98 生命周期及五应用原生链路 | 待补 | 待验 | 待验 | 待验 |
| 99–106 桥接、出口、辅助端点与图片 | 待补 | 待验 | 待验 | 待验 |
| 107–108 桌面实际渲染与覆盖闭环 | 待补 | 待验 | 待验 | 待验 |

## 任务 2　提取共享上游传输入口

2026-11-10，macOS 27.0（26A428），arm64。任务 1 的来源与接口合同由 `ad5f5ca61` 交付。复用上节核对的母任务 1–7 底座，常驻运行时、凭据签发及官方账号路径分别由后续任务验收。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/forwarder.rs`、`proxy/providers/adapter.rs` 与仓库快照及来源映射的 SHA256 一致。来源 handler 先选供应商、组装认证与协议请求，再发送 HTTP。本项仅接入现有 reqwest 传输路径。

- `model_relay::upstream::execute` 接收调用者已经授权的目标、成形请求和取消信号，请求 URL 须精确匹配目标，显式 Host 覆盖被拒绝。沿用现有客户端配置和重定向限制，响应头等待与透传响应体均可取消。
- `app_routing::SharedTransport::forward` 是内部 Rust 数据面适配入口，接收已授权的目标及上游凭据，保留上游 HTTP 状态和响应元数据供后续协议层处理。该入口独立于 `RelayBinding` 和 `RelayIdentity`。会话仍逐请求校验绑定令牌、项目和租户并解析当前供应商凭据，状态映射与 Chat、Messages、Responses、Gemini 转换留在会话路径。
- 环回 HTTP/TCP 夹具核对方法、路径、查询、正文和独立的上游认证头，覆盖目标与 Host 不匹配、请求前取消、响应头等待取消、SSE 取消与截断、gzip 解压、错误状态和不跟随重定向。WSL 连接地址见来源映射，Windows、Linux、WSL 范围外且未验证。

实测槽位 **1**。全部 Cargo 命令的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。首次工作区红灯命令退出 101，因会话摘要依赖的 `full_access` 字段不匹配而执行 0 项。后续在 `07edaedc8` 源码快照叠加本任务文件验证，重新编译会话摘要依赖。格式化另用源码副本，仅同步本任务相关格式，保留无关内容。

| 阶段 | 确切命令 | 结果 |
| --- | --- | --- |
| 红灯 | `cargo test -p vw-agent --lib cc_route_002_ -- --nocapture` | 退出 101，执行 2 项，2 项行为断言失败。目标不匹配时实际收到上游 418，响应头等待取消后仍超时。 |
| 格式化 | `cargo fmt --all` | 退出 0。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_002_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 两次因其他任务进入全仓检查窗口而中断，均退出 130。等待串行窗口后复跑退出 0。 |
| 会话基线 | `cargo test -p vw-agent --lib acp_model_source_relay_ -- --nocapture` | 退出 0，21 项通过，0 项失败，1 项真实 Gemini CLI 用例按既有标记跳过。 |

红灯和绿灯链接阶段均报告既有 macOS `__eh_frame` 大小警告。提交的 Rust 文件与验收副本逐一比较一致。限定提交范围的 `git diff --cached --check` 退出 0，只读 `plan.md` 无差异。槽位 1 的本任务锁已释放，编译缓存保留。状态为 **已完成**，下一步为任务 3 的响应引用隔离。

## 任务 3　隔离共享响应引用作用域

2026-11-10，macOS 27.0（26A428），arm64。已交付共享索引与现有会话生产入口，状态为 **已完成，待回归**。应用数据面的真实入口验收待任务 8 和 36 接通后执行。

- `app_routing::reference_scope::References` 是唯一引用存储实现。键区分会话和应用，所有正向、反向及 item 查找均匹配完整作用域。应用键使用 `ProjectScope` 携带租户和项目，另含执行主机、应用、账号、供应商和凭据代次。生产代码当前仅从经会话 relay 校验的身份构造会话键。
- `ProviderRelayBackend` 为原生 `POST /v1/responses` 的 JSON 和 SSE 共用索引。保留索引 4096 条上限、标识长度检查、反向匹配顺序和会话 `RelayBinding` 的 900 秒上限、代次隔离与撤销。
- 应用索引测试逐一改变平台、公司、项目、执行主机、应用、账号、供应商和凭据代次，检查正反向 response 查找及 item 所有权。真实会话回环验证本地引用、跨租户拒绝、SSE 引用在后续 JSON 请求回放，以及在途撤销。固定来源与快照已核对。WSL 连接地址见来源映射，Windows、Linux、WSL 范围外且未验证。

实测槽位 **1**。所有 Cargo 命令使用 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。格式化在临时源码副本执行，仅复制本任务文件的格式化结果。

| 阶段 | 确切命令 | 结果 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 退出 0。 |
| 红灯 | `cargo test -p vw-agent --lib cc_route_003_ -- --nocapture` | 首次退出 101，测试夹具使用了 `ProjectId` 未提供的 `Display`，执行 0 项。修复后退出 101，执行 2 项，两项均因共享索引尚只接收会话键，首次应用写入返回 403。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_003_ -- --nocapture` | 退出 0，2 项通过，0 项失败。 |
| Responses 基线 | `cargo test -p vw-agent --lib acp_model_source_relay_responses_ -- --nocapture` | 退出 0，7 项通过，0 项失败。 |
| 绑定基线 | `cargo test -p vw-agent --lib acp_model_source_relay_binding_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

测试链接阶段报告既有 macOS `__eh_frame` 大小警告。任务的 8 个 Rust 文件与格式化副本逐一比较一致，差异检查退出 0。各阶段的槽位锁均已释放，缓存保留。

任务 8 接通逐请求应用凭据后，由获准目标解析把应用账号、供应商和实际凭据代次绑定到鉴权结果。任务 36 需把该结果接到同一引用索引的 JSON/SSE 读写。届时通过真实 macOS 应用数据面和回环上游补验同作用域回放、跨作用域拒绝、代次变化与撤销，以及会话和应用间的双向隔离。

## 任务 4　定义常驻服务状态机

2026-11-10，macOS 27.0（26A428），arm64。状态为 **已完成，待回归**。按授权采用批次 3 已提交的共享引用实现 `fa43a48b1` 作为基线，并交付状态机的最小生产监听调用点。

- `vw_agent_apps::routing::lifecycle_state::Lifecycle` 管理停止、启动中、运行、停止中和失败。只有成功占有启动转换的调用方可以绑定。运行时重复启动返回现有地址，转换期间冲突，绑定失败后可重试。
- `vw_agent::app_routing::RoutingListener` 是独立的监听所有者。它在状态锁内保留启动权，绑定后发布 TCP 回环监听。接收循环在解析或转发前关闭每条连接。停止信号由监听任务消费，任务先释放套接字再发布终态，因此发起停止的调用方取消等待后，监听器仍能退出并恢复可启动状态。
- 状态机用例覆盖五个状态、启动与停止冲突、幂等以及失败恢复。TCP 用例使用真实回环，覆盖并发启动得到同一个运行地址或转换冲突、连接被关闭、端口占用后重试、停止后释放端口及停止调用方取消。

验证在基于 `fa43a48b1` 的临时源码快照叠加本任务文件执行，以隔离其他工作区改动。只同步本任务七个 Rust 文件的格式化结果，并逐个比较其内容与验证快照一致。两阶段实际使用槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。阶段结束释放本任务锁，保留缓存。

| 阶段 | 确切命令 | 结果 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红灯和绿灯阶段各退出 0。 |
| 红灯 | `cargo test -p vw-agent-apps --lib cc_route_004_ -- --nocapture` | 退出 101，执行 2 项，1 项通过，1 项因 `Starting` 期间第二次启动错误取得启动权而失败。 |
| 状态机绿灯 | `cargo test -p vw-agent-apps --lib cc_route_004_ -- --nocapture` | 退出 0，2 项通过，0 项失败或忽略。 |
| TCP 回环 | `cargo test -p vw-agent --lib cc_route_004_ -- --nocapture` | 退出 0，3 项通过，0 项失败或忽略。链接器报告既有 `__eh_frame` 大小警告。 |
| 工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 提交差异 | `git diff --cached --check`（限定本项路径） | 退出 0。 |

任务 5 在同一个监听所有者上完成端口 0、IPv4/IPv6 和绑定错误合同。任务 7 和 10 接通网关持有及授权启停后，用真实 macOS 网关进程回归并发启停、状态投影、失败恢复、关闭时端口释放，以及关闭桌面和结束会话时监听仍可用。

## 任务 5　绑定监听地址与实际端口

2026-11-10，macOS 27.0（26A428），arm64。沿用任务 4 提交 `691032a9b` 的 `vw_agent::app_routing::RoutingListener`。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/server.rs` 与 `services/proxy.rs` 已和快照及来源映射逐一核对 SHA256。来源通过内核返回的地址发布实际端口；本项只发布运行地址，不回写端口 0 的配置。

- `start(0)` 默认绑定 IPv4 回环。`start_on` 接受 IPv4 或 IPv6 字面量，允许 IPv6 外层方括号，并将 IPv4 映射地址转为 IPv4 后检查是否为回环。域名、URL、附带端口以及非回环地址在绑定前明确拒绝。
- 启动返回及 `bound_address()` 均读取实际绑定地址。绑定成功才发布运行地址；端口占用返回 `AddressInUse`，状态为失败且无地址，可以重试。停止后不再发布该地址。
- 当前常驻数据面鉴权和显式主机级授权尚未接通，因此非回环一律拒绝，回环连接也在解析或转发前关闭。任务 8–10 接通鉴权和授权后再开放非回环，任务 10/12 投影状态和配置接口。WSL 客户端地址留在来源映射，Windows、Linux、WSL 范围外且未验证。

验证在 `691032a9b` 的临时源码快照叠加本项两个 Rust 文件后运行，以隔离共享工作区的其他改动。各阶段实际使用槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。

| 阶段 | 确切命令 | 结果 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红灯和绿灯阶段均退出 0。 |
| 红灯 | `cargo test -p vw-agent --lib cc_route_005_ -- --nocapture` | 退出 101，3 项通过，1 项因有效的方括号 IPv6 回环地址返回 `InvalidAddress` 而失败。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_005_ -- --nocapture` | 退出 0，4 项通过，0 项失败或忽略。通过真实 TCP 连接验证默认 IPv4 与 IPv6 的实际端口，覆盖映射地址、非法及非回环地址、占用和重试。 |
| 监听回归 | `cargo test -p vw-agent --lib cc_route_004_ -- --nocapture` | 退出 0，3 项通过，0 项失败或忽略。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 提交差异 | `git diff --cached --check`（限定本项路径） | 退出 0。 |

测试链接报告既有 macOS `__eh_frame` 大小警告。两个提交的 Rust 文件与验证快照逐字一致；只读 `plan.md` 无差异。本项各阶段的槽位锁已释放，缓存保留。状态为 **已完成**，下一步为任务 6 的连接回收。

## 任务 6　停止并回收连接任务

2026-11-10，macOS 27.0（26A428），arm64。以任务 5 提交 `8b86cd074` 为验证基线。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/server.rs` 与直接调用方 `services/proxy.rs` 已和仓库快照、来源映射核对 SHA256。来源的停止路径等待监听循环，连接任务独立运行。

- `vw_agent::app_routing::RoutingListener::stop` 先令 `app_routing::shutdown::serve` 关闭监听套接字，再取消并等待其持有的所有连接任务。连接 future 持有请求和响应流。5 秒内仍未退出返回 `ListenerError::StopTimeout`，状态保持 `Stopping` 直至完成回收。等待方取消也不影响回收。
- 回环测试在同一个监听所有者上运行 Hyper HTTP/1 夹具，使用生产 `SharedTransport::forward` 访问受控 TCP 上游。实测等待响应头的 HTTP 请求、已收到事件的 SSE、已接收的空闲 TCP、上下游关闭和任务退出、拒绝新连接及同端口立即重绑。另一用例让连接任务暂时无法让出执行权，验证显式超时、停止期间拒绝重启和新连接、释放任务后恢复。
- 生产回环入口继续在解析前关闭连接。任务 7 接通网关所有权，任务 8 接通数据面凭据。WSL 连接地址见来源映射，Windows、Linux、WSL 范围外且未验证。

验证在隔离的 `8b86cd074` 源码快照叠加本任务文件后执行。仅同步本任务文件的格式化结果。实际槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。

| 阶段 | 确切命令 | 结果 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红灯和绿灯阶段均退出 0。 |
| 红灯 | `cargo test -p vw-agent --lib cc_route_006_ -- --nocapture` | 退出 101，执行 2 项，两项分别因停止后 SSE 未关闭、连接任务仍运行时停止误报成功而失败。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_006_ -- --nocapture` | 退出 0，2 项通过，0 项失败或忽略。 |
| 生命周期回归 | `cargo test -p vw-agent --lib cc_route_004_ -- --nocapture` | 退出 0，3 项通过，0 项失败或忽略。 |
| 绑定回归 | `cargo test -p vw-agent --lib cc_route_005_ -- --nocapture` | 退出 0，4 项通过，0 项失败或忽略。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 隔离快照首次退出 0。共享工作区的并发检查结束后，串行复跑退出 0。 |
| 提交差异 | `git diff --cached --check`（限定本项路径） | 退出 0。 |

测试链接器报告既有 macOS `__eh_frame` 大小警告。本项 Rust 文件和依赖清单与验证快照逐一比较一致，只读 `plan.md` 无差异。各阶段的槽位锁已释放，缓存保留。状态为 **已完成**。

## 任务 7　随网关启动与退出

2026-11-10，macOS 27.0（26A428），arm64。状态为 **已完成**。采用批次 6 提交 `ec6df98da` 的监听器与回收逻辑。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `services/proxy.rs` 快照 SHA256 与来源映射一致，启动调用方为 `mode/controller.rs::startup_app`。本项使用 ShiJu 的既有管理员策略作为启动授权来源。

- `gateway::bootstrap` 通过 `app_routing::gateway_owner::run_with_routing` 持有唯一的 `RoutingListener`。独立网关响应 SIGINT 或 SIGTERM，daemon 取消网关服务后等待其返回，两条路径均等待 `RoutingListener::stop` 回收连接。回收超过既有五秒限额会返回错误。真实子进程复测使用默认线程栈，启动 Future 的大型状态移到堆上。
- macOS 网关从管理员通过 `SHIJU_AGENT_APPS_ACCESS_FILE` 显式指定的绝对路径读取既有主机策略。复用同一加载器对目录、文件所有者、权限及符号链接的检查。在线本机条目可设置 `routing_startup`，例如 `{"enabled":true,"listen_port":8080}`。省略 `listen_address` 时绑定 `127.0.0.1`。端口接受 0 或 1024–65535。缺少显式启用、没有唯一合格主机、配置损坏或权限校验失败时不恢复监听。非回环地址拒绝。客户端模式和普通成员授权不参与启动决策。
- macOS 独立网关与 daemon 子进程均从同一持久策略恢复同一端口。实测无桌面进程、断开客户端后仍可连接、没有显式策略入口时保持关闭、原生 Claude 配置字节保持一致。收到 SIGTERM 后，子进程在网关函数返回与进程退出之间按生产绑定方式重绑原端口。另用真实 TCP 连接将清理暂停，验证所有者等待清理完成才返回。
- 当前回环连接仍在协议解析前关闭。数据面凭据、控制接口和配置更新由后续任务交付。Iced 关闭调用链已核查，真实关窗与已认证路由联验留给任务 93。WSL 连接地址留在来源映射，Windows、Linux、WSL 范围外且未验证。

验证在 `ec6df98da` 的隔离源码快照叠加本任务文件后执行。排除了 21 个无关格式化差异及共享工作区中同一文件的额度路由改动。实际槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。

| 阶段 | 确切命令 | 结果 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红灯和最终绿灯阶段均退出 0。 |
| 红灯 | `cargo test -p vw-agent --lib cc_route_007_ -- --nocapture` | 退出 101，1 项执行且失败。网关健康检查通过，常驻监听连接返回 `ConnectionRefused`。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_007_ -- --nocapture` | 退出 0，3 项通过，0 项失败或忽略。覆盖真实进程恢复和退出、缺失或不可信授权拒绝、连接回收等待。 |
| 状态机回归 | `cargo test -p vw-agent --lib cc_route_004_ -- --nocapture` | 退出 0，3 项通过。 |
| 绑定回归 | `cargo test -p vw-agent --lib cc_route_005_ -- --nocapture` | 退出 0，4 项通过。 |
| 停止回归 | `cargo test -p vw-agent --lib cc_route_006_ -- --nocapture` | 退出 0，2 项通过。 |
| daemon 回归 | `cargo test -p vw-agent --lib daemon::tests -- --nocapture` | 退出 0，42 项通过。 |
| 供应商启动恢复回归 | `cargo test -p vw-agent --lib agent_apps::recovery_tests -- --nocapture` | 退出 0，1 项通过。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 提交差异 | `git diff --cached --check`（限定本项路径） | 退出 0。 |

测试链接器报告既有 macOS `__eh_frame` 大小警告。暂存的 Rust 文件与验证快照逐字一致，只读 `plan.md` 无差异。槽位锁已释放，缓存保留。

## 任务 8　签发与撤销应用路由凭证

2026-11-10，macOS 27.0（26A428），arm64。复用批次 7 提交 `e5a864b17` 的网关监听所有者。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/browser_guard.rs`、`proxy/handlers.rs` 与仓库快照及来源映射的 SHA256 一致。来源只对 Claude Desktop 使用单独 token，本项扩展到按授权作用域隔离。

- `app_routing::route_auth::RouteAuth` 由 `RoutingListener` 持有。签发接收模块内部的 `VerifiedRouteBinding`，绑定租户、项目、执行主机、应用、账号与账号代次。使用 256 位系统随机数生成独立凭证。内存注册表保存 SHA256 摘要，原始凭证只随内部签发结果返回，撤销句柄单独持有摘要和签发代次。轮换使同一作用域的旧凭证和在途请求失效，旧撤销句柄不会影响新凭证。
- `RoutingListener::start`、`start_on` 在同一回环监听上接入 HTTP 鉴权，逐请求检查单一 Bearer 或 `x-api-key`，未授权时在读正文前返回 401。进入内部处理前移除凭证头，获准绑定和签发代次可与分发选中的可信目标复核。当前每条连接处理一个请求，撤销会丢弃整个连接任务的 HTTP future，包括正在等待的处理和正在写出的响应流。生产协议分发未就绪，鉴权成功也只返回 503。
- 生产签发仍待任务 86。现有 `ModeApplyRequest` 缺项目选择器，`RouteScope` 只含租户、主机和应用，供应商编辑授权没有项目成员与路由动作许可。任务 86 需在现有模式事务内取得这两项许可及账号代次，向授权客户端配置交付凭证，并在切换、脱离和账号失效时轮换或撤销。任务 36 需把获准绑定与可信目标复核接到真实 Responses 分发。WSL 地址保留在来源映射，Windows、Linux、WSL 范围外且未验证。

验证在 `e5a864b17` 的隔离源码快照执行。红灯使用槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。绿灯及监听回归使用槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与对应槽位相同。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红灯和绿灯均退出 0。仅同步本任务文件的格式。 |
| 红灯 | `cargo test -p vw-agent --lib cc_route_008_ -- --nocapture` | 退出 101，1 项失败。旧监听直接断开，客户端收到 `IncompleteMessage`，未收到预期的 401。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_008_ -- --nocapture` | 退出 0，3 项通过。覆盖管理凭证、实际会话令牌与其他应用凭证的拒绝，读正文前拒绝、逐维匹配、歧义头、轮换、重复撤销和旧撤销句柄隔离。真实回环 TCP 上游记录路径并证实撤销关闭等待响应头的 HTTP 以及已经开始的 SSE。 |
| 状态机回归 | `cargo test -p vw-agent --lib cc_route_004_ -- --nocapture` | 退出 0，3 项通过。 |
| 绑定回归 | `cargo test -p vw-agent --lib cc_route_005_ -- --nocapture` | 退出 0，4 项通过。 |
| 连接回收回归 | `cargo test -p vw-agent --lib cc_route_006_ -- --nocapture` | 退出 0，2 项通过。 |
| 网关回归 | `cargo test -p vw-agent --lib cc_route_007_ -- --nocapture` | 退出 0，3 项通过。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 槽位 1，退出 0。 |
| 差异检查 | `git diff --cached --check`（限定本项路径） | 退出 0。 |

本项 Rust 文件和依赖清单与绿灯快照逐一比对一致。测试链接器报告既有 macOS `__eh_frame` 大小警告。槽位 1、2 的任务锁均已释放，缓存保留。独立凭证和监听鉴权已交付，真实附接与协议分发联验待回归。

## 任务 9　拦截浏览器与主机伪造

2026-11-10，macOS 27.0（26A428），arm64，已完成。按授权采用批次 8 提交 `beb9678b6` 的生产监听鉴权作为基线。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/browser_guard.rs`、`proxy/handlers.rs` 与仓库快照的 SHA256 一致。

- `RoutingListener::start_http` 继续使用同一监听器的 `route_auth::serve`。`browser_guard::allows_host` 在调用分发器或消费正文前校验 Host 和绝对请求目标的 authority。只接受连接实际绑定的回环 IP 或 `localhost`（忽略大小写并兼容一个尾点），显式端口须匹配实际端口。拒绝重复、格式错误或伪造的 Host，HTTP/1.0 继续兼容无 Host。按当前客户端投影收紧来源的主机名范围，无 DNS 查询。监听器继续拒绝非回环绑定。
- 常规路由出现 Origin 即返回 403。`/claude-desktop/v1/messages` 和 `/claude-desktop/v1/models` 的 Origin 例外要求有效的 Claude Desktop 独立凭证，仍校验 Host。真实回环测试中，浏览器 POST 和 WebSocket 升级请求、DNS 重绑定、错误 authority 及跨应用凭证均被拦截。带 `Expect: 100-continue` 的客户端只发送请求头，首条响应即为最终状态。IPv4/IPv6 本机请求仍须通过独立鉴权。
- 回环测试通过内部可信绑定签发夹具凭证。通过闸门和鉴权的请求到达当前分发占位响应 503。生产凭证签发和真实上游联验按授权后置。WSL 连接地址保留在来源映射，Windows、Linux、WSL 范围外且未验证。

红灯、绿灯和回归均使用共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。验证使用隔离的基线源码快照。绿灯恢复了非本项格式化文件，并逐一比对了本项 Rust 文件和依赖清单。测试链接器报告既有 macOS `__eh_frame` 大小警告。两阶段均已释放槽位锁并保留缓存。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 两阶段均退出 0，仅纳入本项格式差异。 |
| 红灯 | `cargo test -p vw-agent --lib cc_route_009_ -- --nocapture` | 退出 101，1 项通过、3 项失败。浏览器 POST、重绑定 Host 和其他应用凭证访问带 Origin 的 Desktop 端点均收到 503，预期为 403。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_009_ -- --nocapture` | 退出 0，4 项通过、0 项失败。 |
| 鉴权回归 | `cargo test -p vw-agent --lib cc_route_008_ -- --nocapture` | 退出 0，3 项通过。 |
| 监听回归 | `cargo test -p vw-agent --lib cc_route_005_ -- --nocapture` | 退出 0，4 项通过。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异检查 | `git diff --cached --check`（限定本项提交路径） | 退出 0。 |

## 任务 11　原子更新运行参数

2026-11-10，macOS 27.0（26A428），arm64。以批次 10 的 `ff417dc2e` 为基线。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `services/proxy.rs` 与 `proxy/types.rs` 的仓库快照匹配来源映射的 SHA256。核对了来源全局配置入口、应用级入口和占用端口恢复测试。

- `app_routing::config_apply::RoutingConfigStore` 使用母任务 2 的 `GatewaySettingsStore` 和当前配置的 `GlobalDatabase`。监听记录按租户和执行主机存储。超时记录另按租户、执行主机、项目和应用存储。三项超时分别校验，只有空闲超时允许 0。更新整条应用超时记录时保留其他作用域和监听记录。
- 全局更新沿用网关监听控制许可。重绑先准备新回环端口，再以原记录为条件写入数据库。绑定或写入失败时保留旧记录、旧监听任务和日志状态，释放预备套接字。成功提交后旧监听停止接收并回收连接，新监听接管同一鉴权状态。日志开关在运行中即时生效，仅输出无请求内容或身份的授权事件。端口 0 保持为配置中的系统分配意图，实际端口由监听状态返回。
- 批次 10 的授权启动入口读取持久设置。管理员策略继续决定能否自动启动，已存禁用可取消本次启动恢复，手动启停仍执行原有主机授权。活动路由阻止改绑。任务 12 接入配置 API/SDK 时检查已落盘的客户端附接，任务 21 消费应用超时执行请求限时，任务 87 负责原生客户端连接地址重投影。WSL 连接地址保留在来源映射，Windows、Linux、WSL 范围外且未验证。

SQLite 使用临时文件库，PostgreSQL 使用独立本机 18.4 实例和隔离模式。双库均用真实回环 HTTP/TCP 验证端口冲突和数据库拒绝更新时旧端口仍返回 401、候选端口可回收、日志开关热更新、成功切换、非法输入拒绝及不同应用、项目与主机的隔离。网关子进程通过 SDK 验证持久禁用抑制自动启动，并在授权手动启动时使用持久端口配置。

验证在 `ff417dc2e` 的隔离源码快照完成，排除了无关格式化差异。红灯和绿灯均实际领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 为该路径下的 `build`。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红、绿阶段均退出 0。 |
| 红灯 | `cargo test -p vw-agent --lib cc_route_011_ -- --nocapture` | 退出 101，两库各 1 项均因端口冲突后配置被覆盖而失败。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_011_ -- --nocapture` | 退出 0，3 项通过，0 项失败或忽略。 |
| 生命周期回归 | `cargo test -p vw-agent --lib cc_route_010_ -- --nocapture` | 退出 0，3 项通过。 |
| 路由回归 | `cargo test -p vw-agent --lib app_routing:: -- --nocapture` | 退出 0，30 项通过，2 项子进程夹具由父测试调用。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 首次检查及共享工作区其他 Clippy 退出后的串行复核均退出 0。 |
| 差异审查 | `git diff --cached --check`（限定本项提交路径） | 退出 0。 |

提交的 Rust 文件与绿灯快照逐字一致。测试链接器报告既有 macOS `__eh_frame` 大小警告。槽位 1 的任务锁均已释放，编译缓存保留。状态为 **已完成**。

## 任务 12　接通运行参数接口

2026-11-10，macOS 27.0（26A428），arm64。采用任务 11 的 `e19bfdf7a` 为基线。已核对 CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `commands/proxy.rs`、`types/proxy.ts` 与仓库快照的 SHA256，并检查了 `lib/api/proxy.ts` 和 `lib/query/proxy.ts` 的直接调用。来源的 `tests/proxy_commands.rs` 覆盖计价配置，运行参数行为由本项的独立测试验收。

- `GET/PUT /v1/app-routing/hosts/{host}/config` 使用 `RoutingConfig`。SDK 方法为 `app_routing_config` 和 `app_routing_set_config`。配置启用位控制管理员允许时的启动恢复。端口 0 由操作系统分配，实际绑定端口通过状态接口读取。
- `GET/PUT /v1/app-routing/hosts/{host}/projects/{project}/apps/{app}/config` 使用 `AppRoutingConfig`。SDK 方法为 `app_routing_app_config` 和 `app_routing_set_app_config`。支持 Claude Code、Codex、Gemini、Grok Build，未保存时依次返回 60、120、600 秒。首字节、空闲和非流式总超时分别接受 1–120、0–600、60–1200 秒，空闲 0 表示关闭。
- 两组入口复用网关认证、管理员成员策略和对应主机的 `routing_read` / `routing_manage`。应用配置还要求同一条许可中的 `routing_apps` 包含对应项目及应用，例如 `[{"project_id":"p1","app_id":"codex"}]`。未显式授予的项目或应用默认拒绝。配置和错误响应沿用 `Cache-Control: no-store` 与结构化 `ApiErrorBody`。
- `RoutingConfigStore` 继续在执行端保存设置。PUT 校验完整 JSON 对象、未知字段、数值与回环地址。改绑在现有控制许可内检查已提交和待提交的客户端附接。端口占用或存在附接返回 409 `config_not_applied`，旧配置与监听保持可用。任务 21 消费应用超时，任务 87 负责原生客户端连接地址重投影。WSL 连接地址保留在来源映射，Windows、Linux、WSL 范围外且未验证。

验证使用 `255e05085` 的隔离源码快照。各阶段实际使用共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 为该路径下的 `build`。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红、绿阶段退出 0，仅同步本任务文件的格式。 |
| API 红灯 | `cargo test -p vw-agent --lib cc_route_012_ -- --nocapture` | 退出 101，1 项失败。已获主机读取许可的 SDK 客户端请求配置，基线只返回无结构化错误体的 401。 |
| SDK 红灯 | `cargo test -p vw-gateway-client --lib cc_route_012_ -- --nocapture` | 退出 101，1 项失败。携带未知字段的 200 响应被误解码为成功。 |
| API 绿灯 | `cargo test -p vw-agent --lib cc_route_012_ -- --nocapture` | 退出 0，1 项通过，0 项失败或忽略。通过真实回环网关验证成员与权限、租户/主机/项目/应用隔离、非法字段和值拒绝、端口占用及已落盘附接保护、改绑后的鉴权、重启后的配置读回。 |
| SDK 绿灯 | `cargo test -p vw-gateway-client --lib cc_route_012_ -- --nocapture` | 退出 0，1 项通过，0 项失败或忽略。验证方法、路径编码、认证头、请求与响应字段、结构化 HTTP 错误及未知字段解码错误。 |
| 网关生命周期回归 | `cargo test -p vw-agent --lib cc_route_010_ -- --nocapture` | 退出 0，3 项通过。 |
| SDK 生命周期回归 | `cargo test -p vw-gateway-client --lib cc_route_010_ -- --nocapture` | 退出 0，1 项通过。 |
| 原子更新回归 | `cargo test -p vw-agent --lib cc_route_011_config_apply_expected_behavior -- --nocapture` | 退出 0，1 项通过。 |
| 持久配置回归 | `cargo test -p vw-agent --lib cc_route_011_lifecycle_reads_the_persisted_listener_config -- --nocapture` | 退出 0，1 项通过。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | `git diff --cached --check`（限定本任务路径） | 退出 0。 |

本项 Rust 文件和 Cargo 清单与绿灯快照一致。测试链接器报告既有 macOS `__eh_frame` 大小警告。槽位锁已释放，缓存保留。状态为 **已完成**。

## 任务 13　原生校验请求覆盖字段

2026-11-10，macOS 27.0（26A428），arm64。基线 `b598b3dd1` 含任务 12 运行参数 API/SDK（`798d036e2`）。核对 CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `requestOverrides.ts`、`requestOverrides.test.ts` 和 `proxy/forwarder.rs`，仓库快照的 SHA256 均匹配来源映射。已核对通用供应商表单、Grok 表单、转发器和相邻用例。复用母任务 2、3 的执行主机授权和应用身份，以及母任务 4 任务 23、68 的请求选项契约。原生客户端配置和账号租约由各自的生产所有者提供。

- 沿用 `vw-agent-apps::providers::validation` 的唯一校验规则。来源前端、来源转发器、当前 Rust 规则和回归用例覆盖同一组 43 个受保护头。请求 DTO 要求 headers/body 为 JSON 对象，拒绝非字符串头值及同名重复 JSON 键。Rust 校验 HTTP token、除 TAB 外的 ASCII 控制字符、trim 和大小写归一后的重复头、受保护头与顶层 `stream`。合法头名保存为小写，保留嵌套供应商字段。
- `validation::merge` 省略空 headers/body 成员。显式清除仍保存为 `localProxyRequestOverrides: {}`，回读显示未配置。省略覆盖字段仍保留旧值，历史空成员也可正常读取。沿用 Claude/Codex 应用限制。
- 生产写入链为 `GatewayClient::app_provider_request_options_save` → `PUT /v1/app-providers/{app}/{id}/request-options` → 授权 handler → `AppProviderStore::request_options_save` → `providers::request_options::update` → `validation::merge` → 档案 CAS。只读摘要沿用 POST `/v1/app-providers/{app}/{id}/request-options/read`。本项验证保存入口的校验和归一，实际请求应用与转发顺序由任务 15 验证。Windows、Linux 和 WSL 在范围外，WSL 地址仍见来源映射。

共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 为该路径下的 `build`。只保留本项 Rust 文件的格式化差异。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红灯和绿灯阶段均退出 0。 |
| 红灯 | `cargo test -p vw-agent-apps --lib cc_route_013_ -- --nocapture` | 退出 101，2 项通过，1 项因保存结果保留空 `body` 失败。 |
| 绿灯 | `cargo test -p vw-agent-apps --lib cc_route_013_ -- --nocapture` | 退出 0，3 项通过，0 项失败或忽略。 |
| 原生校验回归 | `cargo test -p vw-agent-apps --lib cc04_validation_ -- --nocapture` | 退出 0，6 项通过。 |
| SDK 回归 | `cargo test -p vw-gateway-client --lib cc04_validation_ -- --nocapture` | 退出 0，3 项通过。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 首次在 `vw-desktop` 写编译元数据时磁盘耗尽，退出 101。单作业首次重试在 `vw-config-types` 创建临时目录时磁盘耗尽，退出 101。复用已生成的缓存再次单作业重试，退出 0。两次重试使用 `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1`。 |
| 差异审查 | `git diff --cached --check`（限定本项提交路径） | 退出 0。 |

状态为 **已完成**。已释放本项槽位锁，保留构建缓存。任务卡由原会话在集成工作区收尾。

## 任务 14　过滤私有参数并保护 Schema 属性

2026-11-10，macOS 27.0（26A428），arm64。任务 13 的请求覆盖校验由 `5255827c6` 交付。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/body_filter.rs` 与仓库快照及来源映射的 SHA256 一致。来源 `forwarder.rs::prepare_upstream_request_body` 使用空白名单过滤正文。

- `routing::private_params::filter_private_params` 递归处理对象和数组，只在已识别的工具 Schema 内保留 `properties`、`patternProperties`、`definitions` 与 `$defs` 的声明名。Schema 定义对象内的私有字段及其他正文中伪装成名称映射的私有字段仍会移除。纯策略覆盖 Chat、Messages、Responses、Gemini 的来源工具 Schema 形状。
- 为解除过滤与协议分发的调度依赖，接通现有常驻监听上的单一路径。生产调用链为 `RoutingListener::serve_http` → `route_auth::serve` → `dispatch::forward_json` → 过滤 → JSON 序列化 → `SharedTransport::forward`。只接受已绑定目标的 Codex `POST /v1/chat/completions` 非流式 JSON。目标 URL 和供应商认证与同一代次的授权绑定一并持有。撤销或轮换使旧绑定和在途请求失效。未授权和未就绪分别返回 401、503，其余入口和不支持的正文形状明确拒绝。
- 回环夹具经真实监听和能力凭证进入生产分发，受控上游收到绑定目标、独立认证及过滤后的实际正文。用例验证未就绪、旧凭证、方法和路径、流式请求及缺少 JSON 媒体类型的拒绝，并检查上游 Cookie 不回传。凭证与目标的生产发布由既有模式事务所有者接入，原生 Chat 的别名、SSE 和完整协议验收归任务 34。Windows、Linux 和 WSL 范围外且未验证，WSL 地址仍见来源映射。

集成目录首轮使用槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。复核使用 `22617ffe6` 加本项 Rust 文件的临时源码副本，槽位 **15**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-15`。两槽位的 `CARGO_BUILD_BUILD_DIR` 为各自目标目录下的 `build`。任务源码直接落在集成目录，八个 Rust 文件与绿灯副本逐一比较一致。两个槽位的任务锁均已释放，缓存保留。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 集成目录首轮红灯 | `cargo test -p vw-agent-apps --lib cc_route_014_ -- --nocapture` | 退出 101，执行 0 项。并行 OMP 改动的 `providers/omp.rs:339` 对 SHA256 输出使用 `LowerHex` 导致 E0277。 |
| 格式化 | `cargo fmt --all` | 源码副本红、绿阶段均退出 0，只同步本项文件。 |
| 行为红灯 | `cargo test -p vw-agent-apps --lib cc_route_014_ -- --nocapture` | 退出 101，2 项因私有字段残留触发行为断言失败。 |
| 策略绿灯 | `cargo test -p vw-agent-apps --lib cc_route_014_ -- --nocapture` | 退出 0，2 项通过，0 项失败或忽略。 |
| 生产监听回环 | `cargo test -p vw-agent --lib cc_route_014_ -- --nocapture` | 退出 0，1 项通过，0 项失败或忽略。 |
| 凭证鉴权回归 | `cargo test -p vw-agent --lib cc_route_008_ -- --nocapture` | 退出 0，3 项通过，0 项失败或忽略。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | `git diff --cached --check`（限定本项提交路径） | 退出 0。 |

`vw-agent` 聚焦测试报告既有 macOS 链接器 `__eh_frame` 大小警告。状态为 **已完成**。

## 任务 15　应用请求覆盖的确定顺序

2026-11-10，macOS 27.0（26A428），arm64。直接依赖任务 14 的 `51187057a`。核对 CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/forwarder.rs`、`requestOverrides.ts` 和相邻用例，三份快照的 SHA256 均匹配来源映射。核对了转发构造入口、供应商表单和转发器的覆盖测试。

- 沿用网关持有的 `RoutingListener`、项目和执行主机授权、应用标识和代次绑定。目标、上游认证和供应商的 `ProviderRequestOptions` 快照一起绑定到 `JsonTarget`。发布新绑定前复用 `providers::validation::validate`，拒绝受保护头、大小写归一后重名头和顶层 `stream`。校验失败保留当前凭证和路由。原生配置事务、账号凭据租约及生产发布分别由对应所有者接入。
- 现有 `GatewayClient::app_provider_request_options_save` 与 `PUT /v1/app-providers/{app}/{id}/request-options` 继续写同一供应商记录。数据面消费同一契约的 `localProxyRequestOverrides`。测试通过既有 `request_options::update/read` 取得发布快照。
- 已授权的 Codex 非流式 `POST /v1/chat/completions` 先校验并解析客户端输入。原生 Chat 的协议转换为恒等操作。`dispatch::prepare_request` 复制转换后的文档，先过滤私有字段，再用 `override_apply::apply` 深合并对象、替换数组与标量、保留显式 null，并重新过滤覆盖带入的私有字段。允许的覆盖头仅加入本次目标的出站请求。最后写入 JSON 媒体类型与该绑定的上游认证。
- 回环上游捕获两个供应商收到的实际请求，验证覆盖、认证、媒体类型和代次隔离。复用同一份解析文档准备两个目标，验证首个目标的覆盖和过滤均未污染原件。另验证非法覆盖不会撤掉既有绑定。跨协议和流式请求由后续协议任务验收。Windows、Linux 和 WSL 范围外且未验证，WSL 地址仍见来源映射。

各阶段均通过原子锁领取共享槽位 **3**。`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，`CARGO_BUILD_BUILD_DIR` 与之相同。各阶段的任务锁均已释放，缓存保留。下表回归命令中的 `$CARGO_TARGET_DIR` 为该绝对路径。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 红、绿阶段均退出 0。从格式化输出仅写回本项文件。 |
| 红灯 | `cargo test -p vw-agent --lib cc_route_015_ -- --nocapture` | 退出 101。3 项行为断言失败。实际出站正文与同一解析文档的首次构造均缺少覆盖，含受保护认证头的配置也未被拒绝。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_015_ -- --nocapture` | 退出 0，3 项通过，0 项失败或忽略。 |
| 鉴权回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-377675ed6b51d388" cc_route_008_ --nocapture` | 退出 0，3 项通过。 |
| JSON 过滤回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-377675ed6b51d388" cc_route_014_ --nocapture` | 退出 0，1 项通过。 |
| 请求选项 API/SDK 回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-377675ed6b51d388" cc04_ui_endpoints_options_ --nocapture` | 退出 0，2 项通过。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | `git diff --cached --check`（限定本项提交路径） | 退出 0。 |

红、绿测试链接均报告既有 macOS `__eh_frame` 大小警告。可选的 `cargo test -p vw-agent --lib cc_route_008_ -- --nocapture` 因集成分支并行更新触发重复编译而被中止，执行 0 项。该阶段退出 130，随后使用同一绿灯二进制完成上述回归。状态为 **已完成**。

## 任务 16　原生校验并发送 User-Agent

2026-11-10，macOS 27.0（26A428），arm64。直接依赖任务 15 的 `44972f56c`。已核对 CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `userAgent.ts`、`userAgent.test.ts` 和 `proxy/forwarder.rs`。三份文件与仓库快照逐字节一致，SHA256 匹配来源映射。查阅了来源表单、`ProviderMeta::custom_user_agent_header`、转发器 UA 选择及相邻头部覆盖测试。沿用母任务 2、3 的项目与执行主机授权和应用标识，复用母任务 4 的请求选项契约，原生配置写入和账号租约由其原有执行端负责。

- 通过现有 `localProxyRequestOverrides.headers` 的 `User-Agent` 键配置路由 UA。沿用 `GatewayClient::app_provider_request_options_save` 及 `PUT /v1/app-providers/{app}/{id}/request-options`，摘要由现有 read 接口返回。没有新增字段、端点或存储。解析器 `providers::validation::custom_user_agent` 由保存、摘要、路由和模型发现共用。
- 保存省略空白 UA，旧档案中的空白值在摘要及执行时同样视为未设置。剥除边缘合法空白，保留内部 TAB 与合法 UTF-8 字节。拒绝 ASCII 控制字符（TAB 除外），包括边缘 CR/LF。沿用 8192 字节限制及脱敏错误。新选项非法时复用发布前校验。
- `RoutingListener` → `RouteAuth` → `dispatch::forward_json/prepare_request` → `override_apply::apply` → `SharedTransport`。任务 15 的授权绑定携带保存的请求选项。macOS 回环夹具通过相同的监听器、授权和传输捕获上游实际收到的请求，检查合法 UA 按字节发送一次，以及旧空白值不生成头。生产授权签发、客户端头部保留及账号指纹路径分别由对应后续任务验收。Windows、Linux、WSL 范围外且未验证，WSL 地址保留在来源映射。

红灯和绿灯阶段动态领取共享槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，`CARGO_BUILD_BUILD_DIR` 与之相同。格式化从 stdout 仅写回本项文件。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all -- --emit stdout` | 红、绿阶段均退出 0。 |
| 红灯 | `cargo test -p vw-agent-apps --lib cc_route_016_ -- --nocapture` | 退出 101，1 项执行、1 项失败。空白 UA 被摘要标为已配置。 |
| 绿灯 | `cargo test -p vw-agent-apps --lib cc_route_016_ -- --nocapture` | 退出 0，1 项通过，0 项失败或忽略。 |
| 上游回环 | `cargo test -p vw-agent --lib cc_route_016_ -- --nocapture` | 退出 0，1 项通过，0 项失败或忽略。链接报告既有 macOS `__eh_frame` 大小警告。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | `git diff --cached --check`（本项暂存差异） | 退出 0。 |

槽位锁已释放，缓存保留。状态为 **已完成**。

## 任务 17　保留安全的头部线协议

2026-11-10 至 2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 16 的 `aa88e2027`。CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/hyper_client.rs`、`proxy/response_processor.rs` 与仓库快照逐字节一致，SHA256 匹配来源映射。检查了来源 `server.rs` 的入站头部捕获、`forwarder.rs` 的认证和实体头处理，以及逐跳头剥离用例。沿用母任务 2、3 的执行主机、项目和应用身份，复用母任务 4 的供应商请求选项。原生配置写入和账号凭据租约继续由母任务 4、6 的各自执行端负责。

- 已授权的 Codex 非流式 `POST /v1/chat/completions` 通过 `route_auth::serve`、`dispatch::forward_json` 和 `SharedTransport::forward_ordered`。监听器在现有 HTTP/1 头部时限和大小限制内获取完整头部前缀并原样交给 Hyper。原始信息只用于保留 `Accept`、`Accept-Language` 和 `User-Agent` 的拼写和顺序。取值使用 Hyper 已解析的请求头，同名多值保持顺序，供应商覆盖优先。
- 所有 `Connection` 字段中声明的逐跳头先被移除。请求正文重建时剥离旧长度、编码和摘要等实体头。执行端为绑定目标生成 Host、长度和认证，响应侧继续只向应用返回 JSON 媒体类型及正确的传输帧。
- 保留线序的直连路径使用 HTTP/1.1，HTTPS 使用平台证书校验。Hyper 负责解析上游响应，沿用现有压缩解码中间件。命中代理时现有客户端继续负责出口，代理路径的原始拼写和顺序留给任务 25、26。Windows、Linux、WSL 范围外且未验证，WSL 地址保留在来源映射。
- 回环夹具通过真实授权监听和出站 TCP，验证分片头部、大小写混合的交错重复值、逐跳头和客户端凭证剥离、供应商认证、重建正文及生成后的长度，以及响应逐跳头和 Cookie 不回传。

格式化及测试使用共享槽位 **3** 和 **1**。对应的 `CARGO_TARGET_DIR` 分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` 与 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。`CARGO_BUILD_BUILD_DIR` 与该阶段目标目录相同，使用 `CARGO_INCREMENTAL=0`。下表中的二进制命令使用槽位 1。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all -- --emit stdout` | 各轮退出 0，仅写回本项 Rust 文件。 |
| 首轮编译 | `cargo test -p vw-agent --lib cc_route_017_ -- --nocapture` | 槽位 3，退出 101，执行 0 项。测试改为在授权模块内注册，复用私有目标绑定接口。 |
| 行为红灯 | `cargo test -p vw-agent --lib cc_route_017_ -- --nocapture` | 槽位 1，1 项执行、1 项失败。上游只收到默认的 `accept: */*`。记录退出码的脚本在测试期间被改写，包装进程退出 127，未捕获 Cargo 退出码。 |
| 红灯复现 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-1a5158046c3572f0" cc_route_017_ --nocapture` | 同一二进制再次触发允许头的拼写与顺序断言，退出 101，1 项失败。 |
| 首轮绿灯构建 | `cargo test -p vw-agent --lib cc_route_017_ -- --nocapture` | 退出 101，执行 0 项。修复解码中间件闭包的请求正文所有权，使监听任务满足 `Send`。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_017_ -- --nocapture` | 退出 0，1 项通过，0 项失败或忽略。 |
| 相关回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-d5cd992c0fb3d499" <过滤前缀> --nocapture` | 分别运行 `cc_route_002_`、`cc_route_008_`、`cc_route_014_`、`cc_route_015_`、`cc_route_016_`，退出码均为 0，通过数依次为 3、3、1、3、1。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 槽位 1，退出 0。 |
| 差异审查 | `git diff --cached --check`（限定本项路径） | 退出 0。 |

红、绿测试链接均报告既有的 macOS `__eh_frame` 大小警告。各阶段的任务锁均已释放，构建缓存保留。状态为 **已完成**。

## 任务 20　约束请求与缓冲大小

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 18 的 `7226ff37f` 和任务 19 的 `3df4d0225`，已核对两项回环证据。CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/server.rs` 和 `proxy/content_encoding.rs` 与仓库快照、来源映射的 SHA256 一致。核对了 `services/proxy.rs` 的监听入口、`proxy/response_processor.rs` 的聚合调用和相邻的解压边界用例。沿用网关持有的监听器、项目和执行主机授权及 Codex 应用身份。原生配置写入和账号租约继续由现有契约提供。

- `app_routing::body_limits` 为授权的非流式 `POST /v1/chat/completions` 统一约束客户端线体、各层解压输出、供应商覆盖后重建的出站 JSON 和待发送的上游正文，每项上限 4 MiB。读到超限分块即停止，声明长度在读取前校验。请求的原始 HTTP/1 头捕获保留长度和分块编码同时出现的信号，避免 Hyper 规范化时丢失。畸形请求返回 400，超限请求返回 413。两条上游传输路径均在完整聚合后提交成功状态，超限、损坏或分帧错误返回 502。
- 每个监听器最多保留 8 个活跃缓冲名额，满载返回 503。请求读入和解压沿用 10 秒期限，超时返回 408 并归还名额。成功响应持有名额直到缓冲字节释放。上游沿用现有传输期限。SSE 断线夹具验证当前非流式入口拒绝该媒体类型。WSL 地址保留在来源映射，Windows、Linux、WSL 范围外且未验证。

### 验证记录

各阶段的 `CARGO_BUILD_BUILD_DIR` 与所领 `CARGO_TARGET_DIR` 相同，`CARGO_INCREMENTAL=0`。共享槽位均已释放，编译缓存保留。

| 槽位 | `CARGO_TARGET_DIR` |
| --- | --- |
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` |
| 3 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` |
| 8 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-8` |

| 阶段 | 槽位 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- | --- |
| 红灯 | 3 | `cargo test -p vw-agent --lib cc_route_020_ -- --nocapture` | 退出 101，4 项失败。旧实现为过大声明和截断的上游响应发出 200。过大的请求声明及满载请求均未在夹具期限内拒绝。前两次因夹具访问私有接口退出 101，执行 0 项。 |
| 格式化 | 1、3 | `cargo fmt --all` | 两次均退出 0。在临时源码副本执行，只回写本任务 Rust 文件。 |
| 绿灯首跑 | 1 | `cargo test -p vw-agent --lib cc_route_020_ -- --nocapture` | 退出 101，执行 0 项。集成工作区 Dsh 适配器的 `serde_yaml::Value` 比较类型错误阻断编译，已由其任务修复。 |
| 绿灯重试 | 8 | `cargo test -p vw-agent --lib cc_route_020_ -- --nocapture` | 退出 101，3 项通过、1 项失败。转发客户端复用了夹具连接，导致下一次等待新连接超时。为夹具响应补上 `Connection: close`。 |
| 最终绿灯 | 1 | `cargo test -p vw-agent --lib cc_route_020_ -- --nocapture` | 退出 0，4 项通过，0 项失败或忽略。 |
| 相邻回归 | 1 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-d7cc4815ce201d46" cc_route_014_ cc_route_017_ cc_route_018_ cc_route_019_ --nocapture` | 退出 0，6 项通过，0 项失败或忽略。 |
| 全仓检查 | 1 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | — | `git diff --cached --check`（限定本项路径） | 退出 0。只读计划无差异。 |

回环用例覆盖两条上游传输路径上的过大声明和提前断开，以及 4 MiB 恰好通过、分块越界、截断和冲突分帧、gzip 解压越界、重建请求越界和超时回收。最终测试的 Rust 文件与提交内容逐一核对一致。macOS 链接器报告既有的 `__eh_frame` 大小警告。本项状态为 **已完成**。

## 任务 21　区分首包、空闲与总超时

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 20 的 `99f335fd9`。核对了 CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/handler_context.rs` 与 `proxy/response_processor.rs`，仓库快照和原提交 SHA256 均匹配来源映射。查阅了来源 `handlers.rs`、`forwarder.rs` 的期限调用、首块回放及持续心跳用例。沿用网关监听器、项目与执行主机授权和 Codex 应用身份。原生配置写入与账号租约仍由现有契约负责。

- 授权的 `POST /v1/chat/completions` 从 `RoutingConfigStore` 读取已验证租户、执行主机、项目和应用的超时配置。读取发生在每次转发前，缺省沿用 60 秒首包、120 秒空闲、600 秒非流式总时限。无法读取或校验配置时返回 503。
- `app_routing::timeouts` 为等待响应头和首个有效 Chat SSE 事件共用一个截止点。心跳、注释及仅含角色或用量的帧不刷新时限。成功开始后，后续有效输出刷新独立的空闲期限，空闲设为 0 时关闭。首包和非流式总超时返回 504，已发送响应头后的空闲超时以正文错误结束响应。释放上游正文及现有缓冲名额。
- 非流式从转发开始到缓冲及解压结束共用一个总期限。应用监听器的有序直连与 reqwest 路径使用相同策略。现有连接超时与会话 relay 的总时限继续生效。
- SSE 验收以原样转发的 LF/CRLF 帧为限。首包前累计缓冲和单个待观察帧各以 4 MiB 为限。SSE 请求以 `Accept-Encoding: identity` 协商响应，JSON 压缩响应继续沿用已有有界解码。回环夹具通过既有签发与认证链附加模拟供应商。WSL 地址保留在来源映射，Windows、Linux、WSL 范围外且未验证。

### 验证记录

`CARGO_BUILD_BUILD_DIR` 与对应的 `CARGO_TARGET_DIR` 相同。红灯、格式化和 Clippy 使用 `CARGO_INCREMENTAL=0`，集成绿灯使用 `CARGO_INCREMENTAL=1`。所有本项锁均已释放，槽位缓存保留。

| 槽位 | `CARGO_TARGET_DIR` |
| --- | --- |
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` |
| 3 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` |

| 阶段 | 槽位 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- | --- |
| 红灯首跑 | 2 | `cargo test -p vw-agent --lib cc_route_021_ -- --nocapture` | 退出 101，执行 0 项。集成工作区同期诊断模块的类型错误阻断编译，已由其任务修复。 |
| 红灯复现 | 2 | `cargo test -p vw-agent --lib cc_route_021_ -- --nocapture` | 基于 `b0c5ec2fd` 的临时源码副本，退出 101，1 项失败。停滞的 JSON 正文返回 502，预期为 504。 |
| 格式化 | 1、2、3 | `cargo fmt --all` | 中间一次因夹具字面量语法退出 1，修正后退出 0。在临时源码副本执行，只回写本项 Rust 文件。 |
| 绿灯重试 | 1 | `cargo test -p vw-agent --lib cc_route_021_ -- --nocapture` | 首次退出 101，执行 0 项，补齐夹具时钟初始化。第二次退出 101，3 项通过、1 项失败，改为显式推进虚拟时钟并分别检查截止前后。 |
| 最终绿灯 | 1 | `cargo test -p vw-agent --lib cc_route_021_ -- --nocapture` | 退出 0，4 项通过，0 项失败或忽略。 |
| 相邻回归 | 1 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_014_ cc_route_017_ cc_route_018_ cc_route_019_ cc_route_020_ --nocapture` | 退出 0，10 项通过，0 项失败或忽略。 |
| 全仓检查 | 1 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | — | `git diff --cached --check`（限定本项路径） | 退出 0。只读计划无差异。 |

隔离 SQLite 配置及回环 HTTP/TCP 用例覆盖响应头和分段 JSON 共用总截止点、首包前持续心跳、分片 SSE 正常输出、有效输出刷新空闲期限、只有心跳时的空闲中断、禁用空闲时跨过 601 秒及终止后上游套接字回收。相邻回归覆盖授权分发、头部线协议、压缩解码和缓冲上限。macOS 测试链接器报告既有 `__eh_frame` 大小警告。本项状态为 **已完成**。

## 任务 22　传播取消与客户端断线

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 21 的 `b5c95ea1f`。已核对 CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/response_processor.rs` 和 `proxy/forwarder.rs`，两份文件的 SHA256 均与仓库快照和来源映射一致。核对了来源 handler 的转发入口、流内活跃连接守卫及用量完成保护。沿用母任务 2、3 的运行时、执行主机和应用身份，以及任务 8、14 的鉴权和目标绑定。原生配置写入与账号凭据租约仍由母任务 4、6 的所有者负责。

- 授权的 Codex `POST /v1/chat/completions` 在读完请求体后启用 `app_routing::cancellation::ClientDisconnect`。即使 Hyper 缓冲了流水线字节并停止读套接字，连接任务仍能观察客户端 EOF 或重置。监听器只处理每条连接的首个请求，流水线尾部字节不进入分发。
- 客户端断开与凭据撤销均丢弃同一个 HTTP 连接 future。首包前丢弃转发，出流后丢弃响应体及其上游所有者。有序直连的连接任务和 reqwest 的响应流随所有者释放，每次请求只归还自己的一个缓冲许可。取消一条 SDK 请求保留同凭据的其他在途请求。撤销可重复调用，旧凭据随后返回 401。数据面禁用 reqwest 的协议级重试。
- 使用真实回环 TCP 上游与 `GatewayClient::inference_openai_chat`。覆盖普通及流水线客户端在首包前与 SSE 空闲时断开、SDK 取消、同凭据并行隔离、重复撤销、上游 EOF 或重置、许可余额及无新请求。验收覆盖路由请求的终止与资源释放。持久用量由母任务 17 负责，请求终态发布由任务 85 负责。Windows、Linux、WSL 范围外且未验证，WSL 地址保留在来源映射。

### 验证记录

共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 与之相同。全仓格式化输出到 stdout，只格式化本项文件。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 普通断线基线 | `cargo test -p vw-agent --lib cc_route_022_ -- --nocapture` | 退出 0，1 项通过。普通连接在首包前和 SSE 空闲时已能回收上游。 |
| 行为红灯 | `cargo test -p vw-agent --lib cc_route_022_ -- --nocapture` | 退出 101，1 项通过、1 项失败。流水线客户端在首包前断开，3 秒后上游套接字仍未关闭。 |
| 全仓格式化 | `cargo fmt --all -- --emit stdout` | 退出 0。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_022_ -- --nocapture` | 退出 0，2 项通过，0 项失败或忽略。 |
| 相邻回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_008_ cc_route_014_ cc_route_017_ cc_route_018_ cc_route_019_ cc_route_020_ cc_route_021_ --nocapture` | 退出 0，17 项通过，0 项失败或忽略。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 两轮退出 0，使用 `CARGO_INCREMENTAL=0`，后一轮单独运行。 |
| 差异审查 | `git diff --cached --check`（限定本项路径） | 退出 0。 |

槽位锁已释放，缓存保留。测试链接器报告既有 macOS `__eh_frame` 大小警告。状态为 **已完成**。

## 任务 23　约束并发与背压

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 22 的 `062c4d6cb`。CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/forwarder.rs` 与仓库快照、来源映射的 SHA256 一致。核对了 `handlers.rs` 的调用、活跃连接守卫向响应传递的路径及相邻测试。沿用母任务 2、3 的运行时、执行主机和应用身份。原生配置与账号凭据租约由母任务 4、6 的既有契约负责。

- `app_routing::dispatch::forward_json` 在授权的 Codex `POST /v1/chat/completions` 路径接入 `capacity::RouteCapacity`。同一可信租户、项目、执行主机和应用最多同时转发 2 个请求，另允许 1 个等待最多 2 秒。满队列或等待超时返回 429。凭据轮换复用同一路由配额，客户端声明的项目或请求头无法改写配额归属。
- 继续在读取正文前保留监听器级 8 个缓冲许可。请求完成上传和校验后才等待上游许可，以便断线检测取消排队。普通响应的字节所有者和 SSE 响应流持有相应许可。成功、上游失败、客户端断线和队列超时均释放对应许可。控制状态读取沿用独立的生命周期快照。
- 回环 TCP 夹具验证其他授权项目可在单路由饱和时独立转发，控制快照仍可读取，队列超时和满队列均返回 429，以及成功、失败和取消后的配额复用。慢读用例向 reqwest 与有序直连分别发送约 16 MiB 的有限 SSE 流，两条传输路径都在下游暂停时阻止上游写完，并在恢复读取后完整传输。Windows、Linux、WSL 范围外且未验证，WSL 地址保留在来源映射。

### 验证记录

槽位 **2** 用于红灯、格式化、绿灯和回归，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。槽位 **1** 用于排查红灯构建前置，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。槽位 **3** 用于运行改动前的回归基线，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与目标路径相同。回环测试使用隔离的临时 home。

| 阶段 | 槽位 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- | --- |
| 构建前置排查 | 2、1 | `cargo test -p vw-agent --lib cc_route_023_ -- --nocapture` | 退出 101，0 项。槽位 2 出现既有 `vw-api-types` 缓存导致的 `E0432` 与依赖元数据 `E0463`，仅更新相关 Cargo 指纹。槽位 1 由并行 MCP 代码的 `E0277` 阻断，修复后重跑。 |
| 行为红灯 | 2 | `cargo test -p vw-agent --lib cc_route_023_ -- --nocapture` | 退出 101，1 项通过、1 项失败。前两条请求仍在途时，同路由的第三条已抵达上游。慢读背压基线通过。 |
| 全仓格式化 | 2 | `cargo fmt --all -- --emit stdout` | 退出 0。工作区写入仅对本项文件执行 `rustfmt --edition 2024 --config skip_children=true`。 |
| 绿灯 | 2 | `cargo test -p vw-agent --lib cc_route_023_ -- --nocapture` | 退出 0，2 项通过，0 项失败或忽略。 |
| 聚焦串行复测 | 2 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_023_ --test-threads=1 --nocapture` | 连续 3 次退出 0，每次 2 项通过。 |
| 并行相邻回归 | 2 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_008_ cc_route_010_ cc_route_011_ cc_route_014_ cc_route_020_ cc_route_021_ cc_route_022_ --nocapture` | 退出 101，17 项通过，3 项失败。PostgreSQL 夹具缺少 `VIBEWINDOW_TEST_POSTGRES_URL`，另有首包超时及持久监听状态用例失败。 |
| 串行相邻回归 | 2 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_008_ cc_route_010_ cc_route_011_ cc_route_014_ cc_route_020_ cc_route_021_ cc_route_022_ cc_route_023_ --skip cc_route_011_config_apply_postgres_expected_behavior --test-threads=1 --nocapture` | 退出 101，20 项通过，1 项失败。首包超时和状态 API 用例通过，持久监听状态仍为 `Failed`。 |
| 改动前基线 | 3 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_011_lifecycle_reads_the_persisted_listener_config cc_route_021_stream_first_output_deadline_includes_headers_and_heartbeats --test-threads=1 --nocapture` | 退出 101，1 项通过、1 项失败。改动前的缓存二进制同样返回 `Failed`，首包超时用例通过。 |
| 全仓 Clippy | 2 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。先前退出 101 和中断退出 143 的轮次遇到并行熔断测试的 `E0308` 或 `drop_non_drop`，待对应代码修复后复跑通过。 |
| 差异审查 | — | `git diff --cached --check`（限定本项提交路径） | 退出 0。只读计划无差异。 |

测试链接器报告既有 macOS `__eh_frame` 大小警告。本项占用的槽位锁均已释放，编译缓存保留。状态为 **已完成**。

## 任务 24　统一协议错误边界

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 23 的 `e9bb64853`。核对 CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/error.rs`、`error_mapper.rs` 和 `handlers.rs`。固定 Git 对象、仓库快照与来源映射的 SHA256 均匹配。检查了来源路由注册、HTTP 错误映射及 Codex 413 和 SSE 失败用例。沿用母任务 2、3 的网关运行时、执行主机授权及应用标识。母任务 4、6 提供原生配置事务与上游账号租约契约。

- `app_routing::protocol_errors` 由数据面监听器的鉴权和分发共用。Messages 使用 `type: error`，Responses 和 Chat 使用 OpenAI 的 `error` 对象，Gemini 使用数字 `code` 与规范 `status`。按路径选择封装。四种协议均通过真实监听入口验证本地 401。已接通的 Chat 路径用不同固定代码与消息区分本地 413、上游 413、上游 401 和上游超时。其余上游失败沿用 502。错误回复仅由状态和错误来源构造。 [Claude API 错误](https://platform.claude.com/docs/en/api/errors) · [OpenAI API 错误](https://developers.openai.com/api/docs/guides/error-codes) · [Gemini GenerateContent 错误](https://ai.google.dev/gemini-api/docs/generate-content/api-errors)
- Chat 流首包提交前的失败返回 HTTP JSON。已输出后保留原 HTTP 状态，空闲超时、上游断开或错误事件返回一个脱敏错误终帧后结束。仅转发完整的正常帧，分片错误正文不进入响应或路由日志，正常 `[DONE]` 仍结束流。其他三种协议验证终帧序列化，原生上游联验归任务 35–37。Responses 终帧采用 Codex 可识别的 `response.failed`。Windows、Linux、WSL 范围外且未验证，WSL 地址保留在来源映射。 [Claude SSE](https://platform.claude.com/docs/en/build-with-claude/streaming) · [OpenAI Responses SSE](https://developers.openai.com/api/reference/resources/responses/streaming-events) · [Codex 流解析](https://github.com/openai/codex/blob/main/codex-rs/codex-api/src/sse/responses.rs)

### 验证记录

共享槽位 **3** 用于首次红灯构建，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。槽位 **2** 用于行为红灯、绿灯与回归，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段的 `CARGO_BUILD_BUILD_DIR` 同目标路径。回环测试使用独立临时 home。

| 阶段 | 槽位 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- | --- |
| 首次红灯构建 | 3 | `cargo test -p vw-agent --lib cc_route_024_ -- --nocapture` | 退出 101，0 项。并行的高可用测试中 `AppProviderStore::new(db)?` 导致 E0277。该处修复后复跑。 |
| 行为红灯 | 2 | `cargo test -p vw-agent --lib cc_route_024_ -- --nocapture` | 退出 101，3 项失败。401 缺少 JSON 类型，上游 413 变为 502，SSE 空闲超时导致 HTTP 分块截断。 |
| 全仓格式化 | 3、2 | `cargo fmt --all -- --emit stdout` | 退出 0。工作区只用 `rustfmt --edition 2024 --config skip_children=true` 格式化本项 Rust 文件。 |
| 绿灯 | 2 | `cargo test -p vw-agent --lib cc_route_024_ -- --nocapture` | 退出 0，5 项通过，0 项失败或忽略。 |
| 首轮相邻回归 | 2 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_008_ cc_route_009_ cc_route_014_ cc_route_015_ cc_route_016_ cc_route_017_ cc_route_018_ cc_route_019_ cc_route_020_ cc_route_021_ cc_route_022_ cc_route_023_ cc_route_024_ --test-threads=1 --nocapture` | 退出 101，33 项通过，1 项失败。任务 19 的旧断言要求 502 正文为空。更新断言以校验 502 脱敏 JSON、上游正文及编码头隔离。 |
| 聚焦复测 | 2 | `cargo test -p vw-agent --lib cc_route_024_ -- --nocapture` | 退出 0，5 项通过，0 项失败或忽略。 |
| 最终相邻回归 | 2 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_008_ cc_route_009_ cc_route_014_ cc_route_015_ cc_route_016_ cc_route_017_ cc_route_018_ cc_route_019_ cc_route_020_ cc_route_021_ cc_route_022_ cc_route_023_ cc_route_024_ --test-threads=1 --nocapture` | 退出 0，34 项通过，0 项失败或忽略。 |
| SDK 回归 | 2 | `cargo test -p vw-gateway-client --lib inference_and_stream_clients_preserve_protocol_events -- --nocapture` | 退出 0，1 项通过，0 项失败或忽略。 |
| 全仓 Clippy | 2 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | — | `git diff --cached --check`（限定本项提交路径） | 退出 0。只读计划无差异。 |

测试链接时报告既有 macOS `__eh_frame` 大小警告。本项的槽位锁已释放，构建缓存保留。状态为 **已完成**。

## 任务 25　实现自定义 HTTP 与 SOCKS 出口

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 24 的 `3ca4f0e79`。CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/http_client.rs` 和 `proxy/hyper_client.rs` 与仓库快照、来源映射的 SHA256 一致。检查了 `forwarder.rs` 的代理分流、`server.rs` 的头部捕获调用，以及代理配置和转发测试。沿用母任务 2、3 的网关运行时、授权执行主机和应用标识。原生配置事务与上游账号租约由母任务 4、6 的现有所有者提供。

- `SharedTransport::with_custom_proxy` 为目标绑定构建独立传输。已授权的 `POST /v1/chat/completions` 通过 `JsonTarget` 使用该传输，回环用例验证轮换一个执行主机的绑定后，另一执行主机仍走原出口。任务 28 接入出口控制 API 和 SDK，任务 86 接入生产绑定发布。
- HTTP 和 HTTPS 代理在保留头部线序时使用 CONNECT，HTTPS 先校验代理的 TLS 证书。代理 Basic 认证仅出现在 CONNECT，供应商认证仅在隧道建立后的请求中发送。回环验证头部拼写和顺序、HTTPS 上的认证与双向传输、不可信代理证书拒绝，以及代理返回 407 时应用收到脱敏 502 且无直连。
- SOCKS5 和 SOCKS5H 复用 reqwest 的认证握手。回环夹具观察到 SOCKS5 使用本地解析后的 IP，SOCKS5H 传送原始域名。校验只接受四种明确的协议，拒绝缺少主机、无效端口、附加路径、查询或片段的端点。Windows、Linux、WSL 范围外且未验证，WSL 地址保留在来源映射。

### 验证记录

红灯、绿灯与回归使用共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。回环用例使用独立临时 home。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 红灯 | `cargo test -p vw-agent --lib cc_route_025_ -- --nocapture` | 4 项失败。HTTP 实际走直连，SOCKS 返回 502，非法协议被接受，拒绝认证夹具超时。首次包装脚本在运行中被改写，退出 127，未捕获 Cargo 退出码。再次执行原命令时共享源码已变更并触发重编译，主动中止重编译，退出 130，执行 0 项。 |
| 红灯复验 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-d7cc4815ce201d46" cc_route_025_ --nocapture` | 退出 101，4 项触发相同失败。 |
| 格式化 | `cargo fmt --all -- --emit stdout`；`rustfmt --edition 2024 --config skip_children=true crates/vw-agent/src/app_routing/{mod,outbound_custom,outbound_custom_tests,shared_transport}.rs` | 均退出 0，仅写入本项 Rust 文件。 |
| 绿灯 | `cargo test -p vw-agent --lib cc_route_025_ -- --nocapture` | 退出 0，6 项通过，0 项失败或忽略。 |
| 相邻回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_002_ cc_route_017_ cc_route_024_ --test-threads=1 --nocapture` | 退出 0，9 项通过，0 项失败或忽略。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | `git diff --cached --check`（限定本项提交路径） | 退出 0。只读计划无差异。 |

测试链接时报告既有的 macOS `__eh_frame` 大小警告。槽位锁已释放，编译缓存保留。状态为 **已完成**。

## 任务 26　实现 macOS 系统代理与直连例外

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 25 的 `9467b550e`。CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/http_client.rs` 和 `commands/global_proxy.rs` 与快照及来源映射 SHA256 一致。核对了来源的转发器、监听端口登记、代理配置调用和回环用例。沿用母任务 2、3 的网关运行时、执行主机和应用身份，以及母任务 4、6 的原生配置事务和账号租约契约。

- `outbound_system::SystemProxy` 在构建 `SharedTransport` 时通过 SystemConfiguration 只读取得代理快照。获准的 `POST /v1/chat/completions` 经原有监听、鉴权和分发路径进入共享传输。显式代理优先，其次为环境代理、macOS 对应协议的静态代理、系统 SOCKS5（代理端解析 DNS），最后直连。macOS 的主机名、通配符、CIDR、简单主机名例外及环境 `NO_PROXY` 适用于默认出口。PAC、WPAD 与动态重读未纳入本项。
- 回环上游覆盖 `localhost` 及子域、IPv4 回环、IPv6 回环和映射地址，始终直连。指向当前监听地址的系统代理绕过，显式代理或上游直接指向当前监听地址返回脱敏 502。通过真实回环 TCP 检查 CONNECT、原始头部拼写和直连，另检查自转发拒绝在拨号前发生。系统设置由 `scutil --proxy` 和 SystemConfiguration 只读核对，测试网络使用隔离夹具。
- 任务 28 负责出口配置与状态接口，任务 86 负责生产绑定发布。Windows、Linux、WSL 范围外且未验证，WSL 连接地址留在来源映射。

### 验证记录

红灯、绿灯、相邻回归与全仓 Clippy 使用共享槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。一次中止的绿灯构建使用槽位 **4**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。各阶段的 `CARGO_BUILD_BUILD_DIR` 同对应路径。测试通过 `VW_MODEL_RECOVERY_TEST_ROOT` 使用独立临时目录。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 红灯构建 | `cargo test -p vw-agent --lib cc_route_026_ -- --nocapture` | 退出 137，已生成红灯二进制。为隔离测试目录，在运行用例前结束该 Cargo 进程。 |
| 隔离红灯 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-bbcd40fe2bf49e0c" cc_route_026_ --nocapture` | 退出 101，1 项失败。代理收到回环上游的 CONNECT，应用返回 502。 |
| 格式化 | `cargo fmt --all -- --emit stdout`，随后 `rustfmt --edition 2024 --config skip_children=true crates/vw-agent/src/app_routing/{dispatch,listener,mod,outbound_custom_tests,outbound_system,outbound_system_tests,shared_transport}.rs` | 最终两条命令均退出 0。只写入本项 Rust 文件。 |
| 聚焦绿灯 | `cargo test -p vw-agent --lib cc_route_026_ -- --nocapture` | 退出 0，7 项通过，0 失败或忽略。 |
| 相邻回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-e6259d379726497c" cc_route_002_ cc_route_017_ cc_route_024_ cc_route_025_ --test-threads=1 --nocapture` | 首轮退出 101，14 项通过，撤销凭证夹具在 401 后遇到连接重置。让该用例只发送请求头后复验退出 0，15 项通过，0 失败或忽略。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |
| 差异审查 | `git diff --cached --check`（限定本项提交路径） | 退出 0。只读 `plan.md` 无差异。 |

测试链接报告既有的 macOS `__eh_frame` 大小警告。本任务持有的槽位锁均已释放，编译缓存保留。状态为 **已完成**。

## 任务 27　拒绝不安全 TLS 与转发回环

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 26 的 `b3d2ef58a`。CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/upstream_guard.rs`、`proxy/http_client.rs` 与仓库快照和来源映射的 SHA256 一致。核对了 `forwarder.rs` 的目标校验和来源相邻测试。沿用母任务 2、3 的网关运行时、执行主机授权和应用身份。原生配置事务和账号租约沿用母任务 4、6 的契约。

- 授权的 `POST /v1/chat/completions` 经 `dispatch::forward_json` 使用 `SharedTransport::forward_ordered`。`transport_security::check_origin` 以已绑定回环监听器的实际地址和端口校验目标，统一比较 IPv4 映射地址。同端口直连域名在拨号前检查全部解析结果，有序 TCP 和 reqwest 均复用核验过的地址，并保留原始 HTTP 主机及 TLS 身份。单独绑定在另一地址的服务仍可使用相同端口。
- 代理需要独立解析同端口域名时，在发出代理连接前返回脱敏 502。现有系统代理已知自指时继续绕过，显式代理已知自指时拒绝。HTTPS 按原始目标校验证书链与主机名，生产传输沿用平台信任。reqwest 禁用自动重定向，有序传输只执行一次 HTTP 交换。
- 真实回环 TCP/TLS 夹具验证未受信证书、信任同一证书链时名称匹配与不匹配、携凭据的 307 原请求和重定向目的端零连接、解析到本监听器时零拨号、同端口代理端零拨号及同端口 IPv6 上游成功。Windows、Linux、WSL 范围外且未验证，WSL 连接地址保留在来源映射。

### 验证记录

红灯、首次格式化、首次绿灯、相邻回归和首轮 Clippy 使用共享槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。移除未使用的导入后，复跑格式化、聚焦测试及第二轮 Clippy 使用共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。并行改动补齐后第三轮 Clippy 使用共享槽位 **5**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5`。各阶段的 `CARGO_BUILD_BUILD_DIR` 同对应目标路径，回环测试使用独立临时目录。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| 行为红灯 | `cargo test -p vw-agent --lib cc_route_027_ -- --nocapture` | 退出 101，3 项通过，2 项失败。未核验的同端口域名到达代理，独立 IPv6 上游被误拒为 502。 |
| 格式化 | `cargo fmt --all -- --emit stdout`，随后 `rustfmt --edition 2024 --config skip_children=true crates/vw-agent/src/app_routing/{mod,outbound_custom_tests,outbound_system,shared_transport,transport_security,transport_security_tests}.rs` | 退出 0，只写入本项 Rust 文件。移除未使用的导入后以相同的 Cargo 命令复跑，并仅对 `outbound_system.rs` 执行相同参数的 rustfmt，两条命令均退出 0。 |
| 聚焦绿灯 | `cargo test -p vw-agent --lib cc_route_027_ -- --nocapture` | 槽位 2 和 1 均退出 0，每轮 7 项通过，0 项失败或忽略。 |
| 相邻回归 | `cargo test -p vw-agent --lib -- cc_route_002_ cc_route_017_ cc_route_024_ cc_route_025_ cc_route_026_ --test-threads=1 --nocapture` | 退出 0，22 项通过，0 项失败或忽略。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 槽位 2 退出 101，定位并移除旧的 `SocketAddr` 导入。槽位 1 退出 101，由并行 MCP 改动的未使用导出和 Responses 测试的 `StatusCode` 导入阻断。相关改动补齐后，槽位 5 退出 0。 |
| 差异审查 | `git diff --cached --check`（限定本项提交路径） | 退出 0。只读 `plan.md` 无差异。 |

测试链接报告既有 macOS `__eh_frame` 大小警告。本项占用的槽位锁均已释放，保留编译缓存。状态为 **已完成**。

## 任务 28　接通出口配置与状态接口

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 27 的 `ef9474ffd`。核对了 CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `commands/global_proxy.rs`、`lib/api/globalProxy.ts` 和调用者 `hooks/useGlobalProxy.ts`。两份来源文件的 SHA256 与冻结快照和来源映射一致。复用母任务 2、3 已交付的网关认证、租户、主机和应用身份，原生配置及账号上游继续使用母任务 4、6 的契约。

- `GET /v1/app-routing/hosts/{host}/egress` 使用主机级读许可，PUT 使用管理许可。两者沿用成员认证、租户与执行主机检查、`no-store` 及结构化错误。获准的项目路由继承所属租户和主机的出口。SDK 为 `GatewayClient::app_routing_egress` 与 `GatewayClient::app_routing_set_egress`。
- `RoutingConfigStore::apply_egress` 持有控制许可，先解析 URL、检查已绑定监听器回环并构建传输，再将加密 URL 和模式写入租户/主机设置，写入成功才切换运行快照。请求在转发前读取快照，在途请求持有已取得的传输。更新失败返回脱敏错误并保留旧出口。进程重启恢复已保存的设置。
- `RoutingEgressStatus` 只返回配置模式、生效模式、协议和凭据存在标记。显式更新 URL 仅写入且调试输出隐藏整条 URL。首次缺少设置时取系统快照，系统无代理时生效模式为直连，同时配置多个协议时状态优先显示 HTTPS 出口使用的代理协议。直连例外仍按每个目标判断。连接可用性检测归任务 29。
- 聚焦用例使用独立临时目录、真实网关 HTTP、SQLite 写失败触发器和回环 HTTP CONNECT 夹具。覆盖读写权限、跨租户和主机拒绝、错误结构、状态及存储脱敏、失败保留旧出口、重启恢复、请求按主机选择代理以及回环直连。Windows、Linux、WSL 范围外且未验证，WSL 连接地址保留在来源映射。

### 验证记录

SDK 红灯、初次网关红灯构建和基线 HTTP 复现使用共享槽位 **6**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。全仓格式化使用槽位 **7**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7`。聚焦复验使用槽位 **5**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与目标目录相同。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| SDK 行为红灯 | `cargo test -p vw-gateway-client --lib cc_route_028_ -- --nocapture` | 退出 101，1 项失败。断言复现写入请求的调试格式包含密码。 |
| 网关红灯构建 | `cargo test -p vw-agent --lib cc_route_028_ -- --nocapture` | 退出 101，0 项运行。并行聚合改动的两个 `Path` 导入导致 E0252/E0107，随后由其所属任务修复。 |
| 网关基线 HTTP 红灯 | `python3 /tmp/shiju-route-028-baseline.py`，启动槽位 6 的 `vw_agent-e6259d379726497c --ignored --exact gateway::api::handlers::app_routing::lifecycle_api_tests::gateway_child --nocapture` | 断言退出 1。包含批次 27 用例、未包含任务 28 的缓存网关二进制中，同一成员授权读生命周期状态返回 200，合法出口更新实际返回 401，复现缺失出口入口。 |
| 格式化 | `cargo fmt --all -- --emit stdout`，再对本项 Rust 文件执行 `rustfmt --edition 2024 --config skip_children=true` | 退出 0，只写入本项 Rust 差异。 |
| SDK 聚焦复验 | `cargo test -p vw-gateway-client --lib cc_route_028_ -- --nocapture` | 退出 0，1 项通过，0 失败。 |
| 网关聚焦复验 | `cargo test -p vw-agent --lib cc_route_028_ -- --nocapture` | 退出 0，2 项通过，0 失败或忽略。真实回环流量证明数据库提交失败和自转发校验失败都保留旧代理。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 槽位 5，退出 0。 |
| 差异审查 | `git diff --cached --check` | 退出 0。仅暂存本项改动。 |

测试链接报告既有 macOS `__eh_frame` 大小警告。聚焦复验与 Clippy 已各自返回 0，外层临时槽位脚本在三条命令结束后因运行期间编辑脚本退出 2。已核对本项槽位锁全部释放，编译缓存保留。状态为 **已完成**。

## 任务 29　接通可取消代理连接测试

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 28 的 `7039ab326`。固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `commands/global_proxy.rs`、`hooks/useGlobalProxy.ts` 与冻结快照、来源映射的 SHA256 一致。检查了 `GlobalProxySettings.tsx`、SDK 包装和相邻代理测试。复用母任务 2、3 的网关、执行主机和项目应用授权，以及母任务 4、6 的配置与账号契约。

- `POST /v1/app-routing/hosts/{host}/projects/{project}/apps/{app}/egress/probes` 接受空 JSON，返回 202 和既有 `DiagnosticJobIdentity`。同路径追加 `/{task}`，GET 查询，DELETE 取消。三者均重新核验租户、成员、执行主机、项目应用和 routing_manage 权限。任务身份绑定授权主体，其他成员、项目和任务类别不可读写该任务。
- DTO 使用 `app_routing/outbound_probe.rs` 和既有任务协议新增的 `outbound_proxy` 类别。SDK 提供 `app_routing_start_outbound_probe`、`app_routing_outbound_probe_status`、`app_routing_cancel_outbound_probe`，保留任务状态、阶段、延迟、HTTP 状态及稳定错误码。父模块和现有分域路由直接注册，沿用现有 lib 导出。
- 探测读取任务 28 已发布的出口快照，固定对 ShiJu 发布清单 `https://www.shiju.cloud/agents/shiju-agent/latest/manifest.json` 发起 HTTPS HEAD。该目标复用现有 `vw_shared::update::desktop_release::SHIJU_MANIFEST`。每项使用独立短期客户端、平台 TLS 校验、禁止重试与重定向，沿用监听器回环检查。直接出口或系统例外返回 unsupported，不发起直连。调用者提供 URL、代理 URL 或时限字段均返回 400。
- 复用 `DiagnosticJobs` 的宿主并发、检查点、保留期和取消协议。路由控制持有一个宿主注册表，存放在仅当前用户可访问的状态子目录。10 秒期限在提交前确定，排队后再次检查期限；运行中取消或超时丢弃请求及独立客户端，终态在资源退出后发布。原始网络错误、代理 URL、密码和成员令牌不进入结果。
- 隔离网关、动态回环代理及 HEAD 夹具验证授权边界、固定 CONNECT 目标、代理认证、取消后的 EOF、重复取消、超时后的 EOF、无重试、407 分类、脱敏及 HEAD 302 返回后不跟随重定向。成功响应分支由真实回环 HEAD 验证，SDK 通过真实 HTTP 验证结构化成功结果。Windows、Linux、WSL 范围外且未验证，WSL 地址映射保持原记录。

### 验证记录

共享槽位 **2** 用于 SDK 红灯和网关基线复现，**6** 用于格式化、SDK 通过验证、首次网关通过验证尝试和全仓 Clippy，**1** 用于网关通过验证及依赖回归。对应绝对 `CARGO_TARGET_DIR` 分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-6`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，各实际执行阶段的 `CARGO_BUILD_BUILD_DIR` 与该目录相同。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| SDK 红灯 | `cargo test -p vw-gateway-client --lib cc_route_029_ -- --nocapture` | 槽位 2，退出 101，1 项失败。现有任务 DTO 拒绝 outbound_proxy 类别。 |
| 网关红灯构建 | `cargo test -p vw-agent --lib cc_route_029_ -- --nocapture` | 槽位 2，退出 101，0 项运行。旧基线编译在独立 HTTP 红灯取证后主动终止，未将构建退出用作行为证据。 |
| 网关 HTTP 红灯 | `python3 /tmp/shiju-task29-baseline.py`，使用槽位 2 的 `vw_agent-e6259d379726497c` | 退出 1，1 条行为断言失败。缓存二进制包含任务 28，生命周期与出口配置均返回 200，已授权项目探测返回 401，预期 202。独立 HOME、策略、动态端口，子进程已回收。 |
| 格式化 | `cargo fmt --all -- --emit stdout`，本项文件再执行 `rustfmt --edition 2024 --config skip_children=true` | 槽位 6，退出 0。全仓输出只读，实际写入仅本项文件，排除相邻未提交改动。 |
| SDK 通过验证 | `cargo test -p vw-gateway-client --lib cc_route_029_ -- --nocapture` | 槽位 6，退出 0，2 项通过，0 失败。 |
| 首次网关通过验证尝试 | `cargo test -p vw-agent --lib cc_route_029_ -- --nocapture` | 槽位 6，退出 101，0 项运行。并行模型诊断的租约入参、ProviderId、delete 入参与移动值共 11 项编译错误；对应任务修复后重跑。本任务未修改这些文件。 |
| 网关通过验证 | `cargo test -p vw-agent --lib cc_route_029_ -- --nocapture` | 槽位 1，退出 0，2 项通过，0 失败或忽略，运行 10.66 秒。 |
| 自定义出口回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-e6259d379726497c" cc_route_025_ --nocapture` | 槽位 1，退出 0，6 项通过，覆盖 HTTP、SOCKS、认证和 HTTPS 代理证书。 |
| 出口配置回归 | `"$CARGO_TARGET_DIR/debug/deps/vw_agent-e6259d379726497c" cc_route_028_ --nocapture` | 槽位 1，退出 0，2 项通过。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 槽位 6，退出 0。 |
| 提交审查 | `git diff --cached --check`，逐个暂存 Rust 文件经 `rustfmt --edition 2024 --config skip_children=true --emit stdout` 解析并逐字节比对 | 退出 0，18 个任务文件通过核对。共享文件仅包含本项补丁，相邻聚合和 MCP 改动保留在工作区。 |

网关测试链接出现既有 macOS `__eh_frame` 大小警告，聚焦测试退出 0。SDK、网关和依赖回归共 12 项通过。状态为 **已完成**，仅本项差异进入提交，自己的槽位锁已释放并保留缓存。

## 任务 30　扫描有限回环代理端口

2026-11-11，macOS 27.0（26A428），arm64。直接依赖任务 28 的 `7039ab326`，复用任务 29 的宿主诊断注册表及授权接线 `e8bba5115`。核对 CC Switch 4.0.25 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `commands/global_proxy.rs`、`hooks/useGlobalProxy.ts`，两者 SHA256 与仓库快照、来源映射逐字节一致；同时检查 API 包装及 `GlobalProxySettings.tsx` 的扫描调用。母任务 2、3 的网关、执行主机、项目应用授权，以及母任务 4、6 的原生配置和账号契约均已有生产接口，本项不增加这些母能力。

- `POST /v1/app-routing/hosts/{host}/projects/{project}/apps/{app}/egress/scans` 接受空 JSON，返回 202 和 `DiagnosticJobIdentity`。同路径追加 `/{task}`，GET 查询、DELETE 取消。沿用已有应用诊断作用域，显式包含 app，避免推断项目的默认应用；入口逐次核验租户、成员、执行主机、项目应用和管理许可，任务仅对原主体及原作用域可见。扫描与代理连接测试分用 `outbound_scan`、`outbound_proxy` 类别，彼此的查询和取消路径不能混用，响应沿用 no-store 和结构化错误。
- `RoutingOutboundScanRequest` 拒绝未知字段，无 IP、URL、端口范围或自定义时限。执行地址仅由 `Ipv4Addr::LOCALHOST` 和来源固定八端口 7890、7891、1080、8080、8888、3128、10808、10809 构造。每端口最多 100ms，提交前确定 2 秒网络期限，获得宿主并发许可后再次检查剩余时限，过期不发起连接。复用 `DiagnosticJobs` 的宿主并发、任务容量、进度、取消、检查点和保留期，不增加第二套任务所有者。
- 每个候选报告 `RoutingOutboundScanEvidence { port, tcpOpen, protocolCandidates }`，关闭或超时端口报告 false 和空协议列表。TCP 可连只提供来源协议提示，7890 提示 HTTP 和 SOCKS5，其余按来源单一协议提示；未执行协议握手，不声称代理可用。连接成功后立即关闭，不发送应用数据，不改出口配置。排队超过网络期限后返回结构化 Timeout，取消在等待或连接过程中释放 future/socket，资源退出后才发布终态。
- SDK 为 `app_routing_start_outbound_scan`、`app_routing_outbound_scan_status`、`app_routing_cancel_outbound_scan`，保留任务进度、候选证据和错误。实际文件为三个 crate 下的 `app_routing/outbound_scan.rs` 及独立测试，直接父模块注册；已有 lib/client/handlers 分域导出足够，无需重复导出。仅新增本项路由和诊断结果分支。
- 隔离临时 HOME、策略和动态网关验证授权、非法目标字段、八个固定候选的进度与结果，以及任务类别隔离。独立动态 TCP 夹具验证无应用数据和关闭连接；可控 pending future 验证取消、端口时限和绝对期限，取消返回前以 Drop 标记确认在途连接 future 已释放。无需占用任何固定候选端口。Windows、Linux、WSL 范围外且未验证，WSL 地址映射保持原记录。

### 验证记录

红灯、格式化、聚焦测试和首次 Clippy 动态领取共享槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；Clippy 复验领取 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与目标目录相同，阶段结束释放自己的锁并保留缓存。

| 阶段 | 确切命令 | 退出码与有效测试 |
| --- | --- | --- |
| SDK 红灯 | `cargo test -p vw-gateway-client --lib cc_route_030_ -- --nocapture` | 退出 101，1 项断言失败：现有任务 DTO 拒绝 outbound_scan 类别。 |
| 网关 HTTP 红灯 | `python3 /tmp/task30-baseline.py`，使用槽位 3 的 `vw_agent-e6259d379726497c` | 退出 1，1 条行为断言失败。缓存二进制包含任务 28；同一主体的生命周期、出口配置返回 200，已授权扫描请求返回 401，预期 202。隔离子进程已回收。 |
| 网关红灯构建 | `cargo test -p vw-agent --lib cc_route_030_ -- --nocapture` | 退出 101，0 项运行。在独立 HTTP 红灯取得后终止旧基线编译；不作为行为证据。 |
| 首次 SDK 通过验证尝试 | `cargo test -p vw-gateway-client --lib cc_route_030_ -- --nocapture` | 退出 101，0 项运行。本项测试移动借用证据的 E0507 已修复。 |
| 首次网关通过验证尝试 | `cargo test -p vw-agent --lib cc_route_030_ -- --nocapture` | 退出 101，0 项运行。本项整理导入产生的三个 E0252 已修复；取得编译错误后终止该失败构建。 |
| 格式化 | `cargo fmt --all -- --emit stdout`，再对本项文件执行 `rustfmt --edition 2024 --config skip_children=true` | 退出 0。全仓输出只读，写入限定为本项文件。 |
| SDK 聚焦复验 | `cargo test -p vw-gateway-client --lib cc_route_030_ -- --nocapture` | 退出 0，2 项通过，0 失败。 |
| 网关聚焦复验 | `cargo test -p vw-agent --lib cc_route_030_ -- --nocapture` | 退出 0，4 项通过，0 失败或忽略，运行 0.41 秒。 |
| 首次全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 槽位 3，退出 101。桌面 `app/message/chat/input/file_search.rs:87/96/101/106` 四个未使用函数；所属改动随后移除了这些函数。本项未修改这些文件。 |
| 全仓 Clippy 复验 | `cargo clippy --all-targets --all-features -- -D warnings` | 等已有 Clippy 全部退出后领取槽位 2，退出 101。并行渠道配置改动中的 `crates/vw-config-types/src/channels/chat_tests.rs:81–109` 存在 12 个 E0422/E0609/E0599：`ChannelExecutionConfig`、`instance_execution`、`validate_instance_execution` 缺失。本项不修改这些文件。 |
| 差异审查 | `git diff --cached --check` | 退出 0，仅暂存扫描实现、注册、独立测试与本项证据。 |

SDK 与网关共 **6 项通过，0 失败**。网关链接报告既有 macOS `__eh_frame` 大小警告；Clippy 的依赖 `iced_winit` 报告弃用警告。首次交付的全仓 Clippy 因无关并行改动未通过，当时状态为 **已完成，待回归**，保留任务 30 卡。当次恢复条件：渠道配置实现与测试对齐后，重新领取共享槽位并运行相同全仓 Clippy 命令。自己的槽位 3、2 均已释放，缓存保留。未修改 `plan.md`，未宣称母任务 8 完成。

### 集成工作区追加回归（2026-11-11）

本轮从集成工作区开始时的 HEAD `37a8de813f70e7f62c6281c3c2fc1a4e5fa76726` 执行，包含该工作区已有未提交改动；未修改任务 30 的 Rust 实现或测试。SDK 和网关行为用例先通过，随后其他会话继续更新提供商事务、模型目录和桌面归档 UI，全仓检查记录了这些后续集成错误。以下结果替代上文的待回归恢复条件。

| 命令 | 共享槽位 | 退出码与有效测试 |
| --- | --- | --- |
| `cargo test -p vw-gateway-client --lib cc_route_030_ -- --nocapture` | 3 | 退出 0，2 项通过，0 失败，运行 0.01 秒。 |
| `cargo test -p vw-agent --lib cc_route_030_ -- --nocapture` | 3 | 退出 0，4 项通过，0 失败或忽略，运行 0.67 秒。 |

四次全仓检查均实际执行 `cargo clippy --all-targets --all-features -- -D warnings`，等待已有 Clippy 退出后按顺序动态领取槽位，未修改无关失败文件：

| 次序与槽位 | 退出码 | 检查时的失败及后续核对 |
| --- | --- | --- |
| 1，槽位 2 | 101 | `vw-provider-resolver/src/provider.rs` 的 E0382，`api_id` 移动后借用。相关改动随后已将借用改为 `api.id`，据此重跑。 |
| 2，槽位 1 | 101 | 原生事务 `project_binding.rs` / `project_binding_tests.rs` 尚未落盘、`check_pending_project` 缺失、聚合测试未使用导入，以及桌面 fixture 缺 `Provider.protocols`。文件、方法与 fixture 随后由其所属改动补入，据此重跑。 |
| 3，槽位 3 | 101 | `aggregation/claude_publish.rs:15` 引用私有的 `providers::projection`，E0603。其所属改动随后改用公开的 `claude::patch_with_residue`，据此重跑。 |
| 4，槽位 7 | 101 | 归档 UI 的 `archived_chats.rs` / `archived_chats_tests.rs` 中 Info/TimeInfo 导入错误和 `state/settings.rs:23` 未使用 re-export；同时 `app_providers/preview_tests.rs:194`、`codex_catalog_tests.rs:285`、`gemini_tests.rs:202` 的 `Pending` fixture 缺新增 `project_preimage` 字段。归档导入在检查期间已由其所属改动修正，仍无完整全仓通过证据。 |

实际绝对目标目录为：槽位 3 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`（行为测试与第三次 Clippy），槽位 2 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，槽位 1 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，槽位 7 `/Users/xiangmin/worker/code/shiju/target/rust-slot-7`。各阶段 `CARGO_BUILD_BUILD_DIR` 与 `CARGO_TARGET_DIR` 相同。全部 Cargo 子进程已退出，自己的槽位锁已释放，缓存保留；未执行全量单元测试或跨平台验收。

当前状态仍为 **已完成，待回归**。本轮有效行为测试 **6 项通过，0 失败**，全仓 Clippy 未通过，不删除任务卡。恢复条件是原生事务生产者完成 `Pending.project_preimage` 对已有调用者夹具的契约对齐，归档 UI 完成编译与 lint 接线，再在稳定的集成工作区重新执行上面两条聚焦测试和全仓 Clippy。按 AGENTS.md 不扩展修改其他会话的无关实现；未发现本项扫描用例失败，未改动只读 `plan.md`。

### 独立生产网关验收与回归重试（2026-11-11）

本轮基线为 `a3dd368a6ebb97dcf7b8abef669d2f56e2ab7b21` 及当前集成工作区已有变更。重新核对了任务卡的 AISDK E0382 证据；相关改动随后已在异步闭包之前计算协议，旧错误已修复，不再作为本项阻塞条件。扫描生产实现、DTO 和 SDK 均未改动。

为避免无关 `cfg(test)` 模块的编译故障阻断生产路径验收，参考已有 `cc04_accept_transactions` 的公开 Gateway 子进程模式，新增显式集成目标 `crates/vw-agent/tests/cc_route_030_outbound_scan.rs`。该目标使用正常生产库的 `gateway::run_gateway`、真实授权策略、路由和 SDK，使用临时 HOME、策略和凭证目录，不 mock 内部边界，不新增依赖或 feature，也不删除原四项单元测试。子进程只监听动态控制端口；扫描保持来源八个固定回环候选，不占用这些端口、不发送应用数据、不改出口配置。

| 阶段 | 确切命令 | 槽位、退出码与有效测试 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all -- --emit stdout`，再仅对新测试执行绝对路径 `rustfmt --edition 2024 --config skip_children=true` | 槽位 5，退出 0；全仓只读输出，未写其他会话文件。 |
| SDK | `cargo test -p vw-gateway-client --lib cc_route_030_ -- --nocapture` | 槽位 5，退出 0，2 项通过、0 失败或忽略，0.02 秒。 |
| 首次集成构建 | `cargo test -p vw-agent --test cc_route_030_outbound_scan cc_route_030_ -- --nocapture` | 槽位 5，退出 101，0 项运行；范围外 `app_providers/generation_target.rs` 的 `UnsupportedSavedTarget` 移动后复用导致 7 个 E0382。相关改动补入 Clone/Copy 后重试。 |
| 同轮原聚焦构建 | `cargo test -p vw-agent --lib cc_route_030_ -- --nocapture` | 槽位 5，退出 101，0 项运行；同一生产库 E0382 快照。 |
| 真实生产 Gateway 验收 | `cargo test -p vw-agent --test cc_route_030_outbound_scan cc_route_030_ -- --nocapture` | 槽位 10，退出 0，1 项通过、0 失败或忽略，1 项子进程辅助入口被前缀过滤，0.69 秒；构建 43 分 46 秒。 |
| 原聚焦重试 | `cargo test -p vw-agent --lib cc_route_030_ -- --nocapture` | 槽位 10，退出 101，0 项运行；约 27 分钟后主机交换内存升至约 66GB，编译进程处于 U/I/O 等待且 CPU 进展显著降低，因此仅向自己的 rustc 发送 SIGTERM 撤回资源受阻构建。没有代码编译诊断，不计为行为失败或通过。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 槽位 3，退出 101；`vw-gateway-client/src/client/session_api.rs:467` 的归档删除调用只传 2 个参数，`request_helpers.rs:123` 的 delete_json 要求 3 个，E0061。未修改此范围外调用。 |

真实 Gateway 验收覆盖：只读主体、未授权项目、其他执行主机/租户拒绝；任意 IP、端口、URL 和自定义时限字段拒绝；202/no-store；不同主体、项目和诊断类别不能读同一任务；八候选结果与进度完整，协议字段仅来源提示且不泄漏令牌；取消和重复取消保持终态。父测试结束时已 kill/wait 回收子进程。新增测试 SHA256 为 `f635cd4d7463dd08d6160eeae4ccab42231f61237d2120f21cd871ab04198249`。

本轮有效行为测试 **3 项通过，0 失败**。原四项单元测试历史通过记录仍保留，但不冒充本轮运行结果。槽位 5 的绝对 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5`，槽位 10 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-10`；各自 `CARGO_BUILD_BUILD_DIR` 同目标目录，`CARGO_BUILD_JOBS=2`。全仓 Clippy 另使用槽位 3，绝对 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，`CARGO_BUILD_BUILD_DIR` 同该路径，`CARGO_BUILD_JOBS=2`。三个阶段均已结束、自己的锁已释放，缓存保留。

当前为 **已完成，待回归**，不需要新增权限、用户决策或外部业务产物；保留任务卡，待归档 SDK 调用者完成 delete_json 参数契约对齐，在稳定的集成工作区与资源窗口重跑两条原聚焦测试、独立 Gateway 验收及全仓 Clippy，全部通过后才删除卡片。未修改 `plan.md` 或其他任务实现。

### 最终回归与最小编译适配收尾（2026-11-11）

用户授权在当前集成工作区修复阻断本项验收的最小编译适配，并明确允许本次只读 Clippy 在独占 Cargo 槽位中与其他检查并行。本轮最终验证开始基线为 `e341b1df26a26b353db254aba2a858ba7a63e1d4` 及共享工作区已有变更；验证期间其他会话继续提交聚合和桌面改动，不将结果描述成干净 HEAD 的验收。

旧归档 SDK 缺参已由所属改动修复。为读到真实行为结果，本会话还修正了实际编译阻断：provider 两个夹具路由仅公开至 handlers；MCP String 转 ProjectId、OMO 测试 ProjectId 构造、desktop_mapping 的 PathBuf 分支；既有跨 crate 调用者所需的 ClaudeFile.guard/name、StackState.is_empty、AppWriteGuard.read 可见性；重复 managed 字段；voice 请求主体摘要仅公开至 handlers；Gemini 测试两处传入现有 AuthAccountStore。文件读取仍使用原 resolve 和名称检查，未改鉴权、路径校验或扫描行为。多数适配已随其模块所属的集成提交合入（包括 `e341b1df2`、`4c82f1d56`）；剩余 voice 可见性与本项证据单独提交。Gemini 的两行测试适配保留在所属会话未跟踪的 `app_routing/adapter_selection_tests.rs` 中，不将其他会话整份新文件纳入本项提交；这些验证包含共享工作区的该文件。

先前槽位 11 的失败快照分别记录了原 lib-test 的 24 个诊断、独立验收生产库的 9 个诊断，以及 voice/授权 helper/Gemini 类型契约问题；取得诊断后结束了自己的失败驱动，均不算行为通过。修正后的首次长时间 lib-test 构建又被工具更新中断，0 项运行。后续重新领取槽位，取得以下完整有效结果，替代上文所有“待回归”恢复条件：

| 阶段 | 确切命令 | 槽位、退出码与有效测试 |
| --- | --- | --- |
| 最后一次源码修改后的格式化 | `cargo fmt --all -- --emit stdout` | 槽位 11，退出 0；全仓只读输出，未写其他会话文件。 |
| SDK 聚焦复验 | `cargo test -p vw-gateway-client --lib cc_route_030_ -- --nocapture` | 槽位 11，退出 0，2 项通过、0 失败或忽略，0.01 秒。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 槽位 3，退出 0，2 分 16 秒。 |
| 原网关聚焦复验 | `cargo test -p vw-agent --lib cc_route_030_ -- --nocapture` | 槽位 3，退出 0，4 项通过、0 失败或忽略，0.93 秒；构建 55 分 50 秒。 |
| 独立生产 Gateway 复验 | `cargo test -p vw-agent --test cc_route_030_outbound_scan cc_route_030_ -- --nocapture` | 槽位 3，退出 0，1 项通过、0 失败或忽略，0.35 秒；1 项辅助子进程入口被前缀过滤，构建 14 分 41 秒。 |

槽位 11 的绝对 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-11`，槽位 3 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；各阶段 `CARGO_BUILD_BUILD_DIR` 与该阶段目标目录相同。最后的格式化/SDK 阶段 `CARGO_BUILD_JOBS=1`，最后的 Clippy/网关验证阶段为 2。使用原子锁依次领取首个空闲槽位，未删除其他会话锁。验证进程和 Gateway 子进程均已退出，自己的槽位锁已释放，缓存保留。网关 lib-test 链接器报告既有 macOS linker_messages 警告，测试退出 0；全仓 Clippy 退出 0。

本项最终 **7 项有效测试通过，0 失败，状态为已完成**。固定候选、拒绝任意目标、进度/取消/期限、连接释放、主体与任务作用域隔离和真实生产启动路径均已有有效证据；任务 30 卡按契约删除。不修改 `plan.md`，不宣称其他任务或跨平台验收完成。共享工作区其他会话的未提交变更保持原状。

## 任务 31：选择适配器与认证策略（2026-11-11）

本项只验收 macOS 的九类 ProviderType 选择、目标 URL 与认证头，以及受托管账号作用域约束的按请求读取。原生 Gemini 请求体/响应转换、其余协议和母任务 8 总体验收不在本项结论内；Windows、Linux、WSL 未构建、未验证。

来源固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。已将仓库 upstream 快照与相邻 cc-switch 仓库的固定 Git 对象逐字节比较，退出 0：providers/mod.rs SHA256 `4df3d5b8875c1007181b5a7236a0a13edd394ee79088b5d38734db66d71bf6b7`，adapter.rs `245515ef2139fa22e2b9fa29ce98273d0f1dde3865e20826d5f070cbc3d716fb`，auth.rs `502aa4624ea17f0f19c9c1036878d9c5b8a45cbff18f224752fde61ad0915aa4`。

### 实现与取舍

- `app_routing/adapter_selection.rs::{ProviderType, adapter, auth_headers, target_url}` 按来源显式映射九类：Codex 使用 Codex 家族，Gemini/GeminiCli 使用 Gemini 家族，其余使用 Claude 家族。认证头敏感位保留；空值、CRLF、控制字符、非 ASCII 或过长 token 返回认证错误，不 panic、不在错误中回显凭据。URL 拒绝用户信息、query/fragment、歧义端点与路径遍历；基础 `/v1` 和端点 `/v1/...` 去重。
- `AccountAdapter` 在发布时创建并复用现有 GeminiRefresh、CodexRefresh、CopilotAuth、XaiRefresh，保留账号缓存；`managed_target` 每次通过现有 AuthAccountStore 获取 token，Copilot 使用同一会话的上游地址，Codex account-id 来自同一 owner 租约。未新增登录流程或读取环境/原生 CLI 全局凭据。静态 API key 仍沿用现有 provider 快照及当前记录校验。
- `dispatch::JsonTarget.managed` 将托管凭据接入真实 `forward_json`；转发前检查 tenant/host/account 绑定及已授权 URL，替换旧凭据头。`ManagedCredential::resolve` 使用 boxed future 限制认证交换状态对连接 future 的膨胀。该契约供配置发布调用；本项没有新增公开 Gemini 登录服务或独立控制面 API。
- 经用户明确扩展授权，最小 Gemini 契约复用 `CredentialFileStore` 的 owner/tenant/host/app/account 私有文件、CredentialLease、刷新代次与 publication lease。`AuthAccountStore::{import_gemini_grant,gemini_access_token}` 复用现有数据库 owner 元数据命名空间；导入要求调用者已验证 scope/材料，禁止覆盖既有账号。`GeminiRefresh` 固定 Google token endpoint、禁跳转、限制时限及响应体；刷新失败需重新认证时持久化标记，后续读取直接拒绝，代次校验阻止过期刷新重新发布。无新 AuthService 枚举或登录入口。
- 独立测试 `adapter_selection_tests.rs` 由 `route_auth.rs` 的显式 cfg(test) 路径注册，以使用已有真实授权发布能力。回环测试走 RoutingListener、授权能力和真实私有文件/元数据，捕获线上 bearer 与 Gemini 客户端头并验证凭据轮换。使用 Codex 通用聊天入口仅测试认证线协议，不代表 Gemini 原生协议转换已验收。账号包回环用例只替换外部 Google 服务，不 mock 内部存储或授权。

### 真实红灯与边界

`cargo test -p vw-agent --lib cc_route_031_ -- --nocapture` 在槽位 13 实际运行 6 项，4 通过、2 失败，退出 101。目标行为红灯为 `/v1` 基础路径产生 `/v1/v1/chat/completions`，期望 `/v1/chat/completions`，随后以去重修复。另一失败是测试夹具只写凭据文件、遗漏账号元数据导致 NotFound，已改为先通过真实 import_gemini_grant 导入，再轮换文件；不将夹具错误伪称产品行为。该轮真实 HTTP 线协议轮换已通过。较早的缺符号、聚合类型、字段可见性等编译诊断均为 0 项运行，不算行为红灯。

### 最终版本验证

格式化执行 `cargo fmt --all -- --emit stdout` 及 `cargo fmt --all -- --config skip_children=true --emit stdout`，退出 0；只读输出后对本项文件定向运行 `rustfmt --edition 2024 --config skip_children=true`，不写其他会话文件。未执行全量单元测试、fmt --check、wasm 或 Makefile 命令。

| 环境与命令 | 槽位 | 退出码、有效测试与耗时 |
| --- | --- | --- |
| 稳定副本：`cargo test -p vw-agent --lib cc_route_031_ -- --nocapture` | 6 | 退出 0；6 项通过，0 失败/忽略，运行 0.31 秒，构建 102 分 31 秒。 |
| 稳定副本：`cargo test -p vw-agent-apps --lib cc_route_031_ -- --nocapture` | 6 | 退出 0；1 项通过，0 失败/忽略，运行 0.07 秒，构建 5 分 10 秒。 |
| 稳定副本：`cargo clippy --all-targets --all-features -- -D warnings` | 6 | 退出 0，耗时 5 分 12 秒。 |
| 原工作区：`cargo test -p vw-agent --lib cc_route_031_ -- --nocapture` | 6 | 退出 0；6 项通过，0 失败/忽略，运行 0.28 秒，构建 55 分 45 秒。 |
| 原工作区：`cargo clippy --all-targets --all-features -- -D warnings` | 6 | 退出 101；本项 agent/apps 已检查，桌面其他会话的 Pi/OMP 接线报 8 个 lint 错误，无测试执行。 |

路由六项覆盖九类适配器/敏感认证头、CRLF 等坏 token、九类默认目标/危险路径、协议前缀去重、真实 owner 读取及不同主体/主机拒绝、真实监听器线上凭据轮换。账号用例覆盖刷新请求表单、更新保存、invalid_grant 失效持久化和后续拒绝网络请求。路由测试链接器报告既有 macOS `__eh_frame` 大小警告，测试退出 0。

交付前再次逐字节核对 selector/独立测试、dispatch、route_auth、父 mod、auth_accounts/其 gemini 模块、credential_file/host、刷新模块/测试、auth/mod 和 launch/account 共 13 个关键文件，副本与真实集成目录全部一致。实际工作区六项测试也已执行通过，不以副本通过替代真实接线验证。测试编译读到其他会话早期 computer_control.rs 的 unused_mut 警告；当前文件已由所属改动移除该 mut，本项未重复编辑。

集成 Clippy 的 8 项阻断具体为：`settings/aggregation_omp.rs` 未使用 `core::*` re-export；`ListMessage::PiDirectory` 暴露较私有的 Pi Event；`aggregation_list::Form.selection/generation` 未读取；Pi Event 多个 UI 变体未构造；OMP core 的 `read`、`Form`、`finish`、`error` 未使用。核对发现这些模块和 UI 正由其他会话新增，OMP core 尚无实际调用者，目录 Form.finish 仍丢弃 selection/generation；接通完整 UI 或用 allow 掩盖均超出本项最小编译兼容范围，因此保留现场，不改动其功能。

历史辅助检查：`cargo test -p vw-agent-apps --lib cc_route_031_ -- --nocapture` 在槽位 12 与 13 各实际通过 1 项，退出 0；最后一轮验证成功刷新、invalid_grant 持久化和后续不再发起网络请求。较早版本的全仓 `cargo clippy --all-targets --all-features -- -D warnings` 在槽位 13 退出 0（42 分 25 秒），该记录早于最终 URL/客户端复用修正，不代替最终版本检查。后续共享目录编译还遭遇过无关聚合 AppState Debug 断言及 voice 可见性错误，均已由所属改动修正；不把这些编译失败算作本项行为失败。

使用过的动态共享槽位为 12、13、6，对应绝对 `CARGO_TARGET_DIR` 分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-12`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-13`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-6`，每轮 `CARGO_BUILD_BUILD_DIR` 与目标目录相同。最后阶段 `CARGO_BUILD_JOBS=1`。全部本任务 Cargo 子进程已退出，脚本 EXIT trap 已释放自己的锁，保留缓存；不删除其他会话的锁。

为避免共享源码持续变动影响有效验证，经用户授权从真实工作区复制到 `/private/tmp/shiju-routing31-verify.Xdg44d`，排除 Git、target 和索引缓存；没有创建 Git worktree。该副本验证仍使用原仓库槽位 6 的绝对构建目录。交付前逐字节核对本项源码和真实集成接线，并分别记录副本及实际工作区验证结果，不用副本通过隐藏实际集成失败。源码仅在原工作区修改。

本项 7 个不同用例通过，实际集成路由六项复验通过，状态为 **已完成，待回归**。保留任务 31 卡；恢复条件为 Pi/OMP 桌面所有者完成生产调用与 UI 接线、消除上述 lint 后，重新动态领取共享槽位，在原工作区运行相同全仓 Clippy。未修改 plan.md，未宣称母任务 8 或跨平台验收完成。


## 任务 32：解析应用目标与模型映射（2026-11-13）

仅 macOS 27.0 (26A428)、arm64 自动验收。本项是授权目标内的模型策略及生产调用链接线，不代表 Grok HTTP 协议入口、真实账号或母任务8已验收；Windows/Linux/WSL 未验证。

已读取固定 upstream 的 model_mapper、handler_context 和 handlers 的 resolve_stack_target，以及当前生产 dispatch、route_auth、provider_publication、failover_execution 和相邻策略/目录实现。快照 model_mapper SHA256 为 `60d7fd77ecadea58d047b06583344af967f4efd92ea48528633144284d0433ab`，handler_context 为 `d064a4d2bbdf6574c024444bdcef758ebb0e34c777d6a5838ecabb01f2030452`。

### 规则与执行边界

- `routing::target_model::resolve` 只消费执行端已经授权的 app、provider settings 和客户端模型，不访问数据库、账号或桌面。角色 env key 复用母任务4/5已有 desktop_mapping::defaults；保持来源 fable→opus 降级、haiku/opus/sonnet 优先于默认模型、子代理原名和 5.5 客户端反向别名。仅 Claude 剥离尾部大小写不敏感的 `[1M]`，Unicode 不切坏，非尾部标记不变。
- `dispatch::forward_json` 在 AuthorizedRoute::select_json 之后消费 PublishedProvider 的同一个保存行快照；已有 Aggregation/Desktop 的精确目标模型优先，不重新选择 provider。不发生模型变化时保留原请求覆盖行为。仍使用原 check_provider、授权 binding 和撤销检查，绝不查询其他租户的默认 provider。
- `failover_execution::Plan::forward` 在校验候选行快照后使用该候选 settings 映射原始客户端模型，不把前一候选的映射传给下一候选。只补该行为所需调用，不改变重试/熔断策略。
- `aggregation::model_ids::decode` 对 GrokBuild 返回 Plain，encode 仍拒绝生成 Grok 聚合 ID；Grok 的 `ccs-*` 和 `[1M]` 字面量不借用 Claude/Codex 命名空间。该既有解码器由 AuthorizedRoute::select_json 真实消费。
- 首轮将 Grok 网络入口未就绪误判为纯策略阻塞；继续核查后确认本项可安全接入现有授权发布，不必实现新的 Grok 配置事务。撤销该阻塞判断。Grok HTTP 入口仍由后续对应协议任务验证，不宣称 Grok 网络转发已交付。

### 验证证据

红灯：`python3 scripts/cargo_slot.py cargo test -p vw-agent-apps --lib cc_route_032_ -- --nocapture`（实际由入口内 sh 执行同一 Cargo 命令），槽位4，退出101；3项运行，1通过2失败。断言为 `claude-sonnet-5[1M]` 未变成 `mapped`，及 `模型[1m]  ` 未变成 `模型`，均为行为失败。

绿灯同一聚焦命令：槽位4，退出0，6项通过、0失败/忽略。覆盖角色/default/fable降级、映射后1M、无映射1M/Unicode、其他应用隔离、Grok保留前缀不解码、子代理原名及5.5别名。

格式化：槽位4运行 `cargo fmt --all -- --emit stdout`，退出0；因共享目录存在其他会话活动，按任务31既有方式只读全仓格式化输出，再对本项七个 Rust 文件定向 `rustfmt --edition 2024 --config skip_children=true`，均退出0；最后两处 agent 接线格式化同样退出0。没有写无关格式化差异。`git diff --check` 退出0。

全仓 `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`（同阶段入口内执行）槽位4退出0，耗时1分56秒。本项源码已由 apps/agent 检查；未以零测试或编译替代聚焦行为验证。

槽位4两个绝对目录相同：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`，`CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。红灯、绿灯及 Clippy 前台进程已退出，入口释放自有锁，保留缓存。后续聚合标识兼容复验结果补于下方。

最终复验：一次 `python3 scripts/cargo_slot.py sh -c '... cargo test -p vw-agent-apps --lib cc_route_032_ -- --nocapture && cargo test -p vw-agent-apps --lib aggregation_t04_ -- --nocapture'`，槽位3，退出0；本项6项再次通过，既有聚合标识4项通过，0失败/忽略。两个绝对目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；命令完成后入口释放锁，保留缓存。总计10个不同用例通过，任务32状态为已完成。仅本项源码及证据提交，不修改只读 plan.md；任务卡按约定删除，建议下一步执行后续协议任务并验收对应 HTTP 请求链。

## 任务 33：解析分片 SSE 与 UTF-8（2026-11-14）

macOS 27.0（26A428）、arm64 自动验收。直接依赖任务32的 `abd4c7575`；复用已有授权运行时、执行主机/应用作用域、公共传输及配置/账号合同，不新增这些母能力。Windows/Linux/WSL 未构建、未验证。

### 来源与实现范围

读取冻结 upstream 的 `proxy/sse.rs`、`json_canonical.rs`，以及 forwarder 的流首帧检查、streaming_codex_chat 的调用和相邻测试。与 cc-switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 Git 对象核对：SSE SHA256 为 `f77b3a47511c891eb4dcd6673725826f2472782487877c922eb782998f92f2f7`，JSON 为 `c02077c1c330af29d41ee9b63e7d90424894f92d839e2d06a8bbdbb534b4b68b`。

- `model_relay::sse_framing::Lines` 由公共 `StreamObserver` 与 Messages、Responses、Gemini、原生到 Chat 转换器共同使用，替换重复行扫描。字节残片保留至完整行后再解码，支持跨任意字节切分的中文/四字节字符、起始 BOM、LF/CRLF 和混合分隔符；非法 UTF-8、嵌入裸 CR 明确拒绝。保留4MiB行/pending及事件上限，不读取凭据或执行工具。
- `sse_framing::resident_body` 接入真实 `dispatch::forward_json` 的 SSE 数据面，在 EOF 为尾块补分隔符，再由原协议观察器或转换器验证。常驻来源允许缺少空行的尾块，但绝不将 EOF 当作协议完成；缺少 Chat `[DONE]` 或其他协议终帧仍失败。旧会话不调用这个尾块适配入口，保留其完整分隔符策略。
- 仅规范化 SSE 行边界，不修改 JSON 数据或工具参数；没有引入来源中将空参数强制改为 `{}` 的行为，避免改变语义。既有身份校验、字段策略和模型映射保持原调用链。

### 红灯、绿灯与检查

初始标准 Cargo 聚焦构建因耗时且测试断言补强后已过时而主动结束，退出1、0项运行，不作为行为红灯。另用原生产 StreamObserver 的帧校验代码构建独立原生探针（仅移除 HTTP body 包装、以同名四变体枚举提供协议类型），对起始 BOM 的同一文本事件断言实际失败：split 0，期望1、实际0，1项运行、退出101。该探针只证明局部基线缺陷，不冒充完整 crate 验收。修复后隔离原生入口4项通过，包含真实回环 HTTP；最终以以下仓库原生结果为准。

同一入口 `python3 scripts/cargo_slot.py python3 /tmp/shiju033-validate.py` 动态领取槽位4；`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。

| 验证 | 结果 |
| --- | --- |
| `cargo fmt --all -- --emit stdout`，本次9个 Rust 文件另外用 `rustfmt --edition 2024 --config skip_children=true` 写回；`git diff --check` | 退出0；全仓输出模式避免改写其他会话文件 |
| `cargo test -p vw-agent --lib cc_route_033_ -- --nocapture` | 退出0，4项通过、0失败/忽略；覆盖每个字节切分点、UTF-8/BOM/CRLF、pending上限、非法编码、四协议合法尾块/缺失终帧、工具JSON保真及真实有限回环HTTP |
| `target/rust-slot-4/debug/deps/vw_agent-346fac923f6b2351 shared_stream_observer_ --nocapture` | 退出0，4项通过；覆盖四协议分片、非法终帧/模型变化、生产body原字节/取消及原生转换器客户端模型映射 |

原生测试链接报告既有 macOS `__eh_frame` 超过16MiB的链接器警告，不影响上述8个不同测试的结果。额外 Cargo 观察器回归因共享工作区其他源码更新触发重复重编译，主动仅结束自有 rustc，退出101、0项运行；以上直接运行刚生成二进制的4项结果替代该冗余构建，不伪称被中断命令通过。

`cargo clippy --all-targets --all-features -- -D warnings` 实际执行并退出101。既有桌面编辑器与当前 iced-code-editor v0.5.3 接口不一致：`preview_panel/view/editor_options.rs:34` 调用不存在的 `wrap_column`，`app/message/editor.rs:276` 调用不存在的 `set_wrap_column`；`editor_options_tests.rs:153` 起另有7个同类 E0599。生产库共2个、测试目标共9个诊断，无本项文件诊断；不修复或提交这些无关变更。

本项状态为 **待回归**，保留任务33卡。恢复条件：桌面编辑器所有者对齐依赖接口后，动态领取共享槽位，重新执行上述整仓 Clippy；若解析代码变化，再执行本项原生聚焦测试。槽位4入口已正常退出，锁已释放，缓存保留。未修改只读 plan.md，不宣称母任务8完成。

长构建诊断：对自有 rustc PID46423 采样显示 LLVM ThinLTO、指令选择及调度正在执行，无槽位/编译锁等待；物理占用23.2GiB、峰值39.1GiB，采样前约3分钟仍写入增量依赖图/查询缓存；累计 CPU 时间在连续观测中由67分增至90分。确认有正常进展后等待原生4项通过，未终止其他任务进程。全仓 Clippy 曾仅暂停自有进程组，等待其他检查结束后恢复，保持串行检查。

## 任务34：转发原生 Chat 协议（2026-11-14）

macOS arm64 自动验收。直接前置任务33为“待回归”，按调度契约允许继续实现，未将其当作完成回归。核对冻结 CC Switch 4.0.25 提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`：`proxy/handlers.rs` SHA256 `31fe8c8470f1c7256a1834512817004c67a2f1ce01fea1579485596dcffcc809`、`proxy/providers/codex.rs` SHA256 `56ac9e4f6c1a883403489a1d2f02402b943a937162ee7c4f00a9481893ed8783` 均与来源映射一致，并读取 `proxy/server.rs` 的直接路由注册。复用已注册 vw-agent-apps 底座、成员/执行主机/应用身份、共享传输、配置事务及账号契约。

实现落点：`model_relay::native_chat::is_path` 由 `app_routing::dispatch::forward_json` 调用，仅为常驻 Codex Chat 入口识别来源明确注册的 `/chat/completions`、`/v1/chat/completions`、`/v1/v1/chat/completions`、`/codex/v1/chat/completions`，四者复用同一授权目标和 `prepare_request`/共享传输，不根据客户端路径改变上游 URL。不重复协议转换器，不放宽会话 relay 校验，不执行本地工具。JSON 缓冲及 SSE resident framing/StreamObserver/期限路径沿用既有所有者；工具调用、usage、finish_reason 保持数据透传。

测试：独立 `native_chat_tests.rs` 显式注册，验证注册路径及拒绝未知路径；真实授权回环测试位于既有同职责 `app_routing/dispatch_tests.rs::cc_route_034_authorized_native_chat_json_and_sse`，避免为测试扩大认证内部可见性。覆盖四路径各 JSON/SSE（8次上游请求）、工具历史与工具声明、未知厂商字段、上游凭据替换、中文工具参数、usage/finish_reason；逐路径无凭据请求拒绝，未知前缀、尾斜杠和查询参数不达上游。测试不mock内部认证、路由或共享传输，仅替代外部上游。

红灯：先提取与原 dispatch 等价的单一路径判定，再运行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_route_034_ -- --nocapture`，槽位6；完整 crate 原生运行1项，`/chat/completions` 行为断言失败，退出101（非缺失符号）。同源独立 rustc 探针同样1项失败；修复后局部探针1项通过，不代替完整 crate 绿灯。

最终阶段命令为 `python3 scripts/cargo_slot.py python3 /tmp/shiju034-validate.py`，其顺序执行：

| 验证命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --emit stdout` | 退出0；为避免共享工作区无关格式化差异，全仓只读输出；本项5个 Rust 文件以 `rustfmt --edition 2024 --config skip_children=true` 定向写回，退出0 |
| `cargo test -p vw-agent --lib cc_route_034_ -- --nocapture` | 退出0，2项通过、0失败/忽略，包含四路径八次授权 JSON/SSE 回环及拒绝边界 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 退出0，全仓通过 |
| `git diff --check` | 退出0 |

测试链接报告既有 macOS `__eh_frame` 超过16MiB的链接器警告，不影响测试通过。实际槽位6，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`；命令入口正常退出，锁已释放，缓存保留。本项状态为已完成。Windows/Linux/WSL 范围外、未构建或验证，WSL 地址继续保留来源映射；不宣称母任务8完成。


## 任务35：转发原生 Messages 协议（2026-11-14）

仅 macOS arm64 自动验收；依赖任务34提交 `3a329575b`。沿用已注册 vw-agent-apps、公共传输与成员/项目/执行主机授权，以及母任务4/6原生配置事务与账号作用域。未启动真实桌面或访问真实模型账号，不代表母任务8完成；Windows/Linux/WSL范围外且未验证。

来源冻结提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`：仓库 upstream 的 `proxy/providers/claude.rs` 和 `proxy/handlers.rs` 与相邻 cc-switch 固定 Git 对象逐字节相等，SHA256 分别为 `0abde3dbac0f7e03bd00c194aeed823dd17c66603cd3fff907ff23f3d8e39432`、`31fe8c8470f1c7256a1834512817004c67a2f1ce01fea1579485596dcffcc809`。读取来源适配器、handlers 和 server 路由注册，并复核真实 dispatch、DesktopPublication、messages_stream 与相邻授权/透传测试。RustCodeGraph status 有索引，explore 长时间无输出后终止，回退原文件导航。

实现：`model_relay::native_messages::is_path` 仅识别来源明确注册的 `/v1/messages`、`/claude/v1/messages`，由 `app_routing::dispatch::forward_json` 的 Claude 分支调用。同职责底座已能透传原生 JSON/SSE，因此只补入口差异，不复制转换器。Desktop 保留独立 `/claude-desktop/v1/messages` 与应用凭据；不将其并入普通 Claude 路径。不修改旧会话字段白名单，不读取新凭据、不执行本地工具、不进入 session/prompt。既有私有字段过滤、授权目标、JSON大小限制、SSE framing/观察器及超时沿用原所有者。

### 本次验证

- 红灯：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_route_035_ -- --nocapture`，槽位6，退出101，2项中1通过1失败；来源别名 `/claude/v1/messages` 预期200实际404，失败来自行为断言。原生thinking被旧会话窄转换器拒绝的隔离测试通过。
- `cargo fmt --all` 退出0；已检查差异，无本项引入的无关格式化变更。
- 绿灯：完全相同聚焦命令，槽位6，退出0，3项通过、0失败或忽略。真实有限回环HTTP上游记录请求，覆盖两个来源路径的JSON与SSE、tool_use/tool_result、图文、thinking签名及signature_delta、input_json_delta、usage与stop_reason、供应商扩展字段、未认证拒绝、未知路径/query/尾斜杠及Claude凭据访问Desktop路径拒绝。请求JSON和返回SSE逐值/字节相等，工具仅返回客户端。
- 已构建原生测试产物 `target/rust-slot-6/debug/deps/vw_agent-346fac923f6b2351` 分别以 `cc_route_008_route_auth_binds_each_dimension` 和 `aggregation_t46_desktop_discovery_and_messages_use_one_scoped_listener` 过滤运行，各退出0、1项通过。覆盖独立Desktop应用凭据、租户/项目/主机/账号代次、原生Desktop目标认证和模型映射、Desktop凭据访问普通Messages拒绝及撤销。
- 红绿测试链接报告既有 macOS `__eh_frame` 大小警告，不影响通过。后续Cargo授权回归因共享源码变化重编译，主动中断（入口报Operation not permitted，退出1）；已核对自有子进程及槽位6锁均退出，改为上述已构建产物回归，不将中断当作通过。
- 槽位6两目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`；后续回归与Clippy入口自动领取槽位4，两目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。当前共享源码含其他任务改动，未将验证描述成干净HEAD验收。
- 整仓 `cargo clippy --all-targets --all-features -- -D warnings` 槽位4退出101：`crates/vw-desktop/src/app/task/executor/state.rs:161–167` 的 `GIT_SUMMARY_TAG`、`GIT_SOURCE_BRANCH_TAG`、`GIT_TARGET_BRANCH_TAG`、`GIT_WORKTREE_PATH_TAG` 四个既有未使用常量触发 dead_code。本项未修改该文件，未扩展修复无关桌面功能。入口退出后槽位4/6自有锁均释放，保留缓存。`git diff --check` 通过。

随后共享工作区其他任务删除了上述四个常量，本项未触碰该文件。重新执行 `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`，自动领取槽位3，退出0，22.23秒完成；两目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。三个阶段自有入口均已退出，锁释放、缓存保留。

本项状态为 **已完成**。3项本项用例及2项相邻回归通过，整仓Clippy最终退出0；不修改只读plan.md，不宣称母任务8完成。

## 任务 36　转发原生 Responses 协议

2026-11-14，macOS，仅本平台验收。依赖任务35提交 `8d1f6c2a7`；复用生产发布 `provider_publication::apply`、`RouteAuth`、公共传输及任务3引用所有者。外部运行时、执行主机、应用、原生配置和账号契约沿用既有实现，不新增母能力。核对 CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`；`providers/codex.rs` SHA256 `56ac9e4f6c1a883403489a1d2f02402b943a937162ee7c4f00a9481893ed8783`、`handlers.rs` SHA256 `31fe8c8470f1c7256a1834512817004c67a2f1ce01fea1579485596dcffcc809` 匹配来源映射与仓库快照。

实现落点：`model_relay::native_responses` 仅处理常驻来源差异。`dispatch::forward_json` 接入五个来源 Responses 路径，保持选定上游目标，GET 返回426及 `websocket_not_supported`，触发 HTTP 回退。引用表由每个 `AuthorizedRoute` 独占；作用域来自可信绑定的项目（含租户）、执行主机、应用、账号、账号代次及实际目标URL，撤销及发布代次仍由原鉴权所有者控制。请求原文及请求覆盖后的正文均检查 `previous_response_id` 与 `item_reference`；未知引用返回403，JSON响应和SSE事件登记返回的response/item标识，不改写其内容。登记表沿用4096条/标识长度上限，随路由释放。

原生Responses红灯发现共享 `timeouts::OutputObserver` 将其全部事件当作Chat静默帧，合法流因此返回502。复用现有 `StreamObserver` 增加Responses分类，保留首包、空闲、取消及容量许可语义，不改旧会话白名单、不进入session/prompt或执行工具。JSON仍由原缓冲所有者持有容量许可；SSE仅观察登记，字节原样交付下游。官方上游URL与托管凭据解析仍由既有所有者处理；Windows/Linux/WSL范围外且未验证，WSL地址保留来源映射。

测试：独立 `native_responses_tests.rs` 在父模块显式注册。真实回环上游由 `app_routing/dispatch_tests.rs` 复用生产鉴权与共享传输，覆盖五路径JSON/SSE、工具与扩展字段、上游认证替换、无认证与未知路径拒绝、426回退、引用连续请求及跨主机隔离。独立策略测试覆盖未知response/item引用及作用域隔离，SSE三字节分片测试确认登记与字节保持。

验证记录（所有Cargo命令均由 `python3 scripts/cargo_slot.py` 执行）：

| 阶段 | 入口内命令 | 结果 |
| --- | --- | --- |
| 红灯 | `cargo test -p vw-agent --lib cc_route_036_ -- --nocapture` | 退出101，1项执行且失败；原生SSE返回502而非200。 |
| 最终格式化 | `cargo fmt --all` | 退出0，无无关格式化差异纳入提交。 |
| 最终绿灯 | `cargo test -p vw-agent --lib cc_route_036_ -- --nocapture` | 退出0，3通过、0失败。 |
| Messages回归 | `cargo test -p vw-agent --lib cc_route_035_ -- --nocapture` | 退出0，3通过。 |
| 鉴权回归 | `cargo test -p vw-agent --lib cc_route_008_ -- --nocapture` | 退出0，3通过，覆盖各绑定维度、旧代次和在途撤销。 |
| 全仓Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 退出0，1分03秒。 |
| 差异检查 | `git diff --check`（限定本项路径） | 退出0。 |

红灯及最终验证槽位2：`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。中间一轮绿色尝试中连续引用夹具先于响应执行，得到预期安全拒绝403，已修正夹具顺序；第二轮红灯构建主动中断，退出130，不计成功。最终原生3项通过为有效证据。链接器仍报告既有 macOS `__eh_frame` 大小警告，未影响原生测试或Clippy通过。

额外超时回归：误用 `app_routing::route_auth::timeouts_tests` 得到0项，明确不计入验收。正确Cargo过滤 `app_routing::timeouts_tests` 自动领取槽位3后开始重新编译，主动中断（退出130），改为在槽位入口内执行本次已构建原生产物：`/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/vw_agent-346fac923f6b2351 app_routing::timeouts_tests --nocapture`，退出101，4项执行，2通过2失败。失败为 `cc_route_021_stream_heartbeats_do_not_extend_idle_after_output`、`cc_route_021_stream_idle_zero_disables_idle_without_a_total_timeout`，均在收到Chat夹具后返回502。

随后用不含 `cc_route_036_` 的既有槽位4原生产物运行同一过滤（`/Users/xiangmin/worker/code/shiju/target/rust-slot-4/debug/deps/vw_agent-346fac923f6b2351 app_routing::timeouts_tests --nocapture`），同样退出101、2通过2失败，断言位置分别为 `timeouts_tests.rs:256`、`:300`。旧夹具缺少现有 `StreamObserver::chat` 要求的 `choices[].index`，本项未修改该校验器或这些测试，不将旧夹具问题扩大为本项修复。两次原生产物回归入口均领取槽位3，其两目录为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；槽位4仅为已有基线二进制读取位置，未领取或写入其缓存。自有入口与子进程均退出，自有锁释放、缓存保留。

本项所需3项绿灯及整仓Clippy证据齐全，状态 **已完成**。额外超时回归的两项既有夹具失败保留上述复现，建议由原所有者修复；不将其描述为通过，不修改只读plan.md，不宣称母任务8完成。


## 任务37：转发原生 Gemini 协议（2026-11-14）

macOS 本机自动验收；依赖任务36提交 `6c280ef6c`，复用公共授权路由、可信 `JsonTarget`、传输、配置超时及撤销所有者。Gemini原生配置读写已经存在，生产Proxy配置事务/发布仍未提供Gemini分支；本项不代为实现母任务4。按调度继续完成已授权目标的数据面行为，回环测试通过真实监听器鉴权，不mock内部转换或传输；不将其描述为生产配置切换验收。官方账号与生产配置事务仍由母任务4/6交付。

冻结来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 三文件与仓库upstream逐字节相等：`providers/gemini.rs` SHA256 `c68532f739fe918e7509095bf035e11de4801bc2e251fd9b76070faf9dbc6ae2`、`gemini_url.rs` `d92adf389a2abac34b6c83220cadea445d3147c6ebcd5c8b2b786df91431dacf`、`handlers.rs` `31fe8c8470f1c7256a1834512817004c67a2f1ce01fea1579485596dcffcc809`。读取server入口与handler调用者，RustCodeGraph status/explore检查后以当前源码校对。

实现所有者：`model_relay::native_gemini::Request` 只解析来源三个生成路径前缀 `/v1beta`、`/gemini/v1beta`、`/gemini/v1` 的POST生成/流式生成；规范化一次`models/`模型前缀，流式动作补`alt=sse`。沿用 `transport::gemini_url` 的已授权目标版本/地址与模型安全语法，不从客户端接收目标、凭据或任意查询；未知动作及非SSE查询明确拒绝，发现/其他动作留给各自任务。已有目标构造器不支持的自定义opaque URL仍明确拒绝，不伪称支持来源全部URL模式。`dispatch::forward_json` 从URI取model/stream，不注入Chat model字段，JSON和SSE保留tools、functionCall/functionResponse及thoughtSignature。可信原始API key放`x-goog-api-key`，Bearer保留授权头；托管凭据仍走原租约所有者。

`route_auth::select_json` 只给Gemini普通可信目标开放选择，不修改聚合模型ID解码规则。`timeouts` 接入现有Gemini StreamObserver，完成候选后继续转发合法usage帧直到完整EOF；取消、容量和首包/空闲限制继续生效。纯转发不进入session/prompt，不执行工具，不改变旧会话字段白名单。

红灯：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_route_037_ -- --nocapture`，槽位2，退出101，2项中1通过1失败；原生生成入口预期200实际404，来自行为断言。旧会话严格拒绝扩展字段的测试通过。首次编译6分38秒，链接器报告既有macOS `__eh_frame`大小警告。

本次最终验证：

| 检查 | 结果 |
| --- | --- |
| `cargo fmt --all` | 退出0，已审查本项文件差异，无无关格式化进入本项提交；尾部usage迭代以定向rustfmt格式化 |
| `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_route_037_ -- --nocapture` | 与红灯相同命令；最终退出0，3项通过、0失败，11分08秒构建；覆盖三来源前缀、两个动作、model前缀/目标版本/query、未知路径/动作/query/GET拒绝、未认证拒绝、请求工具/签名及JSON/SSE响应逐值/逐字节保留，含尾部usage；旧会话严格隔离通过 |
| `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings` | 顺序阶段入口内执行，退出0，1分16秒完成 |
| `target/rust-slot-2/debug/deps/vw_agent-346fac923f6b2351 authorized_native_ --nocapture` | 退出0，4项通过，Chat/Messages/Responses/Gemini真实回环回归 |
| `python3 scripts/cargo_slot.py cargo test -p vw-gateway-client --lib app_routing -- --nocapture` | 退出0，7项通过，默认features |
| `git diff --check` | 退出0 |

阶段入口 `python3 scripts/cargo_slot.py sh -c 'cargo fmt --all ... && cargo test -p vw-agent --lib cc_route_037_ -- --nocapture ... && cargo clippy --all-targets --all-features -- -D warnings ...'` 退出0；尾部usage修正后的相同聚焦命令再次退出0。后续Cargo相邻回归因其他任务修改共享依赖开始重复重编译，向自有槽位入口发SIGTERM，入口退出130并清理子进程，未将中断当作通过；改用刚完成最终聚焦测试的同一产物执行上述4项回归。最后独立执行SDK测试。

所有阶段实际槽位 **2**；`CARGO_TARGET_DIR`、`CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。入口退出，锁已释放，缓存保留。共享工作区同时存在其他任务修改，不将结果描述为干净HEAD验收；本项仅提交直接实现/测试/本条证据。状态 **已完成**，仅覆盖授权目标协议数据面，不代表母任务8或Gemini生产配置发布完成。Windows/Linux/WSL范围外、未验证，WSL地址继续保留来源映射。

## 任务38：转换 Messages 到 Chat 请求（2026-11-14）

状态：已完成。macOS 本机自动验收，依赖任务37 `5457e7089`。沿用母任务1冻结快照、母任务2公共传输、母任务3可信应用/执行主机授权、母任务4配置与母任务6账号契约；不新增这些母能力。固定来源提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `src-tauri/src/proxy/providers/transform.rs` 由相邻仓库 Git 对象导出并与仓库 upstream 快照 `cmp` 逐字节一致，SHA256 `cafd48dc3c56ae4dfb5ab40358fe26571b27a4f67fdc109cee00ff7109f022f2`。读取了 Claude adapter 调用者和来源相邻测试；使用 RustCodeGraph status/explore 后结合当前源码校对。

实现：`model_relay::messages_chat_request::to_chat` 只包装现有 `messages::to_chat`，先复用严格字段/能力校验，再按来源移除每个system文本开头的归因行，以换行合并system数组；支持字符串auto/any/none工具选择；工具schema根缺类型时补object与properties，递归清理properties/items中的uri格式。对象工具选择、并行开关、工具调用/结果、URL/base64用户图片和顺序沿用现有所有者。未知字段、未知工具选择、document等不支持输入明确拒绝。旧会话转换器保持原样，测试断言严格策略与常驻来源策略不同。没有读取凭据、抓取媒体或执行工具。

`dispatch::forward_json` 仅在可信选定目标路径为Chat completions时转换常驻Claude请求，两个Messages来源路径共用此入口；Desktop由其既有协议所有者选择后复用同一函数。响应仅复用既有messages_stream适配器，未在本项扩展JSON/SSE转换语义。原生Messages继续原样转发。来源的工具结果媒体提取、推理重放及其他协议能力没有在本项扩展，仍沿用底层明确拒绝边界；不声称完整来源工具媒体/推理支持或母任务8完成。Windows/Linux/WSL范围外、未验证，WSL映射保留。

验证（入口均为 `python3 scripts/cargo_slot.py`）：

| 检查 | 结果 |
| --- | --- |
| 红灯 `cargo test -p vw-agent --lib cc_route_038_ -- --nocapture` | 退出101，1项执行且失败；system实际保留归因并直接连接，预期First换行Second，行为断言红灯。 |
| `cargo fmt --all` | 退出0；限定本项差异审查，无无关格式化纳入提交。 |
| 同命令最终绿灯 | 退出0，4通过、0失败；3项转换边界测试与1项真实有限回环HTTP测试，覆盖两个授权路径、上游认证头、schema、tool_choice和图文顺序。构建14分16秒。 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 退出0，1分15秒。 |
| `cargo test -p vw-gateway-client --lib app_routing -- --nocapture` | 退出0，7通过、0失败，默认features。 |
| 已构建同一原生产物过滤 `cc_route_035_ --nocapture` | 退出0，3通过，原生Messages回归。 |
| 同产物过滤 `desktop_routing --nocapture` | 退出0，0项；不计入验收。 |
| 限定本项 `git diff --check` | 退出0。 |

顺序验证入口：`python3 scripts/cargo_slot.py sh -c 'cargo fmt --all && cargo test -p vw-agent --lib cc_route_038_ -- --nocapture && cargo clippy --all-targets --all-features -- -D warnings && cargo test -p vw-gateway-client --lib app_routing -- --nocapture'`，退出0。所有入口实际槽位2，`CARGO_TARGET_DIR`、`CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。原生产物 `target/rust-slot-2/debug/deps/vw_agent-346fac923f6b2351`。入口及自有子进程结束，锁释放、缓存保留。链接器有既有macOS `__eh_frame`大小警告；全仓Clippy通过。共享工作区存在其他任务变更，只提交本项路径，不作为干净HEAD验收。

## 任务39：转换 Chat 到 Messages 结果（2026-11-14）

验收范围：macOS 本机自动测试；直接依赖任务38 `caf3a9ead`，复用前项核实的母任务1来源快照、母任务2公共传输、母任务3可信项目/主机授权，以及母任务4、6配置/账号契约。再次从 CC Switch 固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 导出 `src-tauri/src/proxy/providers/transform.rs`，与仓库 upstream 快照 SHA256 均为 `cafd48dc3c56ae4dfb5ab40358fe26571b27a4f67fdc109cee00ff7109f022f2`。读取其非流式转换、相邻测试及 Claude adapter 调用者，结合 rustcodegraph 调用图检查真实所有者。

实现：`model_relay::messages_chat_result::from_chat` 包装既有严格 `messages::from_chat`，只补常驻 JSON 来源差异。识别 data.choices 封装，按来源以顶层非空元数据覆盖内层；保留文本数组、refusal 与多工具调用顺序；映射 content_filter/function_call；缓存读取遵循直传非零、nested cached_tokens、DeepSeek hit 顺序，cache creation 按来源字段优先级选择，并从总输入扣除。结果回写客户端模型名。工具只返回给客户端，不执行、不读取凭据、不进入 session/prompt。

`app_routing::dispatch::finish_messages` 在授权常驻 Claude/Desktop 选择 Chat 上游且非流式时接线，保留取消与大小限制；原生 Messages、旧会话严格转换和 SSE 路径不变。未知终止原因、损坏工具 JSON、未知内容类型、异常缓存计数明确返回502，避免来源的静默丢弃/空对象回退；旧 function_call 要求非空调用ID和名称，不伪造关联标识。推理内容仍遵循底层明确拒绝策略，未扩展到后续推理/SSE任务。Windows/Linux/WSL范围外、未验证，保留来源映射；不宣称母任务8完成。

状态：已完成。验证均在 `python3 scripts/cargo_slot.py` 入口内执行。

| 检查 | 结果 |
| --- | --- |
| 红灯 `cargo test -p vw-agent --lib cc_route_039_ -- --nocapture` | 退出101，1项执行并失败，封装结果返回502导致行为断言失败；不是缺失符号。 |
| `cargo fmt --all` | 退出0；本项差异审查通过，未纳入无关格式化。 |
| 同命令最终绿灯 | 退出0，4通过、0失败：封装/refusal、内容数组/工具/缓存/终止映射、畸形数据/旧策略隔离、两个授权Messages路径的真实有限回环HTTP。 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 退出0，2分13秒。 |
| `cargo test -p vw-gateway-client --lib app_routing -- --nocapture` | 默认features，退出0，7通过、0失败。 |
| 限定本项 `git diff --check` | 退出0。 |

最终顺序入口：`python3 scripts/cargo_slot.py sh -c 'cargo fmt --all && cargo test -p vw-agent --lib cc_route_039_ -- --nocapture && cargo clippy --all-targets --all-features -- -D warnings && cargo test -p vw-gateway-client --lib app_routing -- --nocapture'`，退出0。红灯与最终验证实际槽位均为2；`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。中途审查补充标量对象校验后，主动中断一次本项未完成构建并完整重跑；不计该中断为验证证据。最终测试构建10分47秒，原生产物 `target/rust-slot-2/debug/deps/vw_agent-346fac923f6b2351`。有既有macOS链接器 `__eh_frame`大小警告，全仓Clippy通过。自有前台进程结束、槽位锁释放、缓存保留。共享工作区其他任务变更未纳入本项提交。

## 任务40：转换 Chat 到 Messages 流（2026-11-14）

范围：macOS 自动验收。复用任务39 `0420b2e43`、母任务1冻结来源、母任务2公共传输及母任务3可信应用/执行主机作用域，沿用母任务4、6配置/账号合同。固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/providers/streaming.rs` 与仓库 upstream 快照 SHA256 均为 `8d9556b0edfb580b58a7a674e0f11e349ace1ddffae75df5d6ac5b5e068e1289`；已读取其用量、双工具索引测试与 handlers 调用者。

实际所有者仍为 `model_relay/messages_stream.rs::Decoder`，不新增第二套转换器。`resident_response_body` 经 `app_routing/dispatch.rs::finish_messages` 接常驻 Claude/Claude Desktop 的 Chat SSE；旧会话 `response_body` 保持严格转换。复用任务39结果转换器计算三桶互斥的用量。文本增量发送；并行工具按来源索引聚合跨帧参数，完整JSON验证后输出独立内容块，不执行工具。保留4MiB有界传输、错误载荷脱敏和一次错误收口。常驻观察器逐行验证至首个有效终帧，忽略后续重复终帧，不改变旧会话的整块严格检查。

新增独立 `messages_chat_stream_tests.rs`，以 `cc_route_040_` 前缀组织4项：真实回环HTTP按字节发送中文、乱序双工具参数和缓存用量；断流/非法参数/上游错误只发送一次错误且无成功终帧；重复终帧只输出一次成功终态；旧会话用量保持严格语义。无真实凭据，不模拟内部转换器。

默认原生命令 `CARGO_BUILD_JOBS=1 python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_route_040_ -- --nocapture` 在恢复后的槽位2（实现前启动）和槽位4（实现后）均退出101：rustc遭SIGKILL，执行0项。此前槽位10同样遭SIGKILL，槽位3一次限并发重试由本任务中断；均不计行为红灯。采样确认原生构建耗时位于LLVM ThinLTO阶段。尚无行为红灯断言，不能宣称TDD证据完整。

全仓 `cargo clippy --all-targets --all-features -- -D warnings` 在槽位4退出0，包含测试目标；最终日志 `/tmp/shiju-task40-validation.log` 记录 `focused=101 clippy=0`。`cargo fmt --all` 退出0，排除本项以外格式化差异后无额外改动，最终差异检查通过。槽位2、3、4、10的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-N`（N为对应实际槽位号）。缓存保留。

替代原生验证：`CARGO_BUILD_JOBS=1 python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --config 'profile.test.package.vw-agent.opt-level=0' cc_route_040_ -- --nocapture` 在槽位3退出0，4项通过、0失败，默认features未变，仅vw-agent测试包优化级别降为0。产物 `target/rust-slot-3/debug/deps/vw_agent-d51a7b7fffcc3d18`，日志 `/tmp/shiju-task40-lowopt.log`。两个Cargo目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。这证明本项四个边界场景通过，但不替代指定默认优化命令及行为红灯证据；状态为 **待回归**，任务文件保留。所有自有命令已退出，锁释放、缓存保留；Windows/Linux/WSL范围外且未验证。

## 任务45：渲染搜索引用与结果元数据（2026-11-14，未验证）

依赖沿用任务44提交 `23b35c14c` 的托管搜索接线、公共传输与可信作用域；来源为母任务1冻结的 `crates/vw-agent-apps/upstream/src-tauri/src/proxy/providers/transform_responses.rs`。现有 dispatch 的 JSON/SSE 入口继续调用同一结果所有者，无需修改共享分发文件。

新增 `search_citations` 及私有 Markdown 保护模块，复用冻结来源的 Unicode scalar 到 UTF-8 偏移换算、代码区域/引用链接识别及 URL 去重。JSON 渲染引用并以最终消息注解补充搜索来源标题、保留 page_age；失败搜索仍沿用既有错误映射。SSE 文本缓冲至 output_item 完成再渲染，搜索结果延迟至终帧获取最终注解元数据；不执行工具、不接触凭据、不改变旧会话白名单。

已添加 `cc_route_045_` 四个独立测试用例，覆盖中文/emoji、现有链接/引用链接/代码块、来源元数据和 SSE 晚到引用。按用户最新覆盖要求，未运行 Cargo、单元测试、实机测试或最终回归，无红绿灯、有效测试数或通过证据；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 不适用。后续统一验证须检查上述测试及 SSE 完成帧重放、错误搜索与引用晚到行为。仅 macOS 待验收，其它平台未验证。任务文件按用户要求删除，不代表母任务8已验收。


## 任务57：转换 Responses 到 Anthropic 请求（2026-11-14，未验证）

依赖任务56的 `ChatTools` 命名空间/自定义工具映射已在当前工作区交付；沿用公共传输、可信 `JsonTarget`、应用/执行主机作用域及配置/账号契约，不新增配置写入或账号能力。冻结来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `proxy/providers/transform_codex_anthropic.rs` 与仓库 upstream 逐字节一致，SHA256 `4ae2b3248c9cb775f2306c4868b80b4a4c6b00a79e2684eca0f0f08f00a550f9`。来源及当前调用链已读取；依赖代码存在不代表依赖行为已经验证。

`responses_anthropic_request::convert` 复用既有 Responses 控制字段/工具映射与 Messages 请求编码，补充 instructions、system/developer 优先级、图文/URL与base64 PDF文件、工具结果媒体、合并角色、leading user、末尾assistant空白及不完整并行工具回合清理。工具只作为客户端数据传递，不执行、不获取媒体或凭据；不改旧会话白名单。文件ID、非HTTP地址、非法base64、未知内容与未支持的推理回放明确拒绝。未指定输出上限时使用4096；不宣称自动匹配模型能力。`responses_compaction::Request` 与授权 `dispatch` 接线 `/messages` 目标，JSON结果仅复用既有 Messages→Chat→Responses 解码，不扩展任务58的结果语义；Anthropic流式请求明确422，留待对应流任务。

独立测试文件添加4个 `cc_route_057_` 用例，覆盖指令/leading user/末尾空白、媒体及命名空间自定义工具身份、部分并行回合/孤立结果、媒体拒绝与旧会话隔离。按用户本次覆盖要求，未运行任何Cargo命令/脚本、单元测试、实机测试或最终回归，无红绿灯、退出码或有效测试数证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR不适用。代码仅做定向格式化和差异审查，后续统一验证需运行 `cc_route_057_` 并补授权回环HTTP验收。任务卡按用户要求删除，不表示macOS或母任务8已经验收；其他平台未验证。

## 任务58：Anthropic JSON 到 Responses 结果（2026-11-14，未验证）

依赖任务57 `98608279d` 的常驻请求接线与既有公共传输/可信作用域；来源沿用冻结快照，参考相邻 CC Switch `transform_codex_anthropic.rs` 的结果、usage、stop_reason 映射。`responses_anthropic_result::convert` 复用共享 Messages 解码和 Responses 工具身份恢复，保留文本/工具顺序、缓存读写计数、拒绝与截断状态。未知终止原因和 pause_turn 明确拒绝；thinking 与服务端工具保持既有拒绝边界，交由后续任务。真实调用为 dispatch → responses_compaction::Request::response_json → convert，无需重复修改 dispatch。旧会话路径不变。

添加独立 `cc_route_058_` 测试代码。按用户覆盖要求未运行 Cargo、单元测试、实机测试或最终回归；没有通过证据，未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 不适用。仅审查代码差异；任务卡按要求删除，后续统一验证应运行本项测试及常驻回环验收。macOS 尚未验收，其他平台未验证，不表示母任务8完成。


## 任务60：往返推理与不透明状态（代码已交付，未验证）

2026-11-14：按用户本次覆盖要求仅写代码，未运行 Cargo、单元测试、实机测试或最终回归；未领取槽位，CARGO_TARGET_DIR 未设置。不得据此认定母任务8验收通过。

沿用任务59已接通的 Responses/Anthropic 常驻流与既有 AuthorizedRoute、共享传输及可信作用域；来源读取仓库 upstream 的 reasoning_bridge.rs、transform_codex_anthropic.rs 及其相邻用例。配置和账号仍由既有发布所有者持有。

新增 reasoning_bridge，保留来源版本前缀、URL-safe 无填充编码、OpenAI reasoning 输入字段白名单（type/id/summary/encrypted_content），兼容 redacted_thinking 历史载体。Anthropic 封装仅恢复 thinking/signature 或 redacted data，不解码任意 ciphertext，不将载荷用于授权。Messages↔Responses 请求和 JSON 结果沿用现有转换所有者；dispatch 已调用这些所有者，无需重复接线，旧 session transport 不变。

Responses→Messages 流在完整 reasoning item 到达后发送 summary 与 signature；Anthropic→Responses 流保留完整签名分片，在终态追加来源 reasoning 项及其完成事件。后者暂缓 reasoning 展示并在输出末尾呈现，文本与工具仍走既有流。所有不透明状态仍受原有流大小界限约束。新增 cc_route_060_ 回归用例源码，尚未执行；后续统一验证应覆盖 JSON 往返、空 summary、历史封装白名单、签名分片及终态输出索引。Windows/Linux/WSL 范围外、未验证。

## 任务61：切分内嵌思考文本流（代码已交付，未验证）

复用固定 upstream 快照 inline_think 的首块状态机，在 routing/inline_think.rs 提供唯一策略。真实调用链为 app_routing/dispatch → model_relay 的 resident Messages/Responses SSE 适配 → inline_think::normalize；已有 sse_framing 在任意网络字节边界恢复 UTF-8 与 JSON，拆分器消费解码后的 content 增量。只识别正文首块，支持 think/thinking 与来源的混配闭合行为；思考即时输出，工具、finish、DONE、EOF/错误边界冲刷残留，异常结束仍返回原失败事件。非流式与原生 relay 策略不在本项扩展。

任务60已有 reasoning_bridge 及流接线源码；来源快照、常驻监听器和授权 dispatch 已存在。此处仅确认代码依赖，未补做母任务实机验收。新增 cc_route_061_ 测试源码覆盖合法字符边界分片、两种标签、正文标签、未闭合尾部及幂等 flush，供后续统一验证。

按本次用户覆盖要求，未运行任何 Cargo 命令、单元测试、实机或最终回归；没有红绿灯、退出码或有效测试数证据。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。仅作源码审阅及局部 rustfmt；macOS 行为待统一验证，Windows/Linux/WSL 范围外、未验证。


## 任务63：xAI 原生 Responses 差异（代码已交付，未验证）

来源为冻结 upstream 的 transform_codex_responses_xai_sanitize.rs 请求清理函数及 codex.rs 的原生目标门控；直接依赖任务62已有 late_arguments 和常驻 dispatch 接线。公共传输、可信作用域、配置发布和托管账号仍沿用既有所有者；仅确认源码契约存在，不推定母任务验收通过。

新增 model_relay::xai_responses，仅在常驻 native_responses 分支、覆盖参数与目标模型确定后调用 prepare，之后仍执行可信引用检查。以授权 URL 的精确 api.x.ai 主机和 Responses 路径为门控，避免来源 contains 判断误命中相似主机。按冻结来源清理顶层私有字段、include 标记、grok-4.5 采样字段、递归 external_web_access，提升 additional_tools、清理 reasoning null content、过滤不支持工具及失效工具选择、兼容函数参数根 schema。协作消息、模型目录重映射、返回参数改写不属于本次清理行为。普通 Responses 与旧 session relay 不进入此层，无凭据读取或工具执行。

新增 cc_route_063_ 测试源码覆盖清理、幂等、工具选择保留、schema 和非目标隔离，未执行。按用户覆盖要求未运行 Cargo、单元测试、实机测试或最终回归，没有通过证据；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务文件按要求删除；后续统一验证本项用例及 macOS 常驻入口，Windows/Linux/WSL 范围外、未验证。

## 任务65：限定签名与不透明状态纠错（代码交付，未验证）

2026-11-14：按用户覆盖要求不执行 Cargo、单元测试、实机测试和最终回归；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务64已有推理选项规范化提交 bf91f2376，公共传输、可信作用域、配置与账号接口沿用现有源码合同，不推定前置实机验收完成。

读取冻结 upstream 的 thinking_rectifier.rs、opaque_state_rectifier.rs 与当前 dispatch/传输调用链。新增 signature_rectify，仅对明确签名或不透明状态拒绝修改可修复历史；不采用来源对泛化 invalid request 的宽泛匹配。工具参数与 call_id 保留，当前加密 agent 任务返回 unreadable_encrypted_agent_task。普通常驻分发在同一授权 URL、凭据、出口、取消令牌和原超时内最多重发一次；仅检查 HTTP 400/422/502 的有界解码错误体，成功 SSE 和流内错误均不重放。旧 session relay 不接入。

故障转移路径由既有 RetryBudget 独立所有，本次未叠加纠错发送，后续需统一其请求级预算后补齐该路径。未宣称本项全行为完成。新增 cc_route_065_ 测试源码供统一验证，尚未执行。macOS 待验证，Windows/Linux/WSL 范围外、未验证。按用户覆盖要求删除编号任务文件。


## 任务66：拦截安全分类器循环（代码交付，未验证）

依赖任务65提交 `7f60f71b1` 的纠错入口已存在；来源采用母任务1冻结 upstream 的 proxy/classifier_guard.rs，直接调用者为 proxy/forwarder.rs。授权监听器、执行主机、JsonTarget 与配置/账号所有者沿用既有合同；仅确认源码依赖存在，不推定母任务已验收。

新增 routing::classifier_guard 唯一纯策略，逐字沿用来源两个错误子串、tools 非空门控和三次 assistant 分隔的失败工具回合阈值；并行工具结果只计一个回合，普通错误、成功/混合结果及新用户输入解除匹配。dispatch::forward_json 仅对 Claude/ClaudeDesktop 原始 Messages 历史调用，在协议转换、覆盖及 failover prepare 前返回 ClassifierLoop（HTTP 400），明确未发送上游、修复模型映射或余额后发送新用户消息；工具权限检查仍必需。不增加重试或故障转移策略，不涉及旧会话路径。

独立注册五个 cc_route_066_ 测试源码，覆盖阈值、重置、无工具、普通错误、并行结果和文本块。遵照用户最新要求，未运行 Cargo 相关命令/脚本、单元测试、实机测试或最终回归，无通过证据；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 不适用。后续统一验证需执行策略测试并补授权 HTTP 回环零上游发送验收。任务卡按要求删除；macOS 未验收，Windows/Linux/WSL 范围外且未验证，不表示母任务8完成。

## 任务67：限定思考预算纠错（代码交付，未验证）

2026-11-14：直接依赖任务66提交26bea1482已有分类器拦截源码。母任务来源快照、授权运行时、主机/应用、原生配置和账号接口沿用既有合同，仅确认源码可用，不推定实机前置验收通过。读取冻结 upstream thinking_budget_rectifier.rs 与当前 dispatch 调用者。

routing::budget_rectify::repair 仅匹配 HTTP 400 的 JSON error.message/message 中明确 thinking、budget_tokens/budget tokens 与 1024 最小值约束；跳过 adaptive、非对象、不可解析错误和无需修改的 body。沿用来源 budget=32000、max_tokens不足32001时升至64000，保留较大上限及其余字段。dispatch::forward_rectified 仅在 Messages 端点调用，与签名修复共享一次重发额度，保留原授权目标、凭据、出口、取消和外层超时；二次拒绝直接返回，不新增结果记录点，沿用外层响应处理。启用独立故障转移队列时仍由 failover_execution 所有，不在本项扩大其发送预算。

遵照用户覆盖要求，未运行 Cargo、相关脚本、单元测试、实机测试或最终回归，无验证通过证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一验证需覆盖预算错误、其他400、adaptive、无效body、无变化、单次重发及最终结果记录。任务卡按要求删除；macOS未验收，其他平台范围外，不表示母任务8完成。

## 任务68：处理文本模型媒体拒绝（代码交付，未验证）

2026-11-14：任务67的 budget_rectify 与单次修复入口已存在，沿用母任务来源快照、授权运行时、执行主机/应用和配置/账号合同，仅确认源码依赖，未推定实机验收完成。读取固定 upstream media_sanitizer.rs、tool_media.rs 及当前真实 dispatch 调用链。

新增 routing::media_sanitize，复用 models::capabilities 的显式声明并关闭模型名称启发式；目标模型与覆盖确定后，由 PublishedProvider 提供同部署设置进行主动图片替换。未知/支持图片能力保留原图。Messages/Chat、Responses、Gemini 图片以 [Unsupported Image] 留痕，保留缓存控制和工具身份；工具内容支持 JSON 字符串、typed image 与整段 image data URL，遍历上限32，不遍历工具参数/schema，不下载媒体。非图片文件和音频不在本项范围。

forward_rectified 对指定 HTTP 400/404/415/422/501 媒体拒绝才尝试替换，并要求实际发生替换；404仅接受明确 no endpoints found that support image input。与签名/预算修复共享一次重发额度，第二次错误直接返回。沿用同授权目标、凭据、出口、取消与超时；独立故障转移队列沿用既有所有者，不在此扩展。成功 SSE、流内拒绝、旧会话路径不进入媒体重放。

按用户最新覆盖要求，仅写代码，未新增或执行测试，未运行 Cargo、实机测试或最终回归；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR不适用。没有红绿灯、退出码或有效测试数证据。后续统一验证需覆盖能力声明优先级、图片/工具输出/缓存标记、无图与非图片保留、指定错误及二次拒绝不重试、真实HTTP执行链。仅macOS待验收，其余平台未验证；按要求删除编号任务文件，不代表母任务8验收完成。


## 任务69：处理工具回传多模态（代码交付，未验证）

2026-11-14：直接依赖任务68的媒体策略源码已存在；沿用冻结 upstream tool_media.rs/transform_responses.rs 的识别、32层遍历边界与8KiB data URL、16KiB残留base64阈值。仅确认母任务来源、常驻运行时、可信作用域与配置/账号接口的源码合同，不推定外部前置已实机验收。

新增 tool_media 与 tool_media_request，由常驻 Messages/Responses 请求所有者提取工具媒体并清除重复文本载荷。Chat在连续工具结果之后追加用户媒体，避免打断并行工具回合；Messages/Responses/Gemini通过请求局部占位恢复到原生内容位置，不改变共享严格转换器。文件引用仅传递，不访问本机文件；Gemini只接受内嵌数据，无法映射的格式明确拒绝。dispatch已调用这些常驻转换所有者，无需增加重复分发；旧会话入口不接入。

按用户最新要求未新增或运行测试，未运行Cargo、实机测试或最终回归。仅完成代码差异审阅及修改文件格式化；无验证通过证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR不适用。后续统一验证需覆盖图片/文件、JSON字符串、重复base64限长、纯文本稳定、并行工具回合和协议真实HTTP链路。macOS待验收，其他平台未验证，不代表母任务8完成；编号任务文件按要求删除。

## 任务70：注入有界缓存提示（代码交付，未验证）

2026-11-14：任务69工具媒体转换源码已存在，沿用冻结 upstream cache_injector.rs、providers/codex.rs 及既有授权运行时/主机/应用、配置与账号接口；仅核对源码依赖，不声明外部实机前置验收完成。

requestOptions.cacheInjection 显式启用（默认关闭），复用现有请求选项保存/合并 API；false 停止自动注入，保留用户缓存字段。dispatch::apply_cache 与 failover_execution 的每个候选出口消费同一 cache_policy。Messages 保留已有断点和 TTL，按 tools/system/latest/prior-user 顺序最多补足四个，跳过 thinking。OpenAI 与 Kimi coding 的 Chat/Responses 仅在真实 session_id 存在且未提供 prompt_cache_key 时生成 64 位 SHA256 身份，输入包含授权项目/主机/应用/账号/目标/凭据代次；未知上游不注入 key。不持有数据库或记录身份载荷。

新增 cc_route_070_ 独立测试源码覆盖禁用、幂等、已有五断点/TTL、thinking 和账号隔离。遵照本次用户覆盖指令，未运行 Cargo、单元测试、实机或最终回归；无槽位领取，CARGO_TARGET_DIR 未设置。推荐统一验证这些策略测试与真实 HTTP 出口；macOS 尚未验收，其它平台范围外。母任务8未作完成认定。

## 任务72：Codex OAuth 路由传输（代码交付，未验证）

2026-11-14：依赖任务71的 copilot_transport 与 dispatch 接线已存在；已读取固定 upstream 的 codex.rs、codex_oauth_auth.rs/forwarder 及现有账号、运行时、可信作用域合同。仅确认源码接口可用，不推定母任务1–6的外部验收完成。

新增 codex_oauth_transport 官方出口检查与最终身份头检查；ManagedCredential::resolve 在解析凭据前拒绝第三方地址、非 HTTPS、查询/片段及非 Responses 端点。dispatch 消费每次 resolve 的凭据包，拒绝空白账号和无效 Bearer，覆盖客户端/提供商的账号与官方 originator/version。过期刷新、撤销/账号代次检查继续复用 AuthAccountStore::codex_access_token 与 CodexRefresh；不新建 token 缓存或授权流程，不修改旧 session relay。仅支持官方 responses 与 responses/compact 出口，不增加其他协议转换。

遵照用户本次覆盖要求，仅写代码，未运行 Cargo、单元测试、实机测试或最终回归，未新增测试；无红绿灯或验证通过证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一验证应覆盖官方账号头、覆盖优先级、空白账号、过期/撤销后的连续请求及第三方 URL 拒绝。macOS 待验收，其它平台未验证；任务卡按要求删除，不代表母任务8完成。

## 任务75：Gemini OAuth 路由传输（代码交付，未验证）

2026-11-14：直接依赖任务74的 xai_oauth_transport 与 dispatch 接线已存在（abae6a0e3）。已读取冻结 upstream proxy/providers/gemini.rs，以及现有公共传输、可信应用/主机授权、配置发布与 AuthAccountStore::gemini_access_token / GeminiRefresh 接口。仅确认源码依赖可用，不推定母任务1–6外部前置已经实机验收。

新增 gemini_oauth_transport：来源为托管 GeminiCli 时沿用每次 ManagedCredential::resolve 取得的 Bearer，刷新只通过母任务6账号服务；解析凭据前和最终发送前均限制官方 HTTPS generativelanguage.googleapis.com、443、v1/v1beta models 生成端点，查询仅允许流式 alt=sse。最终清除混用的 API key、Cookie 与代理认证，固定 GeminiCLI 客户端头。静态 Gemini 生成端点使用发布的 API key，原生及 Messages 桥接均在覆盖字段处理后统一设置 x-goog-api-key，不按密钥前缀猜测 OAuth，不新增刷新缓存、调度或协议转换器。旧会话 relay 不接入。

遵照用户本次覆盖指令，不运行 Cargo、相关脚本、单元测试、实机或最终回归；未新增测试，无验证通过证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一验证需覆盖 API key / OAuth 来源选择、覆盖头冲突、原生与桥接 JSON/SSE、过期与撤销账号、刷新失败及第三方出口拒绝。macOS 待验收，其它平台范围外；任务文件按要求删除，不代表母任务8完成。

## 任务76：转发 Responses 压缩端点（代码交付，未验证）

2026-11-14：任务75 Gemini OAuth 接线已存在（c744785cb）；来源快照、公共传输、可信应用/执行主机作用域、配置与账号接口源码可用，不推定外部前置实机验收完成。读取冻结 upstream handlers.rs、server.rs、providers/codex_compaction.rs 与当前 dispatch 调用链。

新增 model_relay::compaction，识别 /responses/compact、/v1/responses/compact、/v1/v1/responses/compact、/codex/v1/responses/compact、/grokbuild/v1/responses/compact；dispatch 从同一已选原生 Responses 目标派生 compact URL，复用模型映射、凭据解析、出口及作用域校验。Chat/Messages/Gemini 目标明确拒绝，不伪造摘要。压缩只接受非流式请求（包括覆盖后的最终请求），响应复用有界解压和容量释放，保留原生 JSON 内容与 HTTP 状态，仅成功响应登记引用。压缩不进入签名自动修复重放，避免改写原生压缩状态；旧会话 relay 不变。

按本次用户覆盖要求只交付代码，不新增或运行测试，不运行 Cargo/相关脚本、实机或最终回归。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置，无验证通过证据。后续统一验证需覆盖五个别名、原生及 OAuth 目标路径/身份、模型映射、非支持 provider 拒绝、覆盖后 stream 拒绝、引用作用域、HTTP 错误与原生 compaction 密文/状态保留。macOS 待验收，其它平台范围外。编号任务文件按要求删除，不代表母任务8完成。


### 任务78：V2压缩请求与流（源码已实现，未验证）

2026-11-14：任务77的 compaction_replay、公共传输及可信作用域代码已存在；来源采用冻结 upstream codex_compaction.rs 和 handlers.rs。仅确认源码前置，不推定外部实机验收通过。常驻 dispatch 识别 compaction_trigger，第三方原生 Responses 生成禁用工具选择的摘要请求；Chat 复用既有摘要流，Anthropic 复用既有转换后生成可重放 compaction 条目。官方目标继续原生协议。incomplete 保留原原因，空摘要失败；流沿用取消令牌及上游 Body 生命周期，不进入本地模型或工具循环。新增 cc_route_078_ 测试源码。

按用户本次覆盖要求，未运行 Cargo、单元测试、实机测试或最终回归，未领取槽位，CARGO_TARGET_DIR 未设置；无退出码或通过测试数。下一步统一验证新增请求、流、取消和授权回环场景。

## 任务 80　Gemini 元数据与计数（代码已写，统一验证待执行）

2026-11-14：复用任务79发现入口及任务2/3授权传输底座；来源参考 `proxy/handlers.rs::handle_gemini` 的无请求体 GET 与 URI 动作转发。新增 `model_relay/gemini_metadata.rs`，常驻 dispatch 单独处理 GET 列表/单模型和 POST countTokens，保留可信来源版本及前缀，列表仅放行 pageSize/pageToken；拒绝未知动作、错误方法与客户端凭据查询。计数 JSON 原样转发，不进入生成、工具、缓存注入或故障转移。Messages count_tokens 来源无路由，明确返回501，不估算 token。静态 API key 与托管 Gemini OAuth 分别沿用现有认证；OAuth 的官方主机约束保留。

已添加独立 `cc_route_080_` 用例，但按用户本次覆盖要求未运行 Cargo、单元测试、实机或最终回归；无通过证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一验证需覆盖真实回环转发、认证、压缩、取消及方法边界；macOS 尚未验收，其它平台范围外。主计划未修改。

## 任务 81　独立 Alpha 搜索（代码已写，统一验证待执行）

2026-11-14：直接依赖任务80提交 `51fa25341` 的元数据分发与既有授权监听器可用；读取固定 upstream handlers.rs 的 handle_alpha_search、forwarder.rs 的相邻端点重写以及 server.rs 的四别名/完整URL场景，并核对原始来源 handlers.rs。母任务来源快照、运行时、执行主机/应用、配置发布与账号接口仅确认源码合同存在，不推定外部前置已实机验收。

新增 alpha_search 端点策略，常驻 Codex 分发四个来源别名到所选 Responses/Responses compact URL 的相邻 alpha/search。保留前缀大小写、转义路径、配置查询和客户端查询；不确定的目标返回422。JSON body 解压后原样发送，沿用可信目标选择、应用/主机授权、容量、取消、时限及出口传输，不执行工具或生成转换/修复。保留上游JSON状态及载荷。静态凭据来自发布；托管账号仅支持 Codex OAuth，复用新鲜账号头并限制官方HTTPS搜索端点，其他托管类型明确拒绝。旧会话relay与生成OAuth路径白名单不变。

按用户最新覆盖要求只写代码，未新增或运行测试，未运行Cargo命令/脚本、实机或最终回归；无红绿灯、退出码、有效测试数或通过证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR未设置。后续统一验证需覆盖四别名、完整URL/compact/大小写与转义前缀、query/body保持、opaque目标拒绝、应用/主机/模型权限、刷新及撤销、压缩/取消/超时和上游错误状态。仅macOS待验收，其它平台范围外；按要求删除编号任务文件，不表示母任务8验收完成。主计划只读未修改。

## 任务82：图片生成转发（代码已写，统一验证待执行）

2026-11-14：核对冻结 upstream handlers.rs/server.rs 的 Images generations 转发及四个 Codex 别名，复用任务81独立转发所有者、公共授权/容量/取消/出口传输。来源快照、运行时、应用/主机与配置/账号仅确认源码合同存在，外部验收未确认。新增 image_generation 端点策略：四别名归一到所选 Responses、Chat 或完整 Images URL 的相邻 images/generations，保留前缀及查询，未知目标返回422。JSON请求解压后保持原样，不进入工具或生成协议转换。响应保留上游状态，按JSON、常见图片或二进制类型有界解压收集，清除上游敏感及失效编码/长度头。托管凭据仅允许官方 Codex OAuth 图片端点，旧会话白名单不变。

按用户最新覆盖要求未新增或运行单元测试，未运行任何Cargo命令/脚本、实机测试或最终回归；无验证通过证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR未设置。后续统一验证四别名、完整URL、查询/请求保持、JSON与二进制类型、压缩前后大小限制、取消/超时、错误状态及授权撤销；仅macOS待验收。按要求删除编号任务卡，主计划只读未修改。
