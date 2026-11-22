# 任务 10 适配 aionrs

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 aionrs 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/aionrs.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/aionrs.rs`、`patches/acp-model-source/aionrs`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_aionrs_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/aionrs.md`、`docs/dev/acp/skill-coexistence/aionrs.md`。

## 上下文

审计基线版本为 0.2.11。MCP 现状｜旧 ACP 桥拒绝非空列表；引擎 JSON Stream 能挂载，原生 stdio 与子进程隔离冲突。模型来源现状｜原生有模型配置；受审桥、私有中继；通用，挂载依赖私有 Unix socket。原生工具现状｜普通及挂载保留 Read/Write/Edit/Glob 安全范围，挂载叠加已发现服务工具。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v2 桥将隔离 stdio 投影到同端口 HTTP，过滤服务名单、绑定单次授权和回传），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_aionrs_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_aionrs_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 固定候选制品 | 当前阻塞 |
| --- | --- | --- |
| x86_64 | 上游 CI 构建目标 `x86_64-unknown-linux-gnu`。固定源码本机生成 `aionrs`，ShiJu 同架构提供 `aionrs-acp`。验收校验两者 ELF64 小端、机器号 62 和回执摘要。 | 上游发布无二进制附件。缺独立受审制品、Linux 等价沙箱及 Docker amd64 证据。 |
| aarch64 | 上游 CI 构建目标 `aarch64-unknown-linux-gnu`。同一固定源码及补丁，本机生成对应架构的引擎和桥。验收检查机器号 183 和回执摘要。 | 上游发布无二进制附件。缺独立受审制品、Linux 等价沙箱及 Docker arm64 证据。 |

- 已核对 [0.2.11 发布 API](https://api.github.com/repos/iOfficeAI/aionrs/releases/tags/v0.2.11) 的 `assets: []`。可用 [固定源码](https://codeload.github.com/iOfficeAI/aionrs/tar.gz/8e61a90329fa9f67c4fdf7e97fe02c24dba33f75) 的 SHA-256 为 `102f0682858a4f02b505b0413578a0de508a1837cd7b93e5626643d63dd06688`。现行 `read-only-skills.patch` SHA-256 为 `4c56c494fdddbb82eebe47481c79048b5abafa7135db388532bd117af150a0f1`，对固定源码干跑命中三个文件。上游锁定 Rust 1.96.1 并含 `aws-lc-sys`，候选源构建需要 Python 3、patch、Cargo、目标 GNU C 工具链和 CMake。现有安装脚本的引擎参数为 `cargo build -j 1 --locked -p aion-cli --bin aionrs`。桥为 `vw-acp --bin aionrs-acp` 的同架构产物。普通发现用 `aionrs-acp`、无参数，现行桥优先使用同目录引擎，随后才查 PATH。Linux 普通路径也没有本轮运行证据。
- 任务 48 对应入口是 `managed_runtime::{install, reviewed_agent_command, verify_reviewed_agent_executable}`、`managed_runtime/install.py` 的 aionrs 分支和 `model_source::drivers::aionrs::configure`。候选格局复用 v3 的绝对 `aionrs-acp` 固定入口、无供应商参数、同目录 `aionrs` 和 `build-receipt.json`。回执包含版本、固定源和补丁摘要、桥和引擎摘要。验收校验回执自洽，后续生产登记需对应每架构受审产物或可信构建证明。继续维持 `drivers::capabilities` 和 `skill_mounts::validate` 对 Linux 的受管门控。Linux 普通 `cli_local` 与 Apple Silicon 生产分支沿用现状。
- Chat Completions 接线复用 `drivers::aionrs::configure` 绑定选模和认证回环端口，`AionrsBackend::create_session_with_mcp` 与 `Process::attach_mcp` 经私有 Unix socket 绑定同端口 HTTP 投影。现有受管桥仅代理选定会话的 stdio 服务，过滤工具目录，按工具名与参数单次消费审批，并在取消、终态或进程销毁时撤权。其 Seatbelt 文件、进程和网络策略由 `Process::spawn` 与 `mcp::Broker::start` 执行，后两者明确限制 Darwin arm64。Linux 的现有 Bubblewrap 包装隔离了回环中继且没有派生限制，Landlock 的逐命令入口返回 Unsupported。安全续接需要分别约束引擎与 stdio 服务，保留引擎只访问绑定端口、stdio 无网络/派生、工作区和私有路径读写边界及失败关闭。相关生产改动超出本任务独占文件边界。
- 原生技能和所选技能分别走上游工作区扫描与 `skill_mounts::aionrs::publish` 的私有符号链接视图。引擎应用只读技能补丁，禁止脚本型技能。现行隔离 HOME 隐藏用户配置，通过引擎私有配置禁止自动批准。原生已配置 MCP 没有随受管配置导入的受审途径，不能以会话外挂覆盖该项。桥禁用引擎持久会话，`AionrsBackend::load_session` 明确拒绝，尚缺安全恢复、旧会话撤权及任务 4 的提示轮/原始调用 ID 执行绑定。任务 49 请在两个组合分别登记上述限制。
- 任务 48 请在 `acceptance_support/mod.rs` 加入 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_aionrs_tests;`。验收从真实网关入口走基线、挂载、撤除、拒绝四阶段，模型夹具核对实际 `POST /v1/chat/completions`、原生及外挂技能来源、同调用 ID 续轮及 MCP 审计增量 0/1/0/0。撤除阶段使用新会话。旧代码预计首先在基线的受管选模门控报错。测试只在已提供候选时运行，Linux 夹具校验 ELF 架构并要求显式候选目录。该入口尚无原生静态 MCP、同会话恢复撤权或写入授权正例。
- 静态核对已通过源码和补丁 SHA-256、补丁干跑、定向 rustfmt、Python AST 解析及差异空白检查。未安装、编译或运行测试。接线后每个非 root Linux GNU Docker 容器提供 `/usr/bin/ruby`、Python 3 和 procps，将 `SHIJU_ACP_AIONRS_GATEWAY_BIN` 指向同架构网关绝对路径，`ACP_AIONRS_REVIEWED_DIR` 指向带两制品及回执的候选目录，再运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_aionrs_contract -- --ignored --nocapture`。预期命中 1 项，当前无红绿退出码和测试数。

### 共享接线后的续查

- 任务 48 已通过 `acceptance_support/linux_platform_tests.rs` 注册网关验收。两种 Linux GNU 目标可复用本次移植的 `skill_mounts::aionrs::publish`。它复用宿主技能租约，发布前要求每个链接解析到工作区下的规范目录。Apple Silicon 的合法技能路径仍使用现有视图。外部门控、普通 CLI 和审批策略维持现有路径。待授权时运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_aionrs_contract` 检查两个独占技能用例，随后用上一节命令运行单项网关验收。
- 固定提交的 `aion-mcp/src/tool_proxy.rs::execute` 仅把服务名、工具名和参数传给 `McpManager::call_tool`，而 `json_stream` 审批只在本地携带 `call_id`。现有同端口代理可取得会话、工具目录、参数及一次性授权，固定版执行帧没有原始调用 ID。任务 4 的租约不能单独弥补这条协议断点。两种架构的原生和外挂 MCP 需要受审的执行 ID 扩展或等价入口，原生静态服务也必须通过同一受控投影。
- 已复核原生恢复与宿主回放。固定版 `json_stream/pre_message.rs` 只在首条消息前允许注入服务，`dispatch.rs` 拒绝轮中注入。ACP 桥的 `load_session` 拒绝恢复。宿主回放可重建消息上下文，不能恢复原生会话状态或证明旧引擎的服务已撤权。需要先销毁旧进程及借据，再建立受审的新启动及恢复入口。
- 固定源码构建可补无上游二进制附件，故附件缺失不单独阻塞。两个 Linux 组合继续标为已阻塞。所需输入为可独立验证来源的同架构桥和引擎、适用于两种进程的等价隔离执行入口，以及上述审批和恢复协议能力。已交付固定源参数、技能视图和已注册验收入口。两种 Linux 目标均未执行 Cargo、Docker 或供应商进程。

### Linux MCP 调用身份候选

- 新增两架构共用的 `patches/acp-model-source/aionrs/linux-mcp-call-id.patch`，SHA-256 为 `d4a571d9455ec21c61bf35359d5ab56aa43740ad78ae1a2295c7309aab22f6a1`。先对 0.2.11 固定源码应用 `read-only-skills.patch`，再叠加此层。两个补丁按顺序实际应用成功，四个新层文件均通过 rustfmt 语法解析。未编译或执行上游。
- `execute_tool_calls_with_approval` 用同一 `ContentBlock::ToolUse` 进行批准与执行。候选沿 `execute_single` 把该 ID 传给 `McpToolProxy`，再由 `McpManager` 放进 `tools/call.params._meta.shijuCallId`。现有 Streamable HTTP 传输直接序列化请求。仅当 Linux 受管入口显式注入 `SHIJU_AIONRS_MCP_CALL_ID=1` 时携带字段，缺 ID 在执行前返回错误。普通 CLI 和 Apple Silicon 沿用无该字段的请求。该字段来自执行上下文并位于模型工具实参之外。
- 任务 48 的接线契约。为 Linux 两目标使用单独运行时修订并将叠加补丁摘要纳入可信构建回执。只有在受管 Linux 桥已把所有选中 MCP 投影到私有代理后，才从清空后的引擎环境注入该开关。桥在允许 `tool_request` 时从真实 `msg_id`、`call_id`、目录服务、工具和精确参数发放单次授权。代理从对应挂载路由和 `params._meta.shijuCallId` 核对租约并执行前消费，向下游发送前移除该内部字段。缺失、错误或已消费的 ID 失败关闭。终态、取消、撤除和重新启动均须失效旧授权。本次交付上游执行帧，现有生产桥没有注入开关或接受该字段，既有对外门控照常生效。
- Linux GNU x64/ARM64 的受管模式都仍受限于引擎与 stdio 进程的等价隔离入口、受审双制品和原生静态服务/会话恢复。执行 ID 缺口已缩小到宿主代理消费接线。后续补同工具同参数不同 ID、缺失 ID、重放与撤权后零新增审计的真实入口断言，再运行已注册网关验收。
