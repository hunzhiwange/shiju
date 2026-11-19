# aionrs 官方工具与 MCP 共存复验

2026-11-19，Darwin arm64。固定受管 v2 桥的直接 ACP 回环三轮通过。挂载会话内一次性批准了官方 `Read` 与 RustCodeGraph 查询，两个实际结果均按原调用 ID 进入续轮。撤除后新会话的目标服务访问及执行均为零。

## 入口与权限

- 注册 key 为 `aionrs`，预设为 `acp-aionrs`。合约固定原生 `aion-cli` 0.2.11，源码提交 `8e61a90329fa9f67c4fdf7e97fe02c24dba33f75`。本次从本机私有缓存复制历史受审制品到独立目录，桥和引擎摘要分别为 `b2ab720e864645b1b0dc681fcf852873c501d3a8f19de7b9a1ffc2408c9a466a`、`aae767bf2d5132d730a77526132fe3c689ff2cf60c81284d0b7a86f97ffa1662`，与历史正式网关验收一致。引擎版本命令返回 `aionrs 0.2.11`。[受审制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [历史网关验收](../../mcp-acceptance/aionrs.md#指定模型正式网关三轮)
- 以同目录的 `aionrs-acp` 无参数启动，复用驱动沙箱模板和 `SHIJU_AIONRS_{SANDBOX,STATE,WORKSPACE,MCP,MCP_TOKEN}` 私有绑定。由桥以 `sandbox-exec -f <policy> <sibling-aionrs> --json-stream --no-color --project-dir <state>` 启动引擎。原生握手须为 0.2.11、`default` 模式且支持审批。工具自动批准关闭。三轮使用同一个临时工作区、模型别名 `local-probe` 和 Chat Completions 回环协议，每轮重新启动桥并新建连接和会话，使用独立状态和临时令牌。宿主仅回复逐次允许或拒绝。
- 复用既有 ACP 补充探针，将原生端的同端口 MCP 请求经私有 Unix socket 交给真实桥，桥启动隔离的 RustCodeGraph 1.2.10 stdio 服务。只索引临时工作区的单文件 TypeScript 夹具，官方读取独立的随机标记文件。标记及查询结果由工具产生，夹具只用它们校验续轮。目标服务的 allow 包含 query 和 node，deny 同时包含 node，callers 未加入 allow。实际发现后只公布 query。
- 原生 aionrs 有模型配置入口。ShiJu 用受审桥、私有回环中继和会话绑定配置。入站协议层新建会话接受这里验证的 stdio 挂载，握手声明远程 HTTP 和 SSE 为 false。引擎侧 HTTP 是桥的内部投影，依赖私有 Unix socket。工具及模型配置机制属于原生通用能力，受管沙箱的本次实测范围为 Darwin arm64。旧桥的非空列表拒绝及原生 stdio 与子进程禁令的冲突保留在历史记录中。[分组审计](../3.md#aionrs)

## 三个独立新会话

下表的工具 ID 同时用于 ACP 审批、工具事件和模型 `tool_call_id`。五个样本分别启动桥，`session/new` 在各自连接内返回 `aionrs-0`。`session/prompt` 的 JSON-RPC ID 均为 3。

| 轮次 | 执行者和调用 ID | MCP 请求与执行数 | 模型请求和结果 | ACP 终态 / 桥退出码 |
| --- | --- | --- | --- | --- |
| 无挂载 | 原生 `Read` · `unmounted_0` | 0 请求，0 执行 | 2 次，后一轮收到当轮文件标记 | `end_turn` / 0 |
| 挂载 | 原生 `Read` · `allow_0`，RustCodeGraph `rustcodegraph_query` · `allow_1` | initialize、initialized、tools/list、tools/call 各一次，MCP JSON-RPC 调用 ID 为 0 | 3 次，两条结果各按原 ID 回传，查询实际命中 `createRueSiteApp` | `end_turn` / 0 |
| 撤除 | 原生 `Read` · `removed_0` | 0 请求，0 执行 | 2 次，后一轮收到当轮文件标记 | `end_turn` / 0 |

正向四个调用的 ACP 更新均由 `in_progress` 到 `completed`。桥的官方工具活动标题为 `aionrs tool`，以同 ID 的审批标题 `Read` 和续轮标记确认归属。挂载工具的模型全名为 `mcp__ruejs_graph_probe__rustcodegraph_query`。三轮模型夹具均看到原生 `Read/Write/Edit/Glob`，其余三项仅核对目录。

## 拒绝、取消和缺证项

- 拒绝附加样本的 ACP 审批 ID 对应工具 `deny_0`，返回 `deny` 后工具终态为 `failed`。MCP 完成发现，`tools/call` 为零。模型续轮在 `deny_0` 收到拒绝结果，提示为 `end_turn`，桥退出 0。
- 取消附加样本在 `cancel_0` 等待审批时发送 `session/cancel`。提示以 `cancelled` 结束，模型请求共一次，目标服务只有发现请求，`tools/call` 为零，桥退出 0。五次退出后私有 socket 均已删除，未发现残留的本次进程。
- 握手回读 `loadSession=false`，无 resume 能力声明。实际 `session/load` 返回 `-32602`，`session/resume` 返回 `-32601`。
- 当前直接 ACP 回环的 HTTP 夹具承担中继转接，观察的是引擎发给该夹具的完整目录。生产中继的目录过滤以当前源码和历史正式回执为依据。历史真实模型网关三轮目标工具成功数为 0 / 101 / 0，当时没有官方工具执行。本次无真实上游模型调用、网关 SSE 或网关模型绑定回读，也没有原生引擎单独退出码。回环未复现官方工具被挂载裁剪，因此未改适配器或执行 Cargo。[生产中继过滤](../../../../../crates/libs/model-relay-http/src/aionrs_mcp.rs) · [历史正式网关证据](../../mcp-acceptance/aionrs.md#指定模型正式网关三轮)

## 复验记录

本机私有证据为 `/tmp/a4.Aqmmvz/probe.py` 和 `/tmp/a4.Aqmmvz/trace.txt`。从仓库根目录执行下列命令，探针退出 0。探针使用临时工作区内的只读验收数据和私有制品副本。

```sh
ACP_RUN=/tmp/a4.Aqmmvz
python3 "$ACP_RUN/probe.py" "$ACP_RUN/bin/aionrs" "$(command -v rustcodegraph)" \
  "$ACP_RUN/workspace" crates/libs/llm-runtime/src/acp/model_source/drivers/aionrs.rs
```

新增回执按任务要求运行 `git diff --no-index --check /dev/null docs/dev/acp/mcp-policy-audit/coexistence/aionrs.md`，无空白错误诊断。
