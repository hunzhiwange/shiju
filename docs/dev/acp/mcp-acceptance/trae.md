# TRAE CLI 动态挂载 RustCodeGraph

2026-11-19。受管 TRAE CLI 0.121.3 能在新会话通过原生 stdio 挂载 RustCodeGraph，执行三个指定工具，并将输出用于指定模型的答复。严格验收出现模型输出及额外工具调用偏差。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境与请求

- Darwin arm64。私有回环网关复用 ShiJu 0.6.0，二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。使用开发 SQLite 和授权文件的私有副本，只在私有配置启用内置预设。已安装的 `trae-0.121.3-model-source-v1` 三项文件摘要均匹配仓库合同。入口为 `traecli acp serve`，上游 build commit 为 `72fe00ff52f56b0eb11a37bd561d4f10a4a44354`，本地组合二进制 SHA-256 为 `f77806ca15efea1c33ffc4c3c08ab3f1c57111883dd3313822e673791484adde`。
- 每轮用新会话，同一租户 `10001 / 100010005`、预设与后端 `acp-trae / trae`、`full_access=true` 和模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。选模来源为 `session_model`。原提示来自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。
- RustCodeGraph 1.2.10 使用原生 stdio、`--no-watch` 及会话服务名 `ruejs_graph_probe`。RueJS 为 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。工作区副本索引 3687 个文件，工作区原有改动删除了 `app/router/index.ts` 和 `app/entry-server.tsx`。同提交的干净副本索引 3478 个文件，包含这两个文件，路由工厂位于 `app/router/index.ts:787`。两个副本均命中 `app/app.tsx:32` 的 `createRueSiteApp`。

## 协议与模型工具声明

- 源码中 `llm-runtime/src/acp/session.rs` 将解析后的工具配置传入 ACP client，`acp-client/src/client/protocol.rs` 将其序列化至 `session/new.params.mcpServers`。真实网关轮次保留 SSE 及选模回读，未抓取完整的 ACP 请求或上游模型请求。
- 独立原生线协议探针使用同一受管启动器和无供应商凭据的回环模型端点。ACP `initialize` 返回协议 v1、`loadSession=true`、MCP 的 HTTP 与 SSE 能力为 true。实际测试传输为 stdio。探针明确发送三条 ACP `session/new` 请求，配置依次为 `[]`、原生 stdio 服务定义、`[]`。三次均能创建新会话并调用本地模型端点。挂载时模型请求包含八项 `mcp__ruejs_graph_probe__rustcodegraph_*`，其中有 `query`、`callers` 和 `node`；两次空配置的模型请求均没有此服务的定义。
- 探针端点故意返回 HTTP 400，仅用于核对工具声明。下列真实模型轮次独立验证执行和答复。

## 真实模型三轮

| 副本与轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 工作区 · 无挂载 | `ses_1efcaf721001IsTnHqCEeLofNf` | 200 / 0 / 9.158 | 零工具事件。答复含 `MCP_NOT_AVAILABLE` 和额外说明，成为首个严格验收偏差。 |
| 工作区 · 挂载 | `ses_1efcb1cbc001NYKfGOhcLMWpHb` | 200 / 0 / 186.236 | 三种指定工具均取得实际结果。第一次额外调用是 `rustcodegraph_status`。 |
| 工作区 · 撤除 | `ses_1efcdf889001wpJObDA3o3iU64` | 200 / 0 / 8.651 | 零工具事件，答复为 `MCP_NOT_AVAILABLE`。 |
| 干净提交 · 无挂载 | `ses_1efccd42d001X8ofTZvc228ILH` | 200 / 0 / 9.399 | 零工具事件，答复为 `MCP_NOT_AVAILABLE`。 |
| 干净提交 · 挂载 | `ses_1efccfd470011M21g4dBD1cG7c` | 200 / 0 / 234.549 | 三种指定工具均取得实际结果。首个工具调用仍为额外的 `rustcodegraph_status`。 |
| 干净提交 · 撤除 | `ses_1efd094a9001ovw6dWjJYgQIfv` | 200 / 0 / 8.002 | 零工具事件，答复为 `MCP_NOT_AVAILABLE`。 |

六轮各收到一个 `chat.done)，没有 `chat.error`。答复比较已去除 `<think>` 段。工作区挂载轮有 140 条工具状态事件，按调用 ID 计为 28 次 `query`、7 次 `callers`、22 次 `node` 和 13 次额外图谱调用。22 次 `node` 中有 4 次输出文件读取失败，SSE 状态仍为 `completed`。干净副本挂载轮有 234 条工具状态事件，按调用 ID 计为 65 次 `query`、3 次 `callers`、24 次 `node` 和 25 次额外图谱调用。额外调用均来自同一图谱服务，包括 `status`、`files`、`explore` 和 `callees`。

工作区挂载轮的 SSE 工具开始及完成事件可通过调用 ID 对应。以下结果也在干净副本复现。

| 工具 | 工作区调用 ID | 实际输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_a92c8141665f4b5888b0488d` | `createRueSiteApp` 定位至 `app/app.tsx:32`。 |
| `rustcodegraph_callers` | `call_1a35e6ad9d5b45c2ac1193c7` | 返回调用者 `mountRueSiteApp`，位于 `app/app.tsx:39`。 |
| `rustcodegraph_node` | `call_7cdb6ef462d94ef1bbce5a8f`、`call_2fe8af9482a540fa912651ee`、`call_21311ef1d8194db5a45d2a6e` | 回读 `app/app.tsx` 的挂载分支、`useApp`（`packages/runtime/src/hooks/useApp.ts:4`）与 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`）。 |

两个挂载答复都引用四段调用链，解释浏览器守卫、缺失 `#app` 时抛错、挂载时安装路由和 i18n 插件、`/rust-layers` 的布局绕过条件。干净副本通过 `call_5d99f2eff37441b29c028c01` 和 `call_84316446a1b0407a9a855b44` 读到实际路由工厂和 SSR 入口，答复也给出了默认 browser history 和静态 SSR 渲染路径。两份答复依据 `replaceChildren` 与运行时追加节点推断取消清空会保留旧 DOM，并各给出两项带预期的回归场景。场景未执行。工作区答复把快照旧链接的点击效果断言为无效，源码只能证明缺少应用绑定，原生导航仍有可能。干净副本答复将点击效果表述为可能触发整页刷新或更新应用视图。

## 权限与结论

六轮 summary 回读均为 `acp-trae / trae`、`full_access=true`。运行中 ACP options 均显示已连接、指定模型和 `access_status.support=client_only`。终态回读均显示连接已释放，所选模型仍可读。SSE 未出现交互审批、shell 或写工具事件。本次只覆盖请求的访问级别与这些图谱调用。

图谱发现、工具执行和撤除隔离均已复现。严格验收阻塞在指定模型的额外输出及额外图谱调用，工作区挂载轮还有四次源文件读取失败。驱动、受管入口和公共转发未修改，本次未运行 Cargo。私有预设已恢复停用，两份临时挂载已撤除，私有网关退出且端口关闭。临时授权文件和 SQLite 副本已清理，原始证据保留在本机私有目录。
