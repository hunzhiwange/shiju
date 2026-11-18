# GLM Agent 动态挂载 RustCodeGraph

2026-11-19，GLM Agent 1.17.1 已实测动态发现、调用和撤除隔离。完整验收阻塞于指定模型调用了额外工具。

## 环境

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
