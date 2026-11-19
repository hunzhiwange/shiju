# Qwen Code 官方工具与 MCP 共存复验

2026-11-19 在 Darwin arm64 直连 Qwen Code 0.26.7 原包和受审 v1。两版在默认工具搜索下均可在同一挂载会话执行官方 ReadFile、ToolSearch 和三个 RustCodeGraph 工具，并把结果送入模型续轮。撤除后的新会话对目标服务零访问。

## 入口、模型与权限

- 注册 key 为 `qwen`，预设为 `acp-qwen`。通用发现先尝试本机 `qwen`、`qwen-code`，回退到 `npx @qwen-code/qwen-code@latest`。本次固定版原包来自 `npm pack @qwen-code/qwen-code@0.26.7`，使用私有缓存。归档 SHA-256 为 `4cb0587ce0eb368230b128ffd4ff999e7b517865145a26fdab14b7e744c307f0`，SHA-512 SRI 与合约一致。实际执行 `node <原包>/cli-entry.js --acp --experimental-skills`。[注册入口](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 受审 `qwen-0.26.7-model-source-v1` 复制自受管安装，六个文件的 SHA-256 全部与合约一致。入口 `cli-entry.js` SHA-256 为 `89c392458627a27448ffaed32102554f0cb72a70f4ec1e60527fa56068afb387`。以 Node 24.22.3 执行 `node <受审包>/cli-entry.js --acp --shiju-model-source --auth-type openai --model <私有别名> --openai-base-url <回环端点>/v1`。两入口的 `--version` 均返回 0.26.7 并退出 0。通用回退的在线选版未执行。[受审补丁](../../../../../patches/acp-model-source/qwen/patch.json)
- 原包使用 Qwen 原生 `security.auth`、`modelProviders.openai` 和私有用户配置。受审样本复现 `QWEN_HOME`、`QWEN_RUNTIME_DIR`、`QWEN_CODE_SYSTEM_SETTINGS_PATH`、`OPENAI_*` 与 `SHIJU_QWEN_MODEL_LOCK`、`SHIJU_QWEN_GENERATION_CONFIG`。ACP config 回读 OpenAI 私有别名，两种入口的主模型请求均抵达动态回环端口 `/v1/chat/completions`，请求体模型为 `fixture`。受审样本的 `authenticate(qwen-oauth)` 返回 `Managed Qwen authentication is locked`。管理员安全配置合并及会话技能根职责由源码核对，本机未配置管理员策略，探针未挂载技能。[宿主驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/qwen.rs) · [既有生命周期探针](../../../../../patches/acp-model-source/qwen/probe.py)
- 握手为 ACP v1、`qwen-code 0.26.7`，声明 load、resume、MCP HTTP/SSE。本次执行使用 stdio。私有 HOME、工作区和历史与用户配置隔离，macOS 沙箱只放行回环联网。新会话 ACP 权限模式回读为原生 `auto`。宿主未实现文件读写回调，官方 ReadFile 由 Qwen 读取私有工作区文件。挂载轮三次 MCP 审批均按各自 ID 选择 `allow_once`。未传入启动审批绕过参数。
- 目标服务 `ruejs_graph_probe` 指向真实 RustCodeGraph 1.2.10，SHA-256 为 `68cd787626f55a5fff5a556b2bbb449ae0fea597775fc2b40cd3f4e814dee42c`。私有索引含一个 Rust 文件，服务以 `serve --mcp --path <私有图谱目录> --no-watch` 启动。stdio 记录器原样转发目录、参数和结果，保留全部八项服务工具。用于历史严格验收的三工具目录转发器与 Qwen 的发现配置是两个独立环节。[历史网关验收](../../mcp-acceptance/qwen.md)

## 三轮结果

默认组用 `ToolSearch` 精确检索 `select:mcp__ruejs_graph_probe__rustcodegraph_query,mcp__ruejs_graph_probe__rustcodegraph_callers,mcp__ruejs_graph_probe__rustcodegraph_node`。私有工作区直接声明组仅设置 `tools.toolSearch.enabled=false`，保留默认权限和同一个服务。挂载组在发现就绪后开始提示。初次原包紧接建会话发出搜索时，搜索先于目标目录就绪并返回未找到。取证探针随后等待 stdio 发现，保留 Qwen 默认发现行为。

| 入口和发现方式 | 新会话轮次 | ACP 会话 ID 前缀 | Qwen 工具和调用 ID | MCP 执行数 | 提示终态／进程退出 |
| --- | --- | --- | --- | --- | --- |
| 原包，默认 | 无挂载 | `37ef5027` | ReadFile `c-native`、ToolSearch `c-search` | 0 | `end_turn`／0 |
| 原包，默认 | 挂载 | `00fd9c41` | ReadFile `c-native`、ToolSearch `c-search`、ToolCall `c-query`／`c-callers`／`c-node` | 3 | `end_turn`／0 |
| 原包，默认 | 撤除 | `491b949a` | ReadFile `c-native`、ToolSearch `c-search` | 0 | `end_turn`／0 |
| 受审，默认 | 无挂载 | `28b860a5` | ReadFile `c-native`、ToolSearch `c-search` | 0 | `end_turn`／0 |
| 受审，默认 | 挂载 | `0cd2d2f6` | ReadFile `c-native`、ToolSearch `c-search`、ToolCall `c-query`／`c-callers`／`c-node` | 3 | `end_turn`／0 |
| 受审，默认 | 撤除 | `a00f33f3` | ReadFile `c-native`、ToolSearch `c-search` | 0 | `end_turn`／0 |
| 原包，直接声明 | 无挂载 | `e7909259` | ReadFile `c-native` | 0 | `end_turn`／0 |
| 原包，直接声明 | 挂载 | `4e709638` | ReadFile `c-native`、直接 MCP `c-query`／`c-callers`／`c-node` | 3 | `end_turn`／0 |
| 原包，直接声明 | 撤除 | `739b93b7` | ReadFile `c-native` | 0 | `end_turn`／0 |
| 受审，直接声明 | 无挂载 | `e2d7c55b` | ReadFile `c-native` | 0 | `end_turn`／0 |
| 受审，直接声明 | 挂载 | `463adad0` | ReadFile `c-native`、直接 MCP `c-query`／`c-callers`／`c-node` | 3 | `end_turn`／0 |
| 受审，直接声明 | 撤除 | `a0011314` | ReadFile `c-native` | 0 | `end_turn`／0 |

各挂载轮的发现帧为 `initialize`（ID 0）、`prompts/list`（1）、`resources/list`（2）、`tools/list`（3）。无挂载、撤除轮均无任何服务帧，默认 ToolSearch 返回三项未找到。所有 ReadFile 结果均在续轮回传该轮文件的随机标记。默认挂载轮的 ToolSearch 结果按 `c-search` 回传三项完整定义，随后走原生 ToolCall。直接声明轮在模型目录直接提供三项的完整名称。

| ACP 与模型调用 ID | 实际执行方和工具 | MCP 请求 ID | 续轮核对的实际输出 |
| --- | --- | --- | --- |
| `c-query` | RustCodeGraph `rustcodegraph_query` | 4 | `graph_probe` 与 `call_graph_probe` 在 `lib.rs:1`、`lib.rs:2` |
| `c-callers` | RustCodeGraph `rustcodegraph_callers` | 5 | `call_graph_probe` 在 `lib.rs:2` |
| `c-node` | RustCodeGraph `rustcodegraph_node` | 6 | `lib.rs` 两行源码及调用表达式 |

四个挂载轮的 ACP 更新均为 `completed`。目标服务的三项执行均成功，OpenAI Chat Completions 续轮以相同的 `tool_call_id` 读到对应服务输出。

## 拒绝、取消与恢复

| 样本 | 原包／受审会话前缀 | 执行、回传与终态 |
| --- | --- | --- |
| 短名对照 | `7a5c8aae`／`27c39200` | 服务列出八项。ToolSearch `c-search` 返回三个短名均未找到，ReadFile `c-native` 成功，MCP 零执行，`end_turn`／0。 |
| 单次拒绝 | `d79b49ed`／`46a2f1e6` | ReadFile、ToolSearch 完成，ToolCall `c-query` 选择 `reject_once` 后 ACP 更新为 `failed`。服务已发现，零 `tools/call`，`end_turn`／0。 |
| 取消 | `26578383`／`47b94fd5` | 收到模型请求后、回包前发送 `session/cancel`，零工具执行，`cancelled`／0。 |
| 加载及恢复 | `55e5480a`／`a4c3673a` | 分别用新进程对原会话 ID 调用 `session/load`、`session/resume`。重传服务时，两种恢复方式均实际执行 ReadFile、ToolSearch 和三项 MCP，服务 ID 4／5／6 成功。发空服务列表时，两种方式均只执行本轮原生工具，目标服务零访问。所有提示均为 `end_turn`／0。 |

恢复轮的当前调用 ID 在原名后加 `-load-mounted`、`-load-removed`、`-resume-mounted` 或 `-resume-removed`。首次探针在历史会话复用了模型工具 ID，触发 Qwen 的重复调用防护。补齐当轮唯一 ID 后，逐项对照实际服务帧、ACP 完成事件和同 ID 的模型结果。

## 取证范围

原包与受审版在同一默认发现和审批模式下均完成共存。该版本的精确检索要求使用完整注册名。短名对照确认缺口出现在发现阶段，完整名已覆盖原先缺失的 `node`。本次无适配器代码变更。管理员策略合并、技能来源、宿主项目名单投影、真实供应商、新网关和非本机平台没有当前运行证据。历史网关只读验收见[原回执](../../mcp-acceptance/qwen.md)。未运行 Cargo。

私有目录 `/private/tmp/qwen-coexist33.CV7syP` 保留探针、stdio 记录器和逐轮 JSONL。设置 `NODE_BIN`、`GRAPH_BIN` 后可用 `python3 probe.py <original|managed> <search|direct> before mounted removed` 复验三轮，用 `python3 probe.py <original|managed> search mounted deny cancel short` 复验短名、拒绝、取消及恢复。运行 `python3 verify.py` 对六组已保存的 `*.final.jsonl` 校验，28 个样本通过，退出 0。固定入口版本、原包 SRI 和六项受审摘要已核对。新增回执通过 `git diff --no-index --check /dev/null` 空白检查。
