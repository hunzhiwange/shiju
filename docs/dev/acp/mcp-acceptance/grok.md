# Grok Build 动态挂载 RustCodeGraph

2026-11-19 完成指定模型三轮实测。Grok Build 原生 ACP 可以动态发现、调用 RustCodeGraph 并读回结果。任务 20 保留为已阻塞。首个合同偏差出现在无挂载轮，模型调用了一次 Grok 内置的 `search_tool`。挂载轮出现额外工具与失败调用，撤除轮也没有只回复约定标记。

## 环境与请求

- Darwin arm64，Grok Build `1.0.77 (50ca579fa996)`，本机固定版入口 SHA-256 为 `197c66a63e5ac46e07450b0ca21d141ce665050a412180f185dadfae1c7a5b2a`。私有回环网关使用既有 ShiJu 0.6.0 开发构建，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。仅在私有租户副本启用内置 Grok 预设并指定固定版入口。
- 使用 [统一验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。三轮均使用租户 `10001 / 100010005`、`acp-grok / grok`、`session_model`、`full_access=true` 及 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出到私有目录，RustCodeGraph 1.2.10 索引 3478 个文件，命中 `createRueSiteApp`。使用原生 stdio 与 `--no-watch`，仅在挂载轮通过副本的 `.vwacprc.json` 提供 `ruejs_graph_probe`。挂载过程中观察到指向该副本的 RustCodeGraph 子进程。

## 三轮结果

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 可见答复和工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efa92912001rD6GhnuAGSjgFh` | 200 / 0 / 5.501 | `chat.done` | `MCP_NOT_AVAILABLE`。内置搜索一次，返回空目录，目标工具调用为零 |
| 挂载 | `ses_1efa93fda001qZIYiHKWyXimYN` | 200 / 0 / 143.100 | `chat.done` | 给出调用链、分支和快照分析。三种目标工具均成功，另有其他工具及两次失败调用 |
| 撤除 | `ses_1efab706e00107yqNGJumOw5WN` | 200 / 0 / 6.428 | `chat.done` | 包含 `MCP_NOT_AVAILABLE`、进度和解释。内置搜索一次，返回空目录，目标工具调用为零 |

三轮均有一个 `chat.done`、零个 `chat.error`，无需续传。运行中和终态的 UI summary 均确认 `acp-grok / grok` 及 `full_access=true`。ACP options 均保留指定模型、提供商和来源，连接从运行中的 `true` 变为终态的 `false`。运行中访问级别为 `client_only`。

| 目标工具 | 代表性调用 ID | 实际完成输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_6a51ffe6c4c14e5ab1c95716` | `createRueSiteApp` 在 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_577d874fcce14bfa82cd4660` | 调用者 `mountRueSiteApp` 在 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_3f315e83bdfe4bd7a7dd0d3d` | `packages/runtime/src/hooks/useApp.ts:4` 转调 `_$createApp` |
| `rustcodegraph_node` | `call_3dd244c625ee4673ae37bfef` | 运行时工厂在 `packages/runtime/src/compiler-runtime/app.ts:57`，挂载时依次安装已注册插件 |

同一调用 ID 可关联开始和完成事件。完成事件的 `UseTool` 输入包含限定名，输出中的 `server_name` 和 `tool_name` 标识实际执行者。挂载轮成功完成 query 34 次、callers 7 次、node 16 次，另完成内置搜索 3 次以及同服务的 files 6 次、callees 4 次、explore 3 次和 status 1 次。一次 files 调用将参数放在 `tool_input` 之外，Grok 拒绝解析。另一次调用了不存在的 `ruejs_graph_probe__rustcodegraph_use_tool`。未观察到 shell、写入或其他 MCP 服务调用。

答复正确追踪 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp`，说明 SSR 跳过浏览器挂载、缺少 `#app` 时抛错、路由和 i18n 在根工厂渲染前安装，以及根组件按 `/rust-layers` 路径决定是否绕过布局。答复指出调用图未收录模块顶层调用。移除清空后新树追加在快照之后是源码推断，答复提出首页冷启动及从首页导航至无布局路由两个场景，均未实际执行。首页可用既有 `.site-header` 检查重复，但快照专属标记需要测试夹具提供。第二个场景还需注册 `/rust-layers` 或以夹具提供该路由，当前路由表未注册此路径。

## 原生挂载与审批边界

- 宿主建会话入口把选中服务传入 ACP `session/new.params.mcpServers`。独立原生探针以含真实 stdio 定义的建会话请求复核，ACP v1 握手成功，声明 HTTP、SSE 及会话加载支持。新会话选择 `shiju-session`，回环 Chat Completions 请求中出现 `ruejs_graph_probe` 已连接及 8 个工具的提示，模型工具列表暴露 `search_tool` 与 `use_tool`。原生工具发现依赖这两个入口，未找到固定版 ACP 受管入口可用的直接展开选项。传输切换无法解决本次观察到的额外调用和答复偏差。 [Grok Build MCP 文档](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/07-mcp-servers.md)
- Grok 模型驱动为各会话生成私有 `GROK_HOME` 和绑定模型的 TOML。网关也使用隔离 HOME。两次控制轮的原生搜索均返回空工具目录，未继承同名常驻服务。挂载轮的工具由原生 Grok 通过 stdio 调用，指定模型根据工具结果继续推理。
- 独立回环模型夹具各发出一次真实 query 请求，Grok 均发送 `session/request_permission`。选 `reject_once` 时以 `cancelled` 结束，原因为 `PermissionRejected`，未向夹具回传工具结果。选 `allow_once` 时，续轮收到真实结果 `createRueSiteApp` 位于 `app/app.tsx:32`，最终 `end_turn`。三轮正式请求未人工回复审批。权限探针保留原生审批流程，并使用独立配置和占位中继令牌。

本次保留现有原生适配，未改驱动或权限策略。失败集中在当前 Grok 工具发现路径及指定模型执行约束，既有生命周期用例不覆盖该偏差，本轮未执行 Cargo。重新验收需要受管入口可按会话直接声明目标工具并保留原生审批，再用原提示与模型复跑三轮。私有网关退出 0、端口关闭，私有租户配置及临时 MCP 文件已清理。RueJS 副本的 4108 个归档普通文件与原提交一致，索引外无新增文件。原始证据留在本机 0700 私有目录。
