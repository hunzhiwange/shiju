# Stakpak 动态挂载 RustCodeGraph

2026-11-19，验收阻塞于 Stakpak 0.3.88 的会话 MCP 工具发现。无挂载和撤除对照通过。挂载轮宿主已初始化 RustCodeGraph，模型仍报告工具不可用。

## 验收环境

- Darwin arm64，私有回环网关使用 shiju 0.6.0，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。通过私有实例安装受管 Stakpak，安装状态回读为 true。原生程序 SHA-256 为 `70c559eff250a5be0dcdb19f31f13dca86847ee57648dba78e531506bb482ebc`，启动器为 `bff6d5b2b49b92ce759be1e00cb6955cc0ec7bf181122ae553544197721aca30`，均与受管清单一致。
- 私有实例启用 `acp-stakpak`，沿用租户 `10001 / 100010005` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。提示取自提交 `189d22dc1`，UTF-8 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)
- RueJS 私有副本基于 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 工作目录。RustCodeGraph 1.2.10 索引 3686 个文件，定位到 `app/app.tsx:32` 的 `createRueSiteApp`。挂载轮使用 `ruejs_graph_probe` 原生 stdio，关闭文件监听。

## 三轮真实请求

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 答复及工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efc35095001trBAX333LuO2or` | 200 / 0 / 5.265442 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，0 条工具事件 |
| 挂载 | `ses_1efc3669a001l5ld7pmbuhC8w4` | 200 / 0 / 5.927033 | `chat.done` ×1 | `MCP_NOT_AVAILABLE` 及工具不可用说明，0 条工具事件 |
| 撤除 | `ses_1efc37eb60010rQGICF2qF3PYG` | 200 / 0 / 5.043792 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，0 条工具事件 |

三轮均无 `chat.error`。终态 UI summary 回读绑定 `acp-stakpak / stakpak` 和 `full_access=true`。运行中 ACP options 显示 `connected=true`、`access_status.support=client_only`、后端版本 `0.3.88`，结束后连接释放。运行中及终态均回读 `session_model`、指定提供商和完整模型 ID。三种目标工具 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 均无调用或输出。挂载轮的调用链、行号和静态快照分析均无答复可核对。三轮也没有其他工具请求或审批事件。

## 第一处失败边界与复跑条件

- 挂载轮宿主完成了 RustCodeGraph MCP 初始化。仓库 ACP 新会话路径将所选服务带入 `session/new.params.mcpServers`。隔离探针使用相同摘要的受管启动器，记录实际发送的 JSON-RPC 帧和回环模型入口收到的工具定义。ACP 握手为 v1，`loadSession=true`，声明 `http=true` 和 `sse=true`。
- 探针分别发送空列表、同一 RustCodeGraph stdio 定义和两个指向回环探针的 HTTP、SSE 定义。四次均成功建会话并以 `end_turn` 结束，回环只收到模型请求。每次模型请求都仅有 `stakpak__view`、`stakpak__create` 和 `stakpak__str_replace`，没有探针 MCP 连接或工具声明。固定版源码的 `new_session` 没有读取 `mcp_servers`，`prompt` 使用进程启动时装载的工具集。[Stakpak 0.3.88 ACP 实现](https://github.com/stakpak/agent/blob/v0.3.88/cli/src/commands/acp/server.rs)
- 另在隔离的 `SHIJU_STAKPAK_RUNTIME` 写入启动期 MCP 配置。原生模型请求仍只包含三个内置工具。使用同一 sandbox profile 单独验证 RustCodeGraph 进程启动，`sandbox-exec` 退出 71 并报告 `Operation not permitted`。本机 RustCodeGraph 的 `serve --help` 只列出 stdio MCP，现有受管路径未找到可将其接入 Stakpak 且保留会话、审批和撤销的回环转换。复跑需具备这些边界的受审原生版本或兼容入口。[Stakpak 启动期 MCP 实现](https://github.com/stakpak/agent/blob/v0.3.88/cli/src/commands/agent/run/mcp_init.rs)

自有网关和探针已退出，端口已关闭。已撤除私有挂载并清理租户配置、凭据和数据库副本。原始 SSE 和探针证据保存于本机私有目录。此次只更新验收记录，现有生命周期夹具未覆盖该原生工具发现边界，未运行 Cargo 或桌面构建。
