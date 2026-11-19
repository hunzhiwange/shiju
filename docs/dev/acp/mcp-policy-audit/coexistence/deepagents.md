# DeepAgents 0.1.42 工具共存复验

2026-11-19，macOS arm64 上的受管 v4 通过直接 ACP 三轮复验。同一挂载会话、同一次提示内，宿主批准的原生 `read_file` 和真实 RustCodeGraph 查询均成功，模型续轮收到两个原调用 ID 对应的结果。撤除后新会话对目标服务零调用。受测挂载路径保留全部八项原生工具声明，本次提交验收回执。

## 制品、入口和权限

- 注册 key 为 `deepagents`，原始入口为 `npx -y deepagents-acp@0.1.42`，另有本地 `deepagents-acp` 候选。受管冷装在私有目录使用仓库安装器和固定 npm 锁，按锁安装 56 个包，供应包完整性及五个受管文件摘要通过。原包 `dist/cli.js` SHA-256 为 `6574016c7212831f66c79a2412e7de56c75363293c182fdc311749b8bb251238`，受管版为 `99235b0c39283be84bef07a2d3beb908533e93d5697e929eac62b450a01a7f52`。原包自报 `v0.1.42`，受管 ACP 握手为协议 v1、`agentInfo.version=0.0.1`、`loadSession=true`，MCP HTTP/SSE 均为 false。 [注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L520) · [合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L719) · [安装器](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/install.py)
- 实际通过受管 `node_modules/.bin/deepagents-acp --model openai:gpt-4o --workspace <私有副本>` 启动，解析至已核对摘要的 `dist/cli.js`。本次使用 Node v24.22.3。两个版本均有 `--model`，实际模型请求均为 `gpt-4o`。以私有 HOME、合成认证和 `OPENAI_BASE_URL=http://127.0.0.1:<临时端口>/v1` 启动，关闭 LangChain/LangSmith tracing。受管走 Chat Completions，原包走 Responses，两种回环协议均由本地夹具响应。
- ACP 模式实读为 `agent`，客户端声明读写文件能力。本轮只提供指定只读文件的宿主 RPC，要求原生和 MCP 分别逐次批准。原生模型目录为 `delete`、`edit_file`、`glob`、`grep`、`ls`、`read_file`、`task`、`write_file`。受管守卫只允许已批准的宿主读写和获选 MCP 进入执行器。宿主文件 RPC 失败继续受守卫拦截。 [守卫](../../../../../patches/acp-model-source/deepagents/guard.mjs) · [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/deepagents.rs)
- 真实服务为 stdio `ruejs_graph_probe`，使用 RustCodeGraph 1.2.10，二进制 SHA-256 为 `68cd787626f55a5fff5a556b2bbb449ae0fea597775fc2b40cd3f4e814dee42c`。从本地 RueJS `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出私有副本并索引 3478 个文件，启动参数为 `serve --mcp --path <私有副本> --no-watch`，无额外 MCP 环境变量。服务策略允许 query 和 status，同时以规范化 ID 拒绝 status，模型只收到该服务的 query。 [发现与策略](../../../../../patches/acp-model-source/deepagents/mcp.mjs)

## 同一受管入口的三轮

三轮使用同一个项目副本、相同 ACP 模式和三个全新会话。挂载轮另用只读样本中的独立标记验证实际宿主读取。表中的退出码来自 DeepAgents CLI。

| 轮次 | ACP 会话 | 目录及执行 | 模型请求数 / 提示终态 / 进程退出 |
| --- | --- | --- | --- |
| 无挂载 | `sess_a40c5ad965a5450a` | 8 项原生，零工具事件，目标 MCP 零调用 | 1 / `end_turn` / 0 |
| 挂载 | `sess_e408426ab2eb47cb` | 同 8 项原生及 1 项 MCP，均经批准后各执行一次读取和查询 | 3 / `end_turn` / 0 |
| 撤除 | `sess_074a477bdcb64f1e` | 8 项原生，零工具事件，目标 MCP 零调用 | 1 / `end_turn` / 0 |

| 模型工具名 | ACP 审批、更新与续轮共用 ID | 执行方及真实输出 | ACP 终态 |
| --- | --- | --- | --- |
| `read_file` | `mounted-native` | ACP 宿主收到批准后的 `fs/read_text_file`，实际读取私有工作区的 0400 样本并返回标记 | `completed` |
| `ruejs_graph_probe__rustcodegraph_query` | `mounted-mcp` | 已选 stdio RustCodeGraph 执行 `rustcodegraph_query`，`search=createRueSiteApp` 命中 `app/app.tsx:32`，`isError=false` | `completed` |

第二次模型请求收到文件结果，第三次请求携带两个工具的原 ID 与结果，两者均与 ACP 完成事件匹配。无挂载和撤除的空清单不触发桥的服务发现，模型目录和事件均为零 MCP。独立服务端调用计数和 stdio 子进程退出码未采集。探针退出后核对无本次 DeepAgents、Python 桥或 RustCodeGraph 进程残留。

## 加载、拒绝与原版对照

- 在同一挂载会话提示完成后，原选集 `session/load` 成功。仅更改同一服务的允许名单、以及移除服务的两次加载分别返回 `-32603` 和 `DeepAgents MCP selection differs from current session`，随后原选集仍可加载。覆盖同进程恢复，未测跨进程恢复或独立 resume。
- 独立拒绝轮 `sess_40c7a3a658834602` 对 `denied-native` 和 `denied-mcp` 均回复 `reject-once`。原生无宿主读 RPC，ACP 状态为 `completed`，结果明确写明执行被拒。MCP 在进入服务调用前被守卫拒绝，ACP 状态为 `failed`，该终态无 `rawOutput`。模型收到两条同 ID 的 `Tool execution denied`，三次模型请求后 `end_turn`，CLI 退出 0。
- 固定原包在相同工作区和 ACP 模式下完成无挂载、显式挂载及撤除三轮，三轮均为八项原生、零 MCP 声明及工具事件，各一次模型请求、`end_turn`、CLI 退出 0。挂载会话 `sess_fd93cd4cea3a4a10` 接受带挂载的 `session/new`，首个模型请求的目录无 MCP，加载时清空清单也被接受。对应原包的 new/load 未接入 `mcpServers`。历史 v2 的独立断点是非空 new/load 在模型提示前显式拒绝。 [历史验收](../../mcp-acceptance/deepagents.md)

## 验证范围与复跑

- `node patches/acp-model-source/deepagents/guard_tests.mjs <受管包目录>`、`python3 scripts/acp_onboarding_deepagents_acceptance.py <受管入口>` 和 `python3 patches/acp-model-source/deepagents/mcp_test.py <受管入口> <RustCodeGraph> <已索引副本>` 均退出 0。现有探针覆盖批准、拒绝、取消及继续、未审本地编辑拦截、宿主文件错误及三个真实图谱工具。
- 私有目录 `/private/tmp/shiju-deepagents-12.9Xicqdvo`（0700）保留 `coexist.py`、两个制品副本和逐轮摘要。复跑 `python3 <私有目录>/coexist.py <RustCodeGraph> reviewed` 或将最后一个参数换成 `original`。回环进程已退出，临时只读标记已删除。
- 本轮为真实工具配合合成模型的 ACP 协议证据。网关、真实模型及 Cargo 未重跑。历史 v4 网关 248 次成功的图谱调用另有回执，本次新增证据是同轮原生读取及仅变更服务策略时拒绝加载。ShiJu 普通及指定模型启动都核验受审入口，指定模型走通用 ACP 能力接口和私有中继。平台以合约及代码为据，Linux 可受管安装但非 Darwin 拒绝 stdio 挂载，Windows 在受管安装入口拒绝。Linux 和 Windows 未做实机复验。 [历史真实模型回执](../../mcp-acceptance/deepagents.md#最终集成回归与受管-v4) · [启动核验](../../../../../crates/libs/acp-client/src/client/process.rs#L85) · [平台入口](../../../../../crates/libs/acp-agent-registry/src/managed_runtime.rs#L376)
