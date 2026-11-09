# 额度来源核对第 4 组

核对 CC Switch 4.0.25 固定快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7`：7 项均与原始清单的 SHA-256 和仓内参考副本一致。6 项为运行时来源，1 项为回归测试证据，无纯展示资源。下文的任务编号属于余额套餐额度与查询脚本计划。[来源清单](../../../../../.plans/2026-11-05-CC能力完整融合/来源清单.json) · [命令与事件覆盖](../../../../../.plans/2026-11-05-CC能力完整融合/命令与事件覆盖.md) · [来源覆盖索引](../../../../../.plans/2026-11-05-余额套餐额度与查询脚本/来源覆盖索引.md)

## 1. 紧凑额度列与展开额度条

`QuotaLines` 接收投影行、行数上限（默认 2）、查询时间、加载状态、可选刷新回调和整列明细。空行不渲染；合并行取最近的未来重置时间，预留倒计时格对齐。相对更新时间和倒计时每 30 秒更新显示，不发网络请求。忙时不接受重复点击；手动刷新等待请求结算、加载结束和至少 600 毫秒的转动，再显示结果 1 秒。失败行、Promise 拒绝或 refetch 返回 `isError` 都可判失败，所以短期保留旧成功值时仍能报告本次失败。后台刷新只使卡片变淡。任务 49 负责卡片接入与刷新反馈，任务 39 负责旧快照保留策略。[计时和倒计时](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L63-L115) · [刷新反馈](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L118-L192) · [紧凑列](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L194-L305)

有整列明细时点列展开、另按刷新图标；只有单行明细时该段独立展开，其余仍可刷新。展开的 `QuotaBars` 接收行、标题、查询时间、加载状态、刷新回调与页脚；按档渲染比例条或说明（如重置到期日），保留提示和无障碍数值，加载中禁用刷新。任务 50 独占展开交互与呈现。[整列明细](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L397-L468) · [拆分交互](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L472-L478) · [展开额度条](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L608-L737)

## 2. 额度投影规则

| 实现所有者 | 输入、输出与异常行为 |
| --- | --- |
| 任务 41 · 多窗口 | 按档位、已用百分比和重置时间生成行。剩余取 `max(0, round(100 - 已用))`（缺值按 0 已用）；已用模式改变数字与条长，颜色和挑选仍按剩余，低于 20% 警示、0 为用尽。条长钳在 0–100，无有限比例画满；缺失、无效或已到的重置时间不显示倒计时。超过行限且各行都有短文案时，优先固定最短窗口（并列按输入顺序），末行最多合并两个剩余最低的条目；否则挑剩余最低的行再恢复原顺序。[百分比与时间](../../../../../crates/vw-agent-apps/upstream/src/components/quota/quotaRules.ts#L64-L179) · [紧凑行选择](../../../../../crates/vw-agent-apps/upstream/src/components/quota/quotaRules.ts#L421-L456) |
| 任务 42 · Credits 与重置次数 | Credits 只显示有限正数；数量本地化至两位小数，上游按每 Credit 0.04 美元取整显示估算，舍入后不足 1 写 `<$1`，没有官方美元余额语义。重置次数滤除已到期项，按本地日期分组，无法解析的日期归无到期；使用后端已排序的最早有效项生成摘要，距到期少于三天警示。仅一次且无附加内容时不展开。[重置摘要](../../../../../crates/vw-agent-apps/upstream/src/components/quota/quotaRules.ts#L182-L257) · [Credits 估算及到期分组](../../../../../crates/vw-agent-apps/upstream/src/components/quota/quotaRules.ts#L260-L350) |
| 任务 43 · 余额与失败 | 余额以剩余金额和可选总额、单位生成两位小数文本；欠费保留负值，无正总额的正余额没有比例。正余额不按低百分比警示，非正余额整行危险色。过期与查询失败为独立状态（排序值分别为 -1、-2），查询失败的非空原因另列。[余额、过期、失败](../../../../../crates/vw-agent-apps/upstream/src/components/quota/quotaRules.ts#L352-L419) |

## 3. 已用／剩余偏好

`useQuotaDisplay` 读共享 `["settings"]` 查询，仅显式 `used` 返回已用，未加载或无可用值回退剩余；hook 不保存也不触发额度刷新。常规设置选择器自动保存 `quotaDisplay`，上游走 `get_settings`／`save_settings`；余额、Credits、重置次数不随此偏好切换。任务 38 负责设置存储与作用域，任务 51 负责原生配置交互，任务 41 消费百分比显示偏好。[读取偏好](../../../../../crates/vw-agent-apps/upstream/src/components/quota/useQuotaDisplay.ts#L1-L8) · [选择与自动保存](../../../../../crates/vw-agent-apps/upstream/src/components/settings/sections/GeneralSection.tsx#L105-L126) · [共享查询](../../../../../crates/vw-agent-apps/upstream/src/lib/query/queries.ts#L93-L98) · [设置命令](../../../../../crates/vw-agent-apps/upstream/src/lib/api/settings.ts#L34-L40)

## 4. Codex 额度保护设置

`CodexQuotaProtectionSettings` 从 Codex 授权设置接收已保存配置和保存回调。默认关闭，5 小时和周窗口的阈值分别为已用 95% 和 98%，各自接受 1–99 的整数。开关即时保存；只在启用时显示阈值，保存中禁用操作，阈值未变或非法不能提交。恢复默认保留开关状态；回调返回 `false` 或抛错保留草稿，抛错显示保存失败；后端落盘前再次校验。保护阈值与显示偏好无关。母任务 20 独占设置与保存，母任务 8 独占代理准入和响应观察（以对应账号最新已用额度判断）；本计划任务 7/59 只记录额度数据合同和交接。[默认值与保存状态](../../../../../crates/vw-agent-apps/upstream/src/components/settings/CodexQuotaProtectionSettings.tsx#L9-L54) · [输入与恢复](../../../../../crates/vw-agent-apps/upstream/src/components/settings/CodexQuotaProtectionSettings.tsx#L56-L149) · [设置接入](../../../../../crates/vw-agent-apps/upstream/src/components/settings/CodexAuthSettings.tsx#L136-L139) · [后端校验](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/settings.rs#L381-L412) · [保存入口](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/settings.rs#L1109-L1112) · [代理准入与观察](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/proxy/codex_quota_protection.rs#L292-L316)

## 5. 授权中心单账号额度

`AccountQuotaColumn` 输入登录显示名、投影状态、查询时间、加载状态和刷新回调。输出档位、比例条或明细、对齐的重置时间、相对更新时间与带账号名的刷新按钮；加载中阻止重复刷新。订阅转换无快照时区分初始加载与隐藏，凭据缺失或解析失败隐藏，查询失败显示原因，成功空档通常隐藏。Copilot 另处理 refetch 错误并显示 Premium；Kimi 成功空档显示未订阅。Claude、Antigravity、Kimi、xAI、Copilot 和外部 Codex 包装组件按显式账号读取，行组件不单独轮询，也不直接监听额度事件。任务 43 负责状态投影，任务 53 独占账号行接入、共享快照与刷新联动；供应商适配归各自查询任务。[状态与行交互](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L42-L212) · [状态转换与 Copilot](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L274-L349) · [xAI／Kimi／AGY／Claude](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/AccountQuota.tsx#L351-L468) · [Codex 包装](../../../../../crates/vw-agent-apps/upstream/src/components/CodexOauthAccountQuota.tsx#L10-L47)

## 6. 授权中心手动批量刷新

`RefreshAllQuotaButton` 在授权中心页头调用 `refetchQueries({type: "active", predicate})`。筛选键前两段：Claude 官方、Copilot、Codex OAuth、xAI OAuth、Kimi OAuth 和 AGY OAuth 的 `quota`，另匹配 DeepSeek 的账号状态键（余额）。匹配查询中有任何正在获取就禁用按钮并转动；账号行共享查询状态，失败由各行显示，页头未聚合结果，也没有独立批量 Tauri 命令或事件。任务 40 负责服务端的自动调度、合并请求和批量结果；任务 45 负责刷新接口，任务 53 负责桌面手动触发、逐项失败和断连后重读。[筛选与触发](../../../../../crates/vw-agent-apps/upstream/src/components/settings/auth/RefreshAllQuotaButton.tsx#L12-L60) · [页头入口](../../../../../crates/vw-agent-apps/upstream/src/App.tsx#L1744-L1756) · [DeepSeek 状态键](../../../../../crates/vw-agent-apps/upstream/src/components/providers/DshAccount.tsx#L26-L33)

## 7. Coding Plan 检测回归样例

`codingPlanProviders.test.ts` 只提供离线输入和期望，不验证网络。任务 10 持有识别规则与回归：OpenCode Go 新旧路径命中、按量 Zen 不命中；MiniMax 允许清单内域名及大小写变化，拒绝伪造后缀和仅路径出现的域名。URL 按应用配置提取：Claude 系列与 Gemini 读各自环境字段，Codex、Grok Build 读 TOML，OpenCode/MCode 读 `options.baseURL`，Pi/Oh My Pi/OpenClaw 读 `baseUrl`，Hermes 读 `base_url`；未知应用返回空。[路径与域名用例](../../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.test.ts#L20-L62) · [配置字段用例](../../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.test.ts#L64-L134)

新增供应商时，检测命中才预填 `token_plan`；测试覆盖 OpenCode Go 的多应用预设、非 Claude 的 Kimi/智谱、ZenMux 仅 Claude、火山方舟只预选不开启、已有自定义配置保留。任务 10 持有新建时的自动选择，任务 51 持有用户显式配置与凭据必填校验。上游 `usage_script` 字段是配置载体，`token_plan` 走原生查询，不执行 JS。[自动预填用例](../../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.test.ts#L141-L252) · [预填实现](../../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L168-L206) · [新增供应商调用](../../../../../crates/vw-agent-apps/upstream/src/hooks/useProviderActions.ts#L87-L96) · [原生查询命令](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/coding_plan.rs#L1-L19)

## 合同与验收交接

- 任务 7 的正式合同与原生额度模块尚未交付。任务 7 需对接母任务 2 的主机权限、任务和事件，母任务 4 的供应商设置存储及母任务 6 的授权引用。账号快照、批量刷新和事件须按同一授权身份作用域匹配；原始凭据只在执行端使用。[原生模块树](../../../../../crates/vw-agent-apps/src/lib.rs#L1-L15)
- 上游命令表列有 `get_coding_plan_quota`、`get_subscription_quota` 和各账号查询；订阅和供应商命令可发 `usage-cache-updated`，桥接器按种类写对应查询缓存。上述组件以查询状态消费结果；这不能证明每种账号命令都发该事件。任务 53 应依据任务 7 合同和网关事件实现共享快照联动。[事件桥接](../../../../../crates/vw-agent-apps/upstream/src/hooks/useUsageCacheBridge.ts#L22-L43)
- 任务 58 负责 macOS 原生界面的设置保存、展开、部分失败、断连恢复及明暗主题实测；任务 59 负责覆盖与额度保护交接。本批证据限于来源、归属和链接。
