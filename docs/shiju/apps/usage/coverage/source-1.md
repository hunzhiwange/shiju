# 额度来源核对第 1 组

核对 CC Switch 4.0.25 快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 7 项主责来源（6 项 Rust、1 项图稿提示词）。7 项内容的 SHA-256 均与原始清单一致，6 项 Rust 与仓内参考快照逐字节一致。以下任务编号属于余额套餐额度与查询脚本计划，「母任务」编号属于 CC 能力完整融合计划。[来源清单](../../../../../.plans/2026-11-05-CC能力完整融合/来源清单.json) · [命令与事件覆盖](../../../../../.plans/2026-11-05-CC能力完整融合/命令与事件覆盖.md) · [来源覆盖索引](../../../../../.plans/2026-11-05-余额套餐额度与查询脚本/来源覆盖索引.md)

## 1. Claude 官方订阅文章封面

图稿提示词指定文章标题、账号切换卡片、终端图示和宣传边界。提及用量展示仅用于说明文章主题，正文没有查询输入、输出、错误或可执行规则。任务 59 负责归档为说明资源，封面和品牌文案无需移植。[封面提示词](https://github.com/farion1231/cc-switch/blob/ca58c0313360eae7c851fbd7553c1614c7af03e7/docs/images/claude-official-accounts/prompts/01-cover-claude-subscriptions.md#L15-L44)

## 2. 余额命令 `get_balance`

Tauri 命令接收 `base_url`、`api_key`，TS 调用字段为 `baseUrl`、`apiKey`。命令直接返回余额服务的 `Result<UsageResult, String>`，不写缓存、不发事件。供应商卡片另由余额模板分支解析该应用的供应商凭据，调用同一服务并向上传播传输错误。任务 44、45 分别负责快照读取和刷新边界，任务 47、48 负责对应 SDK，任务 11–16 负责服务查询，任务 55 负责 macOS 余额账号验收。[命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/balance.rs#L1-L6) · [TS 调用](../../../../../crates/vw-agent-apps/upstream/src/lib/api/subscription.ts#L43-L47) · [供应商余额模板](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L821-L829)

## 3. 套餐命令 `get_coding_plan_quota`

命令接收基础 URL 与 API key，也接受可选火山控制面 AK/SK、套餐供应商标识、团队组织和项目 ID。TS 调用对应的 camelCase 字段。返回 `SubscriptionQuota` 或传输错误。缺凭据、未知供应商、鉴权、HTTP 或业务失败由服务形成确定性失败快照。团队版以显式 `zhipu_team` 区分同域名的个人版，三项凭据缺一返回未找到，且团队无套餐不查余额。火山分支要求 AK/SK。[命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/coding_plan.rs#L3-L25) · [TS 调用](../../../../../crates/vw-agent-apps/upstream/src/lib/api/subscription.ts#L23-L42) · [服务分派](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1812-L1918)

独立套餐命令将个人版的 `ZhipuNoPlan` 折叠回套餐失败结果。供应商用量入口保留这个分支，只有明确无个人套餐才改查余额。任务 10 负责能力识别，任务 16 负责回落，任务 17–25 负责各套餐适配，任务 51 负责配置，任务 56 负责 macOS 套餐账号验收。网关和 SDK 分别由任务 44–45、47–48 交付。[套餐结果折叠](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L136-L150) · [供应商回落分支](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L729-L759)

## 4. 官方订阅命令 `get_subscription_quota`

TS 传 `tool`，命令使用本地 CLI 既有登录，`agy` 有账号状态时调用默认账号查询。返回 `SubscriptionQuota`，包含凭据状态、额度档、可选余额与重置信息、错误和查询时间。仅在返回 `Ok(snapshot)` 且 `tool` 能解析为 `AppType` 时发出 `usage-cache-updated`（`kind: "subscription"`、`appType`、`data`），随后写 `UsageCache` 并调度托盘刷新。发事件失败只记录日志，缓存仍更新。TS 监听器用 `subscriptionKeys.quota(appType)` 写入查询缓存。确定失败快照覆盖旧值并清掉旧托盘后缀，传输 `Err` 不写缓存也不发事件。[命令与事件](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/subscription.rs#L8-L45) · [快照结构](https://github.com/farion1231/cc-switch/blob/ca58c0313360eae7c851fbd7553c1614c7af03e7/src-tauri/src/services/subscription.rs#L68-L83) · [TS 调用](../../../../../crates/vw-agent-apps/upstream/src/lib/api/subscription.ts#L8-L10) · [TS 缓存监听](../../../../../crates/vw-agent-apps/upstream/src/hooks/useUsageCacheBridge.ts#L9-L42)

任务 26–30 负责订阅适配，任务 39 负责保留短期成功缓存，任务 40 负责刷新调度，任务 44 负责读取快照，任务 45 负责刷新、取消与受权事件，任务 53 负责客户端联动。任务 57 验收 macOS 官方账号，任务 59 核对三条命令和等价事件。上游命令本身只有查询及事件副作用，取消来自后续任务的网关合同。

## 5. Codex 官方额度保护

保护层在官方 POST 完成鉴权后接收 bearer 和可选工作区 ID，按 JWT `sub`（解析不到则用 token）与工作区组合哈希区分账号，仅用于缓存。官方响应的用量头（含错误响应）更新下次检查的数据。优先读 active-limit 前缀，再考虑 `x-codex-*`，按 300 或 10080 分钟识别五小时或周窗，读取已用百分比与重置时间。没有可解析的用量百分比头时保留原快照。没有新鲜快照（300 秒或任一窗口已重置）才用同账号查 `wham/usage`，等待上限 5 秒。额度快照识别 `five_hour`、`seven_day`、`weekly_limit`，跳过负数和非有限值，缺失或无法解析的重置时间记为未知。并发查询期间已写入的新鲜快照优先。[读取与缓存](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/proxy/codex_quota_protection.rs#L32-L278) · [查询与响应观测](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/proxy/codex_quota_protection.rs#L292-L316) · [官方请求调用点](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/proxy/forwarder.rs#L1463-L1477) · [官方响应调用点](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/proxy/forwarder.rs#L2844-L2848)

任一窗口达到阈值时返回本地 `CodexQuotaProtection`（HTTP 403）。禁用、无效配置或未知用量时放行，查询失败也短期缓存空读数。默认关闭，两窗口默认 95% 和 98%，配置仅接受 1–99 的整数。任务 27 独占官方额度查询和窗口数据输出，任务 7 固化交接边界，任务 59 核对交接。母任务 8 独占请求拦截、响应观测、保护缓存和阻断验收。母任务 20 独占配置验证、持久化与设置界面验收。[保护行为测试](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/proxy/codex_quota_protection.rs#L450-L700) · [状态码映射](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/proxy/error.rs#L161-L165) · [配置默认值与约束](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/settings.rs#L381-L412)

## 6. AGY 账号额度服务

`fetch(access_token)` 带 bearer 向 Google `retrieveUserQuotaSummary` 发 POST，单次请求超时 20 秒，读取完成后检查 256 KiB 上限。UA 标识应用及本机 AGY CLI 版本，进程内只探测一次，探测超时 5 秒，失败使用上游快照固定版本 `1.4.6`。解析 `groups[].buckets[]` 的 ID、剩余比例、重置时间，与 CLI `/usage` 共用 `collect_tiers`，识别 Gemini 及第三方五小时/周窗，剩余比例裁剪到 0–1 再转为已用百分比，无有效档时为解析失败。[请求、版本探测和错误](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/agy_account/quota.rs#L23-L219) · [共用档位映射](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription_agy.rs#L48-L111)

401 返回 `Unauthorized`，账号层尝试续期后再查一次。403 调用 `loadCodeAssist`，仅无当前档且有资格原因时归类 `Ineligible`，其余按 HTTP 错误处理。网络与解析错误在服务层有独立类型，账号层会把多数错误折叠成失败快照，任务 7、30 需明确临时网络失败的缓存语义。macOS 指定账号时直接使用该账号令牌，未指定时查当前登录，识别不到已收录账号才回落 CLI `/usage`。任务 30 独占查询及回落，任务 9 负责账号定位，母任务 6 负责凭据取得和续期，任务 57 负责 macOS 实号验收。[403 资格原因](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/agy_account/quota.rs#L221-L239) · [账号续期](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/agy_account/mod.rs#L386-L409) · [命令与回落](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/agy_account.rs#L20-L69)

## 7. 供应商余额服务

`get_balance` 根据基础 URL 识别供应商，用固定官方地址查询。供应商请求超时为 15 秒。余额以 `UsageResult.data` 输出，数字字段兼容 JSON 数字和数字字符串。发送及成功响应读体中断返回传输错误。空 key、未知供应商、401/403、其他非 2xx、完整响应体解析失败等返回确定性失败，401/403 附无效凭据行。余额为零或负数与凭据有效性分别表达。[入口及数值解析](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L533-L573) · [识别及错误结构](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L18-L69)

| 查询 | 固定端点、输出与边界 | 实现所有者 |
| --- | --- | --- |
| DeepSeek | `api.deepseek.com/user/balance`。以每币种 `total_balance` 为剩余，至少一项为正时隐藏已知非正项，`is_available` 不决定凭据有效性。快照读取总余额，没有单独投影赠金或充值额。[解析](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L71-L151) | 任务 11 |
| StepFun | `api.stepfun.com/v1/accounts`，识别 `.ai` 与 `.com` 基础域名。以 `balance` 输出 CNY，缺失或不可解析时上游默认零。[查询](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L153-L209) | 任务 12 |
| SiliconFlow | `api.siliconflow.cn` 或 `.com` 的 `/v1/user/info`，分别输出 CNY 或 USD。缺 `data` 为失败，缺失或不可解析的 `totalBalance` 上游默认零。[查询](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L211-L286) | 任务 13 |
| OpenRouter | `openrouter.ai/api/v1/credits`，USD 余额为 `total_credits - total_usage`，保留总额与用量。缺失或非有限金额为失败，key 的消费上限不参与余额计算。[查询](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L288-L362) | 任务 14 |
| Novita | `api.novita.ai/v3/user/balance`，`availableBalance / 10000` 得 USD。缺失或非有限值为失败。[查询](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L364-L433) | 任务 15 |
| 智谱个人无套餐回落 | `open.bigmodel.cn` 或 `api.z.ai` 的 `/api/biz/account/query-customer-account-report`，Authorization 为裸 key。优先 `availableBalance`，缺失或不可解析时取 `balance`，业务失败或无有效金额为确定失败。上游记录国内 CNY 实测，国际 USD 与响应结构未实测。[查询及解析](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L435-L529) | 任务 16 |

任务 43 负责余额与失败状态投影，任务 55 验收 macOS 余额账号。DeepSeek 的赠金字段列在上游响应注释中，任务 11 需区分上游已观察的总钱包行为与本计划要求的赠金展示。[余额边界测试](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L575-L772)

## 合同与验收交接

- `vw-agent-apps` 已注册，但其原生模块树尚无 `usage`，任务 7 的 `docs/shiju/apps/usage/contract.md` 尚未交付。任务 7 对齐母任务 1–6 的实际交付，尤其核对母任务 2 的主机执行、任务/事件及权限，母任务 4 的供应商配置和母任务 6 的账号绑定及凭据引用。任务 8–9 落实 DTO 与定位，任务 44–48 落实网关和 SDK。[工作区成员](../../../../../Cargo.toml#L1-L21) · [原生模块](../../../../../crates/vw-agent-apps/src/lib.rs#L1-L14)
- 客户端只传身份与凭据引用。查询执行端解析授权凭据及允许的目标主机，公开结果和事件不带密钥或原始上游错误载荷。按结构化错误区分临时失败与确定失败，缓存隔离应用、账号、主机及凭据版本。上游域名子串匹配、IPC 明文凭据参数与响应原文错误仅作为来源证据。
- 本组验证限文档与固定快照。任务 54–58 分别验收 macOS 网关链路、余额账号、套餐账号、官方订阅账号和原生界面。任务 59 汇总覆盖与验收证据。
