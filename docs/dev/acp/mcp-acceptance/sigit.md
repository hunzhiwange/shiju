# siGit Code 动态挂载 RustCodeGraph

2026-11-19，验收已阻塞。受审 siGit 1.7.10 在建会话前被宿主拒绝。原生探针确认，开启显式 stdio MCP 会在审批前启动服务并继承中继环境，获批调用仍缺少受审执行路径。

## 环境与请求

- Darwin arm64，私有回环网关使用已有 ShiJu 0.6.0 构建，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。从已有受审安装复制原生入口及其收据至私有网关。入口 SHA-256 为 `b98bdfdc85884922297704df67d5b9441a8fd399352457bfd75cc09f86934520`，与收据一致。源码与补丁摘要匹配受管清单，原生握手自报 1.7.10。
- RueJS 使用 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 干净副本。RustCodeGraph 1.2.10 索引了 3478 个文件。挂载轮通过副本的 `.vwacprc.json` 选择原生 stdio 服务 `ruejs_graph_probe`，参数包含 `--no-watch`。
- 每轮使用同一 curl 请求独立建会话。租户为 `10001 / 100010005`，绑定 `acp-sigit / sigit`，指定 `session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 和 `full_access=true`。提示取自 `189d22dc1`，UTF-8 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)

## 三轮网关结果

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态与答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f00ed092001W2TaJMm8jLlBOI` | 200 / 0 / 7.455 | 一个 `chat.done`，去除 `<think>` 段后最终答复恰为 `MCP_NOT_AVAILABLE`。 |
| 挂载 | `ses_1f00eefa4001lAx8ZqOiDeL57T` | 200 / 0 / 4.632 | `chat.model_recovery` 的 `phase=stopped`，随后一个 `chat.error`。无模型答复。 |
| 撤除 | `ses_1f00f02af001e4NxDf6yNva8R1` | 200 / 0 / 6.925 | 一个 `chat.done`，去除 `<think>` 段后最终答复恰为 `MCP_NOT_AVAILABLE`。 |

挂载轮首个失败边界为 `validate_mcp_contract`，原因为 `siGit reviewed host-only runtime does not support MCP servers`。网关未发送携带 MCP 的 `session/new`。三轮全部为零工具事件，目标查询、调用方查询和节点查询均未在指定模型会话中执行，无工具结果或调查答复可核对。

三轮 UI summary 均确认绑定及 `full_access=true`。两轮对照的 ACP options 在线确认 `connected=true`、指定模型以及 `support=client_only`。挂载轮仅确认受审发行版本，`connected=false`，无实际选模及工具审批。

## 原生边界和独立工具证据

- 以相同摘要的受审入口直连隔离 ACP 宿主，模型使用私有回环夹具。握手为 ACP v1，声明 HTTP MCP 和 `sse=false`。保留驱动的 `SIGIT_MCP=off` 时，显式 stdio 列表可进入原生建会话，但服务未启动，模型只收到五种受审宿主工具。固定源码的 `connect_session_servers` 在该环境值下忽略显式服务。
- 仅在隔离探针中移除该环境开关后，真实 stdio 服务在 `session/new` 内启动，早于模型提示和工具审批。启动探针确认子进程继承了模拟中继 bearer。模型收到 RustCodeGraph 的工具定义并请求查询，siGit 发出 `session/request_permission`。宿主选择 `allow_once` 后，原生通知虽标记 `completed`，模型续轮收到的实际结果为 `Error: tool mcp__ruejs_graph_probe__rustcodegraph_query requires a verified host route; native fallback is disabled`。HTTP 使用同一受审工具分发路径。现有 Cortex 转换依赖 Cortex 的 `clientTools` 回调，siGit 没有对应宿主执行入口。
- 新起进程以空列表建会话，模型侧目录不含 MCP 工具。夹具请求 `read_file`，宿主选择 `reject_once`。原生工具状态为 `failed`，模型收到拒绝原因，宿主未收到文件读取回调。
- 独立直连同一个真实 stdio 服务，`rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 返回 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。三次调用成功。这些输出仅来自独立 MCP 客户端。

保留驱动、受管补丁、协议门禁及现有断言。后续需要受审的会话级执行入口，确保 MCP 启动不继承中继凭据、调用使用宿主审批，并将实际结果及错误状态回传模型。达到这些条件后复跑三轮与审批回归。

私有网关、原生入口及工具进程已退出，临时挂载与私有凭据副本已清理。原始记录留在本机 0700 目录，用户网关健康检查通过。

## 定向回归

`python3 scripts/cargo_slot.py cargo test -j 1 -p acp-client --lib acp_onboarding_sigit_mcp_selection_rejects_before_new_or_load` 退出码 0，实际执行 1 项并通过，318 项被过滤。该用例确认非空选择在 `session/new` 和 `session/load` 发送前失败。复用 Cargo 槽位 3，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均指向仓库根目录下的 `target/rust-slot-3`。槽位已释放，缓存保留。
