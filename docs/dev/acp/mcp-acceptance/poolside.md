# Poolside 动态挂载 RustCodeGraph

2026-11-19，验收阻塞于受管 Poolside 的 `session/new` 门禁。无挂载和撤除对照通过，挂载轮在发起模型推理前被拒绝。

## 验收环境

- Darwin arm64，受管 Poolside 1.0.16。原生程序 SHA-256 为 `5dcb0162fd21a535e4f67117125e7d205e27a6881862aa223d0424082848f874`，启动器为 `a52c42721d5badf57aff54da940a4e44e9173f20da415376404609a29c98c646`，均与受管清单一致。私有网关回读已安装。
- 私有回环网关复用 shiju 0.6.0 构建，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。私有实例启用 `acp-poolside`，使用租户 `10001 / 100010005`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model` 和 `full_access=true`。
- 提示取自提交 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 私有副本取自 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32` 的 `createRueSiteApp`。挂载轮使用原生 stdio `ruejs_graph_probe`，关闭文件监听。[复用请求](../ACP验收.md#可复用的验收请求)

## 同一请求的三轮结果

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 答复及工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efbd85b00016VRIZd96kiztzU` | 200 / 0 / 10.708320 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，0 条工具事件 |
| 挂载 | `ses_1efbdb0c8001TWslP4dFBXsHnu` | 200 / 0 / 1.528858 | `chat.error` ×1 | 建会话失败，无答复，0 条工具事件 |
| 撤除 | `ses_1efbdb832001yddpC784AHDjq1` | 200 / 0 / 10.330277 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，0 条工具事件 |

三轮终态 UI summary 都回读 `acp-poolside / poolside` 与 `full_access=true`。两轮对照在运行中回读 `connected=true` 和 `access_status.support=client_only`，运行中及终态的 ACP options 均确认指定提供商、完整模型 ID 和 `session_model`。终态连接释放。挂载轮 ACP options 的 `connected=false`、`selected_model=null`，未触发原生选模或工具审批。三种目标工具 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 均零调用和零输出，调用链、行号与静态快照分析无可验收答复。

## 失败边界与复跑条件

- 宿主在挂载轮完成 stdio MCP 初始化。受管启动器在处理 `session/new` 时拒绝非空 MCP 列表。SSE 原始错误包含 `Poolside requires the reviewed workspace and no external MCP servers`。隔离探针固定相同的规范工作目录，向受审启动器实际发送非空 `mcpServers`，复现 JSON-RPC `-32000`，私有回环模型未收到推理请求。
- 独立原生探针使用同一启动器生成的私有设置、必需的 Docker 沙箱和回环模型替身。ACP v1 握手报告 `pool-acp` 1.0.16、`loadSession=true`、`mcpCapabilities={}`。分别发送空列表与同一 RustCodeGraph stdio 定义，原生端均建会话并以 `end_turn` 完成短提示，`/v1/chat/completions` 请求均只声明 `read` 和 `write`。原生端接受请求的行为不足以证明工具可用，移除门禁也无法使这条路径向模型声明目标工具。
- 保留受管 Docker 只读工作区、宿主写入审批和模型中继约束。此次三轮无文件或终端工具请求。复跑需可验证的模型工具入口，并在该入口中保留会话、租户、审批和撤销边界。

网关和探针均已退出，回环端口关闭。本轮两个已退出的沙箱容器按私有工作目录的挂载来源核实后移除。私有挂载已撤除，RueJS 副本的 4108 个普通文件与导出内容相同，私有租户配置及数据库副本已清理。原始 SSE 与脱敏探针摘要保存在本机私有目录。本次只更新文档，生命周期夹具未覆盖受管启动器的拒绝分支，未运行 Cargo 或桌面构建。
