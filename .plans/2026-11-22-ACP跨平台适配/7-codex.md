# 任务 7 适配 codex

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 2】 依赖：批次 1

状态：已阻塞

来源任务：无

目的：完成 codex 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/codex.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/codex.rs`、`crates/libs/acp-access-policy/src/access_policy/codex.rs`、`patches/acp-model-source/codex`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_codex_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/codex.md`。

## 上下文

审计基线版本为 2.5.0 / Codex 0.165.0。MCP 现状｜原生 MCP 已执行回传；旧 Chat→Responses 不完整明细导致终答失败。模型来源现状｜原版有配置选模；受审私有 Responses 中继；通用。原生工具现状｜普通/挂载沿原权限；历史挂载实测 83 次目标及 2 项官方资源发现；Chat 兼容路线关闭托管 web_search。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

1. 锁定桥 2.5.0 与引擎 0.165.0 作为起点。核对两个 Linux 架构的引擎包、私有历史及技能根、Chat→Responses 终帧与原生 Responses 的调用 ID，避免普通 npm 浮动版本误入受管路由。在本编号文件记录可供批次 3 复用的实现与接线模板，覆盖 npm/本机引擎。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_codex_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_codex_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

- Linux GNU x64 与 ARM64 的宿主分别消费 npm `@openai/codex-linux-x64` 与 `@openai/codex-linux-arm64` 别名，对应 `@openai/codex@0.165.0-linux-x64` 和 `@openai/codex@0.165.0-linux-arm64` 的上游 musl 制品。已核对 npm 版本和架构，流式检查两个归档的布局及摘要。桥、包装器、引擎和沙箱/代码模式辅助制品见 [Linux 制品清单](../../patches/acp-model-source/codex/linux-engine.json)。依赖 Node.js、npm 可选依赖、patch 和目标系统的 Linux 沙箱环境。
- 任务 48 在 `acp_agent_registry::managed_runtime::{install_builtin_agent, reviewed_agent_command, verify_reviewed_agent_executable}` 按宿主架构接入清单。受管安装使用 `--global=false --package-lock=true --include=optional --ignore-scripts --save-exact`，固定 `@agentclientprotocol/codex-acp@2.5.0 @openai/codex@0.165.0`。校验锁文件中的桥、包装器及对应架构归档完整性，按顺序应用原补丁与 Linux 补丁，校验最终 Linux 摘要。发现、显式覆写、启动与恢复复用同一验证，拒绝路径重定向并流式校验已安装引擎和辅助文件。Linux 桥启动固定同级包装器并验证平台别名解析，拒绝 `CODEX_PATH`、嵌套副本和包装器自带的回退路径。注册入口为绝对 `node_modules/@agentclientprotocol/codex-acp/dist/index.js`，参数为空，生产驱动追加 `cli --shiju-model-source`。
- 模型沿用 `prepare_profile` 和 `drivers::codex::configure` 的私有配置、历史引用和回环令牌。原生 Responses 保留调用 ID。Chat 转换沿用同 ID 工具续轮和已修复的终帧明细，关闭托管搜索。Apple Silicon 沿用已审 `e0dfd526…` 摘要与原模式。普通 cli_local 保留独立发现和启动。批次 3 可复用固定 npm 桥与上游平台别名、归档和运行入口双层校验、驱动追加受管参数及普通 CLI 独立路径。任意本机引擎替换尚缺受审契约。
- 受审桥在 `createSessionConfig` 合并原生和外挂 stdio/HTTP MCP，原生同名条目优先。原生与外挂技能复用 `skillsExtraRootsSet`、`shiju.skillMounts` v1 和宿主私有视图。SSE MCP 未获支持证据。Linux 两架构均继续拒绝完整受管能力。原生 read-only 沙箱允许部分操作直接执行，原生静态 MCP 没有宿主执行前校验，外挂命令和 URL 同样没有接入任务 4 的执行租约。桥的 elicitation 仅在上游请求时触发，thread/server 关联也无法为每次执行提供原调用 ID 和实参绑定。需要受审的逐调用拦截与执行前授权消费，或等价的宿主约束，才能证明批准后续轮及拒绝、撤除后的服务审计零新增。任务 49 对两架构均登记此限制。
- 任务 48 在 `acceptance_support/mod.rs` 以 Linux GNU x64/ARM64 的 `cfg(test)` 条件注册 `platform_codex_tests`。已有断言覆盖环境桥拒绝、实际受管桥摘要、协议选择和 ACP 初始化。误装旧桥时 Linux 摘要断言失败。工具、技能、调用 ID 续轮及拒绝/撤销后的 Linux 服务审计尚缺完整正例，后续可复用 `acceptance_support::codex::run` 和 `skill_mounts::run` 连接真实夹具。
- 静态检查通过，覆盖原包摘要、两层补丁应用和产物摘要、`node --check`、Rust 源码格式解析、JSON 解析和局部 `git diff --check`。未安装、编译或执行测试。两架构待授权后分别运行原定 Cargo 命令，准备受审隔离安装后再追加 `-- --include-ignored` 执行真实握手。暂无红绿退出码或命中数。

## 续查与追加交付

- 已复核固定版引擎源码。普通服务可设 `default_tools_approval_mode = "prompt"`，但应防止工具级配置、持久批准和原生执行策略接管。当前桥默认走 `mcpServer/elicitation/request`。该表单有线程、可空的轮次及服务名，没有原始工具调用 ID。桥通过线程与服务索引单个候选，多候选时无法配对，送给宿主的参数也可能是展示参数。继续使用此路线需要引擎暴露明确的原调用身份。关闭 `ToolCallMcpElicitation` 时，引擎的 `item/tool/requestUserInput` 暴露 `itemId`，可评估桥侧按线程、轮次及 item 关联同一 `item/started` 的服务、工具与原始参数。该回退的非根线程直接拒绝，普通 user-input 处理器也未将它映射成宿主逐次许可。[固定版 MCP 审批](https://github.com/openai/codex/blob/rust-v0.165.0/codex-rs/core/src/mcp_tool_call.rs) · [固定版请求映射](https://github.com/openai/codex/blob/rust-v0.165.0/codex-rs/app-server/src/bespoke_event_handling.rs) · [表单协议](https://github.com/openai/codex/blob/rust-v0.165.0/codex-rs/app-server-protocol/src/protocol/v2/mcp.rs)
- 引擎实际向 MCP 发送 `params._meta.callId` 和会话/线程标识。这提供了后续复用宿主中继的入口，但现有 `ManagedMcpApproval` 只把工具名、实参和有效性回调交给 `VtcodeApproval`，执行端只按名和实参消费。相同轮中的同工具同实参无法与获批的调用 ID 区分。需要新宿主合同双端绑定调用身份，并且原生静态服务也须经过可撤销的执行边界。Codex 的 PreToolUse hook 有部分覆盖，例如 `write_stdin` 没有自己的 PreToolUse，原生沙箱内读操作还可能直接执行，因此配置 hook、MCP prompt 或普通 read-only 单项均无法满足完整验收。Linux x64/ARM64 的原生操作审批、静态/外挂 MCP 执行与撤销仍为实现阻塞。保持现有门控，等价执行边界需在任务 7 独占文件范围之外补齐。[固定版调用元数据](https://github.com/openai/codex/blob/rust-v0.165.0/codex-rs/core/src/mcp_tool_call.rs) · [固定版 write_stdin](https://github.com/openai/codex/blob/rust-v0.165.0/codex-rs/core/src/tools/handlers/unified_exec/write_stdin.rs)
- 已补独占 `acp-agent-registry/src/managed_runtime/codex.rs`。任务 48 在 `managed_runtime.rs` 用 `#[path = "managed_runtime/codex.rs"]` 并按 Linux GNU x64/ARM64 条件声明 `mod codex`，在 Linux Codex 的机器制品校验分支调用 `codex::verify(directory)`，该方法返回经验证的绝对入口。工作阶段与启动阶段复用它，继续校验原契约的桥/包装器包名与版本，并保留安装器的归档完整性验证。切换到 Linux 新修订目录，避免与 Apple Silicon 的 v2 摘要共用安装位置。需保持注册的普通 cli_local 发现路径和受管能力门控独立。新增两个目标的上游 rg 摘要，校验函数流式验证桥、包装器、引擎和固定助手，拒绝路径重定向或缺失、不可执行的原生文件。
- 独立验证器断言覆盖执行位、内容更改、缺失文件和路径重定向。任务 48 注册后，在获准的两个 Linux Docker 目标运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p acp-agent-registry --lib acp_platform_codex_contract`；原 `llm-runtime` 契约继续覆盖协议与受管握手。已通过两份 Rust 源文件格式检查、JSON 解析及限定差异检查。未运行 Cargo 或供应商运行时。

## 原生 Responses 的宿主外挂候选子集

- 已在固定桥的实际 `CodexAppServerClient` 通知和请求分派中接入候选。锁定线程、当前轮、`item/started` 的原始 `item.id`、服务名、工具名与对象实参。仅匹配单个原生审批题 `mcp_tool_call_approval_<itemId>`，仅向宿主发 `allow_once/reject_once`，批准后再次检查轮、处理器及挂起记录。模糊 form、重复开始、完成后回复、跨轮、错误题目、缺 ID 和取消均不能发放原生 Allow。动作标识沿原样 `toolCallId` 和 `mcp__<server>__<tool>` 进入现有宿主确认。Codex Native `tools/call` 携带相同 `params._meta.callId`，新解码器拒绝 JSON-RPC ID、`shijuCallId` 和双字段。未改 Apple Silicon 原补丁与普通启动。
- 任务 48 只为 Linux GNU x64/ARM64、隔离的 `session_model`、原生 Responses、单项 `ReviewedStdioSource::Mounted` 接候选。使用 `reviewed_linux_stdio` 的宿主固定目录、文件摘要、租约和工具名单，为 `prepare_linux_stdio` 添加 Codex 专属 owner 分支，复用 `VtcodeApproval::new(true)` 和 `Projection::approval_config`。在受该 bearer 绑定的 Codex 专属 HTTP 执行分支用 `drivers::codex::host_mcp_call_id(&message["params"])` 代替读取 `_meta.shijuCallId`，校验失败不进入执行器。保持其他适配器协议原样，别把原生 callId 转发到 stdio。把唯一投影 server 及回调写入 `profile.managed_mcp`，调用 `configure_host_mcp_candidate(profile, endpoint, server_name, service_url)` 完成私有驱动绑定，再通过真实 ACP 初始化并以宿主投影替换原始挂载。两个 `SHIJU_CODEX_HOST_MCP_*` 值只能来自该绑定。进程必须走已审 GNU 隔离，禁用真实用户配置、静态 MCP 和任意网络出口。使用 Linux 制品清单的新摘要。
- 候选只声明上述宿主外挂工具。不发布原生文件、终端、资源、静态 MCP、技能、Chat 转换或子代理能力。桥限 read-only，禁用 shell、code mode、协作、Apps 与 web search，并拒绝原生升级请求。固定版剩余原生内置项仍受引擎目录和隔离约束，不能作为逐次审批能力。现有公开门控与普通 cli_local 保持。若共享部署不能隔离工作区/用户配置、限制出口或收紧工具目录，则继续拒绝候选。
- 两架构恒定测试输入：只挂载受审 stdio 服务 `fixture`，其 `read_probe` 对 `{"value":"challenge"}` 写独立调用计数并返回 `MCP_FIXTURE_RESULT`。Responses 夹具以 `call_fixture_1` 调用该已声明工具；期望宿主确认看到 `mcp__fixture__read_probe`、原始实参与原 ID，独立服务只执行一次，模型续轮收到相同 ID 的 `function_call_output` 和标记。对照包含无交互、拒绝、等待中取消、撤销后恢复、相同工具和实参的不同 ID、错误服务、参数或双 ID、同 ID 重放；服务计数均不能增长。第二个不同名称服务与项目静态 MCP 均在候选建会话前拒绝。测试入口沿 `acp_platform_codex_contract`，额外阶段需接通真实宿主中继。
- 本轮静态证据：从固定原包重放两层补丁，Linux 最终摘要与清单相符，Node 语法、Rust 源码格式及限定 Git 差异检查通过。新增原生 ID 解码断言。未运行 Cargo、桥、引擎或工具。原任务整体仍为已阻塞，剩余实现为候选的共享安装/隔离/中继接线及原生工具完整审批合同。
