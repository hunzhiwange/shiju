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

任务保留为已阻塞。新版网关构建在等待外部 Rust 工作区检查期间未取得运行机会，已结束本任务的排队进程，未领取 Cargo 槽位。待编译资源可用时，使用 `python3 scripts/cargo_slot.py cargo build -j 1 --locked -p vw-cli --bin shiju` 获取新二进制，再用固定 curl 复跑三轮，核对真实模型答复、工具范围和终态。
