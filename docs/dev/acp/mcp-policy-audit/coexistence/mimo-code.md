# MiMo Code 原生工具与 MCP 共存复验

2026-11-19 在 Darwin arm64 使用受管 MiMo Code 0.1.17 完成隔离网关三轮。挂载轮的原生 `read` 与 MCP `readonly_read_probe` 由同一次模型响应发出，各自执行并按原调用 ID 回到模型续轮。独立 ACP 探针确认撤除后目标服务零请求，单次拒绝与取消均为零下游调用。

## 入口和权限

- 注册 key 为 `mimo-code`，网关预设为 `acp-mimo-code`。从既有固定制品复制 `mimo` 和 `build-receipt.json` 到隔离目录。构建回执的版本、源码和补丁摘要匹配当前合约，实算二进制 SHA-256 为 `c086c1bd5c4ada962880f7636de5b07fc442db19edccf3e383887f6f3c47f7ae`。启动形态为 `/usr/bin/sandbox-exec -f <私有策略> <受管 mimo> acp --pure`。ACP v1 握手自报 OpenCode 0.1.17。[注册与审计](../5.md#mimo-code) · [制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 复用已有 ShiJu 0.6.0 网关，SHA-256 为 `1d37a1d46ad916b7671e7d1cb9c2be65d0cf951e7e7fd516b32d7c03afa91ada`。使用隔离配置和合成凭证，以 `session_model` 选用私有回环模型。挂载轮回读 `connected=true`、`full_access=true` 和 `access_status.support=client_only`，各轮终态 ACP 连接均释放。
- 驱动通过 `MIMOCODE_CONFIG` 注入私有模型和精确名单，关闭项目配置、外部配置兼容入口和延迟 MCP 搜索。模型与 MCP 使用各自的本地凭证和同一个回环端口。MiMo 侧 Seatbelt 限制进程、工作区写入和外部网络，宿主桥在独立沙箱内启动绝对路径 stdio 服务，筛选工具目录并在调用时复核。实测 build 主提示在空目录和挂载时均保留 `read/edit/write`。驱动同样为 compose 保留三项，为 plan 保留 `read`，挂载时追加 MCP。权限规则为默认拒绝、`read` 允许、`edit` 和 MCP 询问，`write` 沿用默认拒绝。本次执行限只读工具。[驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/mimo_code.rs) · [宿主桥](../../../../../crates/vw-agent/src/session/llm/acp/model_source/mimo_mcp.rs)

## 隔离网关三轮

每轮经 `POST /v1/chat/stream` 建立新会话。三轮均为 HTTP 200、curl 退出 0、单一 `chat.done`，隔离网关退出 0。

| 轮次 | 会话 | 工具、执行方与状态 |
| --- | --- | --- |
| 无挂载 | `ses_1f2bfaac7001JyxmbJfyl9IQLe` | 主提示有原生目录，模型输出 `MCP_ABSENT`，零工具事件。 |
| 挂载 | `ses_1f2bfb9be001sjqp0xrP48CsSP` | MiMo 执行 `read`（`native_read`），读到 `NATIVE_FILE_MARKER`。宿主 stdio 服务执行 `read_probe`（ACP 和模型 ID 均为 `mcp_read`，stdio ID 为 `2`），返回 `MCP_LOCAL_MARKER`。两项均为 `completed`，模型收到两项结果后输出 `COEXIST_OK`。SSE 重放的完成事件按调用 ID 去重。 |
| 撤除 | `ses_1f2bfd163001KgUk78CXij4Abo` | 移除项目挂载后新建会话，主提示恢复原生目录，模型输出 `MCP_ABSENT`，零目标工具声明和调用事件。 |

额外的逐服务拒绝轮 `ses_1f2bfe14f001DEqC3FZ0it63Ek` 将 `read_probe` 加入逐服务拒绝名单。主提示保留原生目录，目标工具未发布，零调用事件，HTTP 200、curl 0、`chat.done`。

## 独立 ACP 探针

用相同受管二进制、私有模型配置、`--pure` 和 MiMo 进程沙箱直连 ACP。回环夹具在模型端口提供 HTTP MCP 路由并记录下游 stdio 请求，夹具侧服务由探针启动。各轮 ACP 进程及 stdio 夹具退出 0，模型监听已关闭。

| 轮次 | ACP 会话 | 工具与结果 |
| --- | --- | --- |
| 无挂载 | `ses_ffe5e0d40dec9ffeg9fQJuFtj9` | MiMo `read`（`native_read`）完成并续轮。目标路由零请求，`end_turn`。 |
| 挂载 | `ses_ffe5e0d40d7f4ffek6FwQgpsoy` | MiMo `read`（`native_read`）完成。MCP `readonly_read_probe`（`mcp_read`）经审批 RPC `0` 单次允许，stdio `read_probe`（ID `3`）返回标记。两个 ID 的结果均进入续轮，`end_turn`。 |
| 撤除 | `ses_ffe5e0d40d196ffeU5f6w07EC1` | 新进程使用空 MCP 配置，MiMo `read`（`native_read`）完成并续轮。目标路由零请求，`end_turn`。 |
| 单次拒绝 | `ses_ffe5e0d40cb22ffenmVvzxusgs` | MiMo `read`（`native_read`）已完成，MCP（`mcp_read`）经审批 RPC `0` 选择 `reject_once` 后标为失败。下游只收到握手和目录发现，零 `tools/call`，终态 `end_turn`。 |
| 审批中取消 | `ses_ffe5e0d40c476ffePBccSeq748` | MiMo `read`（`native_read`）已完成，等待 MCP（`mcp_read`）审批时发送 `session/cancel` 并返回审批取消。工具标为失败，下游零 `tools/call`，MiMo 本轮报告 `end_turn`。 |

挂载轮结束后，在原私有配置仍有效时分别发送 ACP `session/load` 和 `session/resume`，RPC ID `10`、`11` 均成功返回同一会话。此项覆盖原生会话握手，撤除后的隔离由新会话单独验证。

## 证据范围

早期受管 stdio 启动断点及精确名单诊断、指定真实模型的历史生产复验继续保留。本次新证据来自回环模型，覆盖当前受管 Darwin arm64 入口。未重跑冷安装、未打补丁的上游原包全链、真实供应商模型、旧会话撤除后的恢复执行与其他平台。没有观察到无意裁剪，未改适配器或运行 Cargo。[历史验收](../../mcp-acceptance/mimo-code.md)

本机私有目录 `/private/tmp/shiju-mimo-coexist.vGb09R` 保留 `gateway.py`、`direct.py`、`mcp.c` 与 SSE、ACP、模型及下游日志。用 `python3 gateway.py` 复跑网关，用 `python3 direct.py <before|approve|removed|deny|cancel>` 复跑对应 ACP 探针。网关、ACP 和 stdio 子进程均已退出，项目挂载已清理。
