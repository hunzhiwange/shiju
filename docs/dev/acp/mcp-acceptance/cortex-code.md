# Cortex Code 动态挂载 RustCodeGraph

2026-11-19，挂载轮未通过。Cortex Code 的 ACP 建会话入口丢弃 `mcpServers`，导致模型无法调用挂载服务。

## 环境和请求

- Darwin arm64，私有回环网关使用此前验收的 ShiJu 0.6.0，SHA-256 为 `ac9b96bce8a3aee0cb6102dc98a9b365efdfccfe0cddc4901df7860685b56877`。通过已核验的发行归档缓存安装 Cortex Code `1.2.46+175654.6f8561fd271f`（`model-source-v2`），HTTP 200。启动器、发行程序和本地中继组合摘要均与仓库受管清单一致。ACP 握手为协议 v1，agentInfo 版本为 `1.0.0`，支持加载会话，未声明 HTTP 或 SSE MCP 能力。
- 复用 [验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。沿用租户 `10001 / 100010005`、`session_model`、`full_access=true`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。预设与后端为 `acp-cortex-code / cortex-code`。
- RueJS 为 `main` 上 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的工作区副本，含已有未提交变更，索引 3687 个文件。挂载使用名为 `ruejs_graph_probe` 的 RustCodeGraph 1.2.10 原生 stdio 服务和 `--no-watch`。宿主在挂载轮启动了 RustCodeGraph。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 答复 | `chat.tool` 数 |
| --- | --- | --- | --- | --- | --- |
| 无挂载 | `ses_1ef93aedb0016zt6j3B10JeQn9` | 200 / 0 / 14.268 | 一个 `chat.done` | `MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1ef93e76b001swAJmq6tk2B2bL` | 200 / 0 / 12.081 | 一个 `chat.done` | `MCP_NOT_AVAILABLE` | 0 |
| 撤除 | `ses_1ef9417650014yc9jpkU9ht4Dq` | 200 / 0 / 11.822 | 一个 `chat.done` | `MCP_NOT_AVAILABLE` | 0 |

三轮均无 `chat.error`。UI summary 均确认绑定 Cortex Code 和 `full_access=true`。ACP options 终态均显示 `connected=false`，所选模型、提供商与 `session_model` 来源符合请求。无挂载和撤除答复符合合同。挂载未产生三种工具的开始、完成或失败事件，无调用结果可关联到服务名，答复也没有按合同给出调用链、分支、静态快照推断或回归场景。

## 断点及权限

- 直接向同一受管启动器发送含真实 stdio 定义的 ACP `session/new`，建会话成功，回环模型仅收到 `read`、`write`、`edit`，提示以 `end_turn` 结束。固定发行包的 `newSession` 实现只用 cwd、调用方会话 ID、连接名创建会话，并把 `_meta` 交给会话设置，未读取 `mcpServers`。把同一服务改为指向模型中继同端口的 HTTP 诊断地址，结果相同，监听器没有收到 MCP 请求。
- `--only-explicit-mcp-servers` 保持开启。固定版 `acp serve --help` 未列出 `--mcp-config`。该发行版拥有 `_meta.clientTools` 扩展，私有日志确认显式名称可以注册，但受管本地组合的声明和调用白名单只允许三个原生文件工具。ShiJu 的 ACP 客户端当前也拒绝扩展回调。基于该扩展的适配需要宿主按会话验证和执行工具、保留审批和撤销，并处理加载会话时的工具绑定。原生 stdio 子进程受现有沙箱禁止，网络仅允许绑定的模型中继端口。 [Snowflake ACP 文档](https://docs.snowflake.com/en/user-guide/cortex-code/cortex-code-acp) · [Snowflake CLI 发行记录](https://docs.snowflake.com/en/user-guide/cortex-code/changelog)
- 独立运行同一 RustCodeGraph stdio 服务，三个真实 `tools/call` 均成功。 `rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 命中 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。这些结果来自独立 MCP 连接，Cortex 没有读回。
- 独立权限探针收到一次原生 `write` 的 ACP 审批请求。选 `reject_once` 后目标文件不存在，模型续轮见到 `Denied by user`。三轮真实请求均无工具调用，无法验证 MCP 的审批行为。保持受管门禁，未修改驱动、补丁或公共协议，未运行 Cargo。

网关正常退出，端口关闭，私有租户配置副本和临时 MCP 配置已清理。保留原始 SSE、会话回读与探针在本机 0700 私有验收目录。复跑前需有支持会话级 MCP 且保持现有审批边界的受管入口。
