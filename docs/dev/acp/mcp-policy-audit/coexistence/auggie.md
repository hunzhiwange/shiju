# Auggie 0.37.1 工具共存复验

2026-11-19，macOS arm64 直接 ACP 回环三轮通过。默认发现保留了受测的 18 项原生工具，挂载后新增一个 stdio MCP 工具。同一提示轮实际执行原生读取和 MCP 读取，两个结果均按原调用 ID 进入模型续轮。撤除后新会话对目标服务零访问。现有补丁无需改动。

## 制品、入口与权限

- 受管注册 key 为 `auggie`，合约固定 `@augmentcode/auggie` 0.37.1。CLI 实际输出 `0.37.1 (commit e8d38687)`。以本机已安装原版为输入，复制至私有目录后应用仓库受审补丁。本次核对原始 bundle 和两个产物的文件摘要，未重新下载 npm 归档。 [注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L429) · [受管合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L310)
- 原版 `augment.mjs` SHA-256 为 `7d809fef34f9cb0eae57559ced737e225ec74a390cf325ea453871d847d716c9`。受管 `augment.mjs` 为 `1e39bfa85bb3ebe44afb6d627bdd3202550508d5224c837dc4370576f555cb18`，`session-model.mjs` 为 `d50926db8682babc44174201b0591e7b309e00e5f7f5989c924f610e7b1a7081`，均匹配合约。
- 按现有探针执行 `node <受管入口>/augment.mjs --acp --model selected --augment-cache-dir <私有缓存> --augment-session-json <私有会话文件>`。设置隔离 HOME、`AUGMENT_DISABLE_AUTO_UPDATE=1`、`SHIJU_AUGGIE_MODEL_CONFIG` 和 `AUGMENT_SESSION_AUTH`。模型配置与会话文件为 0600。实际模型绑定回读为 `selected`，Chat Completions 只连临时回环，使用合成响应与夹具认证。ACP 回读为 `default`（原生完全访问模式），客户端能力为空。私有工作区使用默认发现，无 `removedTools` 或缩目录转发器。 [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/auggie.rs) · [原有探针](../../../../../scripts/acp_model_source_auggie_tests.py)

## 三轮直接 ACP

同一私有项目、相同入口及权限模式，每轮新进程、新会话和独立随机读取标记。工作区文件和项目外 MCP 附件的值均来自工具实读。模型夹具只发调用及结束指令。工作区只有探针预置的只读样本，轮后核对内容与目录。

| 轮次 | ACP 会话 ID | 首个模型请求的目录 | MCP 服务审计 | 模型请求数 / 提示终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| 无挂载 | `37e1da3b-2abd-4e9a-90d1-1fd6ca6381fd` | 18 项原生 | 无访问、零调用 | 1 / `end_turn` / 0 |
| 挂载 | `eb627343-5d59-43f9-af6b-e3f30aa1d669` | 同 18 项原生及 `read_receipt_receipt` | `initialize`、`tools/list`、一次 `tools/call` | 3 / `end_turn` / 0 |
| 撤除 | `571a9f0b-577b-49e3-9b21-c584880a1b43` | 同 18 项原生 | 无访问、零调用 | 1 / `end_turn` / 0 |

| 模型工具名 | ACP 与模型调用 ID | 执行方及实际结果 | 终态与续轮 |
| --- | --- | --- | --- |
| `view` | `mounted-1` | Auggie 原生文件工具读取 `native.txt` 并返回带行号的本轮标记 | ACP `completed`；第二次模型请求收到同 ID 的 `tool_call_id` |
| `read_receipt_receipt` | `mounted-2` | 本地 stdio 服务 `receipt` 处理 `read_receipt`，MCP JSON-RPC ID `3`，返回独立附件本轮标记 | ACP `completed`；第三次模型请求同时携带两个同 ID 的工具结果 |

## 取消、恢复与权限边界

- ACP v1 公布 `loadSession=true`。前一进程退出后，从私有历史副本以原挂载会话 ID 分别执行 `session/load`，两次均重放六条历史更新，并以原 ID 接受新提示。重传 stdio 清单后重新发现服务，新模型目录含 MCP 工具。清空清单后新模型目录仅含原生工具，目标服务零访问。两次新提示均 `end_turn`，进程退出 0。恢复测试覆盖目录与新提示，未发起新工具调用。初始化未公布独立 resume 能力。
- 在合成模型已收到请求且还未返回响应时发送 `session/cancel`。提示返回 `cancelled`，无 ACP 工具事件，目标服务零访问，进程退出 0。
- 独立的 `reject_once` 探针只在收到 ACP 审批请求时回绝。原生 `view` 和 MCP 工具在受测模式均未触发 `session/request_permission`。独立 MCP 样本实际执行一次并成功续轮，因此逐次审批的首个缺证点是回调。现有 ShiJu `DenyAll` 对 Auggie 在发提示前整轮拒绝，此次只核对了该路径的源码。网关的会话名单、服务名单和 SSE 广播未在直接回环中实测。 [提示前门禁](../../../../../crates/libs/acp-client/src/client/prompt_loop.rs#L34) · [历史权限实测](../../mcp-acceptance/auggie.md#回环与权限)
- 受审管理安装在 macOS arm64。Linux 有 `auggie --acp` 本地入口；Windows 的私有历史在驱动的 ACL 校验处显式拒绝。Linux 和 Windows 本次仅核对入口与边界。未修改的同版原始入口仍缺供应商模型授权下从认证到挂载、执行与撤除的当前闭环。真实模型的既有证据另见历史记录。 [安装平台](../../../../../crates/libs/acp-agent-registry/src/managed_runtime.rs#L375) · [历史模型验收](../../mcp-acceptance/auggie.md)

## 验证与复验入口

`node --test patches/acp-model-source/auggie/session-model.test.mjs` 退出 0，5 项通过。直接回环探针退出 0，受测 CLI 均退出 0，未发现自有探针残留进程。未运行 Cargo、桌面构建或网关实测。

私有证据目录 `/private/tmp/auggie-coexist.WpFVHN`（0700）保留 `probe.py`、`service.py`、`final.txt` 和逐轮 ACP、MCP、模型输入记录。复跑时将两份脚本和已核验的 `package/` 复制到新 0700 目录，再运行 `python3 <新目录>/probe.py baseline mounted removed load-mounted load-removed cancel mcp-deny`。 [现行验收流程](../../ACP验收.md)
