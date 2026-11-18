# OpenCode 动态挂载 RustCodeGraph

2026-11-19，原生 stdio MCP 的发现、调用、结果续轮和撤除后的新会话隔离均有证据。完整 RueJS 调查合同未通过，首个不符合项发生在模型选工具时。

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

## 收尾

模型选工具和缺失路径读取发生在原生 MCP 已可用、工具结果已能回传之后。保留现有 OpenCode 转发、选模与权限实现，Rust 定向测试未触发。两份临时挂载配置已撤除，探针子进程已退出，原网关仍健康。干净快照中 4109 个归档文件及链接与基准一致，排除索引后没有新增文件。请求、响应头、SSE 和协议原始日志留在本机权限为 0700 的私有目录。
