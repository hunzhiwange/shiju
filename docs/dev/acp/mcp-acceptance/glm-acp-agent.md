# GLM Agent 动态挂载 RustCodeGraph

2026-11-19，GLM Agent 1.17.1 的私有 MCP 专用补丁通过严格三轮验收。未应用补丁的历史轮次记录了额外工具、无效文件路径和停止上限。

## 环境

本页三轮验收在 macOS arm64 进行，未在 Windows 或 Linux 复跑。各平台的安装路径见下文[受管接入](#受管接入)。

- 在 Darwin arm64 复用已审核的 npm 发行版 `glm-acp-agent@1.17.1`，入口 SHA-256 为 `d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98`。私有回环网关使用现有 `shiju 0.6.0`，构建 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`，健康检查 HTTP 200。
- 从 `189d22dc1` 提取 [复用请求](../ACP验收.md#可复用的验收请求)，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。三轮均使用租户 `10001 / 100010005`、`acp-glm-acp-agent / glm-acp-agent`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model` 和 `full_access=true`。
- 从 RueJS 的 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出干净副本。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `createRueSiteApp`（`app/app.tsx:32`）。挂载轮使用原 stdio 二进制、`--no-watch` 和服务名 `ruejs_graph_probe`。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl 退出码 / 耗时 | SSE 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efa19987001QrBcp1kkpDLMSA` | 200 / 0 / 2.872 秒 | `chat.done` ×1，答复 `MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1efa1a51f001lmQjdwX1JrvjCm` | 200 / 0 / 190.547 秒 | `chat.done` ×1，调查答复完整，指定工具成功，另有 14 次额外工具调用 |
| 撤除 | `ses_1efa48eb3001ALuD0XlTlxxAmz` | 200 / 0 / 3.187 秒 | `chat.done` ×1，答复 `MCP_NOT_AVAILABLE`，零工具事件 |

三轮均无 `chat.error`，标记按移除 SSE 思考段后的可见答复判定。UI summary 在运行中和终态均回读指定预设和后端、`full_access=true`。ACP options 两阶段均回读指定模型、提供商及来源。连接期间权限为 `client_only`、`requested=true`，终态 `connected=false`。日志另有一次辅助会话标题模型的 HTTP 403，指定模型的聊天流仍正常结束。

挂载轮共有 97 个调用，每个均有开始和完成事件，无失败状态。三种指定工具分别完成 45、5、33 次。代表性结果如下。

| 工具 | 调用 ID | 输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_ec447bb329af4513af58d7fe` | `createRueSiteApp`，`app/app.tsx:32` |
| `rustcodegraph_callers` | `call_ac980079605746589c0b4d16` | `mountRueSiteApp`，`app/app.tsx:39` |
| `rustcodegraph_node` | `call_5a32a1bbac4c40e794b109d2` | `packages/runtime/src/hooks/useApp.ts:4`，读取到委托给 `_$createApp` 的源码 |

首个违约事件为 `rustcodegraph_status`（`call_47a20287f6914d0194c577b4`）。额外调用共 14 次，含 `status` 1 次、`files` 7 次、`explore` 3 次、`callees` 3 次。所有调用均为本轮图谱工具。

## 答复核对

答复按源码行号追踪 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp`，说明浏览器自动挂载、SSR 静态入口、缺失 `#app` 时抛错、插件在挂载根组件前安装，以及 `/rust-layers` 绕过站点布局。答复指出 `callers("mountRueSiteApp")` 未返回模块顶层调用，与 `app/app.tsx:55` 的源码形成对照。

答复从运行时插入节点和卸载范围推断旧快照会残留，并给出预置旧节点后挂载及首屏导航两项可检验场景，均为源码推断。把路由相关查询的空结果归因于索引缺漏超出了所见工具输出。关于静态 DOM 必然翻倍的表述也应限于原有快照和新挂载含相同结构的页面。未执行页面验证。

## ACP 探针与清理

- 独立探针向同一发行版发送 ACP v1 `initialize`，返回版本 1.17.1、`loadSession=true`、`mcpCapabilities.http=true`。三次新进程的 `session/new.params.mcpServers` 依次为空列表、真实 stdio 服务、空列表。探针三轮均返回 `end_turn`。真实网关的 ACP 客户端使用同一 `NewSessionRequest::mcp_servers` 路径，本次未抓取其完整请求帧。
- GLM 在建会话时读 MCP 目录，把 `inputSchema` 放到 Chat Completions 的 `tools[].function.parameters`。回环模型分别观测到 0、8、0 个图谱定义。挂载探针顺序调用三种工具，各收到一次开始和成功完成事件。后续 Chat Completions 请求按原调用 ID 带回三个工具结果，最终回复固定标记。三种 MCP 调用均未发起 `session/request_permission`，与 GLM 原生 MCP 执行路径一致。探针使用独立状态目录和本地占位令牌。
- 宿主驱动将所选模型别名、请求级回环中继令牌和私有配置、会话目录传给 GLM。实际网关状态目录及绑定子目录权限为 0700。租户凭据仅经私有网关鉴权 API 写入受控存储。验收后私有网关退出，监听端口关闭，挂载配置及凭据副本已清理，未发现本轮 GLM 或 RustCodeGraph 子进程残留。
- 仅更新验收记录。现有 GLM 原生挂载和续轮行为已实测，无需修改后端或运行生命周期定向测试。未运行 Cargo。复跑使用同一请求、模型及三轮合同，任务保留至挂载轮只调用指定工具。原始 SSE、会话回执和探针帧留在本机私有目录。

## 共享构建复验

复用共享槽位已生成的 `shiju 0.6.0`，SHA-256 为 `06086ba0fc9580bd82fdcf1327696acb6ee648727ed2b054e1ee32d60d8f37d5`。GLM 驱动与会话生命周期测试自首次验收以来无提交改动。私有网关重新配置原租户、原模型、原权限，工作目录均从同一 RueJS 提交导出并重建索引。三轮 HTTP 请求体除后端与副本路径外沿用原样。

宿主 `allowedTools` 可以限制宿主 MCP 工具，但 ACP 建会话只转发服务声明。已安装的 RustCodeGraph stdio 参数也没有工具选项。复用同批次的私有 stdio 转发器，只公布并接受 query、callers、node，拒绝副本外路径。它沿用真实 RustCodeGraph 并原样返回结果。先测试分页字段 `offset`、`limit` 及路径说明，再补充从副本实际枚举的入口文件名和按路径片段查询的用法。直连与转发的协议对照确认工具目录分别为 8 和 3 项，查询、调用者及从第 787 行读取的路由工厂输出完全一致。转发器拒绝 files 和副本外路径。原生 GLM 回环复验在前后轮各收到零图谱声明，挂载轮只收到三种图谱声明且按原调用 ID 回传全部结果，三轮均以 `end_turn` 结束。

| 目录 | 轮次 | 会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- | --- |
| 原生直连 | 无挂载 | `ses_1f02040dc001LzTogPUW3xMyYF` | 200 / 0 / 2.818 | 标记，零工具 |
| 原生直连 | 挂载 | `ses_1f0204c5b001FiBZ9oldm8CUdX` | 200 / 0 / 157.029 | 40 query、8 callers、34 node，另有 8 次额外工具，2 次实际读取失败 |
| 原生直连 | 撤除 | `ses_1f022b31b001SLw2bPTe1q2j5O` | 200 / 0 / 2.363 | 标记，零工具 |
| 三工具转发 | 无挂载 | `ses_1f020ead2001Y3YH1KAWWeQsVS` | 200 / 0 / 3.146 | 标记，零工具 |
| 三工具转发 | 挂载 | `ses_1f020f7b5001hgIRUJm0Xd54k7` | 200 / 0 / 193.693 | 89 query、10 callers、76 node，另有 12 次 GLM 内置文件工具 |
| 三工具转发 | 撤除 | `ses_1f023ed30001SN6g2WRnkr9SPG` | 200 / 0 / 2.592 | 标记，零工具 |
| 文件名元数据 | 无挂载 | `ses_1f0221dd700178ve6Jf6qNzWNo` | 200 / 0 / 2.644 | 标记，零工具 |
| 文件名元数据 | 挂载 | `ses_1f0222962001vClq1xwYfC6ydA` | 200 / 0 / 126.671 | 52 query、6 callers、32 node，无其他工具，1 次实际读取失败 |
| 文件名元数据 | 撤除 | `ses_1f02418db001J5XpQvSNyjkjFf` | 200 / 0 / 2.252 | 标记，零工具 |

上表各轮单次 `chat.done`，无 `chat.error`。对照轮可见答复均精确为 `MCP_NOT_AVAILABLE`。UI summary 校验原绑定和 `full_access=true`。终态 ACP options 回读指定模型和来源，连接关闭。挂载轮运行中回读相同模型及 `client_only`。较短对照轮的采样早于连接或晚于释放，以终态选模为准。日志仍出现辅助标题模型的 HTTP 403。

- 原生直连首个额外工具为 `rustcodegraph_status`（`call_26931e49148d4bada0af7c8f`），合计 8 次额外调用。原服务读取 `app/router.ts`、`app/i18n.ts` 均失败。三工具转发的第一次错误是 `call_45f7ddd037bb453dbc9278f9` 读取不存在的 `app/router.ts`，同轮共 8 次读取失败。模型还触发了本有的 `list_files` 和 `read_file`。
- 文件名元数据组的 `call_01f05217c0884eeb80bb6d91` 命中 `createRueSiteApp`（`app/app.tsx:32`），`call_24416cc32c964f3ebe8f50cc` 命中 `mountRueSiteApp`（第 39 行），`call_65352a292af74a6690a7f9ec` 读回 `useApp.ts` 第 4 行的运行时工厂委托。模型读到了路由工厂和插件源码。唯一失败为 `call_d875e4746c2b4a6bacde5455`，请求了不存在的 `app/pages/site/SiteLayout.tsx`。原 MCP 直连复放返回 `isError=true`。GLM 的 `sessionMcpTool` 对返回值提取文本后直接发送 `completed`，所以判定采用原工具错误内容。
- 最后一组答复正确引用了调用主链、浏览器及 SSR 分支、目标容器、路由和 i18n 插件及布局谓词。它指出快照和新挂载可能叠加，并提出预填容器和真实导航两项场景。关于 `/rust-layers` 有可用静态页面的推断没有路由注册支持。预填场景要走真实 `mountRueSiteApp`，才会覆盖被调查的清空逻辑。两项场景均未运行。

三轮严格合同仍阻塞于目标模型发出失败的实际工具调用，属于验收范围内的失败。原生挂载、续轮和撤除隔离已复验成功。复验条件为指定模型在原提示与真实工具下发出有效文件参数并且只调用指定三种工具。若验收方接受从真实读取错误中恢复后继续作答，需要明确调整零失败条款。未改运行时代码、全局配置或权限，未运行 Cargo。

准备入口文件名变体时另有一组转发会话在元数据更新前启动，归入附加样本。无挂载会话 `ses_1f021cc57001sk8SMGqMO3TJfU` 和撤除会话 `ses_1f02757ab001K6rqG61llsqfeX` 均 HTTP 200、curl 0、精确标记、零工具。挂载会话 `ses_1f021d64a001Eg61CR20UVkONZ` 首段 HTTP 200、curl 28、300.002 秒，沿同一请求及游标续传，HTTP 200、curl 0、60.464 秒。三轮均收到一次 `chat.done`，无 `chat.error`。挂载轮只有三种图谱工具，194 个调用中有 18 次真实读取失败，首错为 `call_8e3ee947162b49738152f207` 读取 `app/router.ts`。挂载期间模型绑定与权限相同，续传终态采样时连接仍在释放，私有网关随后已退出。

四组副本各自与 RueJS 归档的 4108 个文件或符号链接一致，排除探针索引。私有挂载配置、凭据副本、网关及转发器已清理，端口关闭，未发现本轮相关子进程残留。原始回执仅留在本机私有目录。

## RustCodeGraph 1.2.11 发行版复验

使用同批次已受审的 `shiju 0.6.0`（SHA-256 `2e84923a46704187dfd9d56dfc4b45d6ebf9cfc93398a6bb3a82b1cc61a1f935`）和 RustCodeGraph 1.2.11 发行构建（SHA-256 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`）。GLM 入口仍为 1.17.1，入口摘要与前轮一致。复用原请求、提示哈希、固定 RueJS 提交、租户、模型、服务名和权限。两个独立副本均索引 3478 个文件，新索引直接命中 `createAppRouter`（`app/router/index.ts:787`）。[共享发行构建证据](pi.md)

项目配置沿用宿主 `allowedTools`。GLM 使用标准 ACP session/new，当前无 GLM 专用的宿主工具策略转发，限定目录由同批次已审验的私有 stdio 代理执行。目录列出真实源码入口、分页字段与唯一索引根，允许省略 `projectPath`。第二组采用 CodeBuddy 已验证的 `node.file` 源码扩展名模式。两组均保留实际请求参数和 RustCodeGraph 原始返回。协议对照确认原生目录 8 项、限定目录 3 项，query、callers、分页 node 与直连完全一致，未索引路径也保留原始错误内容。额外工具与副本外路径均被拒绝。[目录元数据验证](codebuddy.md)

| 目录 | 轮次 | 新会话 | HTTP / curl / 秒 | 可见结果 |
| --- | --- | --- | --- | --- |
| 索引根及分页 | 无挂载 | `ses_1f107bd5c001ZeedmHgP3O6deT` | 200 / 0 / 2.788 | 标记后附加解释，零工具 |
| 索引根及分页 | 挂载 | `ses_1f107c8f90011VTSSH4ydkM5rO` | 200 / 0 / 161.431 | 32 query、6 callers、39 node，全部成功 |
| 索引根及分页 | 撤除 | `ses_1f10a407b001upbgOekSUlDp7s` | 200 / 0 / 3.109 | 精确标记，零工具 |
| 源码扩展名模式 | 无挂载 | `ses_1f1098b6f001d14wjGhV7xQ3lF` | 200 / 0 / 2.637 | 精确标记，零工具 |
| 源码扩展名模式 | 挂载 | `ses_1f10996ed001kI6JemkpT1HEOZ` | 200 / 0 / 256.854 | 130 次图谱调用成功、1 次图谱读取失败，另有 2 次内置文件工具，触发 100 轮上限 |
| 源码扩展名模式 | 撤除 | `ses_1f10d83b10013dSfNFwV5PJNks` | 200 / 0 / 2.224 | 精确标记，零工具 |

六个新会话各有一次 `chat.done`，无 `chat.error` 或续传。精确标记为 `MCP_NOT_AVAILABLE`。UI summary 回读 `acp-glm-acp-agent / glm-acp-agent` 与 `full_access=true`，ACP options 两组挂载时均确认 `client_only`、指定模型、提供商和 `session_model`，六轮终态连接释放并保留原模型选择。辅助标题模型仍有独立的 HTTP 403。

第一组挂载的 77 个调用均有同 ID 的开始、完成及对应原始 MCP 成功返回，无其他工具。query `call_89117c9d5bf643aaa9197419` 命中 `createRueSiteApp`（`app/app.tsx:32`），callers `call_275dc6ca3b894112ad5728bd` 返回 `mountRueSiteApp`（第 39 行），node `call_bc8110fbb5844711953c3989` 和 `call_8854dab35d234299a3b35022` 分别读回 `useApp.ts:4` 与 `compiler-runtime/app.ts:57`。query `call_82f1979d38c24c5db9154fc3` 命中旧版缺失的路由工厂。答复引用完整主链、浏览器与 SSR、容器缺失、插件及布局分支，给出静态首屏和站内导航两项场景。重复 DOM 为源码推断，第一项回归描述将移除清空逻辑的条件写反。该组首个合同偏差发生于无挂载轮，提示要求只输出标记，模型在其后给出 143 字符的额外内容（含分隔换行）。

第二组挂载的 49 次 query、9 次 callers、73 次 node 均有开始与完成事件。首个失败 `call_fc98f57932ea42b8961c761f` 请求了不存在且未索引的 `packages/runtime/src/block.ts`。RustCodeGraph 如实返回 `No indexed file matches`，该结果无 `isError`，按实际输出判为失败。随后模型发起两次 GLM 内置目录读取，调用 ID 分别为 `call_31fe8cd3716f40e3a430a178` 和 `call_69264ef01bd34310b43a0034`。GLM 在达到 100 个推理轮次后返回停止说明，虽有 `chat.done`，可见答复没有交付调查结论。提高轮次上限无法补正已经发生的错误调用。

1.2.11 的新路径共实测两组，分别在对照答复格式和挂载工具参数处失败。原生挂载、真实工具输出和撤除隔离已有新证据。严格验收仍需指定模型在原请求下精确完成前后对照，并在同一挂载轮完成零额外工具、零读取失败及准确答复。当前无可在 GLM 现有适配器内同时约束这些行为的受审入口。本轮未改运行时、权限或公共代码，未运行 Cargo。两份副本各 4108 个文件及符号链接与归档一致（索引除外），私有网关、监听端口、挂载、凭据副本和子进程均已清理。

## 私有 GLM 补丁复验

复用相同 GLM 1.17.1、已受审的 ShiJu 构建及 RustCodeGraph 1.2.11，向私有包副本应用 [仅会话 MCP 补丁](../../../../patches/acp-model-source/glm-acp-agent/glm-acp-agent-1.17.1-mcp-only.patch)。补丁用于独立的只读验收入口。该入口的模型工具目录仅包含当前会话挂载的工具，执行器在分发前拒绝其他工具。系统提示按实际目录描述工具可用性，并要求遵循用户的输出格式，保留文件和版本控制约束。工具返回的 `isError=true` 在 ACP 事件中标为失败，原始错误文本仍进入续轮。RustCodeGraph 1.2.11 的未索引路径也可能只返回失败文本，本次另外核对原始 MCP 响应。模型请求内容、模型 ID、租户、项目提交和权限均沿用原合同。只在私有网关的可执行文件搜索路径选取补丁副本。

补丁输入为 npm 发行包 `glm-acp-agent@1.17.1`。原始 `dist/protocol/agent.js` SHA-256 为 `5352ec11b2db683957f5daeb571974863e3f8c3acb1cbcd321cb702b590025f3`，`dist/protocol/system-prompt.js` 为 `c0039856d8d3fca585cf28edd628a1c445cca828ef23d880e4688f1371c38cdc`，`dist/tools/executor.js` 为 `a3614aaa2e14a841fe2791e31465e50d8691130cc157d4dbe002b74efbac3289`。三份修改文件均通过 `node --check`。补丁在干净文件副本上重新应用后，与运行中的版本逐字节一致。

四个原生定向回环场景通过。前后无挂载的新会话均向模型声明零工具。挂载会话只声明三种原名工具，并按原调用 ID 返回真实的 query、callers、node 结果。模型替身强行请求内置 `list_files` 时，执行器返回失败且未触发宿主权限回调。首次回环夹具把 `node.name` 和 `node.file` 一并传给新版，得到重名定义列表。将已知精确路径单独作为 `file` 后正确读到 `useApp` 源码。定向回环使用占位令牌及本地模型替身，真实模型验收另见会话回执。

额外的错误夹具返回显式 `isError=true`，ACP 事件标记失败，续轮按同一调用 ID 收到原始错误文本。三方只读协议对照同时确认真实图谱目录从 8 项限定为 3 项，额外工具和越界请求被拒绝，成功与错误结果均与直连一致。目录从实际副本列出编译运行时源码文件、分页字段、源码扩展名和唯一索引根。请求参数未改写。

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f157059600167gDHchlCjSrlJ` | 200 / 0 / 4.208 | 一次 `chat.done`，只答 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f1571789001b3NGszHFbIeHVY` | 200 / 28 / 300.002，续传 200 / 0 / 33.561 | 一次 `chat.done`，127 次工具调用全成功 |
| 撤除 | `ses_1f15c312a001A7TS6MCD3IXHbv` | 200 / 0 / 3.683 | 一次 `chat.done`，只答 `MCP_NOT_AVAILABLE`，零工具 |

三轮均无 `chat.error`。挂载按原请求标识与最后事件游标续传，未重启推理。254 条 SSE 工具事件对应 41 次 query、7 次 callers 和 79 次 node，逐个匹配开始与完成。原始 MCP 记录有 127 个成功结果，零额外工具或读取失败。代表调用 `call_31598209253041e9878c7742` 返回 `createRueSiteApp`（`app/app.tsx:32`），`call_d87455f81b4d430688e57ae0` 返回 `mountRueSiteApp`（第 39 行），`call_a848ccd10311473bbd812d09` 和 `call_8aac316e9a7c41baae4b2ac5` 分别读回 `useApp.ts:4` 的委托及 `compiler-runtime/app.ts:57` 的运行时工厂。路由工厂由 `call_443faf8677204461b3f6de48` 定位。

答复正确引用调用主链、浏览器与 SSR 分支、缺少 `#app` 的异常、路由和 i18n 插件安装顺序，以及 `/rust-layers` 的布局谓词。答复区分运行时实现和公开声明，交代了模块顶层调用与图谱的差异。快照残留是源码推断。两项验证分别检查带客户端入口的静态页挂载后旧哨兵消失，以及预填模板的快照采集是否混入旧节点。两项均给出正常与回归的预期结果，未执行浏览器或快照测试。

UI summary 三轮均回读原预设、后端及 `full_access=true`。ACP options 运行期间为 `client_only`，回读指定模型和 `session_model`，最终均已释放连接。私有入口启动三次，网关退出、端口关闭、挂载和租户凭据副本已清理，未发现本轮子进程残留。RueJS 归档的 4108 个文件或符号链接内容一致（索引除外）。严格三轮验收通过。补丁仅用于私有验收入口，默认安装、全局配置、审批和撤销路径沿用原配置，未运行 Cargo。

## 受管接入

ShiJu 的 GLM 受管安装代码覆盖 macOS、Linux 和 Windows，使用独立的 `glm-acp-agent-1.17.1-model-source-v2` 目录。安装器校验 npm 包完整性及修改前后的文件摘要。已有 v1 安装需通过一键安装生成 v2。上文是 macOS arm64 历史实测，Windows 和 Linux 的本版运行验收待补。

- **模型选择**。原版 GLM 通过 `ACP_GLM_MODEL` 和 `ACP_GLM_BASE_URL` 接收模型及服务地址。ShiJu 的 `session_model` 路径校验受审产物。Windows 用 Node 启动已校验的 JS 入口，并在启动前校验私有状态目录访问权限。Linux 使用 Unix 状态目录权限校验。
- **MCP 挂载**。原版 GLM 支持 ACP `session/new.params.mcpServers`。ShiJu 的[受管补丁](../../../../patches/acp-model-source/glm-acp-agent/glm-acp-agent-1.17.1-managed.patch)执行对应服务的 `allowedTools` 和 `deniedTools`，拒绝名单优先，官方工具可在现有权限内共存。新建、加载和分叉会话均从当前服务声明解析名单。宿主全量拒绝时仍拒绝带策略的挂载。

实机复验从空的私有配置目录开始，依次验证一键安装、指定会话模型、原生只读工具与 MCP 共存、显式拒绝和撤除后的隔离。
