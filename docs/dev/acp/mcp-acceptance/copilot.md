# GitHub Copilot 动态挂载 RustCodeGraph 验收

## 当前验收结论

按更新后的标准，允许 Copilot 原生工具在原权限下共存，并要求挂载工具实际执行和回传结果。以 1.2.11 限定索引根组的真实三轮判定通过。下文保留旧标准下的阻塞记录，供对照。

- 挂载会话 `ses_1f0f8c31d00105jW7opKvnh49K` 中，`rustcodegraph_query` 成功 28 次、`rustcodegraph_callers` 成功 2 次、`rustcodegraph_node` 成功 44 次。74 次均有相同调用 ID 的开始和成功完成回执，答复引用了工具回传的 `mountRueSiteApp`、`useApp` 和 `_$createApp` 源码位置。另有两次原生 grep，沿用原权限。
- 无挂载与撤除轮均以 `chat.done` 结束，只答 `MCP_NOT_AVAILABLE`，没有工具调用。三轮均无聊天错误或模型恢复。会话绑定为 `acp-copilot / copilot`，运行中读回原指定阿里模型及 `client_only` 权限。
- 显式工具白名单仍可由已有项目配置选择。通过组使用原生工具默认行为和限制为三项的 MCP 服务目录。本次仅按新标准审核原始回执并更新记录。

日期为 2026-11-19。使用已有 `shiju 0.6.0` 网关构建在私有回环实例上验收，二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。实例只复制了指定阿里提供商的配置及宿主授权，并启用了内置 `acp-copilot`。Copilot 使用私有配置目录内的受管发行版 1.0.105，启动参数为 `--acp --stdio --no-auto-update`。核对了现有受管补丁的 `cli-main.js` SHA-256 `1f8978f33a7a54105087ab9e66313a4b973dd852e198ab85b39c3de44959b502`。会话使用独立的 `COPILOT_HOME`，模型经宿主回环中继走 OpenAI Chat Completions，阿里上游凭据只供网关使用。

RueJS 副本取自 `main` 的已提交源码 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引了 3478 个文件，包含 `app/app.tsx:32` 的 `createRueSiteApp`、`app/app.tsx:39` 的 `mountRueSiteApp` 及 `packages/runtime/src/compiler-runtime/app.ts:57` 的 `_$createApp`。

## 请求与会话

所有真实轮次沿用 [可复用的验收请求](../ACP验收.md#可复用的验收请求)。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。每轮执行 `curl -sS -N --max-time 300` 并用同一请求体创建新会话。租户为 `10001 / 100010005`，使用 `acp-copilot / copilot`、`session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 及 `full_access=true`。

| 轮次 | 会话 ID | HTTP / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef7ec8c4001HTGLznumHhlRxJ` | 200 / 7.887 | `chat.done`，`MCP_NOT_AVAILABLE`，0 个工具事件 |
| stdio 挂载 | `ses_1ef7f4a40001Nln4mYGElPH4wc` | 200 / 7.761 | `chat.done`，`MCP_NOT_AVAILABLE`，0 个工具事件 |
| 移除 stdio | `ses_1ef7f68d7001aDQCGOp1ofNEde` | 200 / 7.015 | `chat.done`，`MCP_NOT_AVAILABLE`，0 个工具事件 |

最初的环境探针在启动 Copilot 前触发受管发行校验。将已安装的受审发行复制到私有实例的受管运行目录后通过该校验。表中的三轮均成功启动 Copilot。各轮 `ui/summary` 与 `acp-options` 返回 HTTP 200，确认绑定、完全访问请求、指定选模和发行版本。完成后连接释放。

## stdio 拒绝与 HTTP 路径

stdio 挂载轮的宿主日志确认 RustCodeGraph 初始化成功。使用同一受管入口、隔离的 `COPILOT_HOME`、仅含占位令牌的回环模型和同一 RustCodeGraph 服务重放原生 ACP 握手。ACP v1 宣布 `mcpCapabilities.http=true` 和 `sse=true`。携带 stdio 服务的 `session/new` 返回会话，但原生日志记录 `Rejecting non-http/sse MCP server "ruejs_graph_probe" from client`。随后的 `session/prompt` 以 `end_turn` 结束，回环收到的模型工具表只有 Copilot 内置工具。断点位于原生 ACP 服务接纳，发生在模型工具声明之前。[GitHub Copilot CLI ACP 文档](https://docs.github.com/en/enterprise-cloud@latest/copilot/reference/copilot-cli-reference/acp-server)

HTTP 轮在副本的 `.vwacprc.json` 使用 `type=http` 和一次性 Bearer 请求头，目标为只绑定 `127.0.0.1` 的 Supergateway 4.2.2 Streamable HTTP 端点。桥接进程在私有目录本地安装和启动，通过 stateful 模式把请求交给同一个 RustCodeGraph stdio 程序，只公开提示指定的三项只读工具。未鉴权的探针返回 HTTP 401。模型绑定、提示与访问参数沿用 stdio 轮，Copilot 的原生工具权限保持原设定。[Supergateway 配置文档](https://github.com/supercorp-ai/supergateway/blob/main/README.md)

首次 HTTP 挂载轮 `ses_1ef829101001TDNGVRHPzbs1tP` 返回 HTTP 200、curl 0、203.418 秒，收到一个 `chat.done)，无 `chat.error`。开始与完成事件按调用 ID 匹配，共 92 次调用，均属于 `ruejs_graph_probe`。其中 query 成功 53 次、callers 成功 8 次、node 成功 28 次及失败 3 次。以下为真实完成回执中的代表结果。

| 工具 | 调用 ID | 核对结果 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_07025645f95f4dcbbfe4b0bd` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_3344cfdccb884f1a8ca5444a` | 调用者为 `mountRueSiteApp`，位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_7edc254f306d4344b5a79b23` | `packages/runtime/src/hooks/useApp.ts:4` 显示 `useApp` 将根工厂传给 `_$createApp` |

三次失败来自猜测的 `app/i18n.ts`、`app/router.ts`、`app/router.tsx`。后续成功读取真实的 `app/i18n/index.ts` 和 `app/router/index.ts`。答复追到了运行时工厂，正确说明 SSR 跳过浏览器挂载、缺少 `#app` 会抛错、路由与 i18n 在执行根组件前安装，以及 `/rust-layers` 绕过站点布局。快照残留分析源于源码推断。其中第二个回归场景直接调用 `createRueSiteApp(...).app.mount(...)`，绕过了 `mountRueSiteApp` 的清空步骤，因而不能检验所声称的有无清空差异。

HTTP 挂载期间的 `acp-options` 读回 `connected=true`、`requested=true`、`support=client_only`，完整模型绑定与请求相同。真实轮次未调用原生文件或 shell 工具，工具审批的原生免审批行为未验证。撤除配置后保持桥接进程在线，使用新会话 `ses_1ef85ac25001M4xa1HSoS3jFlk`，HTTP 200、curl 0、5.859 秒，收到一个 `chat.done`，回复 `MCP_NOT_AVAILABLE`，工具事件为零。两轮的 summary/options HTTP 200。

HTTP 挂载复跑 `ses_1ef85fbd8001fI186hiuSJmQVo` 返回 HTTP 200、curl 0、191.023 秒，收到一个 `chat.done`，无 `chat.error`。成功数为 query 46、callers 4、node 26。另有 9 个 node 失败和 1 个 query 被原生工具目录保护拒绝，报告 MCP 服务器指令在调用前发生变化。复跑仍通过三种工具取得源码，提出静态首屏与切路由两种验证场景。桥接日志分别记录宿主 `rmcp` 和 `copilot-cli 1.0.105` 初始化各自的 MCP 会话。再次撤除后的 `ses_1ef88e694001kKAz29aXDY0NWO` 返回 HTTP 200、curl 0、7.655 秒，收到一个 `chat.done`，回复 `MCP_NOT_AVAILABLE`，零工具事件。复跑两轮 summary/options 均 HTTP 200，绑定与权限读回一致。

## 结论

Copilot 1.0.105 的动态挂载可走原生 HTTP MCP，三种真实工具均完成并返回结果，撤除后新会话恢复无工具。两次挂载均包含工具失败，按计划的零失败合同保留为已阻塞。保留 Copilot 的 stdio 拒绝与现有审批，采用已验证的 HTTP 路径复验时应同时核对工具目录稳定性及两项回归场景。生产侧无改动，未执行 Cargo 或定向单元测试。

私有网关和桥接进程均退出 0，监听端口已关闭，已清理授权副本与临时挂载，并复核原网关仍健康。原始响应、日志和 Copilot 会话记录保存在本机 0700 私有证据目录。

## 共享工作区复查与 SSE 复跑

以共享提交 `76db6b6b7` 复核 Copilot 专属驱动、受管补丁、受管清单的 Copilot 条目及 ACP 转发源文件，均与前一轮相同。复用同一已核验网关二进制与 Copilot 1.0.105，重新从同一个 RueJS 提交提取私有副本并初始化索引。使用相同提示、租户和模型，换用 Copilot 原生声明支持的 SSE 传输。Supergateway 4.2.2 与宿主授权分别使用独立的私有主目录。SSE 桥接仍运行原 RustCodeGraph stdio，限制为同三种只读工具，未鉴权请求返回 HTTP 401。桥接观察到 `rmcp 0.8.5` 和 `copilot-cli 1.0.105` 分别完成初始化。

| 轮次 | 会话 ID | HTTP / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1eff077a0001wW7ljtU4cjoaDE` | 200 / 5.464 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| SSE 挂载 | `ses_1eff0f73b001QxGPq6NcnldkzA` | 200 / 202.738 | `chat.done`，121 个工具完成事件，3 个失败 |
| 撤除 SSE | `ses_1eff40f5d001cVEthNygcy271j` | 200 / 5.145 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

三轮 curl 退出 0，均无 `chat.error`，summary/options 均为 HTTP 200。挂载运行中确认 `full_access=true` 请求对应 `client_only`，原模型和会话绑定正确。撤除轮运行时桥接仍在线。SSE 挂载没有复现 HTTP 复跑的工具目录变化，且所有工具事件均属于指定服务。query 成功 79 次，其代表调用 `call_cba8f03236de4355bba00284` 返回 `createRueSiteApp` 位于 `app/app.tsx:32`。callers 成功 9 次，`call_531de74023014f2a948263f6` 返回 `mountRueSiteApp` 位于 `app/app.tsx:39`。node 成功 30 次，`call_6929df0054834fc4b7f4cb88` 回读 `packages/runtime/src/hooks/useApp.ts:4` 的 `_$createApp(factory)`。三个失败均来自模型读取实际不存在的 `app/router.tsx`、`app/i18n.ts`、`app/router.ts`。同一 RustCodeGraph 索引查询 `createAppRouter` 返回无结果，独立核对真实定义在 `app/router/index.ts:787`。

答复覆盖四段调用链、SSR、容器缺失、插件安装和布局绕过，并将快照残留作为源码推断，提出普通文档和布局绕过两项可区分的断言。第二项需要准备带旧布局的 `/rust-layers` 快照夹具，未实际运行。三种传输均已有证据。客户端 stdio 被原生拒绝，两条远程传输均调通真实工具，指定模型仍在两条远程路径产生同类无效输入。保留已阻塞。收尾条件是同一个指定模型在原提示下完成一次无工具失败的挂载轮，或由验收负责人明确允许将已恢复的无效路径调用单列评估。当前证据没有需要修改 Copilot 驱动或受管补丁的转发缺口。

私有网关及桥接均退出 0、端口关闭，临时授权与挂载配置已清理，原网关复查健康。留存私有 SSE、工具回执及会话日志。本次仅更新验收文档，未运行 Cargo。

## RustCodeGraph 1.2.11 与分页声明复验

复用已审 Copilot 1.0.105、Supergateway 4.2.2 的私有鉴权 SSE 入口，以及已有共享 ShiJu 0.6.0 构建（SHA-256 `2e84923a46704187dfd9d56dfc4b45d6ebf9cfc93398a6bb3a82b1cc61a1f935`）。RustCodeGraph 升至已核验发行版 1.2.11（SHA-256 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`），以同一 RueJS 提交重建索引，查询命中 `createAppRouter`（`app/router/index.ts:787`）。复用 Goose 已验证的私有目录筛选及真实入口、分页和索引根元数据。使用原名和原生工具参数，既有 `allowedTools` 与桥接目录均只选择三种目标工具。协议对照确认目录从八项缩至三项，同样的 query、callers、分页 node 返回逐字相同，额外工具和副本外读取被拒绝。不存在路径的原生结果是 `No indexed file matches` 文本且无 `isError`，原样转发，验收按失败语义核对。未鉴权的桥接访问返回 HTTP 401。 [共享发行证据](pi.md) · [工具目录与分页证据](goose.md)

首组三轮均为 HTTP 200、curl 0、一个 `chat.done`，无 `chat.error` 或模型恢复。无挂载 `ses_1f0f58f21001nwjuzz0s7lQPbq`（7.267 秒）与撤除 `ses_1f0f887f4001T0FTqdS0HpXjaR`（5.660 秒）均只答 `MCP_NOT_AVAILABLE`，工具事件为零。挂载 `ses_1f0f6ca8c001Bo41ZunG9KbXXs`（113.986 秒）有 49 个完成回执。模型将 `packages/i18n` 误作独立索引根，query 明确失败（`call_fa96d9bc48434d1c975362c4`）。随后读取存在但未收录的 `index.html`，原生返回 `No indexed file matches`（`call_51ed733cdf8c4cb79aeb37d3`）。另有 47 次取得有效工具结果，未出现旧版的猜测路由文件错误。源码显示，首组答复的仅含占位节点夹具可以断言旧节点是否残留，不能据此断言重复的页头或页脚。

第二组在同一私有工具声明中将可选 `projectPath` 的枚举限定为唯一索引根，并说明 1.2.11 索引不包含 HTML。复用原生工具结果和原有范围检查，既不重写实参，也不改错误。协议对照仍通过。

| 轮次 | 会话 ID | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0f8a7bb0019Eb3nS1rHgxXy8` | 200 / 0 / 5.754 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1f0f8c31d00105jW7opKvnh49K` | 200 / 0 / 156.229 | `chat.done`，74 个目标工具和 2 个额外的原生 grep |
| 撤除 | `ses_1f0fb25a50013F0fk8u7Zx5dMK` | 200 / 0 / 5.657 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

前后两组 summary/options 均为 HTTP 200，挂载运行中为 `connected=true`、`client_only`，绑定同一指定模型和 `full_access=true`。第二组 76 个调用的开始与完成逐一匹配。目标工具分别成功 query 28 次、callers 2 次、node 44 次，无工具错误或缺文件结果。`call_615e6c0587384e1984c12b62` 定位 `createRueSiteApp`（`app/app.tsx:32`），`call_9d63391f1e124f8d8377e27b` 返回 `mountRueSiteApp`（同文件第 39 行），`call_3080a30c6f224876aaf81929` 与 `call_60446d7c3e8a47e98a53a221` 回读 `useApp` 与运行时工厂。答复覆盖原要求的链路、分支和两项预测场景，也指出路由表缺少 `/rust-layers`。核对 `install` 时调用了 Copilot 自带 grep 两次（`call_9c7f216552a142af859599fd`、`call_7fa00429b77c49f2aef4e8e1`），违反三工具约束。答复亦明确报告这两次读取。

仅作诊断，用同一受审 Copilot 和真实 SSE 服务对照回环模型的工具声明。默认同时声明原生工具与三个 MCP 工具。进程使用原生 `--available-tools` 精确列出三个会话工具时，模型请求仅声明这三项。该选项探针使用占位模型令牌，未改变真实验收启动参数或审批。现有单服务器 `allowedTools` 约束 MCP 目录，Copilot 启动配置没有经验证的请求级原生工具白名单入口。接入需将现有会话的显式工具策略传到 Copilot 受管启动边界，涉及共享接线，需先取得授权并验证生命周期与撤除。当前两组各有实际合同失败，保留已阻塞。私有网关和桥接均退出 0，授权和临时挂载已清理。未改生产代码或执行 Cargo。

## 受管启动白名单来源复查

原始请求只在 `options` 中设置 `cwd` 和 `full_access`，没有显式的会话级工具白名单。网关可接收 `acp_allowed_tools`，但这条聊天路径的 `session-engine` 在进入 ACP 运行时前无条件用宿主工具注册表计算结果覆盖该字段。该值包含宿主工具 ID 和别名，也会在没有显式白名单时生成。到了受管启动边界，Copilot 无法从这个值区分调用方明确选择的原生工具与默认宿主工具集合。现有 ACP 元数据把该字段放在 `claudeCode.options.allowedTools`，未形成 Copilot 原生工具声明。

`.vwacprc.json` 的 `allowedTools` 属于单个 MCP 服务的目录策略。把该名单直接作为 Copilot 进程的全会话 `--available-tools` 会额外隐藏原生工具及其他服务的工具，即使调用方没有请求全会话限制。上轮回环已实测这种差异。默认工具表同时有原生工具与三项挂载工具，显式进程参数则只声明三项。该回环证据证明启动参数有效，但不足以授权转换两种策略。

保留已阻塞。下一步需要共享契约明确保留并传递显式会话级限制及其原生工具名称，或者同一指定模型按原请求完成只调用三项工具的挂载轮。只改 Copilot 受管启动不足以提供该来源。本轮未改运行时代码、未运行 Cargo，也未重复发送已有反证的等价验收请求。

## 项目级受管启动补丁

复用现有项目 ACP 启动配置 `acp.copilot.args`。固定入口仍须使用 `--acp --stdio`，只新增对重复 `--available-tools=<字面工具名>` 的识别。工具名由 ASCII 字母、数字、下划线和连字符组成，不能以连字符开头。空值、模式、分隔符及其他 CLI 开关继续拒绝。空过滤表沿用原启动参数。该配置筛选模型可用工具，审批仍沿用原生与宿主现有路径。

在私有项目用现有配置接口设置三项已核验的工具名。原构建读回同一项目覆盖后，原请求 HTTP 200、curl 0，以 `chat.error` 结束，错误为 `Copilot managed launch requires verified ACP arguments`。定向命令 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_model_source_copilot_` 在槽位 3 中复现一项失败、五项通过，失败正是显式过滤参数被拒。槽位 3 的 target 与 build 目录均为仓库的 `target/rust-slot-3`。

补丁网关复用共享槽位 3 本轮构建的私有副本，SHA-256 `25988bf02e238fe5450ca7c941015eaed1c067f5c64123c85aeacf1e2454734a`。新网关的公开 DTO 拒绝旧请求的 `acp_agent` 字段，实测 HTTP 422，尚未建 ACP 会话。依据当前共享验收入口，后续请求仅删除顶层 `acp_agent`，由 `agent=acp-copilot` 绑定原后端。提示摘要、模型、租户与访问参数一致。

先保留私有项目白名单跨越前后对照。挂载 `ses_1f16a6419001LLA5TDrug4zX0G`（HTTP 200、curl 0、189.082 秒）收到一个 `chat.done`，86 个工具回执均属于目标服务，未复现原生 grep。其中调用 `call_435d95e028a748c592b4ae12` 的返回是 `No indexed file matches`，因为 1.2.11 不索引 `packages/rue/package.json`，按工具失败计。无挂载和撤除均未调用工具，Copilot 将未知白名单工具提示放入可见 SSE，不能算精确标记对照。后续组将私有项目白名单和挂载一起启用，撤除时恢复默认启动参数，并据实际索引元数据说明 HTML 和 JSON 不在索引内。

| 私有项目覆盖 | 会话 ID | HTTP / curl / 秒 | 终态及工具 |
| --- | --- | --- | --- |
| 无挂载，默认启动 | `ses_1f16d6e58001dNK6RZe372daf5` | 200 / 0 / 5.779 | `chat.done`，只答 `MCP_NOT_AVAILABLE`，零工具 |
| SSE 挂载，显式白名单 | `ses_1f16d8805001faaOLJNrj8Yfmu` | 200 / 0 / 178.674 | `chat.done`，71 次成功，2 次目录保护拒绝 |
| 撤除 SSE 与覆盖 | `ses_1f170450d001AA9aUKpkCX9617` | 200 / 0 / 5.715 | `chat.done`，只答 `MCP_NOT_AVAILABLE`，零工具 |

三轮 summary/options 均为 HTTP 200，网关确认默认与挂载时的参数数目分别为 3 和 6。挂载连线确认 Copilot 1.0.105、同一指定模型、`full_access=true` 及 `client_only`。回执逐一匹配且仅包含三种目标工具。query 成功 30 次，`call_d281e369730f4dd99e05a4c9` 命中 `createRueSiteApp`（`app/app.tsx:32`）；callers 成功 4 次，`call_5bab72852d3143509bb6099f` 命中 `mountRueSiteApp`（同文件第 39 行）；node 成功 37 次，`call_bcc398a8dec943789fefd5e8` 与 `call_b48d80914add41b3bfc2b539` 读回 `useApp` 与运行时工厂。答复覆盖主链、布局分支及快照预测。两个额外预测分别使用旧节点夹具与已注册页面，未执行浏览器验证。

失败调用 `call_b11591fefca946ef96a04b16` 和 `call_112b4bdbdfbf4ac9a6e76a47` 发生在原生调用前，报 MCP 工具目录因服务器指令变化而拒绝。Copilot 进程日志同一时段记录重建工具与提示上下文。拒绝后相同目标查询仍可成功，且未调用原生 grep。本补丁解决工具范围，严格验收仍需要目录稳定的一轮。未改变原生目录校验。两次私有网关均退出，桥接终止，授权及临时配置已清理。4108 个源文件与固定提交一致，原网关保持健康。

另以同一补丁构建进行三项私有项目反向验证。`--available-tools=*`、`--allow-all`、空 `--available-tools=` 均在启动 Copilot 前返回原受管参数错误，无工具事件或成功终态。测试项目覆盖和授权副本随后清理。

补丁后的 `llm-runtime --lib` 测试目标由共享槽位 6 编译，私有副本 SHA-256 为 `105662b97368e9a7d2a4d1dee5b0afd3d65e2acd5f318f8b169fd57c306f909d`。通过槽位入口以原过滤条件运行，同一目标默认并行执行 5 通过、1 失败，原因是两个测试复用了同一历史目录。单线程重跑 6 项全部通过。随后为新增测试分配独立会话标识，仅修改测试夹具。同仓仍有其他重型编译，未再次编译该夹具，运行时补丁已用共享构建完成上述真实正反验证。
