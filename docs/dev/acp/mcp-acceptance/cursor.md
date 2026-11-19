# Cursor 动态挂载 RustCodeGraph

## 环境

2026-11-19 在 Darwin arm64 使用 [固定验收请求](../ACP验收.md#可复用的验收请求) 的 `189d22dc1` 版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。网关使用租户 `10001 / 100010005`、`acp-cursor / cursor`、`full_access=true` 与 `session_model`，模型为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。RueJS 使用 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的私有干净副本，RustCodeGraph 1.2.10 索引 3478 个文件。

Cursor 固定发行版为 `2026.11.11-b912975`，ACP 握手返回协议 v1，声明支持 HTTP、SSE MCP 和会话加载。验收采用原 stdio RustCodeGraph 和 `--no-watch`，由 `session/new.params.mcpServers` 传入会话级服务 `ruejs_graph_probe`。

## 原版网关三轮

使用 `shiju 0.6.0`，网关摘要 `ac9b96bce8a3aee0cb6102dc98a9b365efdfccfe0cddc4901df7860685b56877`。固定发行冷安装在 120 秒时超时，随后复用已有受审查 v1 发行目录。首次私有网关三轮均在选模处报错。重新在私有实例配置同一租户、模型与授权后，取得下列证据。

| 轮次 | 新会话 | HTTP / curl / 耗时 | 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef9d8d100012mbkBIBZuf642W` | 200 / 0 / 17.714 秒 | 一个 `chat.done`，正文为 `MCP_NOT_AVAILABLE` |
| 挂载 | `ses_1ef9dd3bc001OYpVEUXvyxtZr0` | 200 / 0 / 12.328 秒 | 一个 `chat.error`，`Cursor session model request failed` |
| 撤除 | `ses_1ef9e055e001aSHiWMd8uhpITk` | 200 / 0 / 16.674 秒 | 一个 `chat.done`，正文为 `MCP_NOT_AVAILABLE` |

三轮均无工具事件。UI summary 回读了指定的代理、后端及 `full_access=true`，ACP options 回读了指定模型及来源，结束后连接已释放。独立协议探针确认挂载后 `session/new` 成功，并在 `session/prompt` 收到 ACP `-32603`。该探针在报错时发出的模型请求为零。首个失败边界为受管 `AgentClient.run` 对非空 MCP 目录的拒绝。私有网关正常退出，端口已关闭。

## 适配及原生回归

受管补丁从当前 ACP 会话的原生 MCP 目录发布模型工具定义。调用使用目录里的服务与工具标识，交给 Cursor 原生执行器和审批，结果通过既有 Chat 工具续轮与 Cursor protobuf 历史回传。ACP 工具事件带原生结果，并标记服务错误与拒绝。注册表将新版安装到独立的 `model-source-v2`，以保留已安装的 v1 目录。私有副本应用固定补丁后，351 个运行文件的摘要均与更新后的清单匹配。

| 工具 | 输入 | 原生结果节选 |
| --- | --- | --- |
| `rustcodegraph_query` | `search=createRueSiteApp` | `createRueSiteApp`，`app/app.tsx:32` |
| `rustcodegraph_callers` | `symbol=createRueSiteApp` | `mountRueSiteApp`，`app/app.tsx:39` |
| `rustcodegraph_node` | `name=useApp`，`file=packages/runtime/src/hooks/useApp.ts` | 第 4 行调用 `_$createApp(factory)` |

真实 RustCodeGraph stdio 配合回环模型逐项调用并取得三次 `allow_once`，三个调用都有原生开始、完成以及结果事件。续轮模型请求包含对应的工具调用 ID 及输出。另一轮逐项返回 `reject_once`，三个工具事件均为失败，续轮收到对应拒绝结果。两组测试都加载同一历史并传入空 MCP 目录，旧工具结果可重放，当前模型工具目录只有 `shell`。既有原生脚本的 shell 批准和拒绝回归均退出 0，覆盖实际写入、拒绝后无文件及历史恢复。

定向命令 `node --test patches/acp-model-source/cursor/session-model.test.cjs` 在历史回放断言下由 4 通过、1 失败（退出 1）变为 6 通过（退出 0）。公共 ACP 转发边界未改动，因此未运行其 Rust 单元测试。

## 指定模型原生诊断

另外使用同一提示和指定阿里模型，经私有回环诊断中继直连新版 Cursor ACP，原始上游凭据只留在中继。该诊断取得真实模型和工具证据。网关 HTTP、SSE 与租户权限仍需用新版网关单独验收。

| 轮次 | 原生会话 | 终态与结果 |
| --- | --- | --- |
| 无挂载 | `da513948-66d8-44da-b0a4-81563d5f6fff` | `end_turn`，无工具，正文在 `MCP_NOT_AVAILABLE` 后附加说明 |
| 挂载 | `7a4da600-5e40-40f5-8765-c7cc78fed7ce` | 64 步后 ACP `-32603`，无完整答复 |
| 撤除 | `5fefcd45-d223-4209-9904-32fc082538aa` | `end_turn`，无工具，正文为 `MCP_NOT_AVAILABLE` |

真实挂载轮对三种指定工具均取得成功结果，并在续轮模型请求中看到对应输出。共 59 次工具成功、5 次失败，其中 2 次为诊断客户端拒绝额外的 `rustcodegraph_explore` 和 `rustcodegraph_files`，3 次为服务返回错误。第一轮诊断也在 64 步后报错。模型未给出三部分答复，无法核对答复准确性。原生输出、ACP 轨迹、网关基线 SSE 与回读仅保存在本机私有目录。原 RueJS 工作区和真实租户未收到配置写入。

首轮执行时新版网关构建排队未启动，未领取 Cargo 槽位。后续使用包含适配的共享构建复验。

## 新版网关复验

从共享槽位 3 复制现有 `shiju 0.6.0`，SHA-256 为 `06086ba0fc9580bd82fdcf1327696acb6ee648727ed2b054e1ee32d60d8f37d5`。核对二进制内嵌 Cursor 补丁与清单摘要和当前源码一致。三个私有实例均用 ACP 系统安装入口验证受管 v2 副本，HTTP 200。沿用原租户、模型、提示、权限和同一 RueJS 提交的私有干净副本。

第一组使用原 RustCodeGraph stdio。第二组使用已由同批次验收核对的 stdio 转发器，在挂载轮只公布三个指定工具并补全 `node.offset`、`node.limit` 的元数据。第三组再补充 TypeScript 无扩展名导入的通用定位说明。两组转发器均限制调用工具名和副本内路径，其余请求参数和 RustCodeGraph 结果原样传递。各组无挂载与撤除轮均使用独立新会话。

| 工具目录 | 轮次 | 新会话 | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- | --- |
| 完整 | 无挂载 | `ses_1f012bd49001D5D436sY5mRoPD` | 200 / 0 / 15.527 | `chat.done`，标记准确，零工具 |
| 完整 | 挂载 | `ses_1f012fb490013mX18oNnOHUAoS` | 200 / 0 / 145.441 | `chat.error`，62 次成功、2 次失败 |
| 完整 | 撤除 | `ses_1f01534c3001kcI70SWPLhTmA2` | 200 / 0 / 14.960 | `chat.done`，标记准确，零工具 |
| 三工具 | 无挂载 | `ses_1f01345b5001huld0P3XK4bHVY` | 200 / 0 / 16.283 | `chat.done`，标记准确，零工具 |
| 三工具 | 挂载 | `ses_1f01386d00013ys8ptcUKKLa7B` | 200 / 0 / 193.142 | `chat.done`，46 次成功、5 次失败 |
| 三工具 | 撤除 | `ses_1f0167a95001KlJt0eyGGfktRC` | 200 / 0 / 14.549 | `chat.done`，标记准确，零工具 |
| 路径提示 | 无挂载 | `ses_1f01496ce0013a2jRDNUFgEYVb` | 200 / 0 / 15.548 | `chat.done`，标记准确，零工具 |
| 路径提示 | 挂载 | `ses_1f014d4e4001T6Mp14o1Byr4Lq` | 200 / 0 / 151.933 | `chat.error`，61 次成功、3 次失败 |
| 路径提示 | 撤除 | `ses_1f01727bf001p5ErNy0fLtZOhK` | 200 / 0 / 14.602 | `chat.done`，标记准确，零工具 |

三组均通过 SSE 观测到指定工具的同 ID 开始、完成与真实输出。三工具组中，query `call_599d5e6e5f86428791edc4bc` 命中 `app/app.tsx:32`，callers `call_fc061d16956746b299883704` 返回同文件第 39 行，node `call_ccdec1b931ab40f3bebe77e0` 读回 `useApp.ts:4` 对 `_$createApp` 的调用。网关运行时权限报告 `client_only`。在两个限定挂载轮采样审批队列均为空，未发送人工批准。会话回读均保留指定绑定、`full_access=true` 和原模型，终态连接已释放。

完整组还调用 `files` 与 `explore`。三组首个读取失败均为猜测不存在的 `app/router.ts`。在同一干净索引中，`query createAppRouter` 无命中，按真实文件读取 `app/router/index.ts:787` 则取得定义。完整组和路径提示组都在 64 次工具完成后报错，只有三工具组生成完整答复。该答复的主调用链、SSR 守卫、缺失容器抛错、路由和 i18n 注册及布局绕过均与源码相符，也正确指出 `/rust-layers` 未注册。快照保留及 DOM 追加是源码推断。两项回归场景使用预填快照和实际存在的 `/jsx/basic-elements`，其中对应用根元素确切数量的预期仍需实测。

严格验收仍为已阻塞。新版网关已验证原生挂载、结果回传和撤除，剩余失败出在同一真实挂载轮。继续验收需要指定模型完成零失败的调查，或等 RustCodeGraph 改善当前漏索引定义的定位能力后复跑。若要接受三工具组中已恢复的探索性读取失败，需由验收负责人调整合同。

本轮复跑 Node 定向测试 6 项通过，复用共享构建，未运行 Cargo。三座私有网关、RustCodeGraph 和转发器均已退出，端口和临时租户配置已清理。三份源码副本的 4109 个文件和符号链接与归档一致，新增图谱索引除外。原始 SSE、回读和诊断仅保存在本机私有目录。

## RustCodeGraph 1.2.11 复验

复用任务 8 与 Pi 验收使用的 `shiju 0.6.0`，SHA-256 `7f8fb6ab97e9e6b00e6694ce48376e2d50a3ce4b2bff3fff93004b98db824916`。对照当前源码验证二进制内嵌的 Cursor 补丁以及两项运行文件摘要，私有实例的系统安装入口验证同一受管 v2 成功。RustCodeGraph 1.2.11 发行二进制 SHA-256 为 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`。使用同一 RueJS 提交重建 3478 文件索引，并确认 MCP 查询能直接命中 `createAppRouter`（`app/router/index.ts:787`）。

沿用原提示、模型、租户、权限和服务名。限定目录复用 Goose 的 stdio 转发器，包含分页和默认索引根说明。协议对照确认直连与限定目录的路由符号查询、分页读取和缺失文件响应一致。新版的 `node(file=index.html)` 返回 `No indexed file matches`，即使 SSE 将该调用标为完成，也没有读回源码。第二组仅在工具说明中移除 HTML 入口建议并说明这一限制，实际请求与响应仍原样传递。 [Goose 转发器验收](goose.md) · [Pi 发行构建验收](pi.md)

| 工具说明 | 轮次 | 新会话 | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- | --- |
| Goose 元数据 | 无挂载 | `ses_1f104110d001zdslX4Z68qzBf1` | 200 / 0 / 14.983 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| Goose 元数据 | 挂载 | `ses_1f1044ccf001T4I607w6ha0IUu` | 200 / 0 / 197.746 | 64 次工具完成后 `chat.error`，无完整答复 |
| Goose 元数据 | 撤除 | `ses_1f107528b001LChiJBosY7o5OE` | 200 / 0 / 14.777 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 仅索引入口 | 无挂载 | `ses_1f1070bc500159y9gSA0WDrlsg` | 200 / 0 / 15.225 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 仅索引入口 | 挂载 | `ses_1f107487b0019SE5DQCeUJUETE` | 200 / 0 / 242.608 | `chat.done`，58 次成功、零工具失败 |
| 仅索引入口 | 撤除 | `ses_1f10afda20018O23xeUyR9MADx` | 200 / 0 / 14.450 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 仅索引入口再试 | 无挂载 | `ses_1f10c67c1001CXHzHwqppN4QXL` | 200 / 0 / 15.809 | `chat.done`，标记后追加说明，零工具 |
| 仅索引入口再试 | 挂载 | `ses_1f10ca6c3001QX0yRVkAteLGD2` | 200 / 0 / 270.356 | `chat.done`，57 次成功、1 次失败 |
| 仅索引入口再试 | 撤除 | `ses_1f110c815001ZuX3QFTenyqZy0` | 200 / 0 / 14.010 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

首组只调用三种指定工具，query `call_5ada794bcb4848598def6021` 返回 `createRueSiteApp`（`app/app.tsx:32`），callers `call_f0f40d86e694425f9db2bff8` 返回 `mountRueSiteApp`（同文件第 39 行），node `call_72a2c1623f1d4f12bc6f291c` 读回 `useApp.ts:4` 对 `_$createApp` 的调用。首次未读到请求文件的调用为 `call_bd87c2f28f4c4f019ce4c7ac`，目标为 `index.html`。挂载轮收到一次停止恢复事件和一次 `chat.error`。

第二组只有三种指定工具，query 18 次、callers 3 次、node 37 次，均有同 ID 开始和完成事件，无文件缺失输出、模型恢复或 `chat.error`。答复的启动链、SSR 守卫、容器判空、插件注册、布局分支与快照追加推断可回对源码。第二个验证场景误称预填 `#app` 且客户端始终未挂载会等待超时。实际 `waitForStaticAppHtml` 会将旧节点视为有效内容，稳定时间达到 `settleMs` 即可返回旧 HTML（`packages/server-renderer/src/static.mjs:715–740`）。该场景的预期需要先约束挂载与快照采样的先后顺序。

第三组复用相同说明复核答复。模型给出有效的 `/design/button` 和 `/settings` 导航场景，仍有一条 query 调用把 `packages/i18n` 当作独立索引根（`call_df2878dfbf01475695c4eb5f`），原服务返回索引未初始化。无挂载轮也多答了解释。模型已完成零工具失败的挂载轮，但没有在同一组三轮中同时满足工具、对照和答复准确性。任务保留已阻塞。恢复条件是指定模型在该真实索引下完成同组严格三轮，并给出两项准确的验证预期。

各组挂载中回读 `client_only`，权限队列采样为空。九轮终态均释放连接，回读保持同一模型来源及完整模型 ID。未运行 Cargo 或修改产品代码。三座私有网关及子进程已退出，端口、租户副本和临时挂载已清理。三个 RueJS 副本各 4109 个文件及链接与归档一致，新增索引除外。原始证据仅保存在本机私有目录。

## 受管 v3 的有界收尾

Cursor 的受管 v2 在第 64 次工具执行结束后直接报错。v3 继续限制每轮最多执行 64 次工具，在耗尽预算后用真实历史结果发送一次 `tool_choice: none` 的推理请求。适配器仍验证流终态。若模型在这次请求中返回工具调用，适配器报错并终止执行。常规系统说明要求遵守用户给出的精确备用回复，证据充分后收尾，并明确区分可验证预期和推断。验收请求的用户提示、模型及权限参数保持原样。

原生回环夹具使用 `python3 scripts/acp_model_source_cursor_tests.py <受审入口> --step-limit`，v2 在第 64 次结束后以 ACP `-32603` 退出，v3 发送带全部 64 份工具结果的收尾请求并正常结束，会话重载仍可读回结果。追加 `--refuse-final-tool` 的独立夹具让模型返回第 65 个工具调用，观测到 ACP 拒绝且仅有 64 个工具开始事件。既有 shell 拒绝、MCP 允许与拒绝、加载后撤除回归均退出 0。Node 定向测试 6 项通过，私有 v3 候选的 351 项摘要均匹配。

从仓库根目录在槽位 3 执行 `cargo build -j 1 --locked -p vw-cli --bin shiju`，退出 0，耗时 18 分 49 秒，target 和 build 目录均为仓库的 `target/rust-slot-3`。网关 SHA-256 为 `25988bf02e238fe5450ca7c941015eaed1c067f5c64123c85aeacf1e2454734a`。私有系统安装入口 HTTP 200 并验证 v3。该共享工作区的聊天流请求已移除 `acp_agent` 字段，原请求三次均在反序列化处返回 HTTP 422，未建会话。兼容重试仅移除此字段，通过同一 `acp-cursor` 预设选中后端，保留原提示摘要、租户、模型和权限。

| 轮次 | 会话 ID | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f169dc23001afGamKsz7e87vH` | 200 / 0 / 16.645 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f16a1e82001Wz1oUCriiNKTb0` | 200 / 0 / 275.386 | `chat.done`，三种工具共 58 次完成，一条无源码结果 |
| 撤除 | `ses_1f16e53ce001lTEJWPPlC0qjEf` | 200 / 0 / 17.234 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

挂载轮共 20 次 query、2 次 callers、36 次 node，无额外工具及模型恢复。query `call_809c6087c72e4a15b214a93d` 命中 `createRueSiteApp`（`app/app.tsx:32`），callers `call_103e36bc03854bcf854bb4a9` 命中 `mountRueSiteApp`（第 39 行），node `call_05c1e875020643c982b7ca08` 回读 `useApp.ts:4`。答复涵盖启动链、SSR 守卫、容器缺失、路由与 i18n、布局绕过和快照追加推断，并给出两项验证设想。仍调用 `node(file=index.html)`（`call_1c3768aa510744e89c2773f7`），原样返回 `No indexed file matches`，SSE 的完成状态不代表读到了源码。快照等待场景还需要明确清空已执行后才开始采样。

真实模型在第 58 次工具调用后自行结束，本轮未触发第 64 次后的收尾分支，该分支的正反验证来自原生夹具。三轮回读均为 `acp-cursor / cursor`、原模型、`session_model` 与 `full_access=true`。挂载运行中权限为 `client_only`，采样审批队列为空，终态连接释放。两座私有网关正常退出，端口、临时挂载和私有租户副本已清理。任务保留已阻塞，仍需指定模型在同一组三轮中完成零失败工具调查及准确答复。

## v3 的真实边界复验

复用同一 v3 网关及 RustCodeGraph 1.2.11。定向目录仍只有三种工具，明确说明 HTML 未索引，并将可选 `projectPath` 的目录元数据限定为真实索引根。转发器原样传递调用与输出。用户提示、预设、租户、模型及权限保持上一组一致。

| 轮次 | 会话 ID | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f177518e001R5Oien2KunhgBF` | 200 / 0 / 16.263 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f1779262001T8p54Sta4OMKFD` | 200 / 0 / 199.773 | 64 次工具均成功，收尾 `chat.error`，无可见正文 |
| 撤除 | `ses_1f17aa03a001jGRG9gCbZKhnPU` | 200 / 0 / 15.137 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

挂载轮有 23 次 query、4 次 callers、37 次 node，只有三种指定工具，没有缺失文件或工具错误结果。query `call_9642792c679b4d6d95ae2269` 命中 `createRueSiteApp`，callers `call_d22d39fea1d0449898bacbc0` 命中 `mountRueSiteApp`，node `call_aede7d33c5d1428897dd9ac7` 读到 `useApp` 第 4 行对 `_$createApp` 的调用。运行中权限 `client_only`，审批队列采样为空。三轮回读均确认 Cursor 绑定、`full_access=true`、指定模型和 `session_model`，终态连接均释放。

对同一受审 Cursor v3 启动独立本地诊断中继，使用原提示和实际 RustCodeGraph 复现。第 65 个模型请求携带 64 项真实工具结果及 `tool_choice: none`。阿里上游返回 HTTP 200 和完整 `[DONE]`，终帧却为 `finish_reason: tool_calls`。Cursor 拒绝该调用，并以 ACP `-32603` 结束。将同一收尾请求的工具声明与选择字段移除，保留原结构化工具历史后重放，上游仍返回 `tool_calls`，无正文。微型对照的两种请求形态均可收到正常终帧，当前故障无法归为字段不兼容。

额外诊断把同一批 64 次真实调用及输出整理成文本证据，保留原用户提示，并将证据标为非指令数据。保留默认思考时返回 `length`，无正文。文本证据和 `enable_thinking=false` 组合返回 HTTP 200、`stop`、零工具调用与 6965 字符正文。该组合尚未进入受管实现，也未验证跨模型兼容或严格三轮。任务保留已阻塞，下一处待验证边界是仅在收尾轮使用无工具文本证据并获得准确正文。

本轮仅修改验收记录，未运行 Cargo。私有网关、诊断中继和工具子进程均已退出，端口、挂载与临时配置已清理。保留私有原始请求、终帧及对照结果。

## 受管 v4 与默认工具共存

按新版验收合同和固定请求执行，提示 SHA-256 为 `18806f1a927d3587ee5c685107c89b5dbaeea09614efe9a327e293b649967f38`。直接挂载 RustCodeGraph 1.2.11 原生 stdio，公布服务真实目录与 Cursor 官方 shell。未设置 MCP-only、工具白名单或权限改写。租户、模型、`full_access=true` 和 RueJS 提交保持一致。

现有 `acp_max_turns` 通过 `_meta.claudeCode.options.maxTurns` 发送，Cursor 受管入口不读取该扩展。v4 将该入口的有界工具预算从 64 提到 256，保留达到上限后的只答复推理和越界调用拒绝。新版合同的 v3 共存基线还发现，原生 shell 在执行 `pwd && ls` 时落到了网关进程目录。v4 在 ACP 新建和加载会话时保存实际工作目录，缺失或非绝对路径时拒绝 shell。审批与执行仍使用 Cursor 原生入口。

`--past-old-limit` 原生回归在 v3 返回 ACP `-32603`，v4 实际执行 70 次并回传 70 项结果。v3 在 ACP 进程与会话目录不同时将夹具写到进程目录。v4 复跑确认 shell 写入只出现在会话目录。256 次边界收尾、第 257 次工具拒绝、原生 shell 拒绝及会话加载回归通过。Node 定向测试 6 项通过，351 个运行文件的摘要与更新清单一致。

指定阿里模型的 v4 原生独立诊断会话 `66c419f5-0484-456f-a7ad-70e2ff8d2ce0` 结束于 `end_turn`。首轮声明官方 shell 和 RustCodeGraph 原目录，三种指定工具共成功 19 次（2 次 query、1 次 callers、16 次 node）。诊断客户端拒绝了额外的 status 调用。将完成事件中的原生输出逐项与最后一次实际模型请求里的工具结果按调用 ID 比对，全部匹配。真实答复引用了 `createRueSiteApp`（`app/app.tsx:32`）、`mountRueSiteApp`（同文件第 39 行）、`useApp.ts:4` 和 `_$createApp`。这为结果回传证据，网关三轮单独验收。
