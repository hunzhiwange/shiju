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
| 30 `scan_local_proxies` | POST `R/projects/{projectId}/egress/scans`，空 JSON 对象。仅本机 `127.0.0.1` 的 7890、7891、1080、8080、8888、3128、10808、10809，每端口至多 100 ms，操作总时限 2 秒 | 202 `{operationId}`。状态给出来源协议候选和开放端口，开放端口只代表 TCP 可连。429 达并发上限 | 诊断 `app_routing_egress_scan_start` |
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
