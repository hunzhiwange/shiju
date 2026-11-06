# ACP 完全访问支持矩阵

审计日期：2026-11-06（Asia/Shanghai）；审计平台：macOS / Darwin arm64。状态：静态基线已完成；Codex 与 Claude 固定制品的 macOS arm64 原生权限与撤销已验收，具体入口与边界见 [Codex 证据](evidence/codex.md)、[Claude 证据](evidence/claude.md)。Gemini 已核验为客户端自动批准，原生自动提权因确认及工具/提示词副作用不接入，见 [Gemini 证据](evidence/gemini.md)。Qwen 固定 npm 与现有托管补丁入口已核验原生免审批与撤销，见 [Qwen 证据](evidence/qwen.md)。Qoder 已接入公布的会话 mode 配置，权限工具效果未实机验收，见 [Qoder 接入说明](evidence/qoder.md)。Vibe 保留客户端自动批准，并为显式 auto-approve agent 加入授权及撤销保护，未实机验收，见 [Vibe 接入说明](evidence/vibe.md)。Kiro 已接入 V3 会话 policyPreset，效果未实机验收，见 [Kiro 接入说明](evidence/kiro.md)。OpenCode 使用逐次客户端自动批准，避免写入会话永久授权，见 [OpenCode 接入说明](evidence/opencode.md)。Hermes 已接入会话编辑审批 default↔dont_ask，终端继续客户端批准，未实机验收，见 [Hermes 接入说明](evidence/hermes.md)。iFlow 已接入会话 default↔yolo 并兼容 load 缺少 modes，未实机验收，见 [iFlow 接入说明](evidence/iflow.md)。Copilot 已接入逐次客户端批准，见 [Copilot 说明](evidence/copilot.md)。Kimi 已接入新版 ACP default↔yolo 与逐次客户端批准，未实机验收，见 [Kimi 接入说明](evidence/kimi.md)。CodeBuddy 已接入公布的会话 default↔bypassPermissions 与逐次客户端批准，未实机验收，见 [CodeBuddy 接入说明](evidence/codebuddy.md)。其他后端原生能力仍未知。

Goose 已接入会话 approve 与逐次客户端批准，避免 auto 跳过用户 deny；未实机验收，见 [Goose 接入说明](evidence/goose.md)。

Auggie 已接入逐次客户端批准与当前 host deny 拦截；原生权限与工具执行未实测，见 [Auggie 接入说明](evidence/auggie.md)。

Kilocode 已接入逐次客户端批准与当前 host deny 拦截；pure 保留权限配置，agent mode 不作权限开关；工具效果未实测，见 [Kilocode 接入说明](evidence/kilocode.md)。

范围为 `BUILT_IN_AGENT_DEFINITIONS` 的 22 个稳定 key；别名和 `claude-legacy` 不另计。注册表、启动快照、哈希及逐项官方版本来源见 [证据清单](evidence/README.md)。初始静态审计仅修改文档；后续后端适配与验证按各独立证据文件记账。

## 证据与分类

- E0：对话/历史线索，尚未核验当前运行版本。

- E1：本次仓库源码静态确认，不能推出供应商实际运行效果。

- E2：固定版本的上游源码/制品权限实现确认；包元数据只确认版本身份，不构成权限 E2。

- E3：指定平台、版本、启动路径与模型来源的真实协议和工具结果验收。Codex、Claude、Qwen 的限定路径已有权限 E3，其他后端尚无。

分类使用“原生免审批”“客户端自动批准”“部分支持”“不可用”。Codex、Claude、Qwen 限定路径已核验原生支持，Gemini 为已核验的客户端批准与原生接入拒绝，OpenCode 为源码确认的客户端逐次批准（未实机验收）；其余项为**部分支持（E1 客户端机制；原生未知）**；这不是 22 项实机可用承诺。下表中的 `session_model 不可用` 只指该模型来源已有显式拒绝，不等于 CLI 本地模式不可用。原生未知不得展示为原生支持。

`cli_local` 沿注册表选择命令并使用 CLI 本地模型配置；`session_model` 经 `model_source::prepare` 和对应 `drivers::<key>::configure` 生成私有配置/模型中继。模型路由验收与权限验收分别记账。

## 22 项版本与下一项核验

“基线”来自现有版本清单/托管契约；“官方当前”是本次查询的发布元数据或版本指针，**均不是本次执行所得 CLI 版本**。初始基线的实际安装版本未知；Codex 本次隔离固定制品身份见独立证据，不自动升级用户 CLI。版本不同必须重验，并同时确认适配器与底层引擎。

| key / 证据 | 基线版本 | 官方当前发布 | 当前权限分类 / 模型来源限制 | 下一项核验目标 |
| --- | --- | --- | --- | --- |
| [auggie](evidence/auggie.md) / E1、官方文档 | 0.36.0 | 0.37.0（基线查询） | 客户端自动批准：仅 allow_once，按当前授权处理；不注入原生 permission 规则；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复及跨平台工具效果未实机验收 |
| [claude](evidence/claude.md) / E2、E3 | 0.26.0 / SDK 0.2.96 / CLI 2.1.96 | 0.101.0 | 原生免审批：macOS arm64 固定 npm 入口、当前授权及配置确认后；未知入口降级、无法撤销时拒绝。计划确认需独立交互 | 补齐企业托管 deny、其他平台和 Bun 制品；保持计划确认独立 |
| [codex](evidence/codex.md) / E2、E3 | 2.5.0 / Codex 0.165.0 | 2.5.0 | 原生免审批：macOS arm64 已核验制品、当前授权与配置确认后；cli_local/session_model 均验收。未知入口/平台降级，无法确认撤销时拒绝；活动 turn 必须先取消 | 补齐其他平台、独立文件编辑工具、网络及企业策略验收 |
| [codebuddy](evidence/codebuddy.md) / E1、E2 | 2.179.3 | 2.180.0（基线查询） | 部分支持：按公布配置同步 default↔bypassPermissions，客户端仅逐次批准，问答及计划确认独立；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复与跨平台工具效果未实机验收 |
| [goose](evidence/goose.md) / E1、E2 | v1.57.0 | v1.57.0（基线查询） | 客户端自动批准：同步会话 approve，仅逐次批准；auto 跳过用户 deny，不自动接入；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复及跨平台工具效果未实机验收 |
| [vibe](evidence/vibe.md) / E1、E2 | 3.0.4 | 3.1.0 | 部分支持：客户端自动批准；原生 mode 切换整个 agent，仅允许当前授权下显式选择 auto-approve，撤销未完成则拒绝继续；session_model 保持现有门控 | 用户要求跳过测试；原生执行、恢复、deny 和跨平台效果未实机验收 |
| [hermes](evidence/hermes.md) / E1、E2 | 0.19.0 | 0.19.0 | 部分支持：按当前授权同步 default↔dont_ask 编辑审批，敏感路径及终端仍用客户端审批；保留 session_model 门控 | 用户要求跳过测试；开启、关闭、恢复及跨平台工具效果未实机验收 |
| [snow](evidence/snow.md) / E1、E2 | 0.9.14 | 0.9.15（基线查询） | 客户端自动批准：仅 allow_once，避免写入工作区永久批准；ACP 未实现权限模式；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复及跨平台工具效果未实机验收 |
| [copilot](evidence/copilot.md) / E1、官方文档 | 1.0.105 | 1.0.106 | 客户端自动批准：仅 allow_once；显式启动全放行要求当前授权，不自动注入；保留模型来源门控 | 用户要求跳过测试；开启、关闭、恢复、tools/paths/URLs 与 deny 效果未实机验收 |
| [cursor](evidence/cursor.md) / E1、官方文档 | 2026.10.22-27dcf5f | 2026.11.05-86047df（基线查询） | 客户端自动批准：仅 allow_once，按当前授权处理；显式 force/yolo 要求当前完全访问；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复及跨平台工具效果未实机验收 |
| [droid](evidence/droid.md) / E1、官方文档 | 1.3.0 | 1.5.0（基线查询） | 客户端自动批准：仅 allow_once，按当前授权处理；显式 skip-permissions-unsafe 要求当前完全访问；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复及跨平台工具效果未实机验收 |
| [gemini](evidence/gemini.md) / E2、E3 | 0.66.0 | 0.67.0 | 部分支持：客户端自动批准；原生自动提权不接入，缺少标准回读且修改工具/系统指令。高权限恢复拒绝；session_model 有驱动和版本门控 | 补齐其他平台与企业/原生沙箱；新版本须重新验收 |
| [iflow](evidence/iflow.md) / E1、E2 | 0.5.19 | 0.5.19 | 部分支持：按当前授权同步 default↔yolo，保留 plan 与客户端审批；load 无 modes 依据固定源码发送精确值；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复、子代理及跨平台效果未实机验收 |
| [kilocode](evidence/kilocode.md) / E1、E2 | 7.8.18 | 7.8.27（基线查询） | 客户端自动批准：仅 allow_once，按当前授权处理；pure 保留权限配置，mode 选择 agent；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复及跨平台工具效果未实机验收 |
| [kimi](evidence/kimi.md) / E1、E2 | 2.5.0 | 2.5.2 | 部分支持：按公布配置同步 default↔yolo，客户端仅逐次批准，问答及计划确认独立；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复与跨平台工具效果未实机验收 |
| [kiro](evidence/kiro.md) / E1、官方协议文档 | 3.0.0 | 3.1.0 | 部分支持：new/load 按当前授权请求 V3 policyPreset allow-all 或空列表；客户端自动批准保留，硬拒绝、敏感路径 ask 与 sandbox_network 不被预设覆盖；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复和跨平台执行未实机验收 |
| [opencode](evidence/opencode.md) / E1、E2 | 1.19.5 | 1.19.5 | 客户端自动批准：仅选择 allow_once，按当前 permissionMode 处理；不注入全局 allow，不把 agent mode 当权限开关；session_model 原有门控保留 | 用户要求跳过测试；执行、关闭、恢复和跨平台效果未实机验收 |
| [openclaw](evidence/openclaw.md) / E1、E2 | 2026.10.10 | 2026.11.1（基线查询） | 客户端自动批准：Gateway exec 审批仅 allow-once；不改 elevated_level 或网关 deny；session_model 原有门控保留 | 用户要求跳过测试；开启、关闭、恢复及跨平台效果未实机验收 |
| [pi](evidence/README.md#pi) / E1 | 0.0.22 | 0.0.34 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 核对 ACP adapter 与 Pi RPC 原生工具审批边界 |
| [qoder](evidence/qoder.md) / E1、协议字段采集 | 1.1.82 | 1.1.83 | 部分支持：已接入 mode default↔yolo 与客户端审批；实机效果未测，不宣称已验证原生支持；session_model 原有账号权益门控保留 | 用户要求跳过测试；后续可补工具执行、恢复、deny 与跨平台实机验收 |
| [qwen](evidence/qwen.md) / E2、E3 | 0.26.7 | 0.26.8 | 原生免审批：macOS arm64 固定 npm/现有托管补丁、当前授权和 config 确认后；未知入口降级，无法确认撤销则拒绝；计划独立交互 | 补齐其他平台、原生沙箱、企业策略与完整网关中继验收 |
| [trae](evidence/README.md#trae) / E1 | 0.121.3 | v0.121.3 | 部分支持：客户端自动批准有源码、原生未知；session_model 不可用：需要平台认证，隔离根未核验 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |

初始对话线索 Codex 2.5.0、Claude 0.26.0、Gemini 0.66.0、Qwen 0.26.7、Vibe 3.0.4 已与仓库基线比对。Codex 指适配器 2.5.0，底层 Codex 基线为 0.165.0；Claude 为适配器 0.26.0、SDK 0.2.96、内嵌 CLI 2.1.96。其他适配器/引擎身份见证据条目。

## 协议与执行边界逐项核验

U = 未获取该后端真实权限协议/结果；C = 仅共享客户端源码存在，后端是否调用未知。各行均须分别覆盖普通/托管、cli_local/session_model、开启/关闭及运行中切换；不可用组合记录明确拒绝。

| key | initialize | new/load/resume | configOptions | modes | request_permission | 客户端文件 | 客户端终端 | 原生沙箱 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| auggie | U | 现有 new/load/resume 路径；未实测 | 不推断原生权限配置 | 不推断权限模式 | 代码接入 allow_once；未实机计数 | C；未实测 | C；未实测 | U；保留供应商规则 |
| claude | E3 | E3/E3/E3 | E3：default↔bypassPermissions | E2/E3；以 config 同步 | E3：普通工具 1→0→1；计划另行确认 | C；本例为原生 Write | C；本例为原生 Bash | 不等同沙箱解除；项目 deny 保留 |
| codex | E3 | E3/E3/E3 | E3 | E2；以 config 同步 | E3：本例 0 | C；本例未调用 | C；本例未调用 | E3：readOnly↔dangerFullAccess |
| codebuddy | U | new/load/resume 与 prompt 前已接入；未实测 | E2：mode default↔bypassPermissions，设置响应确认 | E2：公布 default/acceptEdits/plan/auto/dontAsk/bypassPermissions/fullAccess；不自动退出 plan | E2：allow/allow_always/reject；仅逐次批准，问答/计划不自动答复 | C；未实测 | C；未实测 | U；不选择跳过危险检查的 fullAccess |
| goose | U | new/load/resume 与 prompt 前已接入；未实测 | E2：mode=approve，响应确认；不改模型/provider | E2：auto/approve/smart_approve/chat；保留 chat，legacy 回退 approve | E2：allow_always/allow_once/reject_once/reject_always；客户端仅逐次批准，未实测 | C；未实测 | C；未实测 | U；auto 跳过用户 deny，生产不自动选择 |
| vibe | U | 代码接入；未实测 | E2：mode 为 agent 名称，不自动切换 | E2：auto-approve 显式授权保护 | C；未实机计数 | C | C | U |
| hermes | U | new/load/resume 与 prompt 已接入；未实测 | E2：未公布，隐藏编辑配置不主动发送 | E2：default/accept_edits/dont_ask；已接入，无回读确认 | E2：终端和敏感编辑仍回调；未计数 | C；未实测 | C；未实测 | U；不覆盖原生 deny |
| snow | E2：源码声明 loadSession；未实测 | E2：new/load；resume 使用既有 load 回退，未实测 | E2：未实现 | E2：未实现；CLI yolo 不用于 ACP | E2：approve_once/approve_always/reject_once；客户端仅逐次批准，未计数 | C；未实测 | C；未实测 | U；保留敏感命令与 hook 规则 |
| copilot | U | 现有 new/load/resume；未实测 | 不推断原生权限配置 | 不推断权限模式 | 代码接入 allow_once；未实机计数 | C；未实测 | C；未实测 | U；保留供应商限制 |
| cursor | U | 现有 new/load/resume；未实测 | 不推断原生权限配置 | 官方 agent/plan/ask 是工作模式，不自动切换 | 官方 allow-once/allow-always/reject-once；代码仅逐次批准，未计数 | C；未实测 | C；未实测 | U；保留供应商 deny |
| droid | U | 现有 new/load/resume；未实测 | 不推断原生权限配置 | 自治等级不等于免审批，不推断 ACP mode ID | 代码仅逐次批准；未实机计数 | C；未实测 | C；未实测 | U；保留供应商 block 与 sandbox |
| gemini | E3 | E3/E3/-32601；同 ID load 回退 | -32601 | E3：legacy default/yolo；无标准回读 | E3：原始工具 1→0→1；生产保持客户端审批 | C；本例原生 write_file | C；本例原生 shell | E2 刷新管理器；OS 隔离未验收 |
| iflow | U | new/load/resume 已接入；未实测 | E2：无标准配置路由，不杜撰 config | E2：default/smart/yolo/plan；已接入，无标准回读；load 无 modes | C；未实机计数 | C；未实测 | C；未实测 | U；不覆盖原生 deny |
| kilocode | U | E2：new/load/resume；现有路径，未实测 | E2：model/effort/mode，无独立权限开关 | E2：agent 名称，不自动切换 | E2：once/always/reject；代码仅逐次批准，未计数 | C；未实测 | C；未实测 | U；保留供应商规则 |
| kimi | U | new/load/resume 与 prompt 前已接入；未实测 | E2：mode default↔yolo，设置响应确认 | E2：default/plan/auto/yolo；不自动退出 plan | E2：approve_once/approve_always/reject；仅逐次批准，问答/计划不自动答复 | C；未实测 | C；未实测 | U；不覆盖供应商限制 |
| kiro | U | new/load 已接入 policyPreset；resume 走同 ID load；未实测 | 不杜撰权限 config | 不切换 agent | C；未实机计数 | C | C | allow-all 不覆盖 sandbox_network；未实测 |
| opencode | U | 现有 new/load/resume 路径；未实测 | E2：仅 model/effort/mode，无权限开关 | E2：agent 名称，不自动切换 | E2：once/always/reject；客户端仅逐次批准，未实机计数 | C；未实测 | C；未实测 | U；不覆盖原生 deny |
| openclaw | E2：Gateway 桥接；未实测 | E2：new/load/resume；现有路径，未实测 | E2：elevated_level 改网关执行边界，不自动设置 | E2：thinkingLevel，不作权限开关 | E2：allow-once/allow-always/deny；代码仅逐次批准，未计数 | E2：桥接不调用 | E2：桥接不调用 | U；保留 Gateway deny 和沙箱 |
| pi | U | U/U/U | U | U | C | C | C | U |
| qoder | 1.1.83 字段已采集 | new 字段已采集；load/resume 仅代码接入 | mode default↔yolo 已接入，未测执行效果 | 公布 default/acceptEdits/auto/dontAsk/yolo | C；未实机计数 | C；未实机执行 | C；未实机执行 | U |
| qwen | E3 | E3/E3/E3 | E3：mode default↔yolo | E2/E3；以 config 回读确认 | E3：普通工具 1→0→1；deny 保留 | C；本例原生 write_file | C；本例原生 shell | 未验收；不等同解除 OS 隔离 |
| trae | U | U/U/U | U | U | C | C | C | U |

| 边界 | 后续验收所需证据 |
| --- | --- |
| initialize | 记录 protocolVersion、agentInfo、agentCapabilities、clientCapabilities；不可从 CLI help 推断协商结果。[官方初始化](https://agentclientprotocol.com/protocol/v1/initialization)。 |
| new/load/resume | 各自记录支持/拒绝、sessionId 关联、首次 prompt 前实际权限；恢复后当前授权覆盖历史值，无工具在同步前执行。未实现的 resume 不伪装成功。 |
| configOptions / modes | 记录真实选项 ID、枚举值、当前值、设置响应与更新通知；开启后回读，关闭/恢复再次回读。不得把 plan/ask 当工具审批开关。[官方模式](https://agentclientprotocol.com/protocol/v1/session-modes)、[配置选项](https://agentclientprotocol.com/protocol/v1/session-config-options)。 |
| request_permission | 记录回调次数和 offered option kind、实际选择与工具结果；区分原生无回调、客户端自动应答、硬拒绝、取消、缺少允许选项。共享机制仍有远端往返。[官方工具调用](https://agentclientprotocol.com/protocol/v1/tool-calls)。 |
| 文件 / 终端 | 使用临时根验证读写、根外/符号链接、终端创建执行和关闭；区分客户端代执行与代理自身工具，权限改变后两条路径都验证。 |
| 沙箱 / 企业策略 | 单独验证原生 sandbox、OS 限制、企业 deny 和权限撤销；客户端放行不等于沙箱解除。无安全撤销路径应拒绝继续并报错。 |

## 已确认的本地差异与限制

1. `built_in_agent_command` 优先返回通过校验的托管命令；缺失或校验错误会因 `if let Ok(Some(..))` 继续本地/默认回退。用户覆盖还可改变命令。不能把契约存在当作实际使用托管制品的证明。

2. 初始基线的普通 Gemini 曾带 `--approval-mode yolo --skip-trust`；任务 4 改为按授权材料化，任务 7 的固定版本验收后停止自动注入并显式拒绝该参数组合。任务 8 停止为 Qwen 自动注入 `--approval-mode yolo`，改为固定制品会话 config 同步；显式启动 bypass 拒绝。Qoder 初始曾带 `--permission-mode bypass_permissions`，任务 9 已停止自动注入，改为会话 `mode=default/yolo` 同步当前授权，并拒绝显式启动 bypass；工具效果尚未实机验收。托管 Gemini/Qwen 初始 argv 不带这些宽松参数。

3. `AcpClient::with_full_access` 设置独立布尔值；`build_event_client` 将它传给文件处理器，终端处理器及标准审批使用 `permission_mode` 等字段。`resolve_permission_request_with_prompt` 的 ApproveAll 选择允许选项，缺少允许选项时退到第一个选项；取消中的会话直接取消。后续需验证没有允许项和拒绝项优先级，不能据此声称所有请求均批准。

4. `drivers::capabilities` 仅对 Darwin arm64、`verified_release` 非空且必要托管制品可用的组合发布 session_model；`AcpLaunchProfile::new` 非 Unix 明确拒绝。这属于模型来源门控，不证明完全访问已生效。不得将 macOS 的既有模型证据外推到 Linux/Windows。

5. 相邻 `agent_registry_tests.rs`、`managed_runtime_tests.rs`、`client/event_client_tests.rs`、`permissions_tests.rs` 与各 `<key>_tests.rs` 是后续聚焦测试入口。本任务未运行这些测试；现有 fixture 不替代外部 CLI 真实验收。

## 初始静态验收（任务 1）

已从注册表逐项人工核对上述 22 个 key，并用独立集合检查确认无遗漏/重复；逐项读取驱动入口，核对普通、本地候选、托管启动及显式拒绝。执行 `git diff --check` 和文档链接/来源哈希检查通过。未运行 Cargo（按任务的文档静态审计范围），未运行真实权限会话。下一步按后续任务实现共享策略映射，再逐后端积累 E2/E3。
