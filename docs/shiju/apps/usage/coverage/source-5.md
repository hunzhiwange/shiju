# 额度来源核对第 5 组

核对 CC Switch 4.0.25 固定快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。7 项均与原始清单 SHA-256 和仓内参考副本一致。5 项定义运行时行为或数据合同，2 项提供组件回归样例。下文编号属于余额套餐额度与查询脚本计划，另标注母任务归属。[来源清单](../../../../../.plans/2026-11-05-CC能力完整融合/来源清单.json) · [命令与事件覆盖](../../../../../.plans/2026-11-05-CC能力完整融合/命令与事件覆盖.md) · [来源覆盖索引](../../../../../.plans/2026-11-05-余额套餐额度与查询脚本/来源覆盖索引.md) · [第 4 组](source-4.md)

## 1. Coding Plan 检测与默认配置

`detectCodingPlanProvider` 输入 Base URL，按表顺序做不区分大小写的匹配，输出首个供应商 ID，无地址或未命中返回 `null`。

| ID | 匹配或选择规则 |
| --- | --- |
| `kimi` | `api.kimi.com/coding` |
| `zhipu` / `zhipu_team` | `bigmodel.cn` 或 `api.z.ai` 自动命中个人版；团队版与个人版共用地址，须显式选择 |
| `minimax` | `api.minimaxi.com`、`api.minimax.io`、`api.minimax.cn` 按主机名边界匹配 |
| `zenmux` | 前端匹配 `zenmux.`，仅 Claude 新建卡片自动预填 |
| `volcengine` | `volces.com/api/plan` 或 `volces.com/api/coding`，预填但默认关闭，额度请求另用 AK/SK |
| `opencode_go` | `opencode.ai/zen/go` 或 `opencode.ai/inference/go`，按量 Zen 路径不命中 |
| `command_code` | `api.commandcode.ai/provider` 后跟路径分隔符、查询、片段或串尾 |

识别仅用于能力和配置预选。Rust 对 ZenMux 和 Command Code 的子串判定比前端更宽，还区分智谱、MiniMax 的区域。任务 10 独占识别及回归规则，出站权限由任务 7 的合同约束。[匹配表与默认值](../../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L32-L113) · [后端识别](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L32-L79)

`extractBaseUrlForUsageDetection` 从 Claude 和 Claude Desktop 的 `env.ANTHROPIC_BASE_URL`、Gemini 的 `env.GOOGLE_GEMINI_BASE_URL`、Codex 和 Grok Build 的 TOML、OpenCode 和 MCode 的 `options.baseURL`、Pi、Oh My Pi 和 OpenClaw 的 `baseUrl`、Kimi 的 `provider.base_url`、Hermes 的 `base_url` 提取字符串。未知应用、缺配置或非字符串返回 `null`。新建入口保留已有 `meta.usage_script`，否则检测命中才填 `token_plan`，该模板走原生额度查询。导入预览用同一检测表读首个端点，并尊重链接中显式启用设置。任务 10 负责识别和预选，任务 51 负责配置交互及必填校验。[提取与预填](../../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L120-L206) · [新建调用](../../../../../crates/vw-agent-apps/upstream/src/hooks/useProviderActions.ts#L87-L92) · [导入预览](../../../../../crates/vw-agent-apps/upstream/src/components/DeepLinkImportDialog.tsx#L389-L406) · [套餐选择](../../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L1125-L1152)

## 2. 原生查询命令包装

`subscriptionApi` 直接返回 Tauri `invoke` 的 Promise。查询结果可含 `success: false`，命令 `Err` 使 Promise 拒绝；包装层不吞错，也不自行合并结果。下表归属为查询实现所有者。[全部包装](../../../../../crates/vw-agent-apps/upstream/src/lib/api/subscription.ts#L8-L48) · [通用命令注册](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/lib.rs#L1648-L1655) · [账号命令注册](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/lib.rs#L1941-L1957)

| 命令 | 输入、结果与实现所有者 |
| --- | --- |
| `get_subscription_quota` | `tool` → `SubscriptionQuota`。前端查询启用范围为 Claude、Codex、Gemini、Grok Build 和 AGY，分别由任务 26–30 持有。[前端门控](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L89-L117) · [命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/subscription.rs#L18-L45) |
| `get_codex_oauth_quota` / `get_xai_oauth_quota` | 可空 `accountId` → `SubscriptionQuota`。未指定用各自托管默认号，无账号返回 `not_found`。上游 xAI 将空白 ID 视为未指定，Codex 对显式 ID 仅去空白。任务 27、29 各自负责查询，任务 9 负责身份解析。[Codex](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/codex_oauth.rs#L28-L54) · [xAI](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/xai_oauth.rs#L26-L75) |
| `get_claude_account_quota` / `get_claude_default_plan` | 显式账号 ID → 带可选套餐的 `ClaudeAccountQuota`；无参读取 Claude Code 默认登录的套餐 → `ClaudeAccountPlan \| null`。两者归任务 26。[Claude](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/claude_account.rs#L25-L53) |
| `agy_account_quota` | 显式账号 ID → `SubscriptionQuota`，归任务 30。[AGY](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/agy_account.rs#L21-L45) |
| `get_coding_plan_quota` | `baseUrl`、`apiKey`、可选火山 AK/SK、可选套餐标识和智谱团队组织/项目 ID → `SubscriptionQuota`。显式团队版要求 Key、组织、项目齐全；未知供应商给失败快照。任务 17–25 分别持有各套餐及签名。[命令参数](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/coding_plan.rs#L3-L25) · [团队与未知供应商分支](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1858-L1885) |
| `get_balance` | `baseUrl`、`apiKey` → `UsageResult`，结果可含多条套餐数据或错误，任务 11–16 持有各余额适配。[命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/balance.rs#L1-L6) · [结果类型](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/provider.rs#L366-L372) |

上游包装会触发查询，配置弹窗也用它试运行。任务 44/47 持有网关/SDK 的脱敏快照读取，任务 45/48 持有刷新与取消，任务 52 持有试运行交互。任务 7 需明确读取、刷新与试运行边界，公开 DTO 使用授权后的引用，密钥留在执行端。[官方订阅、余额及套餐试运行](../../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L580-L702)

## 3. 托管账号绑定与删除影响

`findManagedAccountUsers` 输入授权类型、拟删除的账号 ID 集合、默认账号和供应商映射，输出应用、供应商 ID、名称及 `viaDefault`。只遍历 Claude、Claude Desktop、Codex 和 AGY，缺列表则跳过。优先匹配同授权类型的显式绑定（Copilot 兼容旧 `githubAccountId`），显式绑别的号不回落到默认。无显式绑定时，仅拟删除集合包含默认号且供应商会跟随该号时计入；无绑定的 Codex 官方卡跟随 CLI 登录，AGY 官方卡只有显式绑定才计入。旧 Copilot/Codex OAuth 卡还检查非 Codex 应用的 Anthropic 地址标记。结果按固定应用顺序分组供删除确认。任务 9 复用显式绑定、默认号与本机登录的定位语义，母任务 6 独占删除影响检查与授权操作。[绑定读取](../../../../../crates/vw-agent-apps/upstream/src/lib/authBinding.ts#L3-L21) · [应用范围与默认号规则](../../../../../crates/vw-agent-apps/upstream/src/lib/managedAccountUsage.ts#L7-L67) · [筛选与分组](../../../../../crates/vw-agent-apps/upstream/src/lib/managedAccountUsage.ts#L69-L125) · [删除前读取供应商](../../../../../crates/vw-agent-apps/upstream/src/components/providers/forms/hooks/useManagedAccountUsers.ts#L9-L31)

## 4. 订阅查询缓存、调度与联动

- 任务 39 持有短期旧值策略。请求 reject 时以真正成功的 `dataUpdatedAt` 为锚，网络、HTTP 429 和 5xx 可保留旧值不足 10 分钟；鉴权、解析等确定性失败立即透出并清旧值。切换应用或账号时清局部快照。首次 reject 或旧值超窗且无可用数据时，hook 合成空档失败快照并带错误原因，保留手动刷新入口。[订阅包装与占位](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L29-L87) · [瞬时失败分类和保留期限](../../../../../crates/vw-agent-apps/upstream/src/lib/query/queries.ts#L120-L244)
- 任务 40 持有服务端调度。上游订阅 hook 仅在启用自动查询且周期大于零时轮询，最短 1 分钟，同时允许后台轮询和窗口聚焦刷新，失败重试一次。Codex、AGY、Kimi、Claude 账号 hook 沿用此间隔规则，xAI 则固定 5 分钟。默认套餐另有独立查询，数据 1 分钟后陈旧，聚焦时可重取，无自动重试。[订阅与 Codex](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L89-L161) · [xAI、AGY、Kimi](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L186-L277) · [Claude](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L283-L351)
- 任务 53 持有卡片和账号中心的结果同步。普通订阅键按应用，托管键按授权类型及账号 ID，Codex/xAI 无显式 ID 时上游用 `default` 占位。任务 9/39 的原生合同须按解析后的身份隔离默认号变化。Claude 查询发现同一账号套餐或限速档变化时使账号状态失效。普通订阅命令仅对可解析应用的 `Ok` 快照写服务缓存、刷新托盘并发 `usage-cache-updated`，传输 `Err` 不发；事件桥只识别脚本与按应用订阅，没有这些托管账号键的直接写入分支。任务 45 持有授权后的刷新事件投递。[订阅与托管键](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L24-L27) · [Codex 默认占位](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L134-L176) · [xAI 默认占位](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L198-L215) · [Claude 套餐联动](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L298-L335) · [事件生产](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/subscription.rs#L24-L44) · [事件消费](../../../../../crates/vw-agent-apps/upstream/src/hooks/useUsageCacheBridge.ts#L9-L43)

## 5. 订阅额度数据类型

任务 8 独占 DTO。快照含工具、凭据状态和提示、查询成功标志、额度窗口、超额用量、可选重置次数和 Credits、错误、可空 Unix 毫秒查询时间。凭据五态为 `valid`、`expired`、`refresh_pending`、`not_found`、`parse_error`；凭据可用与查询成功分别表达。窗口含名称、已用比例、可空 ISO 8601 重置时间及可选美元已用/上限。TS 的 `planLabel` 可来自套餐 `UsageData.extra` 投影，上游 Rust 订阅窗口无此字段。超额用量含启用状态、月限额、已用 Credits、比例和币种；ChatGPT 的重置次数按到期从早到晚列出，`null` 为不过期，Codex Credits 余额另列。Claude 账号的可选 `plan` 含订阅类型和限速档。任务 41/42/43 分别消费窗口、Credits 与重置次数、失败状态。[TS 类型](../../../../../crates/vw-agent-apps/upstream/src/types/subscription.ts#L1-L55) · [Rust 序列化与失败快照](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L15-L115) · [查询时间单位](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L1831-L1836) · [套餐标签投影](../../../../../crates/vw-agent-apps/upstream/src/components/UsageFooter.tsx#L30-L52)

## 6. Codex 额度保护设置回归

模拟保存和乐观更新回滚。覆盖默认关闭、开启用 95%/98%、独立编辑后一起提交、空值/0/100/小数/负数拦截、恢复默认与禁用、保存返回 `false` 或拒绝后保留草稿可重试，以及抛错提示。母任务 20 持有设置交互、持久化及这些回归，母任务 8 持有代理阈值拦截，本计划任务 7/59 记录阈值消费的账号身份和额度数据交接。组件用例的输入是模拟设置和保存结果，证据限于组件行为。[回滚、恢复失败与重试](../../../../../crates/vw-agent-apps/upstream/tests/components/CodexQuotaProtectionSettings.test.tsx#L24-L122) · [默认、校验及禁用](../../../../../crates/vw-agent-apps/upstream/tests/components/CodexQuotaProtectionSettings.test.tsx#L123-L224)

## 7. Kimi 官方账号卡回归

模拟指定账号的额度与登录状态。账号状态补出昵称且现有卡名未含该昵称时，使 `["providers", "kimi"]` 失效；卡名已有昵称则保持列表。有效账号查询成功且窗口为空时显示未订阅，`not_found` 时静默。额度与授权中心共用 `["kimi_oauth", "quota", accountId]`，当前卡自动刷新。任务 32 持有账号额度查询和空窗口结果，任务 43 持有状态区分，任务 49 持有卡片展示，任务 53 持有昵称更新及共享查询。组件样例不提供真实账号或渲染验收，分别由任务 57、58 持有。[模拟输入与断言](../../../../../crates/vw-agent-apps/upstream/tests/components/KimiAccountQuotaFooter.test.tsx#L39-L99) · [卡片调用与昵称刷新](../../../../../crates/vw-agent-apps/upstream/src/components/KimiAccountQuotaFooter.tsx#L19-L66) · [额度命令包装](../../../../../crates/vw-agent-apps/upstream/src/lib/api/kimiOAuth.ts#L51-L53) · [共享查询键](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L252-L277)

## 合同与验收交接

- 任务 7 的查询合同与原生额度模块待交付。实施前核验母任务 2 的主机权限、任务和事件，母任务 4 的供应商设置，以及母任务 6 的授权引用与账号操作。查询执行端解析真实身份，快照、刷新和事件按同一身份隔离。[当前原生模块树](../../../../../crates/vw-agent-apps/src/lib.rs#L1-L15) · [合同任务](../../../../../.plans/2026-11-05-余额套餐额度与查询脚本/7-固化查询边界合同.md)
- 任务 59 汇总覆盖及 Codex 保护交接。真实账号、网关事件、macOS 原生界面仍由后续实现及验收任务验证，本批证据为 7 项来源核对和所有者映射。
