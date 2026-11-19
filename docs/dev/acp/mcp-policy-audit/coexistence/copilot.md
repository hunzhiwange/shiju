# Copilot 1.0.105 原生工具与 MCP 共存复验

2026-11-19，Darwin arm64。官方原版与 ShiJu 受管版分别通过直接 ACP 回环三轮。默认工具目录在两版均为 16 → 17 → 16，挂载轮实际执行官方 `view` 和 SSE MCP 读取，模型续轮收到两个原调用 ID 对应的真实结果。撤除后的新会话对仍在监听的目标服务零访问。两版的目录变更拒绝、审批拒绝和恢复行为一致，未发现受管适配器无意裁剪。

## 制品、启动与权限

- 注册 key 为 `copilot`，原生参数为 `--acp --stdio`。使用本机固定的 `@github/copilot` 和 `@github/copilot-darwin-arm64` 1.0.105。官方 SEA 的 SHA-256 为 `5fd7d15ff02deb08a58593b1b488015bb52c1686014b80b5509e28263cb3ad70`，匹配当前合约。受管 v1 的四个受审文件摘要均匹配合约，`embedded/cli-main.js` 为 `1f8978f33a7a54105087ab9e66313a4b973dd852e198ab85b39c3de44959b502`。复用已安装制品，未重新下载 npm 归档。 [注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L501-L508) · [受管合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L248-L261)
- 分别以官方 SEA 和受管 `embedded/index.js` 执行 `--acp --stdio --no-auto-update`，ACP v1 均读回 1.0.105 和 `http=true`、`sse=true`、`loadSession=true`。独立 `HOME`、`COPILOT_HOME`、模式 0600 的 `COPILOT_PROVIDERS_CONFIG`、`COPILOT_MODEL=probe/probe`，另设 `COPILOT_OFFLINE=true` 和 `COPILOT_AUTO_UPDATE=false`。模型夹具仅通过私有回环 Chat Completions 接收请求，回读 wire model 为 `probe`。受管版设置空 `SHIJU_COPILOT_SKILL_ROOT`，运行受审的技能和插件发现隔离分支。两版的新会话均读回原生 Agent 模式、`allow_all=off`。客户端仅在 MCP 审批回调中选择 `allow_once`，拒绝轮选择 `reject_once`。 [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/copilot.rs) · [技能隔离补丁](../../../../../patches/acp-model-source/copilot/apply.py)
- 平台边界为当前受审 Darwin arm64 原生包及受管提取入口。原版使用原生可选模型，本次通过私有 Provider 绑定回环模型。这里验证通用直接 ACP v1 JSON-RPC 路径，未重跑 ShiJu 网关、其他平台、真实模型或 HTTP MCP。历史网关验证覆盖 HTTP 与 SSE，以及 `client_only` 权限回读。 [历史验收](../../mcp-acceptance/copilot.md)

## 三轮直接 ACP

两版各自使用私有临时项目、只读夹具数据和鉴权 SSE 服务 `receipt`。本地 MCP 只公开 `read_receipt`，它实读项目外的合成标记文件。每轮使用新进程和新会话，撤除轮期间服务仍在线。

| 轮次 | 受管 ACP 会话 ID | 官方 ACP 会话 ID | 两版目录与实际执行 | 提示终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| 无挂载 | `9374ad70-1d3f-438b-b14e-086e862716b6` | `aca48cfa-44e0-4038-a659-bd328fcfceb3` | 16 项原生，零工具事件，目标零访问 | `end_turn` / 0 |
| SSE 挂载 | `86ba6347-ce8a-4ca8-9c7a-78b1bf854c5d` | `b90b20a7-234a-412a-a518-c590b4c1591b` | 同 16 项原生并增加 `receipt-read_receipt`，实际执行以下两项 | `end_turn` / 0 |
| 撤除 | `0777431c-be34-482e-9d49-6932d015f620` | `f4df5fab-10ea-409f-b1e6-b91125b7d061` | 同 16 项原生，零工具事件，目标零连接、发现或调用 | `end_turn` / 0 |

| 两版模型工具 | 模型与 ACP 调用 ID | 执行方 | 回执 |
| --- | --- | --- | --- |
| `view` | `mounted-native` | Copilot 原生读取项目内 `native.txt` | ACP `pending` → `completed`，实读标记按同一 `tool_call_id` 进入第二、第三次模型请求 |
| `receipt-read_receipt` | `mounted-mcp` | SSE 服务 `receipt` 执行 `read_receipt`（MCP JSON-RPC ID `2`） | 审批 RPC ID `0` 单次许可后 ACP `completed`，附件标记按同一 `tool_call_id` 进入第三次模型请求 |

## 恢复、名单和拒绝

以下结果在官方与受管版各实测一次。除取消外均以 `end_turn` 结束，每个 CLI 进程退出 0。

| 探针 | 工具、调用 ID 与执行证据 |
| --- | --- |
| 带 SSE 恢复 | 原挂载进程退出后重用私有历史，以原会话 ID `session/load` 并重传 SSE。重放历史后新提示实际执行 `receipt-read_receipt`，ACP/模型 ID `load-mcp`，审批通过，MCP ID `3`，`completed` 并同 ID 进入模型续轮。 |
| 空清单恢复 | 同一原会话 ID 再次 `session/load`，传入空清单。历史工具事件照常回放，新提示仅见 16 项原生，目标零访问。初始化未公布独立 `resume` 能力。 |
| 显式字面名单 | `--available-tools=receipt-read_receipt` 使目录仅余这一项。调用 `filtered-mcp` 仍触发单次审批，MCP ID `3`，`completed` 并同 ID 续轮。缺省过滤参数的三轮保留原生目录。当前宿主参数验证仍限定字面工具名。 |
| 显式审批拒绝 | `deny-mcp` 请求 `receipt-read_receipt`，审批 RPC ID `0` 返回 `reject_once` 后 ACP `failed`，目标服务零 `tools/call`。该提示轮未产生第二次模型请求。 |
| 目录变更与重试 | 模型取得目录后、返回调用前，SSE 服务修改 `read_receipt` 的描述并发送 `notifications/tools/list_changed`。Copilot 执行前重列目录，旧调用 `changed-mcp` 以 `MCP tool catalog changed` 失败，目标未收到旧调用。同 ID 失败结果进入模型续轮。模型再发 `changed-retry-mcp`，单次许可后由 SSE 服务执行（MCP ID `4`），ACP `completed`，模型收到匹配 ID 的实读结果。 |
| 取消 | 回环已收到模型请求且仍在等待时发送 `session/cancel`。提示返回 `cancelled`，零 ACP 工具调用，目标零访问。 |
| 客户端 stdio | `session/new` 建立会话时日志明确拒绝 `stdio_receipt`，子进程启动标记未生成，模型目录保持 16 项原生。此前 HTTP 与 SSE 的成功执行属于独立历史验收。 |

## 验证与复跑

私有证据目录 `/private/tmp/copilot-coexist.1OkCkH`（0700）保留 `probe.py`、`check.py`、逐版 `summary.json` 和 CLI 日志。复制两份脚本到新的 0700 目录，在具备相同受审制品的主机执行 `python3 <新目录>/probe.py` 和 `python3 <新目录>/check.py`。两条命令均退出 0，回执检查覆盖两版各 10 轮。取消时夹具记录一次连接被对端重置，ACP 已返回 `cancelled`。所有探针 CLI 已退出，回环监听已关闭，临时 Provider 文件已清理。运行时代码未改，未执行 Cargo 或桌面构建。
