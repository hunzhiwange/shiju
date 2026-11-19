# Harn 官方工具与 MCP 共存复验

2026-11-19 在 Darwin arm64 以 Harn 0.10.201 受管 v4 直连 ACP，通过无挂载、挂载、撤除三轮。挂载轮完成一次官方读取和三次 RustCodeGraph 调用，四项结果均以原调用 ID 进入模型续轮。撤除后的新会话对目标工具零调用。原生工具目录保留 `read_file` 和 `write_file`，外部目录按会话追加。未发现需要修复的适配器裁剪。

## 固定入口和授权

- 注册 key 为 `harn`，预设为 `acp-harn`。从已有受管缓存取四项制品到私有目录，以当前合约逐项验 SHA-256，均一致。受审安装目录后缀为 `harn-0.10.201-model-source-v4`。入口 `harn` 无参数，启动 `harn-native serve acp --session-environment isolated <私有 pipeline>`。原生文件自报 `harn 0.10.201`。[注册和入口审计](../4.md#harn) · [制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 四项摘要依次为 `harn` 的 `17ad39da24845bb62914b46b748f30d63ebd11ca5f121af42fd8597a0ed94926`、`harn-native` 的 `23c951212c045c56deea491f9ce9505b8298472daa862334b33413ea31dcdcca`、`agent.harn` 的 `40ce0f80ad9033a8f72aa22d3228cba9f2068781aefaae7450664f312f335cdd`、`shiju_stdio.py` 的 `e6870d58c6c59cb1f1d224a66cdaa8455b34057249912b7bce5b9e21f95a074a`。
- 复用 `session_model` 的私有 provider overlay 形态，Chat Completions 指向动态端口的回环夹具。选模回读为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，它是夹具使用的模型标识。三轮原生模式回读均为 `ask`。探针宿主只响应私有工作区内指定文件的读请求，MCP 逐次批准。环境变量只使用合成中继凭据和临时状态路径，验证过程未授权文件写入或进程工具。RustCodeGraph 1.2.11 摘要为 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`，实际服务以 `serve --mcp --path <私有 RueJS> --no-watch` 经受管 stdio 沙箱启动。RueJS 从 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出，私有索引含 3478 个文件。[受管说明和探针](../../../../../patches/acp-model-source/harn/README.md)

## 当前三轮

三轮分别新建会话。每轮的官方 `read_file` 由 Harn pipeline 发起，经 launcher 转成 `fs/read_text_file`，由 ACP 探针宿主读取该轮生成的文件值。MCP 执行方为挂载的 `ruejs_graph_probe` stdio 服务，Harn 对两类调用均报告 `harn_builtin`。

| 轮次 | ACP 会话 ID | 官方读取 | MCP 工具和调用 ID | 模型请求数 | ACP 终态 / 入口退出码 |
| --- | --- | --- | --- | --- | --- |
| 无挂载 | `01a1f2af-b2e2-75cd-80fc-811033b3c55d` | `read_file`，`call_native_read`，完成 | 0 项声明、0 次调用 | 2 | `end_turn` / 0 |
| 挂载 | `01a1f2af-bca7-7090-b525-0ce8a702cdde` | `read_file`，`call_native_read`，完成 | `rustcodegraph_query` / `call_query`，`rustcodegraph_callers` / `call_callers`，`rustcodegraph_node` / `call_node`，均完成 | 5 | `end_turn` / 0 |
| 撤除 | `01a1f2af-c1ec-759b-9a9b-a0449042e33e` | `read_file`，`call_native_read`，完成 | 0 项声明、0 次调用 | 2 | `end_turn` / 0 |

每项工具在其会话中以同一 ID 从 `pending` 进入 `in_progress` 并完成。挂载轮显示的三个 MCP 工具名都带 `shiju_mcp_ruejs_graph_probe__` 前缀。MCP 的开始事件参数为空，执行更新提供完整参数，宿主收到各 ID 恰好一次审批并回复 `allow_once`。结果分别返回 `createRueSiteApp`（`app/app.tsx:32`）、`mountRueSiteApp`（`app/app.tsx:39`）和 `useApp` 调用 `_$createApp` 的源码。第五次模型请求中，四个工具结果按原 ID 回读，随后夹具输出确认。两个对照轮的模型目录、工具事件和续轮结果均无目标服务。

## 边界与定向验证

- launcher 从独立状态目录启动，三轮各先验证 cwd 不匹配时建会话被拒，随后在绑定目录成功建会话。挂载轮三项允许工具可见，`rustcodegraph_status` 同时列于允许和拒绝名单，最终目录排除该工具。原探针 `mcp_test.py` 退出 0，8 个会话场景和 1 个裸启动握手检查通过，覆盖单次拒绝、审批期间取消且迟到批准无效、`load` 和 `resume` 挂载及从同一会话移除挂载。宿主桥 3 项、pipeline 1 项、安装脚本夹具 2 项均通过。
- 直接调用原生入口完成 ACP v1 握手，非空 `mcpServers` 在 `session/new` 返回 `-32602`，进程退出 0。此前观测的原生替代路径仍受 `tool:mcp:read` 和 `tool:mcp:mutate` 效果上限约束，本次沿用受审桥和原生 `ask`。受管制品经审范围为 Darwin arm64，通用 ACP 预设通过原生 provider overlay 和受审私有 pipeline 提供会话模型，未公布 `cli_local`。裸启动证据限握手诊断。[原生断点与替代路径](../../mcp-acceptance/harn.md#首个失败边界与替代路径)
- 本次复验采用直连 ACP 与回环模型。缺本次网关 SSE、网关会话回读、真实供应商及其他平台证据，两个空挂载轮也未另采独立服务审计。历史 v4 正式网关挂载轮的 9 次图谱和 1 次官方读取为独立证据。未重跑冷安装、Cargo 或桌面构建。[历史 v4 网关验收](../../mcp-acceptance/harn.md#v4-正式网关验收通过)

本机私有证据目录 `/tmp/shiju-harn-coexist.CpvIOyOH` 保留 `coexist.py`、逐轮 ACP 与回环记录、定向探针日志和校验后的运行副本。`python3 <证据目录>/coexist.py <证据目录>/runtime/harn <证据目录>/runtime/rcg <证据目录>/ruejs` 可重跑三轮。检查新增回执的 `git diff --no-index --check /dev/null docs/dev/acp/mcp-policy-audit/coexistence/harn.md` 无空白错误诊断。
