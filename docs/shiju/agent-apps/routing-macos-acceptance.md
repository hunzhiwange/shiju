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
