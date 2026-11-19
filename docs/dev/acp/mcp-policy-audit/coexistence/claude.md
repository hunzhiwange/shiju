# Claude 官方工具与 MCP 共存复验

2026-11-19，Darwin arm64。直连官方 ACP 的两组回环三轮均通过。挂载轮在同一 `session/prompt` 内执行 Claude Code 原生 `Read` 和实际 stdio MCP `read_probe`，两条独立文件标记按原调用 ID 回传给模型。撤除后新会话仍读到本轮原生文件，目标工具从目录消失，服务零调用。

## 入口和协议

- 内置注册 key 为 `claude`，优先发现 `claude-agent-acp`，原版 npm 回退为 `npx -y @agentclientprotocol/claude-agent-acp@^0.26.0`。原版保留原生选模。受管入口要求绝对路径、空参数及对应摘要，转换仅接受 0.26.0，原生 Messages 接受受审的 0.26.0 或 Darwin arm64 的 0.31.0。受管启动以请求级 `_meta.claudeCode.options.settings/model` 注入选模和私有 SDK 设置，当前仓库源码在恢复请求也传递同一元数据。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [启动检查](../../../../../crates/libs/acp-client/src/client/process.rs) · [会话元数据](../../../../../crates/libs/acp-client/src/client/session_meta.rs) · [审查摘要](../../../../../crates/libs/acp-access-policy/src/access_policy/claude.rs)
- 0.26.0 从 `/opt/homebrew/bin/claude-agent-acp` 直接启动。入口 SHA-256 为 `2859ccaa5ba91ed27b80614eae47399370015acbcbc7d01d1408c71f7964565d`，适配器与实际解析到的 SDK 共 10 项受审摘要一致。SDK 0.2.96 内置引擎实报 2.1.96。
- 0.31.0 从私有 npm 前缀的 `node_modules/.bin/claude-agent-acp` 直接启动，使用官方 registry 的本机缓存离线安装固定三包。入口 SHA-256 为 `0aa4c4255951db9614d2867df5e905e23167bb581d1391da50995c6491262239`，适配器、SDK 与原生引擎共 12 项受审摘要一致。SDK 0.2.119 的 Darwin arm64 引擎实报 2.1.119，SHA-256 为 `31db3444309d5d0f8b85e8782e2dcd86f31f7e48c1a1e83d69b09268c7b4f9a2`。两版使用 Node 24.22.3。
- 直连夹具使用私有 HOME、Claude 状态目录和项目，出站限制到当次模型回环端口。每轮生成两个随机标记，分别写入项目内只读探针和项目外 MCP 附件。提示和预置模型响应均未包含标记。三轮只通过 `mcpServers` 切换挂载，项目未写入旧验收的 `permissions.deny`，会话沿用 `default`，无全权授权。原生 Read 由 Claude Code 读取私有文件，未声明 ACP 文件回调能力。MCP 只通过正常 `allow_once` 执行。
- 0.26.0 回读的模型及实际 Messages 请求均为 `deepseek-v4.1-flash-1017`。模拟当前驱动的转换路由设置 `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`，正向三轮的 7 次 Messages 请求均无 `context_management`。同一版本及模型去掉开关，另三轮的 7 次请求均带该字段。历史网关的首个模型请求因此返回 422，原修复仍只作用于转换路由。[驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/claude.rs) · [历史 422 及回放](../../mcp-acceptance/claude.md)
- 0.31.0 原生 Messages 保留实验 beta，三轮的 7 次请求均带 `context_management` 和 `output_config`。实际请求均为绑定的 `claude-opus-4-6`，ACP 目录回读的是 `sonnet` 选项及 `effort=high`。回环以实际出站模型字段核对绑定。该版本只用于原生 Messages。[原版选模与受审路由证据](../../../../shiju/acp-model-source/evidence/claude.md)

## 三轮实测

两个版本均完成 ACP v1 握手。每个 `session/new` 得到不同 ACP 会话 ID。0.26.0 目录工具数为 21 → 22 → 21，0.31.0 为 22 → 23 → 22。每个挂载轮实际记录 `initialize`、`tools/list` 和 `tools/call`，MCP JSON-RPC 调用 ID 为该服务进程内的 `2`。用服务名、唯一调用和附件结果把它对应到 Messages 与 ACP 的 `toolu_mounted_2`。以下调用 ID 在各自会话内关联。

| 版本 / 轮次 | ACP 会话 ID | 调用 ID、执行方和结果 | 服务调用增量 | 终态 |
| --- | --- | --- | --- | --- |
| 0.26.0 无挂载 | `a691af8c-39a4-4d1c-9d08-aa6f044f7e67` | `toolu_before_1`，Claude Code `Read` 成功 | 0 | `end_turn` |
| 0.26.0 挂载 | `7d42ded6-cf89-442d-80cc-aa6602ddfee7` | `toolu_mounted_1`，Claude Code `Read` 成功。`toolu_mounted_2`，stdio `coexist_probe.read_probe` 获批并成功 | 1 | `end_turn` |
| 0.26.0 撤除 | `67a7f41e-ccce-4e75-8c65-051f1539f086` | `toolu_removed_1`，Claude Code `Read` 成功 | 0 | `end_turn` |
| 0.31.0 无挂载 | `18134afa-ee67-47b6-ac59-1033f230872c` | `toolu_before_1`，Claude Code `Read` 成功 | 0 | `end_turn` |
| 0.31.0 挂载 | `32043f6d-2abd-4eec-8386-a96d06c1753f` | `toolu_mounted_1`，Claude Code `Read` 成功。`toolu_mounted_2`，stdio `coexist_probe.read_probe` 获批并成功 | 1 | `end_turn` |
| 0.31.0 撤除 | `ed6d936d-1c93-45e7-bea4-129bd0edf589` | `toolu_removed_1`，Claude Code `Read` 成功 | 0 | `end_turn` |

ACP 对各成功调用报告 `completed`。夹具从同一提示的后续 Messages 请求逐一核对 `tool_result.tool_use_id` 和磁盘标记，再发回 `COEXIST_DONE`。正向轮 Messages HTTP 200，两个探针退出 0，适配器进程分别五次退出 0。每条连接显式关闭会话。

## 拒绝、名单、取消和恢复

两版结果一致：

- 独立拒绝轮先读到原生标记 `toolu_deny_1`，宿主以 `reject_once` 拒绝 `toolu_deny_2`。MCP 初始化并列出目录，实际调用为零。ACP 将该 ID 标为 `failed`，模型收到同 ID 的 `is_error=true` 后结束。
- 正向三轮没有传 SDK 名单。独立样本在 `_meta.claudeCode.options.allowedTools` 显式列出 Read 和目标 MCP，两个调用 `toolu_explicit_1/2` 成功。目标工具按显式预授权执行，未重复请求一次性批准，工具目录仍可见。服务逐工具名单和同名冲突未在本直连样本覆盖。
- 首次流式模型请求已到达回环时发出 `session/cancel`。模型夹具在此之前未发工具指令，ACP 返回 `cancelled`，零工具调用，连接可正常关闭。
- `session/load` 和 `session/resume` 分别在新适配器进程传入原挂载的 ACP 会话 ID 和当前服务定义。各自重传时，新调用 `toolu_load_mounted_1/2` 或 `toolu_resume_mounted_1/2` 读到新标记，服务各执行一次。各自传空表时，`toolu_load_removed_1` 或 `toolu_resume_removed_1` 只执行原生 Read，目录无目标工具、服务零调用。全部以 `end_turn` 结束。Load 重放的历史调用与新调用按 ID 区分。

## 复验边界

本次使用实际官方启动入口、实际 SDK 与 stdio MCP，仅模型响应由回环合成。它覆盖原生工具与 MCP 共存的通用能力，实机范围是 Darwin arm64 和 stdio。未重跑 ShiJu 网关的租户投影、真实上游、HTTP/SSE MCP 或其他平台。转换兼容对照观测的是 SDK 发出的 Messages，对转换器的端到端历史证据见上文链接。检查未发现本项需修复的适配器裁剪，产品源码未改，未运行 Cargo。

私有复验脚本及脱敏观测位于本机 `/tmp/shiju-claude-coexist.yhyrI5/`（0700），可从仓库根目录用 `PYTHONPATH="$PWD/scripts" python3 <私有目录>/probe.py <conversion|conversion-unset|native> <受审入口绝对路径>` 复跑，使用 `PROBE_OUTPUT` 指向私有目录下尚不存在的输出子目录。脚本复用仓库的 Messages 应答与 stdio 文件探针。主要记录为 `conversion-verified/`、`conversion-unset-verified/` 和 `native-lifecycle/` 中的 `observations.json`、`calls.rpc`、`calls.audit`。回环服务、适配器和 MCP 子进程已退出，保留可复验制品和脱敏观测。
