# 额度来源核对第 6 组

核对 CC Switch 4.0.25 固定快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。9 项均匹配原始清单 SHA-256 和仓内参考副本，含 5 份回归样例、4 份配置或缓存来源。任务文件所列 `src/types/index.ts` 在快照中对应 `src/types.ts`。本组无纯展示资源。下文编号属于余额套餐额度与查询脚本计划，另标母任务归属。[来源清单](../../../../../.plans/2026-11-05-CC能力完整融合/来源清单.json) · [命令与事件覆盖](../../../../../.plans/2026-11-05-CC能力完整融合/命令与事件覆盖.md) · [来源覆盖索引](../../../../../.plans/2026-11-05-余额套餐额度与查询脚本/来源覆盖索引.md) · [第 5 组](source-5.md)

## 1. 额度列刷新与展开回归

`QuotaLines.test.tsx` 用模拟时钟、额度行、加载状态和刷新回调驱动紧凑列。刷新图标仅悬停或聚焦时出现，无刷新回调则隐藏。手动刷新等待请求结算和最短转动时长，期间阻止重复点击，完成后短暂显示成功或失败。旧行本来就是失败、或 refetch 失败但仍显示旧成功额度，都应显示本次失败。后台加载只调低透明度。任务 49 持有刷新交互，任务 39 持有旧快照判定。[组件断言](../../../../../crates/vw-agent-apps/upstream/tests/components/QuotaLines.test.tsx#L33-L155) · [反馈实现](../../../../../crates/vw-agent-apps/upstream/src/components/quota/QuotaLines.tsx#L118-L192)

重置明细按钮与刷新按钮独立，打开明细不触发查询。展开额度条把倒计时放在数值之后，卡片仅有可用重置时间才加宽。任务 50 持有明细交互，任务 41 持有重置时间投影，任务 58 验收 macOS 布局和反馈。[明细与倒计时断言](../../../../../crates/vw-agent-apps/upstream/tests/components/QuotaLines.test.tsx#L157-L242)

## 2. 订阅卡片与明细回归

`SubscriptionQuotaFooter.test.tsx` 向 `SubscriptionQuotaView` 注入快照、显示偏好、布局和刷新回调。模拟订阅覆盖下列行为，真实账号验收归任务 57，原生界面验收归任务 58。

| 行为所有者 | 输入与期望 |
| --- | --- |
| 任务 41 · 窗口投影 | Claude Fable 卡固定 5 小时窗口，将周与 Fable 合并，已用模式影响数值和展开比例条，颜色按剩余量判断。缺 Fable 的旧快照保留原档。Antigravity 卡按组取剩余最少的窗口。[Claude 断言](../../../../../crates/vw-agent-apps/upstream/tests/components/SubscriptionQuotaFooter.test.tsx#L86-L195) · [AGY 断言](../../../../../crates/vw-agent-apps/upstream/tests/components/SubscriptionQuotaFooter.test.tsx#L197-L217) |
| 任务 50 · 明细交互 | AGY 打开明细列全四个窗口，独立刷新图标仍可用；普通卡保留点击刷新行为。展开显示各档比例条。[组件断言](../../../../../crates/vw-agent-apps/upstream/tests/components/SubscriptionQuotaFooter.test.tsx#L219-L258) |
| 任务 43 · 失败投影 | `refresh_pending` 与 `expired` 使用不同原因；待刷新不透出原始后端错误。展开状态按登录归属提示运行对应 CLI 或前往授权中心。实际视图对缺凭据和解析错误静默。[组件断言](../../../../../crates/vw-agent-apps/upstream/tests/components/SubscriptionQuotaFooter.test.tsx#L260-L310) · [失败分支](../../../../../crates/vw-agent-apps/upstream/src/components/SubscriptionQuotaFooter.tsx#L207-L282) |
| 任务 42 · Credits 与重置次数 | 重置次数可附在周窗口，明细列出各到期日及不过期项，展开行用最早到期日取代比例条。至少一档用尽才在卡上展示美元估算，排序在重置次数之前；单档保留两类摘要。卡片重置明细可附带 Credits，展开已有 Credits 行时避免重复。无可用余额或重置次数时隐藏对应项。[组件断言](../../../../../crates/vw-agent-apps/upstream/tests/components/SubscriptionQuotaFooter.test.tsx#L312-L459) · [投影入口](../../../../../crates/vw-agent-apps/upstream/src/components/SubscriptionQuotaFooter.tsx#L171-L203) |

## 3. 纯额度投影回归

`quotaRules.test.ts` 直接输入窗口、余额、时间和本地化函数，断言行文案、语气、条长、顺序及明细，无 HTTP 调用。

- 任务 41 持有剩余与已用投影。以断言为准，剩余 20% 正常、19% 警示，耗尽为危险。紧凑行优先最短窗口并合并其余；无短文案时取最低剩余并恢复原顺序。倒计时按渲染时刻重算，过期后回落到详情或行文案。[窗口与条长](../../../../../crates/vw-agent-apps/upstream/tests/components/quotaRules.test.ts#L44-L126) · [挑选与倒计时](../../../../../crates/vw-agent-apps/upstream/tests/components/quotaRules.test.ts#L159-L207) · [提示](../../../../../crates/vw-agent-apps/upstream/tests/components/quotaRules.test.ts#L317-L346)
- 任务 42 持有重置次数与 Credits。过滤过期重置项，同日到期合组，最早有效到期不足三天警示。只展示有限正数 Credits，按每份 0.04 美元给出整数估算，小额可写 `<$1`。[重置用例](../../../../../crates/vw-agent-apps/upstream/tests/components/quotaRules.test.ts#L209-L315) · [Credits 用例](../../../../../crates/vw-agent-apps/upstream/tests/components/quotaRules.test.ts#L348-L370)
- 任务 43 持有余额和错误行。接近零的正余额保持正常，零或欠费为危险，金额显示两位小数并消除显示负零。失败标题与次要原因分行，过期使用危险色。[余额和失败用例](../../../../../crates/vw-agent-apps/upstream/tests/components/quotaRules.test.ts#L127-L157)

## 4. Claude 套餐变化联动回归

`useClaudeAccountQuota.test.tsx` 预置授权状态缓存，模拟同账号额度响应。套餐从 Pro 变为 Max 时使 `["managed-auth-status", "claude_official"]` 失效；套餐及限速档相同、或响应没有套餐时保持原缓存。实际 hook 也比较限速档，找不到对应账号或缺订阅类型时不失效。任务 53 持有联动回归，任务 26 提供查询和套餐字段。[模拟与断言](../../../../../crates/vw-agent-apps/upstream/tests/hooks/useClaudeAccountQuota.test.tsx#L7-L70) · [hook 和失效条件](../../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L283-L336)

## 5. 托管账号绑定回归

`managedAccountUsage.test.ts` 输入账号删除集合、默认账号和供应商映射，输出受影响供应商及是否经由默认号。Copilot 默认号命中显式绑定、默认跟随和旧 URL 标记；非默认号只命中对应绑定（含旧 `githubAccountId`），批量删除合并结果。未绑定的 Codex 官方卡使用 CLI 登录，绑定卡和跟随默认号的 Claude Desktop 卡按应用归组。任务 9 消费显式绑定、默认账号与 CLI 的身份规则，母任务 6 独占删除影响与确认流程。[Copilot 断言](../../../../../crates/vw-agent-apps/upstream/tests/lib/managedAccountUsage.test.ts#L23-L85) · [Codex 断言](../../../../../crates/vw-agent-apps/upstream/tests/lib/managedAccountUsage.test.ts#L87-L127) · [运行规则](../../../../../crates/vw-agent-apps/upstream/src/lib/managedAccountUsage.ts#L7-L125)

## 6. 脚本模板、保存和试运行

- 任务 33 持有四个 JS 模板与参数。`custom` 给 `request` 和 `extractor` 骨架；`general` 以 API Key 请求 `/user/balance`；`newapi` 用令牌和用户 ID 请求 `/api/user/self`，按 500000 换算额度，失败可返回无效套餐及原因；`sub2api` 用 API Key 请求归一化的 `/v1/usage`，不限量的负剩余额不作余额。旧配置可显式覆盖非空 `apiKey`/`baseUrl`，否则回落供应商配置，去除 URL 尾斜杠。任务 35 持有执行端的出站网络约束。[模板](../../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L60-L168) · [变量解析](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/provider/usage.rs#L96-L123)
- 任务 51 持有原生选项与字段校验。余额、Coding Plan、Copilot 和官方订阅走专用查询；ZenMux 可覆盖 URL/Key，火山使用独立 AK/SK，智谱团队附组织和项目 ID。绑定 Claude 或 Codex 官方账号且无既有配置时默认启用，其余按模板和供应商预选。原生选项无需 JS 校验；启用脚本时上游仅检查非空和是否包含 `return`。首次启用可弹授权提示。任务 38 持有保存和作用域，来源将配置写入 `meta.usage_script`，随后使供应商列表、脚本和订阅缓存失效。[初始选项](../../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L375-L435) · [启用和保存检查](../../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L537-L578) · [切换模板](../../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L811-L908) · [保存调用](../../../../../crates/vw-agent-apps/upstream/src/hooks/useProviderActions.ts#L298-L335)
- 任务 52 持有试运行交互，任务 46 持有授权后的配置及试运行接口。上游 `testUsageScript` 接收供应商、应用和草稿参数，复用正式脚本的凭据解析与执行，草稿不落盘。成功且有数据则展示摘要，上游弹窗还直接写主界面的对应查询缓存；空结果、业务错误、命令拒绝显示失败。任务 7 需明确草稿试跑和正式授权快照之间的隔离规则。[原生试运行分支](../../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L580-L734) · [脚本试跑与结果](../../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L736-L784) · [命令和服务](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L891-L921) · [草稿执行](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/provider/usage.rs#L191-L228)

## 7. 短期成功缓存与刷新事件

- 任务 39 独占旧值判定。`resolveDisplayUsage` 输入查询结果、更新时间、上次成功值、当前时间和 reject 标志。网络类、HTTP 429 和 5xx 在距实际成功不足 10 分钟时可保留旧值及成功时间。鉴权、配置、解析等确定失败立即清旧值，避免下一次抖动恢复失信数据。上游也把 reject 后缓存中的成功结果当作旧值，到期丢掉显示数据，由 hook 合成失败占位并保留错误状态。上游文字匹配和通用 reject 路径是参考证据，任务 7 须区分结构化瞬时与确定错误。[判定函数](../../../../../crates/vw-agent-apps/upstream/src/lib/query/queries.ts#L111-L244) · [调用与失败占位](../../../../../crates/vw-agent-apps/upstream/src/lib/query/queries.ts#L280-L305)
- 任务 40 持有自动调度。上游脚本 query 以供应商 ID 和应用隔离，非空 ID 且启用时才自动查询，正周期最短 1 分钟并允许后台轮询；关闭轮询时新鲜期 5 分钟，失去订阅后缓存保留 10 分钟。聚焦不触发，reject 最多重试一次，间隔 1.5 秒。任务 39/7 须在服务端按授权身份、凭据版本与配置隔离，不能只依赖前端查询键或组件 ref。[查询参数](../../../../../crates/vw-agent-apps/upstream/src/lib/query/queries.ts#L246-L290) · [脚本键](../../../../../crates/vw-agent-apps/upstream/src/lib/query/usage.ts#L164-L165)
- 任务 45 持有刷新结果事件，任务 53 持有订阅者联动。`queryProviderUsage` 和 `testUsageScript` 已在上游注册；查询命令对 `Ok` 快照（含业务失败）写脚本缓存、调度托盘更新并发 `usage-cache-updated`，携带 `kind`、应用、供应商和结果，前端按键写入。命令 `Err` 不写缓存或发事件；脚本网络发送和读体失败可走该路径。试运行命令直接返回结果，弹窗的缓存更新由前端完成。[命令注册](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/lib.rs#L1646-L1655) · [查询发布](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L569-L612) · [错误分流](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/provider/usage.rs#L59-L91) · [事件桥](../../../../../crates/vw-agent-apps/upstream/src/hooks/useUsageCacheBridge.ts#L9-L43)

## 8–9. TS 和 Rust 配置与返回合同

`src/types.ts` 与 `src-tauri/src/provider.rs` 共同定义启用标志、语言、代码、可选模板与专用参数。TS 默认关闭、JavaScript、空代码、超时 10 秒和周期 5 分钟。TS 另声明 `autoIntervalMinutes`、`request`，Rust 对应结构没有这两项；执行读取的是脚本里的 `request`。任务 38 持有持久化与兼容，分钟范围 0–1440，0 关自动刷新，Rust 拒绝超上限。任务 34 持有执行时限，脚本缺省 10 秒，HTTP 请求时限按上游实现钳在 2–30 秒。[TS 配置和默认值](../../../../../crates/vw-agent-apps/upstream/src/types.ts#L53-L94) · [Rust 配置](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/provider.rs#L286-L338) · [间隔校验](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/provider/usage.rs#L230-L244) · [请求时限](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L240-L255)

任务 8 持有结果 DTO，`UsageResult` 包含成功标志、可选套餐数组和错误。单套餐的名称、扩展信息、有效性、失效原因、总量、已用、剩余和单位均可选，有效性与整个查询成功分开。任务 36 持有脚本输出校验，接受单对象或非空对象数组，逐项验证后统一为数组；类型错误报出数组位置或反序列化错误。[TS 结果](../../../../../crates/vw-agent-apps/upstream/src/types.ts#L96-L113) · [Rust 结果](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/provider.rs#L340-L372) · [数组验证](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L314-L387) · [结果封装](../../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/provider/usage.rs#L33-L57)

## 合同与验收交接

- 正式查询合同和原生额度模块待交付。任务 7 汇总六组来源，核验母任务 2 的主机权限、任务与事件，母任务 4 的供应商配置存储、母任务 6 的授权身份与凭据引用。查询、试运行、取消、缓存和事件应使用同一授权边界，原始凭据留在执行端。[原生模块树](../../../../../crates/vw-agent-apps/src/lib.rs#L1-L15)
- 任务 55–58 分别覆盖余额、Coding Plan、官方账号及 macOS 界面验收，任务 59 核对全量覆盖和跨母任务交接。本批验证范围为 9 项固定快照来源、归属及文档链接。
