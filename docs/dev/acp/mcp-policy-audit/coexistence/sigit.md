# siGit Code 1.7.10 原生工具与 MCP 共存回执

2026-11-19。受管 v3 入口通过回环模型三轮复验。挂载会话的官方读取和三项 RustCodeGraph 查询均成功，撤除后新会话的目标服务零调用。逐服务名单缺少共享策略接线，显式会话名单到达原生进程后也未过滤目录或阻止调用。

## 制品与入口

- 实测 Darwin arm64。注册 key 为 `sigit`，登记的上游命令为 `npx -y @smbcloud/sigit@1.7.10 --acp`。本次运行从既有私有安装复制的 `sigit-1.7.10-model-source-v3/sigit --acp`，ACP v1 握手回报 1.7.10、支持 load 和 resume，HTTP/SSE MCP 为 false。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L364)
- 固定源码 SHA-256 为 `088809cfc43ceda419913488390a65e830ac8ff66f1b5dd14a8d833d6b46369e`，仓库补丁为 `3884ccd9580594a612ebf06de5636b62bd749ed1098b89b0c4c8a7e5c2440c4e`，入口为 `322287c3fc4b2b88434f66464f56ea55f086d151b0c2682a44633defe90d34ea`。归档和补丁实测摘要匹配当前合约，入口实测摘要匹配复制后的构建收据。[制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [入口校验](../../../../../crates/libs/acp-agent-registry/src/managed_runtime.rs)
- 每轮为独立 ACP 进程、新会话和私有 HOME、`SIGIT_CONFIG_DIR`。模型用 `SIGIT_MODEL=fixture-generic` 指定，回环请求均回读该模型。`OPENAI_BASE_URL` 指向本轮回环服务，`OPENAI_API_KEY` 为合成占位值。`SIGIT_MCP=off` 下仅接受本会话显式声明的隔离服务，夹具使用 `SIGIT_PERMISSIONS=allow`，ACP 工具仍逐次请求 `allow_once` 或 `reject_once`。
- 宿主当前源码要求通用启动与模型驱动均校验受审入口，参数固定为 `--acp`。挂载还要求受管 macOS 会话模型连接、绝对 stdio 命令及空环境覆盖，HTTP/SSE 在建会前拒绝。模型驱动的非 Unix 分支明确拒绝，受审安装和会话模型能力发布限定 Darwin arm64。这些宿主门禁及其他平台为源码核对。本轮直连实测选模、审批和 v3 stdio 路径。[通用进程](../../../../../crates/libs/acp-client/src/client/process.rs) · [挂载门禁](../../../../../crates/libs/acp-client/src/client/protocol.rs#L272) · [选模驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/sigit.rs)

## 先复验显式名单

当前共享策略投影选中服务后没有 siGit 逐服务名单分支。会话允许列表按现有线格式传入 `_meta.claudeCode.options.allowedTools`，仅给 `read_file` 和完整 MCP 查询名。直连重放时，查询、调用方和节点三项仍可见并执行。另把允许 query/node、拒绝 node 的重叠规则以实验性 `shiju.sigit.toolPolicy` 注入服务 `_meta`。v3 仍发布并执行 node。实验字段未成为产品合约。[共享策略投影](../../../../../crates/libs/session-engine/src/acp_defaults.rs) · [会话元数据](../../../../../crates/libs/acp-client/src/client/session_meta.rs)

| 输入 | 获准查询 ID | 列表外或待拒绝节点 ID | 状态 |
| --- | --- | --- | --- |
| 当前会话名单线格式 | `session-allow-query` | `session-allow-node` | 均逐次获批、执行并完成，真实结果进入续轮，进程退出 0 |
| 另注入实验性逐服务 allow/deny | `service-allow-deny-query` | `service-allow-deny-node` | 均逐次获批、执行并完成，真实结果进入续轮，进程退出 0 |

两轮也执行了 `read_file`，ID 分别为 `session-allow-read`、`service-allow-deny-read`，由 ACP 宿主回调读取私有文件。首个共享断点为逐服务元数据投影缺失，v3 自身也缺少对应名单消费。共享接线和失败轨迹由[任务 43](../../../../../.plans/2026-11-19-acp-工具共存/43-集成与证据收口.md)收口。复跑标准为允许项正常执行、重叠拒绝优先、待拒绝工具在真实服务零调用，或无法安全表达名单时在挂载前明确拒绝。

## 同构建三轮回环

工作区来自历史 RueJS 私有副本。RustCodeGraph 1.2.10 以绝对路径运行 `serve --mcp --path <工作区> --no-watch`。每轮清除私有索引边车。挂载后 v3 自动创建 WAL/SHM，主索引摘要不变；夹具未持有 SQLite 事务。官方 `read_file` 由 ACP 宿主读取副本的 `app/app.tsx`，MCP 工具由 v3 沙箱内的 RustCodeGraph stdio 执行。

| 轮次 | ACP 会话 ID | 官方读取 ID | MCP 调用 ID | 工具与进程终态 |
| --- | --- | --- | --- | --- |
| 无挂载 | `b30cd096-22ab-426a-aa0e-29cdc807acc2` | `before-read` | 目标服务零调用 | `completed`，`end_turn`，退出 0 |
| 挂载 | `2cdf684c-c175-494c-8fc5-4bdb49e9e122` | `mounted-read` | `mounted-query`、`mounted-callers`、`mounted-node` | 均 `completed`，`end_turn`，退出 0 |
| 撤除 | `95087fa4-d716-4f08-a64b-3f969ee73c72` | `removed-read` | 目标服务零调用 | `completed`，`end_turn`，退出 0 |

挂载的三项完整工具名分别为 `mcp__ruejs_graph_probe__rustcodegraph_query`、`mcp__ruejs_graph_probe__rustcodegraph_callers` 与 `mcp__ruejs_graph_probe__rustcodegraph_node`。真实结果依次包含 `createRueSiteApp`（`app/app.tsx:32`）、`mountRueSiteApp`（`app/app.tsx:39`）和 `useApp` 转调 `_$createApp` 的源码。ACP 审批、更新与模型续轮结果使用相同调用 ID。两个对照的模型工具表均无目标服务。

- 同一 v3 进程对挂载会话分别以空列表和原挂载列表执行 `session/load`，两次成功。后续模型工具表按本次列表消失、恢复，prompt 都为 `end_turn`。重放的历史工具更新沿用旧 ID。
- 独立拒绝轮 `reject-once-query`、`reject-once-callers`、`reject-once-node` 与 `reject-once-read` 均获 `reject_once`，状态全为 `failed`。模型收到带原 ID 的拒绝结果，无文件读取回调，`end_turn`，进程退出 0。
- 仓库原生夹具定向跑 3 项，文件授权读取和宿主拒绝后禁止本地读取回退均通过。取消用例在释放合成响应后才等待终态，初跑返回 `end_turn`（退出 1）。仅在私有副本将释放时机移至收到取消终态之后，重跑 3 项均通过（退出 0），覆盖取消后续轮、重启 load/resume 和两条官方读取分支。

## 证据与待验证项

私有 0700 证据目录为 `/private/tmp/shiju-sigit-coexist34.NwVKRtAB`，保留线报文、回环模型续轮和复验脚本。复制该目录中的 `probe.py`、`runtime`、`workspace` 到新的私有目录，先运行 `python3 <新目录>/probe.py policy`，再运行 `python3 <新目录>/probe.py coexist`，两次退出 0。脚本 SHA-256 为 `321fb75dbc7180dce15d3f4d4eed01f066293e90437dad36c1ac33256065dedc`。补充原生夹具为 `native-guard.py`，用 `SHIJU_SIGIT_TEST_ENTRY` 指向核验过的 v3 入口。

本轮未发现官方工具目录被裁剪，产品代码无变更。未启动 ShiJu 网关、运行 Cargo、重装制品或调用真实模型。旧 v2 已有真实模型三轮，旧 v3 有真实挂载轮。本次真实模型三轮的首个未执行步骤是使用获授权租户和端点发送无挂载请求，需获授权后沿用[固定 v3 验收入口](../../mcp-acceptance/sigit.md#受管-v3-自动准备-rustcodegraph-索引)及当前共存三轮请求。逐服务名单仍待共享接线与网关实跑。
