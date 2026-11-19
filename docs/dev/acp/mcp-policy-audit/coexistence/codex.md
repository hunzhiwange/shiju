# Codex 官方工具与 MCP 共存复验

2026-11-19，Darwin arm64。Codex ACP 2.5.0／Codex CLI 0.165.0 的受管版与同版原包均通过直接 ACP 回环三轮。挂载轮内，Codex 官方资源发现与只读 stdio MCP 工具各执行一次，两条结果均按原调用 ID 回到模型续轮。撤除后的新会话零目标服务访问。

## 入口与权限

- 注册 key 为 `codex`，预设为 `acp-codex`。从本机受管缓存复制固定制品至私有目录，执行 `node_modules/.bin/codex-acp cli --shiju-model-source`。受管 JS 入口 SHA-256 为 `730bff93eddd77ad1ea1c3ff941ee5fa4fe2b9cb940449e6ee82712295846136`。在另一副本逆向应用仓库补丁，所得原版入口 SHA-256 为 `15de6b2dd5203876a48b12861378bcf87040d623ab2159e05052dc9d54c32b16`，与原版合约一致，以 `node_modules/.bin/codex-acp` 无参数启动。两份副本共用同一固定版引擎制品，`--version` 返回 `codex-cli 0.165.0`，引擎 SHA-256 为 `a45099154d7fb4a5ed8a56f40966a95deb938e508bab23e6985035c709e9a85a`。Node 为 24.22.3。[制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [注册及历史审计](../1.md#codex)
- 原包通过私有原生 `config.toml` 选择模型与 provider。受管版复用驱动所需的 `CODEX_HOME`、`CODEX_CONFIG`、`MODEL_PROVIDER`、`SHIJU_CODEX_MODEL_LOCK`、`SHIJU_CODEX_HISTORY_HOME` 和私有回环鉴权配置。两者选定同一 `local-coexist` 模型别名，实际请求仅发往本机 Responses 夹具。夹具生成确定性工具调用，读取并校验 Codex 在续轮发来的实际返回。配置和历史均位于私有目录。[受管驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/codex.rs)
- 两份入口均回读 ACP v1 和适配器 2.5.0，声明支持 load、resume 和 HTTP MCP，SSE MCP 和 ACP MCP 标志为 false。本次实际挂载使用 `session/new.mcpServers` 的 stdio 传输。每次提示前通过 `session/set_config_option` 设为 `read-only` 并核对回读，三轮原生 turn context 均记录 `on-request` 与 `read-only`。仅在当前 MCP 调用触发审批时选择一次性允许，拒绝和取消使用独立样本。普通原生入口的本地选模属于通用能力，受审私有 Responses 中继的本次实测范围为 Darwin arm64。

## 三个独立新会话

`session/prompt` 的 ACP 请求 ID 均为 4。挂载轮包含 Codex 内置 `list_mcp_resources`（`call_native_read`）及目标服务 `mcp__coexist_probe.read_probe`（`call_attachment_read`）。同一 ACP 会话内分别观察到两项 `in_progress → completed` 和两个同 ID Responses 输出。前者由 Codex 原生资源发现返回 `{"resources":[]}`，后者由 stdio 服务实际读取独立附件。模型提示不含附件标记。

| 入口及轮次 | ACP 会话 | 执行及模型回环 | ACP 终态 / 适配器退出码 |
| --- | --- | --- | --- |
| 受管 · 无挂载 | `01a1f2a1-5a65-7562-b415-10c674abba22` | 无工具，目标服务零请求，1 次模型请求 | `end_turn` / 0 |
| 受管 · 挂载 | `01a1f2a1-6506-7b12-9f13-707f622e3073` | 原生 1 次、MCP 1 次，第三次模型请求收到两个结果 | `end_turn` / 0 |
| 受管 · 撤除 | `01a1f2a1-7013-79f2-88ea-f2ad82405904` | 无工具，目标服务零请求，1 次模型请求 | `end_turn` / 0 |
| 原版 · 无挂载 | `01a1f2a1-9b51-7fa2-85b9-60b1c9aaf053` | 无工具，目标服务零请求，1 次模型请求 | `end_turn` / 0 |
| 原版 · 挂载 | `01a1f2a1-a5de-7ce1-9a28-65956d688d85` | 原生 1 次、MCP 1 次，第三次模型请求收到两个结果 | `end_turn` / 0 |
| 原版 · 撤除 | `01a1f2a1-b165-7123-a83f-25b26ddf9a90` | 无工具，目标服务零请求，1 次模型请求 | `end_turn` / 0 |

两个挂载轮的目标服务收到 `initialize`、`notifications/initialized`、`tools/list`、`resources/list` 及一次 `tools/call`（stdio JSON-RPC ID 为 3）。返回的附件标记与实际文件一致，回环最终文字为 `COEXIST_OK`。对照轮未出现目标命名空间，最终文字为 `MCP_ABSENT`。各轮另有本地测试模型的元数据回退提示。

## 同名配置、单次拒绝、取消与恢复

- 两份入口各用独立样本在私有本地 Codex 配置定义同名服务。两个定义各指向不同附件和审计文件。仍由原生 `list_mcp_resources` 与 MCP `read_probe` 同轮执行，调用 ID 同挂载轮。两份入口均只启动和调用本地定义，ACP 传入的同名定义零请求。模型续轮读回本地附件标记，两轮均为 `end_turn`，适配器退出 0。
- 两份入口各向 `call_attachment_read` 的审批选择 `reject_once`（选项 ID 为 `cancel`）。模型续轮收到同 ID 的拒绝结果并返回 `DENIED`。等待同一审批的独立取消样本发出 `session/cancel`，提示终态为 `cancelled`，模型没有接收 MCP 输出。四个样本均做过服务发现，实际 `tools/call` 为零，适配器退出 0。
- 两份入口的 `session/load` 与 `session/resume` 各从已结束的独立连接恢复原 ACP 会话。挂载恢复轮重新同步只读模式，按同 ID 回传官方 `list_mcp_resources`（`call_load_mounted_native` 或 `call_resume_mounted_native`）和目标 `read_probe`（`call_load_mounted_attachment` 或 `call_resume_mounted_attachment`）。随后关闭连接，以同一会话 ID 和空 `mcpServers` 恢复，目标目录消失，原服务请求计数无增量。八次恢复提示均为 `end_turn`，适配器退出 0。

## 历史终帧与证据范围

现有 Chat → Responses 兼容流仍省略缺少 `reasoning_tokens` 的输出用量明细，保留已有计数。Chat 兼容路线由驱动关闭上游 Responses 托管的 `web_search`。历史修复后实测在同一真实模型挂载会话中完成 83 次目标 MCP 工具及两项官方资源发现。本次直接 ACP 探针验证原生 Responses 解析及适配器执行，不经宿主协议转换。没有重跑网关 SSE、网关绑定回读、真实供应商、其他平台、远程 MCP 或 Cargo。原生引擎随适配器退出，探针只取得适配器退出码。两份固定入口均保有已实测的官方目录和调用路径，本次无适配器缺陷修复。[现有终帧修复](../../../../../crates/libs/model-protocol/src/responses_stream.rs) · [历史真实模型回执](../../mcp-acceptance/codex.md#chat--responses-终帧兼容)

私有证据目录为 `/tmp/shiju-codex-final.7deRt3fn`。从仓库根目录运行该目录下的 `python3 …/probe.py` 与 `python3 …/restore.py`，分别生成 `trace.txt`、`restore.txt` 和逐轮 ACP、Responses 与 MCP 记录。无真实供应商凭据或用户配置被导入探针。新增回执执行 `git diff --no-index --check /dev/null docs/dev/acp/mcp-policy-audit/coexistence/codex.md`，无空白错误诊断。
