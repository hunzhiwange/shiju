# 任务 41 适配 qwen

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 qwen 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/qwen.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/qwen.rs`、`crates/libs/acp-access-policy/src/access_policy/qwen.rs`、`patches/acp-model-source/qwen`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_qwen_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/qwen.md`、`docs/dev/acp/skill-coexistence/qwen.md`。

## 上下文

审计基线版本为 0.26.7。MCP 现状｜原生搜索链已执行及回传，短名漏 node；私有关搜索后三项齐全。模型来源现状｜原生 OpenAI 配置；受审；通用。原生工具现状｜普通/挂载默认可用 ToolSearch；关搜索仅私有取证设置，服务转发器独立。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（选模与认证锁、私有状态、技能根，合入管理员安全配置），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_qwen_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_qwen_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

| Linux GNU 目标 | 固定制品与入口 | 当前证据 |
| --- | --- | --- |
| x86_64 | `@qwen-code/qwen-code@0.26.7`，`cli-entry.js --acp`，内置 x86-64 Landlock、ripgrep 和 OpenTUI | 归档摘要、补丁输出、ELF 架构和执行位吻合。Docker `linux/amd64` 未运行 |
| aarch64 | 同一 npm 归档和 JS 入口，内置 AArch64 Landlock、ripgrep 和 OpenTUI | 归档摘要、补丁输出、ELF 架构和执行位吻合。Docker `linux/arm64` 未运行 |

- [Linux 清单](../../patches/acp-model-source/qwen/linux.json) 固定 npm URL、SHA-256、SHA-512 SRI、两目标原生辅助文件以及模型协议。复用 `qwen/apply.py` 和现行 v2 产物合约。实测静态重放所得 1140 个 JS/JSON 的树指纹为 `b2fdbd34cebf7510cfd4a11fea11804e805a7f09b09301d9b5b6ecea2852769f`。旧权限校验只接受 v1 的受管指纹，v2 技能扫描器和恢复的原始 home 辅助模块使树指纹不同。Linux GNU x64/ARM64 的受管校验现接受 v2 固定指纹，使 `acp-client::spawn_child_at` 可为该产物固定 `QWEN_CODE_MANAGED_NPM_PIN`。Apple Silicon 的指纹规则、v2 安装目录与现有入口沿用原实现。
- 安装接线由任务 48 在 `acp_agent_registry::managed_runtime::{install,installed_directory,reviewed_agent_command,verify_reviewed_agent_executable}` 处理。Linux 修订目录为 `qwen-0.26.7-model-source-linux-v1`，依赖 Node >=22、npm、Python 3。建议 Linux 使用 `npm --global=false --package-lock=true install --registry=https://registry.npmjs.org --ignore-scripts --omit=optional --save-exact @qwen-code/qwen-code@0.26.7`。主包运行依赖为空，PTY、Bun、剪贴板、音频和 sharp 是可选路径，本契约没有固定这些包的传递制品。核对 npm lock 中主包 SRI，应用现行补丁，核对 shared contract 六项摘要，再用 Qwen Linux 验证器检查完整 JS/JSON 指纹和本机三项 ELF。Linux 省略可选包后的真实 ACP 启动仍需双架构验证。
- 在 `managed_runtime.rs` 按 Linux GNU x86_64/aarch64 声明 `#[path = "managed_runtime/qwen.rs"] mod qwen;`。共享 `verify` 完成 npm 合约校验后，对 Linux Qwen 返回 `qwen::verify(directory)` 给出的绝对 `cli-entry.js`，配置 args 为 `["--acp"]`。安装发布前校验、发现、显式命令覆写、启动及恢复应复用该入口，拒绝旧安装目录、重定向和摘要不符。受管启动需在 `acp-client::spawn_child_at` 接入现有私有环境路径，避免继承 `QWEN_CODE_LAUNCHER_PATH`、Qwen SDK 私有环境和其他运行时注入项，并固定 Node 入口。普通 `cli_local` 保留 `qwen`、`qwen-code` 发现及原生 npx 回退。
- 候选模型路径复用 `model_source::prepare_profile`、`drivers::qwen::configure`、OpenAI Chat 选路和 `/v1/chat/completions` 私有中继。现行驱动按会话锁定模型与认证，Unix 历史目录检查 UID、类型、权限，合并 `/etc/qwen-code/settings.json` 管理员安全设置并拒绝管理员选模冲突。选中技能仍由 `skill_mounts::qwen::configure` 校验入口和请求私有视图，再追加受审扫描器的技能根。原生工作区技能保留。MCP 搜索已在 Darwin 对照确认需要完整注册名 `mcp__<server>__<tool>`，目录异步，保留原生 ToolSearch。
- 完整运行契约在两架构均阻塞。现行 `AcpEventClient::request_permission` 的 Qwen 分支识别当前会话外挂服务的工具通知，可以回传单次决策，但供应商默认分类和本机 MCP 配置可在这种通知之外执行，宿主尚不能为每个原生/外挂工具调用绑定执行前授权、撤销和实参。固定包已提供候选私有 ACP 扩展 `qwen/control/external_tool_guard/prepare`，带 session、prompt、toolCallId、工具和实参。启用全工具分支需要私有管理端、`QWEN_CODE_PRIVATE_EXTERNAL_TOOL_GUARD=required-v1`、`QWEN_CODE_PRIVATE_EXTERNAL_TOOL_GUARD_PROVIDER=attached-v1` 和初始化回读 `qwen-code/external-tool-guard-ready`。现有宿主没有此桥。实现后仍须核对原生工具/静态 MCP/外挂 MCP 的每条执行路径、服务启动和连接隔离、继承环境及审批撤销。可评估配合任务 4 的会话中继与执行租约。当前保持 `drivers::capabilities` 的 Linux 对外拒绝。任务 49 应分别登记两架构，历史 Darwin 批准和恢复证据不计入 Linux。
- 测试接线由任务 48 在 `acceptance_support/mod.rs` 按 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]` 注册 `mod platform_qwen_tests;`。用 `SHIJU_ACP_QWEN_GATEWAY_BIN` 指向当前 Linux 容器中完整的网关，`QWEN_COEXISTENCE_RUNTIME` 指向受审安装。夹具先验证入口与本机辅助文件，再经真实网关通过鉴权的 Chat Completions 回环端点，区分项目原生技能和会话技能，核对原生激活及读取、ToolSearch 中的外挂完整工具名、批准后同 ID 回传续轮、拒绝或撤除后外挂服务调用零新增。旧路由应在首轮拒绝 Linux `session_model`。夹具的撤除轮使用新会话，尚未覆盖 Linux 原生静态 MCP、同一会话恢复/取消和本机写入/命令审批。
- 本轮通过归档 SHA-256/SRI、现行补丁应用和六项输出摘要、完整树指纹、两架构 ELF 摘要及执行位、定向 Rust 格式解析、Python AST、JSON 和局部 Git 空白检查。未安装依赖、运行 Node、编译或执行 Cargo/Docker。获准后两目标各运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_qwen_contract -- --ignored`。该测试标为 ignored 因其需要已安装的供应商进程和真实网关。每目标须确认实际执行 1 项及零失败。当前没有红绿退出码及测试数。

提交：`af9fc3bbe`。

## 续查与追加交付

- Linux 受管修订改用 `qwen-0.26.7-model-source-linux-v2`。任务 48 保持原包 SRI 和 npm 参数，先执行 `qwen/apply.py <package>` 并核对原 v2 六个产物，再执行 `qwen/apply-linux.py <package>`。最终六文件合约仅以 Linux 清单 `overlay_files` 覆盖 agent chunk，随后运行 `managed_runtime::qwen::verify` 检查新树指纹和本机 ELF。Apple Silicon v2 及普通 `cli_local` 沿用原入口。网关夹具已读取新修订和叠加摘要。
- 上游 guard 在 ACP 交互执行、通用工具调度及持久化路径已有调用点。Linux 叠加层在 bootstrap config 初始化前要求非空私有父端 capability、全工具 guard 和已连接 provider。这样上游仅拦 shell 的回退分支不会用于 Linux 受管会话。提示必须来自已通过私有握手的连接，而且 `qwen-code/invocation` 解析结果必须与当前 ACP session 相同。厂商 guard 已对缺失调用上下文、请求取消、回包格式错误和执行前 abort 失败关闭。叠加层保留厂商已有的权限检查。
- 宿主协议接线的最小必需项：在 Linux 受审启动时生成进程私有 `QWEN_CODE_PRIVATE_ACP_CAPABILITY`，沿已记录的两项 guard 环境启动。初始化的 `_meta["qwen-code/private-parent-capability"]` 带同一 capability，并校验返回 `_meta["qwen-code/external-tool-guard-ready"] == "required-v1"`。仅当前活动提示发布宿主生成的 `_meta["qwen-code/invocation"] = {version: 1, sessionId, promptId}`。在 `AcpEventClient::ext_method` 接收 `qwen/control/external_tool_guard/prepare`（ACP 线上的方法名前缀由 SDK 处理）。宿主需验证会话、提示、调用 ID、规范工具名、实参和当前项目，并核对许可仍有效后返回 `{allowed: true}`。其余情形拒绝。供应商的 `permissionChecked` 仅说明其自身已判定，不能当作宿主授权。恢复需重新绑定提示，取消或撤除应废止已有许可证。
- 复查了三条替代路径。强制现有 `session/request_permission` 只能覆盖供应商派发到该回调的操作，并且现有 Qwen 类型判断只承认当前外挂服务。复用 `mimo_mcp::prepare` 能投影外挂服务，但 `ManagedMcpApproval` 目前按名字和实参授权。Qwen 在 `chunk-Z6UWCZFA.js` 的真实 MCP `tools/call` 只为启用上下文的客户端携带 session/prompt，不转发 guard 的 toolCallId，因此同提示、同工具、同实参的调用仍无法分别消费许可。本机配置中的静态服务也不会自动投影到宿主。上游私有 `qwen/control/client_mcp/message` 可作为投影候选，但当前 ACP 声明只转换 stdio、HTTP、SSE，宿主没有实现该扩展。供应商 managed engine 会跳过普通 MCP discovery，且依赖供应商私有托管环境，不是现有入口的等价替换。Landlock 只约束它实际启动的进程，不能替代逐次许可或覆盖配置加载时的 MCP 连接、静态命令和 hooks。
- 两架构均仍缺宿主私有 ACP 请求处理、静态与外挂 MCP 的统一投影及与实际执行相同的调用身份消费。未经这三个执行条件无法发布 `session_model`，不需要更换供应商制品。任务 48 可接入新修订的验证与验收模块，保持 Linux 能力门控。当前 `AcpEventClient::ext_method` 一律返回 method-not-found，即使手动提供有效私有握手值也不会获得工具放行。原生静态 MCP、同会话取消/恢复和命令执行仍需要验证。
- 本轮用固定 v2 文件重放叠加层，核对修改后的 agent chunk 摘要及 1140 文件全树摘要，通过 JavaScript 语法、Python AST、Rust 格式和限定 Git 空白检查。未运行安装器、供应商进程、Cargo 或 Docker。

追加提交：`a3fbde674`。
