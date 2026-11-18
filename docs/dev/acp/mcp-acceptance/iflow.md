# iFlow 动态挂载 RustCodeGraph

## 环境

- 2026-11-19 在 Darwin arm64 复用 `shiju 0.6.0`，构建 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。私有回环网关使用租户配置快照，仅在该实例启用 `acp-iflow`。初跑三轮因快照缺少供应商凭据，在启动 ACP 前返回 `ProviderAuthError`。使用现有授权来源补齐私有配置后重跑。
- 三轮使用 [固定验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。租户 `10001 / 100010005`，绑定 `acp-iflow / iflow`，选模 `session_model / alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，请求 `full_access=true`。
- 受管 npm 发行包 `@iflow-ai/iflow-cli@0.5.19`，修订 v1。私有副本的 `entry.js` 和 `iflow.js` SHA-256 分别为 `55710b7cfea2e5aefebfe0acec4eaf3e647d34538ee5835a3d724a49853d1fa6` 和 `3f485f9ca61edd2d1594cc6b5374a8108002ab0ee3488f03bdbcfc4bef89ef66`，与受管清单一致。
- 从 RueJS `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出私有副本。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `createRueSiteApp`（`app/app.tsx:32`）。使用原生 stdio 加 `--no-watch`，只在挂载轮的副本配置 `ruejs_graph_probe`，撤除轮前删除该配置。

## 受管入口和模型声明

宿主选模驱动将 `IFLOW_HOME` 指向当次私有目录，并使用请求绑定的回环模型中继。补丁让用户设置及系统设置读取该目录，锁定模型和授权方式。原生 `newSessionConfig` 以加载后的 MCP 配置为基础，将 `session/new.mcpServers` 按名称覆盖，再初始化当次工具注册表。受管补丁未更改这个合并顺序。

独立协议探针复用同一受管入口，使用一次性回环模型与占位令牌。握手为 ACP v1、`iflow-agent 0.5.19`、`loadSession=true`，未声明 HTTP 或 SSE MCP 能力。按空、stdio、空发送三次 `session/new`，三个提示均结束于 `end_turn`。模型实际收到 21、29、21 个工具声明，只有挂载轮收到八个图谱工具。三轮回执中的 `availableMcpServers` 均为空，模型声明和下文真实工具事件给出了挂载证据。

## 指定模型三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态与答复 | 工具调用 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efadd828001hINgcayztnh6jc` | 200 / 0 / 6.046550 | 一个 `chat.done`，最终正文为 `MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1efadf12e001tD7YYxiGL5t57P` | 200 / 0 / 158.405156 | 一个 `chat.done`，输出调查答复 | 65 次完成 |
| 撤除 | `ses_1efb05d360012X02CvqDiQiIdu` | 200 / 0 / 5.960967 | 一个 `chat.done`，最终正文为 `MCP_NOT_AVAILABLE` | 0 |

三轮均无 `chat.error`、`chat.model_recovery` 或失败工具事件。对照轮的原始 SSE delta 另含 `<think>`，表中记录最终正文。独立的自动标题请求出现另一模型的 HTTP 403，主轮仍正常结束。

会话回读在运行中及终态均确认 `acp-iflow / iflow`、`full_access=true` 和指定的完整模型 ID、提供商及 `session_model` 来源。运行时 `connected=true`、`access_status.support=partial`，结束后 `connected=false`。挂载轮仅观察到只读 MCP 调用，未出现写入、Shell 或审批请求。这批请求未触发原生写操作，不能据此判断原生审批是否会放行。

## 工具结果和验收结论

65 次调用均有同一调用 ID 的开始、更新及完成事件，共 195 条工具事件。开始标题均标记 `ruejs_graph_probe MCP Server`。三种指定工具的代表性原生结果如下。

| 工具 | 完成次数 | 代表性调用和输出 |
| --- | --- | --- |
| `rustcodegraph_query` | 29 | `call_1c324513e35f4acf9f17db7c` 返回 `createRueSiteApp`（`app/app.tsx:32`） |
| `rustcodegraph_callers` | 6 | `call_1aeaef278236420cbbfe1063` 返回 `mountRueSiteApp`（`app/app.tsx:39`） |
| `rustcodegraph_node` | 17 | `call_cf994a02e2654c1db27c0ceb` 返回 `packages/runtime/src/hooks/useApp.ts`，第 4 行的 `useApp` 调用 `_$createApp(factory)` |

最终答复的主调用链、浏览器及 SSR 分支、缺少 `#app` 时立即抛错、路由及 i18n 在根组件工厂前按注册顺序安装，以及 `/rust-layers` 绕过布局，均可与返回源码对应。答复指出了模块顶层调用缺少图边以及 `createAppRouter` 未命中索引。它推断移除清空后旧快照与新挂载共存，并给出重复 DOM 和导航到布局旁路时旧布局残留两项可检验场景。场景中的具体节点数及旧节点交互行为仍需浏览器实测。

统一合同未通过。首个失败点是模型提示阶段的第一条工具调用 `rustcodegraph_status`。随后还有 `callees` 5 次、`explore` 4 次及 `files` 3 次，合计 13 次超出指定三种工具的调用。iFlow 原生 MCP 接纳、指定模型发现和续轮均已有成功证据。保留原有工具声明和权限策略。复验应继续使用固定请求，并要求挂载轮的实际调用仅限原合同指定的三种工具。

本次只提交验收记录。宿主和受管入口均无需修复，因此未运行 Cargo 定向用例。自有网关及子进程已退出，端口关闭，私有凭据和配置快照已清理。原始 SSE、会话回读和协议探针留在本机私有目录。
