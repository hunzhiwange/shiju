# Kimi Code CLI 2.5.0 官方工具与 MCP 共存回执

2026-11-19。原生 ACP 的 stdio 三轮通过新会话验收。HTTP 和 SSE 挂载轮也实际执行了官方读取和 MCP 读取。同进程及跨进程以空名单恢复旧会话时，模型仍能看到旧 MCP 工具声明。

## 制品与权限

- Darwin 27.0.0 arm64。注册 key 为 `kimi`，安装命令为 `npm install -g @moonshot-ai/kimi-code@2.5.0`，启动形状为 `kimi acp`。本次直接下载合约所列的官方 Darwin arm64 可执行文件到私有目录，命名为 `kimi`，以绝对路径加 `acp` 启动。SHA-256 为 `d4dd02d40f18f3a784699fc37ae0c55d676a4704c37a3b91ab96541b0db673a1`，与仓库清单一致，ACP v1 握手回读 2.5.0，声明 HTTP、SSE、load 和 resume。本机 PATH 的 2.5.3 未用于验收。历史 npm 安装的 `dist/main.mjs` 重新核得 SHA-256 `acd6894e2e2db50d37c449eefe9d3549a0177bd2d12cf17a8e1da94c60eb359f`，供源码对照。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L565) · [安装](../../../../../crates/libs/config-acp-install/src/lib.rs#L96) · [版本合约](../../../../shiju/acp-model-source/cli-versions.json#L1548)
- 原版可通过 Provider 配置选模。每个新会话采用独立工作区、`HOME` 和 0700 的 `KIMI_CODE_HOME`，以 0600 的 `config.toml` 设置回环 OpenAI Chat Completions、`session-selected` 和整数限额，回读模式为 `default`。仅在获准的挂载轮为目标读取回送 `approve_once`，其余审批均拒绝。无模式提权，关闭自更新与遥测，只使用合成数据和一次性回环凭据。
- 宿主驱动维持参数和中继校验、私有状态、整数限额及环境隔离。该驱动没有额外制品摘要校验，非 Unix 缺少私有 ACL 校验路径。当前复验直接运行原版和等价私有配置。历史真实模型记录中的 101 次 MCP 调用、普通 Bash 和受管配置修复有独立回执。本次未运行 ShiJu 网关、真实模型、其他平台、npm 冷装或 Cargo。[宿主驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/kimi.rs) · [Kimi 权限控制](../../../../../crates/libs/acp-client/src/client/kimi_access.rs) · [历史回执](../../mcp-acceptance/kimi.md)

## 新会话

stdio 无挂载、挂载和撤除后新会话模型可见工具数为 24 / 25 / 24。两次对照的 24 项官方目录一致，挂载只新增 `mcp__coexist_probe__read_probe`。

官方目录为 `Agent`、`AgentSwarm`、`AskUserQuestion`、`Bash`、`CreateGoal`、`CronCreate`、`CronDelete`、`CronList`、`Edit`、`ExitPlanMode`、`FetchURL`、`GetGoal`、`Glob`、`Grep`、`Read`、`SetGoalBudget`、`Skill`、`TaskList`、`TaskOutput`、`TaskStop`、`TodoList`、`UpdateGoal`、`WaitFor`、`Write`。

| 轮次 | ACP 会话 ID | 官方 `Read` 模型 ID / ACP ID | MCP 模型 ID / ACP ID | 目标服务调用 | ACP 终态 / 进程退出 |
| --- | --- | --- | --- | --- | --- |
| stdio 无挂载 | `session_fe657dd0-e421-496e-8c6e-96f0eea3fa3f` | `native-before` / `0:native-before` | — | 0 | `end_turn` / 0 |
| stdio 挂载 | `session_c4a47127-dfe6-493a-99d3-c77c01cfbfcc` | `native-mounted` / `0:native-mounted` | `mcp-mounted` / `0:mcp-mounted` | 1 | `end_turn` / 0 |
| stdio 撤除后新会话 | `session_5f2ad007-4c2e-4eeb-9d67-74511a31ec6d` | `native-removed` / `0:native-removed` | — | 0 | `end_turn` / 0 |
| HTTP 挂载 | `session_2d4a901b-af42-4b98-b5da-9f1739081071` | `native-mounted` / `0:native-mounted` | `mcp-mounted` / `0:mcp-mounted` | 1 | `end_turn` / 0 |
| SSE 挂载 | `session_aa4d8b57-f120-4505-b0e1-97b0b79e5cf8` | `native-mounted` / `0:native-mounted` | `mcp-mounted` / `0:mcp-mounted` | 1 | `end_turn` / 0 |

Kimi 原生 `Read` 读取工作区只读文件，MCP 服务读取独立附件。两类结果均包含各自的随机标记，按模型 ID 回到同一轮的后续请求，ACP 状态为 `completed`，夹具随后返回结束文本。stdio 目标服务全程记录一组 `initialize=0`、`tools/list=1`、`tools/call=2` RPC ID。HTTP 与 SSE 服务分别记录一次实际调用，挂载轮目录均为 25 项。

无挂载会话另发出只读 Bash 请求，模型 ID `native-deny`、ACP ID `1:native-deny`。显式拒绝后 ACP 报 `failed`，模型收到拒绝结果，文件标记未返回。单独一轮待回环模型收到请求后发 `session/cancel`，ACP 返回 `cancelled`，无工具执行。凭据缺失和未知模型均由现有负向探针复验为 ACP `-32000`、模型请求 0，探针退出 0。

## 空名单恢复

沿用 stdio 挂载会话 ID 同进程依次发 `session/load` 和 `session/resume`，每次显式发送 `mcpServers: []`。跨进程两项各从该会话的私有状态副本启动，同样传空名单。四轮均成功恢复并继续推理。首个撤除缺口出现在恢复后的模型请求，目录仍为 25 项。

| 恢复入口 | 官方读取 ACP ID / 状态 | 旧 MCP ACP ID / 状态 | 目标服务新增调用 | ACP 终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| 同进程 load | `1:native-load` / completed | `1:mcp-load` / failed，审批拒绝 | 0 | `end_turn` / 0 |
| 同进程 resume | `2:native-resume` / completed | `2:mcp-resume` / failed，审批拒绝 | 0 | `end_turn` / 0 |
| 新进程 load | `3:restart-native-load` / completed | `3:restart-mcp-load` / failed，工具未找到 | 0 | `end_turn` / 0 |
| 新进程 resume | `3:restart-native-resume` / completed | `3:restart-mcp-resume` / failed，工具未找到 | 0 | `end_turn` / 0 |

四轮结果均在模型侧回读。模型 ID 与表中 ACP ID 去掉轮次前缀后的部分一致。重放的旧工具事件不计为新执行。原版 npm 源码的 `resumeAcpSession` 将名单传给 `restore`，引擎在已有活动会话时直接返回旧实例。新进程恢复时引擎用请求名单重建会话，实测旧声明仍出现，执行器找不到目标工具且服务没有新握手。同进程旧声明能发起权限请求，本次保持拒绝。宿主恢复接口静态核对会下发当前名单，宿主撤权端到端及跨进程残留声明来源待收口复验。未发现挂载时存在目录裁剪，无安装补丁或运行时代码变更。[宿主 load/resume](../../../../../crates/libs/acp-client/src/client/protocol.rs#L422) · [任务 43](../../../../../.plans/2026-11-19-acp-工具共存/43-集成与证据收口.md)

复验脚本及合成轨迹保存在私有目录 `/private/tmp/shiju-kimi-task23.YriPJ7D2`。在干净的私有目录放置经摘要校验的 `kimi` 后，从仓库根目录依次执行 `python3 <私有目录>/probe.py "$PWD" run` 和 `python3 <私有目录>/restart.py "$PWD" run`，第二步消费第一步的 stdio 挂载轨迹和私有会话状态。脚本 SHA-256 依次为 `e45070e473ff73de6cdef553d77d3c65401c889f6be7c09b0dcfd06cb18f5384`、`b13f2a09c836d181945945b189fc69915e3b3c4e50bbc27c87f050741c5c8585`。两步均退出 0，私有服务关闭，ACP 子进程均退出 0。
