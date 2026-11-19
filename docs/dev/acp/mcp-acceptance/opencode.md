# OpenCode 动态挂载 RustCodeGraph

2026-11-19，显式说明索引覆盖后，OpenCode 的 RustCodeGraph 动态挂载三轮验收通过。下文保留原生发现、结果续轮、撤除隔离及前序失败回执。

## 环境与权限

使用现有回环网关，运行构建 ID 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。网关已启用内置 `acp-opencode`，租户 `10001 / 100010005` 的有效模型目录含 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。验收进程未取得私有网关可复用的凭据，仅在私有 RueJS 副本改动宿主配置。请求取自 `189d22dc1` 中的 [可复用验收请求](../ACP验收.md#可复用的验收请求)，原提示的 UTF-8 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。

网关启动的是 Darwin arm64 的受管 `opencode-ai` 1.19.5，进程加载的 `reviewed/opencode` SHA-256 为 `1a1f10028d3edebddd6df10a465cfa79b58d710875173dbd32137a8b115daa2c`，与仓库固定合约一致。独立握手回报 ACP v1、支持加载和恢复，HTTP 与 SSE MCP 能力均为 true。本次实际通过 `session/new.params.mcpServers` 启用原生 stdio 服务，使用原 RustCodeGraph 二进制和 `--no-watch`。

各轮会话重读确认绑定 `acp-opencode / opencode`、`full_access=true`，运行中 `connected=true`，模型来源为 `session_model`，provider 为 `alibaba-token-plan-cn`，模型为完整指定 ID。终态重读均为 `connected=false`，仍保留同一模型绑定。运行中 OpenCode 权限状态为 `client_only`，表示宿主客户端审批可自动批准，后端原生免审批尚未验证。验收流中的工具事件均来自 `ruejs_graph_probe`。

## RueJS 工作区副本

RueJS 位于 `main`，基准提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。第一组复制工作区实况，索引 3687 个文件。工作区在复制前已删除 `app/router/index.ts`、`app/entry-server.tsx` 等文件，另有移动后的未跟踪文件。第二组从同一提交导出干净源码，索引 3478 个文件，保留路由入口 `app/router/index.ts:787`。两份索引的 `createRueSiteApp` 均命中 `app/app.tsx:32`。

| 工作区副本轮次 | 新会话 | HTTP / curl | SSE 与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef7bb087001UGZoZdEeQYzHuH` | 200 / 0，17.972 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1ef7c2ee0001TmBu6IZShAcgsr` | 200 / 28，300.004 秒。续传 200 / 0 | 游标 2927 续传后收到 `chat.done`，无 `chat.error`，318 条工具状态事件，106 个调用完成，其中 4 个失败 |
| 撤除 | `ses_1ef80c34d001TYpCsLnJ8MtY3n` | 200 / 0，18.728 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

首次挂载的 `rustcodegraph_query` 返回 `createRueSiteApp`（`app/app.tsx:32`）。模型随即多发了 `rustcodegraph_status`，随后 `rustcodegraph_callers` 返回 `mountRueSiteApp`（`app/app.tsx:39`）。`rustcodegraph_node` 回读站点函数、`useApp`（`packages/runtime/src/hooks/useApp.ts:4`）和 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`）。这些调用均有同一调用 ID 的开始、完成及真实输出。模型继续使用 `files`、`explore` 等额外工具，四次读取缺失路由路径失败。

答复还原了调用链、SSR 跳过浏览器挂载、缺少 `#app` 时抛错、挂载时安装路由和 i18n、`/rust-layers` 绕过布局，并将快照残留和两项回归场景写成推断与预期。答复指出无法核实路由工厂实现，并把未跟踪的 `app/en22try-server.tsx` 称为实际 SSR 入口，已有证据只确认该文件存在。挂载轮因额外工具和失败调用未通过。

## 干净提交复验

| 干净快照轮次 | 新会话 | HTTP / curl | SSE 与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef7dd58d001UIpUAb1x4BCqyT` | 200 / 0，17.305 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1ef7e196500181sPjVgzWECDgv` | 200 / 28，300.001 秒。续传 200 / 0，52.952 秒 | 游标 638 续传后收到 `chat.done`，无 `chat.error`，639 条工具状态事件，213 个调用完成，其中 5 个失败 |
| 撤除 | `ses_1ef82ad8d001ITlt26DPxrVLNQ` | 200 / 0，17.377 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

复验同样取得三种目标工具的真实结果，成功次数分别为 query 120、callers 10、node 64。模型还使用了 status、files、callees、explore，并请求不存在的构建产物或在文件路径中拼接行号，产生五个失败调用。两次挂载都使用原请求标识与事件游标续传，合并流分别只有一个 `chat.done`。

复验答复走通完整调用链，用干净源码定位 `app/entry-server.tsx`，正确描述无 `#app` 的异常、路由与 i18n 安装及布局绕过。它解释了静态快照与新应用重复渲染的原因，并提供首屏去重和从 `/rust-layers` 导航后移除旧内容两项预期场景。答复也指出 `createRouter` 等索引缺漏。额外工具和失败调用仍使合同不通过。

## 原生协议与续轮

独立回环探针使用同一受管 OpenCode 二进制与真实 stdio RustCodeGraph。三个独立 ACP 进程的 `session/new` 分别传入空列表、带 `ruejs_graph_probe` 的列表和空列表。三个 `session/prompt` 均以 `end_turn` 结束，本地 Chat Completions 端观测到的目标服务工具定义数依次为 0、8、0。挂载轮可见 `query`、`callers`、`node` 的结构化 schema，实际查询命中 `app/app.tsx:32`，该工具结果出现在续轮请求。回环只用于观测协议边界，真实网关的走查结果另有 SSE 和会话重读佐证。

## 共享工作区复查

以 `76db6b6b7` 为复查起点。对比前次验收后进入共享分支的修改，OpenCode 驱动、ACP 建会话转发、受管合约的 OpenCode 键均无相关变更。使用已授权、预设已启用的现有网关，基于同一干净提交重建索引。RustCodeGraph 1.2.10 的 `query createAppRouter` 仍无命中，但命令行按文件偏移读取可见 `app/router/index.ts:787` 中的真实定义。

扩展独立回环探针，挂载轮按顺序调用 `query(createRueSiteApp)`、`callers(createRueSiteApp)` 和 `node(useApp)`。真实结果分别包含 `app/app.tsx:32`、`mountRueSiteApp` 和 `_$createApp`，三个调用 ID 均在续轮的 tool 消息中匹配。无挂载、挂载和撤除对应目标 schema 数仍为 0、8、0。三个独立 ACP 进程均返回 `end_turn`。

额外验证工具目录是否导致失败。宿主的 `allowedTools` 用于宿主管理的 MCP 工具，原生 ACP `session/new` 只接收服务器定义，RustCodeGraph stdio 也无原生工具列表过滤开关。补充探针在私有目录通过 stdio 转发原始 RustCodeGraph，仅公布三种指定工具、拒绝其他工具名和不同的 projectPath，逐条透传真实调用与输出。两组使用原提示散列、真实模型、原租户、原权限申请和新会话。

### 原生 stdio 复跑

| 轮次 | 新会话 | HTTP / curl | SSE 与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1eff04fd2001671vs4cm9fZl7Y` | 200 / 0，18.074 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1eff096a80014pXRgGf1uRneB5` | 200 / 0，230.108 秒 | `chat.done`，无 `chat.error`，156 个调用，其中 2 个失败 |
| 撤除 | `ses_1eff419d1001H92CafCd1NTkEj` | 200 / 0，16.980 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

挂载轮的 query、callers、node 分别成功 90、10、39 次。首次额外调用仍为 `rustcodegraph_status`（`call_5309d6325e9e479d9850ea4b`），另调用 files 和 explore。两次失败分别请求了不存在的 `app/router.ts` 和 `app/i18n.ts`。答复重建了主调用链并覆盖五项分支和影响，但第二个验证场景所假设的 `/rust-layers` 布局快照缺少源文件依据。首个不符合项仍发生在模型选工具时。

### 三工具 stdio 补充探针

| 轮次 | 新会话 | HTTP / curl | SSE 与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1eff18171001Dap61d0Hh3Ih2s` | 200 / 0，17.803 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1eff1c739001287yT3C81cJWXC` | 200 / 0，282.050 秒 | `chat.done`，无 `chat.error`，156 个调用，其中 2 个失败 |
| 撤除 | `ses_1eff615a8001gX0swhCY4N3sZs` | 200 / 0，17.449 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

挂载轮只有三种目标工具，query、callers、node 分别成功 104、8、42 次。第一个失败仍是 RustCodeGraph 读取不存在的 `app/router.ts`（`call_84ebacda5213469591d1be83`）。第二次失败由探针误将副本内的 `app` 子目录拒于项目根之外。修正探针后独立重放两个参数：首次仍返回文件不存在，第二次由 RustCodeGraph 返回子目录未初始化。模型答复覆盖主调用链、分支和两项快照场景，但仍未取到路由工厂的真实定义。两组会话绑定、指定选模、`client_only` 权限及终态连接释放与之前一致。

本轮所有临时挂载均撤除，匹配两个私有副本的残留图谱进程已退出，用户网关仍健康。完整验收仍取决于指定模型使用有效工具参数并符合三种工具约束。维持当前模型与合同的情况下，需要模型遵循工具约束，并使用当前索引支持的参数。索引公开缺失定义及 MCP 分页能力可以减少模型猜测，相关改动属于 RustCodeGraph。放宽验收合同须由计划负责人决定。

## 收尾

模型选工具和缺失路径读取发生在原生 MCP 已可用、工具结果已能回传之后。保留现有 OpenCode 转发、选模与权限实现，Rust 定向测试未触发。两份临时挂载配置已撤除，探针子进程已退出，原网关仍健康。干净快照中 4109 个归档文件及链接与基准一致，排除索引后没有新增文件。请求、响应头、SSE 和协议原始日志留在本机权限为 0700 的私有目录。

## RustCodeGraph 1.2.11 复验

使用同一受审 OpenCode 1.19.5、现有开发网关、固定 RueJS 提交、原提示、模型与权限。RustCodeGraph 1.2.11 发行构建 SHA-256 为 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`。重建的 3478 文件索引能直接查到 `createAppRouter`（`app/router/index.ts:787`）。当前共享宿主向 Cortex、DeepAgents 和 Pi 下发各自的工具策略，受审 OpenCode 未接收该扩展。本轮仅在私有验收目录复用 Goose 的 stdio 转发及既有入口、分页和索引根说明，由原生 OpenCode 消费工具目录。[发行构建与策略证据](pi.md) · [stdio 路径](goose.md)

直连公开 8 项，限定目录公开 3 项。独立比较确认路由工厂查询、调用者查询和带行号的源码分页输出逐项相等。限定转发拒绝额外工具和越界文件，目录内未索引路径保持图谱实际返回。`app/router.ts` 的回复是 `No indexed file matches`，协议未设置 `isError`。验收另核对真实内容，避免把未命中计为源码读取成功。

| 继承入口说明的轮次 | 新会话 | HTTP / curl / 秒 | SSE 与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0f70458001Q5OoGsuBbkBPL3` | 200 / 0 / 23.236 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f0f75fa6001ls3zf5mFUSiWb4` | 200 / 0 / 297.510 | `chat.done`，76 个完成事件，仅三种指定工具，其中一次文件未命中 |
| 撤除 | `ses_1f0fbeb4d001q00T0OiEjJhLMb` | 200 / 0 / 22.737 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

首个不符合项为 `call_11a5d419cb144ad89331a35c` 读取目录说明推荐的 `index.html`。文件存在，RustCodeGraph 返回该文件未被索引，SSE 将此文本标为 completed。答复仍引用三种工具读回的调用链、SSR 守卫、容器缺失、插件安装和布局条件，并给出静态页首屏去重与非空模板快照捕获两项预期。首组按内容存在文件未命中，保留为对照。

第二组只调整转发器的 tool schema 说明，移除 HTML 入口推荐并注明索引不含 HTML 模板。项目、索引、分页参数、过滤边界和所有调用返回均沿用首组。直连对照再次通过。

| 仅推荐索引入口的轮次 | 新会话 | HTTP / curl / 秒 | SSE 与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0fc4c6e001hTgNU9nX0TKTtL` | 200 / 0 / 19.158 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f0fc9783001WqoltjtLUyRrDA` | 200 / 0 / 252.770 | `chat.done`，89 个完成事件，仅三种指定工具，其中两次文件未命中 |
| 撤除 | `ses_1f10074cc001nPMS6dy0d4f2Tu` | 200 / 0 / 20.023 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

- 两组六轮各收到一个 `chat.done`，均无 `chat.error)，无需续传。UI summary 均为 `acp-opencode / opencode`、`full_access=true`。连接中权限为 `client_only`，终态均已断开，仍绑定 `session_model` 和指定阿里模型。挂载轮的所有调用 ID 均匹配开始与完成。
- 第二组的 38 次 query、1 次 callers 和 50 次 node 均返回原生响应。`call_384981932cb443ebadcd51d4` 查询命中 `createRueSiteApp`（`app/app.tsx:32`）。`call_ea99360f401f41fdb47a94b2` 返回 `mountRueSiteApp`（同文件第 39 行）。`call_ac91e425441f452e9e4f4e81` 和 `call_f47a38dd37994374a95147c2` 读回 `useApp` 与 `_$createApp` 源码。两组均无额外工具调用。
- 第二组又请求 `index.html`（`call_2336d624d87d4dcc9d9319c6`），随后请求 `packages/rue/package.json`（`call_78cdf522955b41cf9b12434f`）。两个受版本跟踪的文件均存在，RustCodeGraph 均原样返回未被索引。新版两次挂载均复现 `index.html` 未命中，第二组另有一次 JSON 未命中。最终答复复原完整启动链、SSR 与浏览器分支、容器缺失、插件安装和当前未注册的 `/rust-layers` 布局分支。静态快照残留是源码推断，两项场景覆盖预置旧节点的 DOM 夹具和静态产物挂载后的去重，未执行浏览器测试。
- 严格合同仍阻塞在模型请求未索引的文件。目前没有需要修补的 OpenCode 转发断点。复跑条件为固定模型使用已索引文件完成走查，或取得能实际读取这两类跟踪文件且已受审的图谱构建。私有挂载已撤除，专属转发器与图谱进程已退出，4109 个归档项内容一致，原网关仍健康。本轮未修改运行时或运行 Cargo。

## 显式索引覆盖说明复验

复用同一 1.2.11 二进制、已建索引和受审 OpenCode。参考 AutoHand 通过组，为三种工具注明职责及客户端可能按相关性展示子集。此版本的 `node` 说明明确指出根 `index.html` 和 `packages/rue/package.json` 均存在但不在索引内，因此无法读取，并列出实际索引的 `app/app.tsx`、`app/entry-server.tsx`、`packages/runtime/src/hooks/useApp.ts` 等入口。保留原输入字段、分页、索引根提示、越界拒绝和真实返回。独立直连对照确认工具目录为 8 对 3，三类真实查询的结果相等。[AutoHand 通过组](autohand.md)

核对同一发行构建的 `init --help`、`index --help` 以及 `v1.2.11` 的语言说明和发现代码。源文件按内置后缀发现，HTML 和 JSON 不在支持列表。未找到启用这两种类型的受审索引选项。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f11cc363001pkZjtJihRVrBdr` | 200 / 0 / 16.191 | 一个 `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f11d02ea001FACw8QlGswhIDs` | 200 / 0 / 203.961 | 一个 `chat.done`，33 次 query、2 次 callers、33 次 node 全部成功 |
| 撤除 | `ses_1f1202042001hMXNVCsoibhZl7` | 200 / 0 / 16.966 | 一个 `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

- 三轮无 `chat.error` 或推理和中继错误。挂载的 68 个调用均匹配同 ID 开始、完成和真实结果，无额外工具、失败状态或未索引文件读取。`call_3b8a5643a0fa490891468d80` 定位 `app/app.tsx:32`，`call_73573bac67124e8990634eac` 返回 `mountRueSiteApp`（同文件第 39 行），`call_322a1577f2454bf0ae54b381` 与 `call_8b45860c74644240b5a2dee9` 回读 `useApp` 和运行时工厂。分页调用 `call_8a5ca17cd8f14fb087a5b9a8` 返回路由工厂所在的第 760–791 行。
- UI summary 三轮均确认 `acp-opencode / opencode` 和 `full_access=true`。ACP options 运行中均为 `client_only`，终态连接全部释放，所选模型仍为 `session_model` 和原定阿里模型。原提示散列复核一致。
- 答复引用原生结果，覆盖完整调用链、浏览器与 SSR、容器缺失、路由和 i18n 安装、布局绕过及调用图差异。它把静态快照影响列为源码推断，给出空模板的快照基线和非空容器的哨兵节点验证。空模板可用实际注册的 `/design/space`。非空夹具的必要断言是旧节点是否残留，只有旧快照含站点头部时才会出现两个头部。未运行浏览器场景。
- 严格三轮验收通过。私有挂载已撤除，本轮转发器与图谱进程已退出，4109 个归档项内容逐项一致，现有网关仍健康。原始证据留在本机 0700 私有目录。本轮省略 Cargo，运行时代码未变。
