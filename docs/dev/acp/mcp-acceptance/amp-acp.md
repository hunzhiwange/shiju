# Amp ACP 动态挂载 RustCodeGraph

2026-11-19，Amp 的挂载轮在 ACP 建会话时被受管入口拒绝。无挂载和撤除轮均通过。任务保留为已阻塞。

## 环境与请求

- Darwin arm64，受管适配器 `amp-acp@0.12.0`，本地 ThreadWorker `@sourcegraph/amp@0.0.1778732631-gc5875c`。安装目录版本为 `amp-acp-0.12.0-model-source-v1`。适配器入口、控制桥、启动器及引擎四个摘要均匹配仓库契约。
- 复用已有回环网关构建，健康接口与文件 SHA-256 均为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。网关使用独立配置根和数据目录。首次三轮在宿主凭据检查处返回 `ACP session_model requires Provider credentials`。经私有网关的租户鉴权接口写入已有凭据后，按同一请求重新执行三轮。
- 从 `189d22dc1` 提取[复用请求](../ACP验收.md#可复用的验收请求)。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。使用 `acp-amp-acp / amp-acp`、租户 `10001 / 100010005`、`full_access=true`、`session_model` 及 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出干净副本。索引 3478 个文件，`createRueSiteApp` 位于 `app/app.tsx:32`。挂载使用原 RustCodeGraph 的 stdio 与 `--no-watch`，服务名为 `ruejs_graph_probe`。私有网关观察到宿主启动了对应的 stdio 进程。

## 三轮结果

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态与答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef8d3dc7001haVg4RHh8nonKy` | 200 / 0 / 3.927 秒 | `chat.done`，最终答复 `MCP_NOT_AVAILABLE` |
| 挂载 | `ses_1ef8d4e5e001w8fG1cotRs1l1m` | 200 / 0 / 1.011 秒 | `chat.error`，`Amp selected MCP isolation is unavailable` |
| 撤除 | `ses_1ef8d5417001N8U8L9zkWyDmB5` | 200 / 0 / 4.022 秒 | `chat.done`，最终答复 `MCP_NOT_AVAILABLE` |

三轮 UI summary 均回读到 `acp-amp-acp / amp-acp` 与 `full_access=true`。无挂载和撤除轮的 ACP options 回读到原指定模型和 `source=session_model`。挂载轮建会话失败，ACP options 无所选模型。三轮均无 `chat.tool` 事件。挂载轮的 `rustcodegraph_query`、`rustcodegraph_callers` 和 `rustcodegraph_node` 均无输出，调用链及答复准确性无法验收。

## 阻塞边界与权限

- 对同一受管入口单独发 ACP v1 请求。握手声明 `loadSession=true`，HTTP 与 SSE MCP 均为 `false`。携带实际 `ruejs_graph_probe` stdio 定义的 `session/new` 和 `session/load` 均返回 JSON-RPC `-32603`，详情同挂载轮。两次空列表 `session/new` 均成功并返回不同的会话 ID。
- 核对 npm 中同版本的原始 ThreadWorker，入口 SHA-256 为 `458435712c9c4e222822847fc9e70080cdf023ffb3c820e959a30425a22a5836`，匹配补丁输入。原生 `--mcp-config` 将 flag 条目并入全局与工作区设置。受管补丁在工具发现前清空这些配置，并将工具执行交给只识别宿主工具白名单的回环桥。直接传入 flag 仍会被清空，恢复原始合并会重新引入环境配置。当前受管路径缺少只接受会话所选服务且经宿主审批的 MCP 执行接口。
- 使用当前受管控制桥和拒绝审批的宿主桩分别探测 `Read`、`create_file`、`edit_file` 和 `Bash`。四次均请求审批且收到拒绝，文件与终端调用均为零。直接请求 MCP 工具返回 `Amp tool has no host route`。现有门禁继续生效。待有会话级 MCP 工具路由后，复跑批准、拒绝、撤除和原模型三轮。
- 本轮修改仅为记录，未执行 Cargo。私有网关和对应 stdio 进程均已退出，端口关闭。已清理私有凭据副本及项目挂载文件，原始 SSE 与 ACP 帧保留在本机权限 0700 的验收目录。
