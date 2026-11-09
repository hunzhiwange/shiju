# 额度来源核对第 3 组

核对 CC Switch 4.0.25 固定快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 7 项 TSX 主责来源。7 项均与原始清单的 SHA-256 和仓内参考副本一致，均有业务行为。以下任务编号属于余额套餐额度与查询脚本计划。[来源清单](../../../../../.plans/2026-11-05-CC能力完整融合/来源清单.json) · [命令与事件覆盖](../../../../../.plans/2026-11-05-CC能力完整融合/命令与事件覆盖.md) · [来源覆盖索引](../../../../../.plans/2026-11-05-余额套餐额度与查询脚本/来源覆盖索引.md)

## 1. Codex OAuth 授权中心账号额度

`CodexOauthAccountQuota` 输入托管 ChatGPT `accountId` 和可选登录名；授权中心仅在开启账号额度且账号不需重新授权时挂载。挂载时启用查询、不启用轮询，绑定同一显式账号的供应商卡与账号行共用查询键 `["codex_oauth", "quota", accountId]`。账号行显示各档、相对查询时间与带账号名的刷新按钮，刷新中防重复点击。无结果时可显示加载，缺凭据或解析失败隐藏额度行，过期、待刷新与一般失败转为原因提示。任务 53 独占跨界面的账号联动与刷新，任务 43 负责失败投影。[账号组件](../../../../../crates/vw-agent-apps/upstream/src/components/CodexOauthAccountQuota.tsx#L10-L47) · [授权中心入口](../../../../../crates/vw-agent-apps/upstream/src/components/providers/forms/CodexOAuthSection.tsx#L432-L442) · [共享查询](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L126-L162) · [刷新与状态](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L166-L212) · [状态转换](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L274-L298)

## 2. Codex OAuth 供应商卡

`CodexOauthQuotaFooter` 输入供应商 `meta`、显示形态、激活状态和刷新间隔。由 `authBinding` 解析托管账号，缺显式绑定时传空给 `get_codex_oauth_quota` 查默认账号，无可用账号返回 `not_found`。即使卡未激活也启用读取；仅激活且间隔为正才轮询，默认 5 分钟、正间隔最少 1 分钟。已绑定的官方卡受用量开关控制并读取设置的间隔。结果、请求状态与手动刷新委托共享订阅视图。命令使用托管 token 和 ChatGPT 工作区 ID 复用官方查询；网络等瞬时失败返回 `Err`，确定性凭据失败返回 `expired`。`Ok` 结果按实际解析出的账号写托盘缓存，无此命令直接发出的额度事件。任务 9 独占账号解析，任务 27 独占 Codex 查询，任务 49 独占卡片接入。[卡片组件](../../../../../crates/vw-agent-apps/upstream/src/components/CodexOauthQuotaFooter.tsx#L21-L47) · [调用条件](../../../../../crates/vw-agent-apps/upstream/src/components/providers/ProviderCard.tsx#L534-L546) · [绑定](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L164-L177) · [命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/codex_oauth.rs#L21-L98)

## 3. Copilot 供应商卡

`CopilotQuotaFooter` 按 `meta` 解析托管账号，与账号行共用 `["copilot", "quota", accountId ?? "default"]`。显式账号走 `copilot_get_usage_for_account`，缺省走兼容命令 `copilot_get_usage`。激活卡每 5 分钟轮询并在窗口聚焦时重查。查询层取 `premium_interactions`，按 `(entitlement - remaining) / entitlement` 转为已用百分比，非正上限时源代码填 0，并携带套餐、重置日期与查询时间。卡片显示 Premium、套餐与进度；无结果或空档不渲染，失败快照仅紧凑卡显示错误与重试。当前 hook 失败是 reject，未构造失败快照或应用订阅的短期成功保留策略；首次 reject 时卡片隐藏，账号行可显示 `query.isError`。任务 31 独占查询及 Premium 数据口径，任务 43 负责结构化失败投影，任务 49/53 分别负责卡片/账号行接入。[卡片组件](../../../../../crates/vw-agent-apps/upstream/src/components/CopilotQuotaFooter.tsx#L18-L80) · [查询转换](../../../../../crates/vw-agent-apps/upstream/src/lib/query/copilot.ts#L22-L62) · [命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/copilot.rs#L201-L221) · [账号行](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L300-L349)

## 4. Kimi OAuth 账号卡

`KimiAccountQuotaFooter` 以供应商 ID 作为账号 ID，和授权中心共用 `["kimi_oauth", "quota", accountId]`。始终允许读取，激活卡默认 5 分钟轮询；无本机账号或凭据返回 `not_found`，需要先在 Kimi Code 使用时返回 `refresh_pending`，需重登时为 `expired`。`kimi_oauth_quota` 查指定账号并复用编程套餐窗口解析；成功无档时卡片与账号行显示未订阅，区分查询失败。页脚另读五分钟缓存的 OAuth 状态，发现同账号昵称但卡名尚未包含时使 `['providers', 'kimi']` 失效。任务 32 独占账号额度查询，母任务 6 持有身份、凭据及续期，任务 53 负责账号与供应商列表联动。[卡片组件](../../../../../crates/vw-agent-apps/upstream/src/components/KimiAccountQuotaFooter.tsx#L23-L68) · [卡片调用](../../../../../crates/vw-agent-apps/upstream/src/components/providers/ProviderCard.tsx#L553-L560) · [查询键](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L248-L277) · [命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/kimi_oauth.rs#L58-L64) · [账号查询](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/kimi_oauth/account.rs#L394-L448) · [账号行](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L380-L416)

## 5. 共享订阅视图和 CLI 页脚

`SubscriptionQuotaFooter` 输入 `appId`、形态、激活状态及间隔，仅激活时启用并显示 CLI 路径。hook 允许 Claude、Codex、Gemini、Grok Build、AGY；卡片调用方未配置间隔时传 0（只禁轮询，仍允许读取）。`get_subscription_quota` 按工具查询；`Ok` 结果按 `appType` 缓存、更新托盘并发 `usage-cache-updated`（`kind=subscription`、`appType`、`data`），桥接器写入对应 CLI 查询键；`Err` 不发事件、不覆盖托盘。任务 26–30 分别负责各工具查询，任务 44/45 分别负责快照读取与刷新/取消接口，任务 53 负责桌面刷新事件。[CLI 页脚](../../../../../crates/vw-agent-apps/upstream/src/components/SubscriptionQuotaFooter.tsx#L358-L393) · [调用条件](../../../../../crates/vw-agent-apps/upstream/src/components/providers/ProviderCard.tsx#L572-L593) · [查询规则](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L89-L117) · [命令事件](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/subscription.rs#L8-L45) · [缓存桥接](../../../../../crates/vw-agent-apps/upstream/src/hooks/useUsageCacheBridge.ts#L22-L43)

| 实现所有者 | `SubscriptionQuotaView` 的输入、输出与边界 |
| --- | --- |
| 任务 41 · 多窗口投影 | 根据已知档位表生成行；紧凑卡隐藏 Sonnet 周窗，AGY 的 Gemini、第三方分别保留已用最高的一档，平手保留先遇到的一档。合并时提供逐档明细，完整视图不合并。显示偏好默认剩余，可切换为已用；未知档在此视图未渲染。[窗口规则](../../../../../crates/vw-agent-apps/upstream/src/components/SubscriptionQuotaFooter.tsx#L47-L164) · [显示偏好](../../../../../crates/vw-agent-apps/upstream/src/components/quota/useQuotaDisplay.ts#L1-L8) |
| 任务 42 · Credits 与重置次数 | 无可显示窗口时不单列附加项。紧凑卡仅在可见档用尽时单列 Credits，且排在重置次数前；若有重置明细，可附 Credits 行。展开视图按重置次数、Credits 排列。上游按固定系数显示近似美元，属于估算。[行组合](../../../../../crates/vw-agent-apps/upstream/src/components/SubscriptionQuotaFooter.tsx#L166-L204) · [Credits 转换](../../../../../crates/vw-agent-apps/upstream/src/components/quota/quotaRules.ts#L260-L317) |
| 任务 43 · 失败、空档与金额 | 无快照、`not_found`、`parse_error` 静默隐藏；失败提供重试，分别显示过期、待刷新或查询错误。展开的过期提示按调用方选择 CLI 或授权中心，Grok Build 用 `grok`，Kimi 传 `kimi`。成功无档时只在提供 `emptyText` 时显示提示并保留刷新。成功展开时启用的 `extraUsage` 单列显示已用与可选月上限，源代码按两位小数且仅 USD 加 `$`。[状态与金额](../../../../../crates/vw-agent-apps/upstream/src/components/SubscriptionQuotaFooter.tsx#L206-L356) |

## 6. xAI OAuth 供应商卡

`XaiOauthQuotaFooter` 从 `meta` 解析显式托管账号，否则查询默认账号，输出经共享订阅视图显示，过期时提示在授权中心重新登录。卡片均启用读取，激活卡固定 5 分钟轮询并在聚焦时重查。授权中心以显式账号复用 `["xai_oauth", "quota", accountId]`，不单独轮询。`get_xai_oauth_quota` 取托管 token 并复用 Grok CLI 的账单查询协议；无账号为 `not_found`，取 token 失败上游统一标为 `expired`，额度查询的瞬时传输失败向上传播。共享的是协议，同一账号才共享对应快照。任务 29 独占协议适配，任务 49/53 分别负责卡片接入和账号联动。[卡片组件](../../../../../crates/vw-agent-apps/upstream/src/components/XaiOauthQuotaFooter.tsx#L21-L42) · [查询规则](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L179-L215) · [命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/xai_oauth.rs#L15-L75) · [账号行](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L351-L378)

## 7. 额度展开明细

`QuotaBreakdownRow` 接收标题、无障碍触发名称、可选汇总行、明细与可选页脚。点击行打开浮层并阻止事件冒泡到卡片；没有单一汇总行就只显示标题。明细逐行显示标签、可选提示、值与状态色，页脚分隔渲染；同一组件用于紧凑卡、展开条和授权中心。它不分组、不发额度请求、不处理查询错误；分组由任务 41（窗口）、42（重置）分别生成，余额与失败提示由任务 43 投影。上游重置规则重新过滤已过期项、按本地显示日期合并次数，无法解析的到期与无到期同组，距到期小于三天警示。任务 50 独占展开交互和呈现。[浮层和行](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaBreakdown.tsx#L23-L117) · [紧凑列调用](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L411-L468) · [展开条调用](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L719-L726) · [账号行调用](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L85-L116) · [重置次数](../../../../../crates/vw-agent-apps/upstream/src/components/quota/quotaRules.ts#L182-L257) · [到期分组](../../../../../crates/vw-agent-apps/upstream/src/components/quota/quotaRules.ts#L320-L350)

## 合同与验收交接

- 任务 7 的合同与 `vw-agent-apps` 原生额度模块尚未交付。接续时对齐母任务 2 的主机权限、任务和事件，母任务 4 的供应商设置与母任务 6 的授权引用。任务 9 要用实际解析的账号隔离缓存；默认账号变化后不能沿用旧账号的 `default` 快照。凭据只留在执行端，公开快照与事件用授权身份及脱敏状态。[原生模块树](../../../../../crates/vw-agent-apps/src/lib.rs#L1-L15)
- 任务 39 独占短期成功保留：上游订阅 hook 按身份丢弃旧快照，10 分钟内只掩盖瞬时错误；无可用旧快照时将 reject 转成可重试失败。Copilot 未接这条路径，需由合同约定。任务 40 独占服务调度，前端挂载、聚焦和轮询仅作来源证据；任务 53 订阅授权范围内的快照变化，处理绑定切换、迟到响应和断连后重读。[订阅缓存](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L29-L87) · [保留规则](../../../../../crates/vw-agent-apps/upstream/src/lib/query/queries.ts#L121-L235)
- 任务 57 验收 macOS 真实订阅账号，任务 58 验收 Iced 的卡片、展开明细、错误与重试、明暗主题及中文渲染，任务 59 汇总覆盖。此处只验证来源、链接与归属。
