# Auggie 动态挂载 RustCodeGraph

2026-11-19，Auggie 能在新会话发现原生 stdio MCP，执行三种指定工具并把结果送入模型续轮。RueJS 严格验收为已阻塞。对照轮的 SSE 混入推理文本，挂载轮存在额外工具调用、一次工具失败和一处答复错误。

## 环境

- Darwin arm64，受管 `@augmentcode/auggie` 0.37.1（`e8d38687`）。入口 `augment.mjs` 和桥接模块 `session-model.mjs` 的 SHA-256 分别为 `1e39bfa85bb3ebe44afb6d627bdd3202550508d5224c837dc4370576f555cb18` 与 `d50926db8682babc44174201b0591e7b309e00e5f7f5989c924f610e7b1a7081`，均与仓库受管契约一致。网关为仓库现有开发构建，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。
- 复用 `189d22dc1` 的请求，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。绑定 `acp-auggie / auggie`、`session_model`、`full_access=true`、租户 `10001 / 100010005` 和模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。在私有回环网关启用 Auggie，通过宿主鉴权 API 写入已有凭据，受管入口只接收替代真实凭据的请求级中继令牌。[可复用的验收请求](../ACP验收.md#可复用的验收请求)
- RueJS 为 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的干净副本。RustCodeGraph 索引 3478 个文件，命中 `app/app.tsx:32`。只在挂载轮将 `ruejs_graph_probe` 写入副本的 `.vwacprc.json`，通过 `session/new.params.mcpServers` 传给 Auggie。使用真实 RustCodeGraph 二进制和原生 stdio，参数为 `serve --mcp --path <副本> --no-watch`。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl | SSE 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef8e9125001fI2pUYm2H8naR5` | 200 / 0，5.772 秒 | `chat.done`，答案为 `MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1ef8ea7f6001XmM5NAMpDvuAte` | 200 / 28，300.001 秒。续传 200 / 0，32.932 秒 | `chat.done`，270 条工具状态事件，135 次调用中 1 次失败 |
| 撤除 | `ses_1ef93bed0001GeM0RqmhFvQUOA` | 200 / 0，7.267 秒 | `chat.done`，答案为 `MCP_NOT_AVAILABLE`，零工具事件 |

挂载轮沿用原请求标识，从事件 512 续传，同一会话由 513 开始恢复。各轮均恰有一个 `chat.done`，无 `chat.error`。各轮 UI summary 确认 Auggie 预设绑定和 `full_access=true`。运行中的 ACP options 确认 `connected=true`，选中的模型、提供商和来源与请求一致，终态保留同一模型绑定且连接已释放。无挂载和撤除轮的最终答案均为单一标记，但普通 `chat.delta` 也携带 `<think>` 内容。

## 工具及答复

挂载轮的 SSE 按同一调用 ID 记录开始、完成与真实输出。模型在第一次 `query` 后立即调用了额外的 `rustcodegraph_status`，随后尝试读取缺失的 `app/router.ts` 并收到工具错误。实际路由文件为 `app/router/index.ts`。前两类差异均发生在 Auggie 已成功发现和执行 MCP 之后。

| 工具 | 代表性 SSE 调用 ID | 实际输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_febc5b63b2744d36a5f63bcd` | 命中 `createRueSiteApp`（`app/app.tsx:32`） |
| `rustcodegraph_callers` | `call_a6f503a5cd2b4c2a97e70b3c` | 调用者 `mountRueSiteApp`（`app/app.tsx:39`） |
| `rustcodegraph_node` | `call_eab7f907907e4d66b5a594cb` | `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）调用 `_$createApp(factory)` |

三种指定工具的成功次数分别为 65、11、45。其余 13 次成功调用为同一服务的 `files`、`explore`、`callees` 和 `status`。未观察到 shell、写入或其它服务调用。

答复还原了 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp` 调用链，正确引用浏览器与 SSR 分支、缺少 `#app` 的异常、路由与 i18n 插件安装以及 `/rust-layers` 布局条件。它解释了移除清空逻辑后静态快照与新应用树可能共存，并给出首屏 DOM 唯一性和卸载后旧快照残留两项待验证场景。答复未定位路由工厂的定义（实际为 `app/router/index.ts:787`）。答复称带尾斜杠或查询串的 URL 会进入布局分支，但路由在 `packages/router/src/index.tsx:342,367-374` 先去尾斜杠并从 `path` 分离后缀，因此该判断有误。未执行浏览器场景。

## 回环与权限

独立回环使用 SHA-256 相同的受管入口、与网关相同的 ACP 启动参数及 Chat Completions 路由。仅该回环的模型响应使用合成数据。原生 `initialize` 返回 ACP v1 与 `loadSession=true`，未公布 HTTP 或 SSE MCP 能力。新进程分别提交空列表、stdio 挂载、空列表，三次都返回 `end_turn`。两个空列表对应的模型请求没有目标服务工具。挂载轮确认三个目标工具的结构化参数，实际 RustCodeGraph 的三条结果分别出现在续轮的同 ID `tool_call_id` 中。

网关运行期 ACP options 报告 `access_status.support=client_only`。回环的三个 MCP 调用均未向 ACP 宿主发送 `session/request_permission`，因此回环无法向这些调用应用 `reject_once`。桌面现有门禁仍要求当前完全访问授权。

定向执行 `node --test patches/acp-model-source/auggie/session-model.test.mjs`，5 项通过。现有 Rust ACP 转发和模型驱动未改动，本次未运行 Cargo。

私有网关及其 Auggie、RustCodeGraph 子进程均已退出，临时挂载配置与私有凭据副本已清除，用户原网关仍返回健康检查 HTTP 200。排除新建索引后，RueJS 副本的 4109 个归档文件和符号链接与基准一致。原始请求、SSE、会话回读和回环证据保存在本机权限为 0700 的私有目录。
