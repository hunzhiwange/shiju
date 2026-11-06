# ACP 完全访问支持矩阵

审计日期：2026-11-06（Asia/Shanghai）；审计平台：macOS / Darwin arm64。状态：静态基线已完成；Codex 固定制品的 macOS arm64 原生权限与撤销已验收，具体入口与边界见 [Codex 证据](evidence/codex.md)。其他后端原生能力仍未知。

范围为 `BUILT_IN_AGENT_DEFINITIONS` 的 22 个稳定 key；别名和 `claude-legacy` 不另计。注册表、启动快照、哈希及逐项官方版本来源见 [证据清单](evidence/README.md)。初始静态审计仅修改文档；后续后端适配与验证按各独立证据文件记账。

## 证据与分类

- E0：对话/历史线索，尚未核验当前运行版本。

- E1：本次仓库源码静态确认，不能推出供应商实际运行效果。

- E2：固定版本的上游源码/制品权限实现确认；包元数据只确认版本身份，不构成权限 E2。

- E3：指定平台、版本、启动路径与模型来源的真实协议和工具结果验收。Codex 的限定路径已有权限 E3，其他后端尚无。

分类使用“原生免审批”“客户端自动批准”“部分支持”“不可用”。除 Codex 限定已核验路径外，其余项为**部分支持（E1 客户端机制；原生未知）**；这不是 22 项实机可用承诺。下表中的 `session_model 不可用` 只指该模型来源已有显式拒绝，不等于 CLI 本地模式不可用。原生未知不得展示为原生支持。

`cli_local` 沿注册表选择命令并使用 CLI 本地模型配置；`session_model` 经 `model_source::prepare` 和对应 `drivers::<key>::configure` 生成私有配置/模型中继。模型路由验收与权限验收分别记账。

## 22 项版本与下一项核验

“基线”来自现有版本清单/托管契约；“官方当前”是本次查询的发布元数据或版本指针，**均不是本次执行所得 CLI 版本**。初始基线的实际安装版本未知；Codex 本次隔离固定制品身份见独立证据，不自动升级用户 CLI。版本不同必须重验，并同时确认适配器与底层引擎。

| key / 证据 | 基线版本 | 官方当前发布 | 当前权限分类 / 模型来源限制 | 下一项核验目标 |
| --- | --- | --- | --- | --- |
| [auggie](evidence/README.md#auggie) / E1 | 0.36.0 | 0.37.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 不可用：Provider 注册被拒绝 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [claude](evidence/README.md#claude) / E1 | 0.26.0 | 0.101.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 核对 SDK permissionMode、企业 deny 和 modes/configOptions |
| [codex](evidence/codex.md) / E2、E3 | 2.5.0 / Codex 0.165.0 | 2.5.0 | 原生免审批：macOS arm64 已核验制品、当前授权与配置确认后；cli_local/session_model 均验收。未知入口/平台降级，无法确认撤销时拒绝；活动 turn 必须先取消 | 补齐其他平台、独立文件编辑工具、网络及企业策略验收 |
| [codebuddy](evidence/README.md#codebuddy) / E1 | 2.179.3 | 2.180.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [goose](evidence/README.md#goose) / E1 | v1.57.0 | v1.57.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [vibe](evidence/README.md#vibe) / E1 | 3.0.4 | 3.1.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 核对 ACP approval mode 与 smart_approve 的区别 |
| [hermes](evidence/README.md#hermes) / E1 | 0.19.0 | 0.19.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [snow](evidence/README.md#snow) / E1 | 0.9.14 | 0.9.15 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [copilot](evidence/README.md#copilot) / E1 | 1.0.105 | 1.0.106 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [cursor](evidence/README.md#cursor) / E1 | 2026.10.22-27dcf5f | 2026.11.05-86047df | 部分支持：客户端自动批准有源码、原生未知；session_model 不可用：第三方 Provider 契约未核验 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [droid](evidence/README.md#droid) / E1 | 1.3.0 | 1.5.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 不可用：双向 ACP 与隔离根未核验 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [gemini](evidence/README.md#gemini) / E1 | 0.66.0 | 0.67.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 比较普通 yolo/skip-trust 与托管无该参数两条路径 |
| [iflow](evidence/README.md#iflow) / E1 | 0.5.19 | 0.5.19 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [kilocode](evidence/README.md#kilocode) / E1 | 7.8.18 | 7.8.27 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [kimi](evidence/README.md#kimi) / E1 | 2.5.0 | 2.5.1 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [kiro](evidence/README.md#kiro) / E1 | 3.0.0 | 3.1.0 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 核对 v3 harness 权限选项及恢复行为 |
| [opencode](evidence/README.md#opencode) / E1 | 1.19.5 | 1.19.5 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |
| [openclaw](evidence/README.md#openclaw) / E1 | 2026.10.10 | 2026.11.1 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 核对 gateway exec 审批与 ACP 客户端回调边界 |
| [pi](evidence/README.md#pi) / E1 | 0.0.22 | 0.0.34 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 核对 ACP adapter 与 Pi RPC 原生工具审批边界 |
| [qoder](evidence/README.md#qoder) / E1 | 1.1.82 | 1.1.83 | 部分支持：客户端自动批准有源码、原生未知；session_model 不可用：需要账号授权及权益 | 核对 bypass_permissions 的 ACP 实际语义与关闭路径 |
| [qwen](evidence/README.md#qwen) / E1 | 0.26.7 | 0.26.8 | 部分支持：客户端自动批准有源码、原生未知；session_model 有驱动和版本门控 | 比较普通 yolo 与托管无 yolo 路径及模式回读 |
| [trae](evidence/README.md#trae) / E1 | 0.121.3 | v0.121.3 | 部分支持：客户端自动批准有源码、原生未知；session_model 不可用：需要平台认证，隔离根未核验 | 固定真实 CLI，采集 initialize/权限选项并验证开启→关闭→恢复 |

初始对话线索 Codex 2.5.0、Claude 0.26.0、Gemini 0.66.0、Qwen 0.26.7、Vibe 3.0.4 已与仓库基线比对。Codex 指适配器 2.5.0，底层 Codex 基线为 0.165.0；Claude 为适配器 0.26.0、SDK 0.2.96、内嵌 CLI 2.1.96。其他适配器/引擎身份见证据条目。

## 协议与执行边界逐项核验

U = 未获取该后端真实权限协议/结果；C = 仅共享客户端源码存在，后端是否调用未知。各行均须分别覆盖普通/托管、cli_local/session_model、开启/关闭及运行中切换；不可用组合记录明确拒绝。

| key | initialize | new/load/resume | configOptions | modes | request_permission | 客户端文件 | 客户端终端 | 原生沙箱 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| auggie | U | U/U/U | U | U | C | C | C | U |
| claude | U | U/U/U | U | U | C | C | C | U |
| codex | E3 | E3/E3/E3 | E3 | E2；以 config 同步 | E3：本例 0 | C；本例未调用 | C；本例未调用 | E3：readOnly↔dangerFullAccess |
| codebuddy | U | U/U/U | U | U | C | C | C | U |
| goose | U | U/U/U | U | U | C | C | C | U |
| vibe | U | U/U/U | U | U | C | C | C | U |
| hermes | U | U/U/U | U | U | C | C | C | U |
| snow | U | U/U/U | U | U | C | C | C | U |
| copilot | U | U/U/U | U | U | C | C | C | U |
| cursor | U | U/U/U | U | U | C | C | C | U |
| droid | U | U/U/U | U | U | C | C | C | U |
| gemini | U | U/U/U | U | U | C | C | C | U |
| iflow | U | U/U/U | U | U | C | C | C | U |
| kilocode | U | U/U/U | U | U | C | C | C | U |
| kimi | U | U/U/U | U | U | C | C | C | U |
| kiro | U | U/U/U | U | U | C | C | C | U |
| opencode | U | U/U/U | U | U | C | C | C | U |
| openclaw | U | U/U/U | U | U | C | C | C | U |
| pi | U | U/U/U | U | U | C | C | C | U |
| qoder | U | U/U/U | U | U | C | C | C | U |
| qwen | U | U/U/U | U | U | C | C | C | U |
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

2. 普通 Gemini 带 `--approval-mode yolo --skip-trust`，Qwen 带 `--approval-mode yolo`，Qoder 带 `--permission-mode bypass_permissions`；托管 Gemini/Qwen 初始 argv 不带这些宽松参数。全部都是启动源码事实，当前无法证明参数与会话完全访问开关联动。

3. `AcpClient::with_full_access` 设置独立布尔值；`build_event_client` 将它传给文件处理器，终端处理器及标准审批使用 `permission_mode` 等字段。`resolve_permission_request_with_prompt` 的 ApproveAll 选择允许选项，缺少允许选项时退到第一个选项；取消中的会话直接取消。后续需验证没有允许项和拒绝项优先级，不能据此声称所有请求均批准。

4. `drivers::capabilities` 仅对 Darwin arm64、`verified_release` 非空且必要托管制品可用的组合发布 session_model；`AcpLaunchProfile::new` 非 Unix 明确拒绝。这属于模型来源门控，不证明完全访问已生效。不得将 macOS 的既有模型证据外推到 Linux/Windows。

5. 相邻 `agent_registry_tests.rs`、`managed_runtime_tests.rs`、`client/event_client_tests.rs`、`permissions_tests.rs` 与各 `<key>_tests.rs` 是后续聚焦测试入口。本任务未运行这些测试；现有 fixture 不替代外部 CLI 真实验收。

## 初始静态验收（任务 1）

已从注册表逐项人工核对上述 22 个 key，并用独立集合检查确认无遗漏/重复；逐项读取驱动入口，核对普通、本地候选、托管启动及显式拒绝。执行 `git diff --check` 和文档链接/来源哈希检查通过。未运行 Cargo（按任务的文档静态审计范围），未运行真实权限会话。下一步按后续任务实现共享策略映射，再逐后端积累 E2/E3。
