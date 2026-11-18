# Dirac 动态挂载 RustCodeGraph

2026-11-19，验收已阻塞。Dirac 0.5.24 的 ACP 建会话入口忽略 `mcpServers`。宿主现有门禁在发出 `session/new` 前明确拒绝非空列表。

## 环境与请求

- Darwin arm64，隔离回环网关复用已验收的 ShiJu 0.6.0，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。本机 Dirac CLI 0.5.24 的入口 SHA-256 为 `c88e043f590a520da3345050e32bbe6930a68cefad60945ef4a7d14648217429`，与此前 Dirac 验收吻合。
- RueJS 使用 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 干净副本。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32`。挂载轮在副本配置 `ruejs_graph_probe`，使用原生 stdio 和 `--no-watch`。
- 三轮以同一个 curl 请求另建会话，沿用租户 `10001 / 100010005`、`session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 与 `full_access=true`。预设为 `acp-dirac`，后端为 `dirac`。提示来自 `189d22dc1`，UTF-8 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态 | 答复与工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1effcbcdf0015hMeKPoaa5fuon` | 200 / 0 / 5.572 | 一个 `chat.done` | 包含 `MCP_NOT_AVAILABLE`，另有开场文字。零 MCP 事件，原生技能列表与 `respond` 共 8 条工具状态事件。 |
| 挂载 | `ses_1effcd394001UdGcYrayHZ8RiM` | 200 / 0 / 1.525 | 一个 `chat.error` | 零工具事件。 |
| 撤除 | `ses_1effcdac6001zeiF3MvEX2aCVt` | 200 / 0 / 3.867 | 一个 `chat.done` | 可见答复为 `MCP_NOT_AVAILABLE`。零 MCP 事件，原生 `respond` 有 3 条状态事件。 |

挂载轮先收到 `chat.model_recovery`，`phase=stopped`，再收到 `chat.error`。两条事件均指向 `Dirac does not consume ACP MCP servers; session-scoped MCP is unavailable`。模型未收到工具声明，三种工具均无调用和输出，答复未能覆盖调用链、分支、快照影响和验证场景。两组对照均确认服务不可见。无挂载轮的额外文字和技能列表调用违反原提示，撤除轮也有 Dirac 原生完成事件。

三轮 UI summary 均确认 `acp-dirac / dirac` 与 `full_access=true`。两组对照运行中 ACP options 确认 `connected=true`、指定模型和 `support=client_only`，终态连接释放，模型绑定仍可回读。挂载轮 options 无有效连接、模型选择或权限状态。真实 MCP 审批缺少可达的工具调用。

## 原生断点与复跑条件

- 以独立状态目录、无上游凭据启动已安装的 `dirac --acp`。握手报告 ACP v1、Dirac 0.5.24、`loadSession=true`，未声明 MCP 能力。分别发送 stdio、HTTP 和 SSE 的 `session/new`，三次均返回会话。stdio 启动标记未生成，HTTP 和 SSE 的独立 MCP 回环监听器收到零连接。诊断模型中继捕获的三个首轮工具列表均只有 Dirac 原生工具，没有 RustCodeGraph 声明。该探针在捕获请求后返回固定诊断错误。
- 固定版发行包的 `DiracAgent.newSession` 使用 cwd 和会话选项创建运行时，未使用 `mcpServers`。直接检查 npm 0.6.1 发行包的同一入口，仍未消费该字段。0.6.1 仅做静态检查。[官方 CLI 文档](https://github.com/dirac-run/dirac/blob/master/cli/README.md) · [npm 0.6.1 发行](https://www.npmjs.com/package/dirac-cli/v/0.6.1)
- 独立 MCP 连接中，`rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 返回 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。三项复核均成功。初次独立探针以 `query` 误传查询参数，按 MCP schema 改用 `search` 后成功。Dirac 没有读取这些独立结果。
- 保留 `validate_mcp_contract`、Dirac 驱动及既有审批策略。复跑需先取得支持会话级工具注册和宿主审批的 Dirac 入口，再验证真实工具闭环、撤除对照以及审批回归。

定向命令 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_dirac acp_onboarding_dirac_rejects_ignored_mcp_servers` 退出 0，1 项通过、4 项被过滤。使用槽位 6，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库的 `target/rust-slot-6`。审批实现和断言无改动，本轮未单独复跑权限用例。

私有网关、原生进程及回环监听器均已退出。私有授权副本和临时挂载文件已清理。原始证据留在本机 0700 验收目录，用户网关与 RueJS 原目录未写入。

## 共享工作区复验

使用当前共享工作区已有的较新 ShiJu 0.6.0 构建，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。重新使用独立网关和 RueJS 副本，网关实际启动的 Dirac 仍为 0.5.24。请求内容、租户、传输及模型沿用首轮。

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态与输出 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0011541001UbjjdhXM6LMaUY` | 200 / 0 / 6.208 | `chat.done`，可见答复为 `MCP_NOT_AVAILABLE`。 |
| 挂载 | `ses_1f0012ed5001Fk7am1cEEPy2YK` | 200 / 0 / 1.625 | `chat.error`，原门禁错误及 `phase=stopped` 重现。 |
| 撤除 | `ses_1f0013617001av2JdbZFAMHIRF` | 200 / 0 / 4.588 | `chat.done`，可见答复为 `MCP_NOT_AVAILABLE`。 |

两个对照均有原生 `respond` 的开始、更新和完成事件。三轮目标工具事件为零，挂载轮所有工具事件为零。三轮 UI summary 均回读正确绑定和 `full_access=true`。对照轮在线确认指定模型和 `client_only`，完成后连接释放且模型可回读。挂载轮全程未建立有效连接，选模和审批均无结果。

- 在私有目录安装 npm 0.6.1，入口摘要为 `9d4b628a77082f661487c14f539cc463fa500dfdd61eb06b330a73519700be5e`。对最新版动态补测三种传输。ACP v1 三次均成功建会话，未声明 MCP 能力，stdio 启动标记缺失，独立 HTTP/SSE 监听器的 TCP 连接数为零。回环模型的三份工具表均仅含 17 个原生工具。此处验证了此前静态检查的实际后果。上游当前源码也仅在 resume 入口将 `mcpServers` 转传给 `loadSession`，后者未消费列表。[Dirac ACP 源码](https://github.com/dirac-run/dirac/blob/225d7cf86d1bd500352b0ef4875b86a1a17b6622/cli/src/agent/DiracAgent.ts)
- 0.5.24 和 0.6.1 的 `--acp --only-tools` 均在握手前退出 1，提示工具选择参数只用于普通 CLI 任务。0.6.1 的私有配置加载了一个无副作用的自定义标记工具。该工具同时出现在有挂载和无挂载的两个 ACP 会话的模型工具表中，表明工具目录与配置档绑定，无法由 `mcpServers` 注册或撤除。未修改用户工具、工作区或全局配置。[CLI 参数处理](https://github.com/dirac-run/dirac/blob/225d7cf86d1bd500352b0ef4875b86a1a17b6622/cli/src/index.ts)
- 核对仓库内的受管路径。Snow 补丁依赖 Snow 已有的 MCP 客户端、会话字段及调用循环。OpenClaw 转换依赖其私有 Gateway 原生 MCP，且原生调用不回调 ACP 宿主审批。Dirac 的私有中继只绑定模型和状态目录，缺少这两条路径所需的注册和回传接口。沿用私有自定义工具需要新增会话隔离、工具转换、逐次审批及撤销接线，超出任务卡要求先确认原生 Dirac 消费列表的边界。
- 当前分支及工作区中的 Dirac 驱动、协议门禁和聚焦目标相对前次提交无变化，复用此前 1 项聚焦测试通过证据。本次复验未再编译。私有网关与原生探针均退出，挂载文件及私有授权副本已清理，用户网关健康检查通过。

仍为已阻塞。解除条件为可安装的 Dirac ACP 入口实际消费会话 MCP 定义，并经宿主审批完成调用及撤除验证。若改用新建受管适配器，需先扩展任务卡范围，明确工具注册、宿主审批和会话撤销的交付要求。
