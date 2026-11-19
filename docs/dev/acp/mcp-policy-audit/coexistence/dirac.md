# Dirac 官方工具与 MCP 共存复验

2026-11-19 在 Darwin arm64 用受管 Dirac 0.5.24 通过三轮回环网关验收。挂载轮的原生 `read_file` 和 70 次只读 MCP 调用均完成，结果按调用 ID 回到模型续轮，原生 `respond` 正常结束提示。撤除后的新会话对目标服务零连接。另实测单次拒绝、取消以及两种逐服务名单拒绝。

## 固定入口与权限

- 注册 key 为 `dirac`，预设为 `acp-dirac`。原版固定入口可由 `npx -y dirac-cli@0.5.24 --acp` 启动，本次直连按同版 `node dist/cli.mjs --acp --config <私有目录>` 实测。原版 CLI SHA-256 为 `c88e043f590a520da3345050e32bbe6930a68cefad60945ef4a7d14648217429`。Node 为 24.22.3。空挂载原版通过原生 `DIRAC_MODEL` 选中回环模型并完成 `respond`，无需补丁。[注册与审计](../3.md#dirac)
- 从已有受管缓存复制固定制品到独立配置目录。Dirac 0.5.24 与 MCP SDK 1.32.2 的锁文件完整性值均匹配合约。网关实测执行 `dirac-0.5.24-model-source-v1/node_modules/.bin/dirac --acp --config <私有目录>`。链接指向 `dist/shiju-dirac.mjs`，入口 SHA-256 为 `330dc9f7ad7bd68b2255bea99410daab3b22ed70605e9177001fd1cbfd09797d`，`dist/session-mcp.mjs` 为 `ce75315c4182d34bcdb39d1a70b5fe3c7a72539677c317fe85a41229957127aa`。网关为既有 ShiJu 0.6.0，SHA-256 为 `1d37a1d46ad916b7671e7d1cb9c2be65d0cf951e7e7fd516b32d7c03afa91ada`。[制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [安装脚本](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/install.py#L720)
- 网关按 `session_model` 将私有回环 Chat 模型绑定至 `DIRAC_MODEL`，由驱动设置 `DIRAC_PROVIDER=openai`、`DIRAC_BASE_URL`、`DIRAC_API_KEY`、`DIRAC_DIR` 和 `DIRAC_DATA_DIR`。本次只使用合成鉴权值、本机隔离配置、默认原生工具目录和只读测试数据。三轮复用历史请求的 `full_access=true`，挂载轮在线回读为 `connected=true` 与 `client_only`，三轮终态均确认 `acp-dirac / dirac`、所选回环模型及连接释放。受管安装与会话模型能力发布范围为 Darwin arm64。驱动的 Unix 分支要求私有状态目录，非 Unix 显式拒绝。挂载限受审入口、宿主审批及绝对路径 stdio，服务 env 为空。[驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/dirac.rs) · [宿主挂载门禁](../../../../../crates/libs/acp-client/src/client/protocol.rs#L272)

## 回环网关三轮

三轮均经独立的 `POST /v1/chat/stream` 建立新会话，HTTP 200、curl 退出 0、SSE 单一 `chat.done`。私有网关最终退出 0。

| 轮次 | 网关会话 | 工具、执行方与完成 ID | 回环模型请求 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f2a9ecb9001JWQYb3o1b8h8xE` | Dirac `respond`，`1795072587242-2`，输出 `MCP_ABSENT`。目标服务零请求。 | 1 |
| 挂载 | `ses_1f2a9f890001fDh6ZO5S4X2qYi` | Dirac `read_file`，`1795072590949-2`，读到本地文件标记。stdio 服务 `readonly-read_probe` 完成 70 次。Dirac `respond`，`1795072678259-3`，输出 `COEXIST_OK`。 | 72 |
| 撤除 | `ses_1f2ab5c2c001xuaL3uchi7BaKD` | Dirac `respond`，`1795072681180-2`，输出 `MCP_ABSENT`。目标服务零请求。 | 1 |

挂载轮核对了全部 70 次调用的 ACP 开始与完成 ID、stdio 请求 ID、模型续轮 ID 和实际结果。首项为 `f853cc64-36f5-49b2-af33-ad7f7908bd8a`（stdio `2`、模型 `mcp_00`、`MCP_VALUE_00`），末项为 `609faaef-047b-4d36-ac57-b18cfe8a1275`（stdio `71`、模型 `mcp_69`、`MCP_VALUE_69`）。原生读文件结果也以模型 ID `native_file` 进入续轮。全部调用目录同时保有 `read_file` 与 `respond`，挂载轮叠加目标 MCP。高频首跑触发宿主中继限流后恢复并完成，表中复跑对回环响应定速，工具事件零失败。

## 拒绝、取消与原版对照

- 网关分别给所选服务配置 `allowedTools: ["read_probe"]` 和 `deniedTools: ["read_probe"]`。会话 `ses_1f2ab6761001sJtyvYC7Uelnrs` 和 `ses_1f2ab6a07001EgDFs6sVLhlrs8` 均为 HTTP 200、curl 0，SSE 以 `chat.error` 返回 `Dirac cannot enforce host MCP tool filters`。宿主预检仍进行 stdio 目录发现，两轮均零 `tools/call`、零模型请求、零 ACP 工具事件，也未启动 Dirac。[服务名单门禁](../../../../../crates/libs/session-engine/src/acp_defaults.rs#L111)
- 直接 ACP 使用同一受管入口及夹具。无挂载 `53548cbd-a515-4aa2-87e7-1831f171c1e5`、挂载 `1d6f0b11-9786-4b05-9e79-cf3975722a37`、撤除 `fd1a37b7-4ff5-4584-8d73-172595528fd2` 均为 `end_turn`，三个适配器进程均退出 0。两组空挂载各只有原生 `respond`（`1795072651900-2`、`1795072655469-2`），目标零请求。挂载轮的原生 `read_file`（`1795072653662-2`）和 MCP（`c8c03ca2-c49c-4576-8b62-5e00c7d1c2c8`）完成，MCP 审批使用同一 ACP ID 单次允许，模型第三次请求读回 `native_file` 与 `mcp_first` 的结果，随后原生 `respond`（`1795072653852-3`）完成。以空列表加载该挂载会话、带挂载恢复或派生，三者均返回 `-32603` 及 `supports only new sessions`。
- 单次拒绝会话 `d113d1eb-fbd6-4f69-b86b-a84958ff3a04` 的原生读文件完成（`1795072657877-2`）。MCP `3216ca0b-fe56-4b6f-9f34-8b47f0a61680` 标记失败，模型按 `mcp_first` 读回拒绝，stdio 零执行，原生 `respond`（`1795072658123-3`）完成，ACP 为 `end_turn`。取消会话 `044f7a73-79c0-4944-be83-fc6e6d2a0ef7` 的原生读文件完成（`1795072660971-2`），等待 MCP `de533d63-0286-44cf-b6e9-1872bb77d98f` 审批时发送 `session/cancel`，ACP 为 `cancelled`，该调用标记失败且无执行、无模型工具结果。两进程均退出 0。
- 原版空挂载与分别声明 stdio、HTTP、SSE 的三个会话均成功 `session/new`，随后各完成一次原生 `respond`（依次为 `1795072645693-2`、`1795072647329-2`、`1795072648995-2`、`1795072650514-2`），均为 `end_turn`、进程退出 0。三类声明均未进入模型工具目录，stdio 未启动，HTTP 与 SSE 探针的 TCP 连接数均为零。首个断点仍为原版在新建会话时忽略 `mcpServers`。
- 受管入口另执行仓库原有 `patches/acp-model-source/dirac/probe.py`，以隔离索引副本和实际 RustCodeGraph 运行，退出 0。挂载轮的 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 各完成一次，空挂载和撤除均为零，拒绝、取消及 load/resume 断言通过。[补丁与原探针](../../../../../patches/acp-model-source/dirac)

## 历史生产与本次边界

历史生产网关以指定真实模型完成三轮，RustCodeGraph 三个目标工具的成功数为 0／70／0，挂载轮分别为 query 40、callers 4、node 26，另有同服务 6 次成功调用。三轮原生 `respond` 完成，ID 依次为 `1795056965090-3`、`1795057200079-44`、`1795057208869-3`。历史原始 SSE 已核对。本次 70 次为当前私有网关与回环模型的新证据，未调用真实供应商、重跑冷安装、运行 Cargo 或验证其他平台。没有观察到受管适配器裁剪原生工具的缺陷，本次提交仅包含回执。[历史生产结果](../../mcp-acceptance/dirac.md#生产网关复验通过)

本机私有目录 `/tmp/shiju-dirac-coexist.N8O1rE` 保留 `gateway.py`、`direct.py`、`verify.py`、逐轮 SSE、ACP 和服务记录，以及全量调用映射 `mounted-ids.tsv`。分别用 `python3` 运行三个脚本可复验，前两个运行探针，第三个核对网关记录。探针、MCP 子进程和网关均已退出，临时挂载已清理。新增回执用 `git diff --no-index --check /dev/null docs/dev/acp/mcp-policy-audit/coexistence/dirac.md` 检查，无空白错误诊断。
