# Cortex Code 受管工具共存

2026-11-19 在 Darwin arm64 从已核验的固定归档离线安装受管版 `1.2.46+175654.6f8561fd271f`。修正后目录后缀为 `model-source-v4`，入口为 `cortex acp serve`。四项文件摘要均匹配受管合约，`cortex-local` 的 SHA-256 为 `f3fa0d6736b14d5ef2e4b80cddc0ef192fa3d685bb0378b93d250d4b1d1bc884`。

本地模型夹具监听 `127.0.0.1`，MCP 使用既有 `ruejs_graph_probe` RustCodeGraph 1.2.11 stdio 服务和私有 RueJS 索引副本。运行前审查入口和两个子进程的 Seatbelt 配置。外层嵌套方案用系统 `true` 验证时返回 `sandbox_apply: Operation not permitted`。随后在现有两个子进程配置内均阻断 Mach 服务查找与注册、Apple Events、POSIX IPC 和钥匙串路径读取。Cortex 只可连接绑定的本地模型端口，MCP 保持禁网。全程使用受管本地组合，未运行供应商原包、启动登录或修改系统权限。

在相同隔离下，修复前的受管组合退出 1。挂载后首个模型请求声明了八个 MCP 工具，缺少 `read`、`write`、`edit`。根因是受管原生补丁在 MCP 注册集合非空时用该集合替换原生工具白名单。修复让模型工具声明与调用校验使用原生白名单和当前注册集合的并集。原生工具继续由 Cortex 执行，MCP 继续经桥校验当前调用并等待宿主逐次批准。

修正后的入口通过一次挂载会话的两次顺序调用。模型目录包含三项原生工具和八项 MCP 工具。三个模型回环请求均返回 HTTP 200，ACP 提示以 `end_turn` 结束，受管入口和探针均退出 0。

| 工具 | 原始调用 ID | 执行方 | ACP 状态 | 模型续轮结果 |
| --- | --- | --- | --- | --- |
| `read` | `call_read` | Cortex 原生文件工具 | `completed` | 同 ID 读回私有文件哨兵 `CORTEX_NATIVE_READ_7367f5c3ac446a64aca878eb` |
| `shiju_mcp_ruejs_graph_probe__rustcodegraph_query` | `call_query` | RustCodeGraph stdio，经宿主 `allow_once` | `completed` | 同 ID 读回 `createRueSiteApp` 查询命中 |

复验命令为 `python3 patches/acp-model-source/cortex-code/probe.py "$ACP_LAUNCHER" "$GRAPH_EXE" "$ACP_WORKSPACE" --coexistence`。安装脚本使用已核验的本地缓存归档，未运行 Cargo。名单、撤除、恢复、多平台和供应商原包均未纳入本轮验收。
