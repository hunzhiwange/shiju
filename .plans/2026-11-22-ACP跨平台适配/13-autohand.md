# 任务 13 适配 autohand

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 autohand 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/autohand.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/autohand.rs`、`patches/acp-model-source/autohand`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_autohand_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/autohand.md`、`docs/dev/acp/skill-coexistence/autohand.md`。

## 上下文

审计基线版本为 0.9.11。MCP 现状｜原版 Code Mode 经搜索及脚本执行并回传；进程白名单曾连 MCP 一起滤掉。模型来源现状｜原生选模，宿主校验参数、回环、工具和温度；能力通用，驱动三平台共用源码。原生工具现状｜默认普通及挂载均可用原生搜索/脚本；私有项目名单才隐藏官方工具。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（无安装补丁；私有 OpenAI 配置，直出及关闭总结参数仅显式启用），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_autohand_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_autohand_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| GNU 目标 | 固定制品及静态证据 | 运行状态 |
| --- | --- | --- |
| x86_64 | `autohand-cli@0.9.11`，`dist/index.js`。锁定 x64 GNU 的 fff、sharp、reflink 及 esbuild 归档均通过各自 SRI 校验。归档内 `vendor/zit/linux-x64/{zit,git-zit}` 为 ELF machine 62。 | `linux/amd64` 未运行。宿主 MCP 和原生写入、命令的逐次审批仍阻塞。 |
| aarch64 | 同一 npm 制品和 JS 入口。锁定 ARM64 GNU 的 fff、sharp、reflink 及 esbuild 归档均通过各自 SRI 校验。归档内 `vendor/zit/linux-arm64/{zit,git-zit}` 为 ELF machine 183。 | `linux/arm64` 未运行。宿主 MCP 和原生写入、命令的逐次审批仍阻塞。 |

- 两架构的上游是 [固定 npm 归档](https://registry.npmjs.org/autohand-cli/-/autohand-cli-0.9.11.tgz)。下载归档的 SHA-512 与 `managed_runtime/contracts.json` 的 SRI 一致。依赖锁的 SHA-256 为 `382247a59e6e548bb27975e0fb5090039bd4c39913d92d8b9b06d4ad6a071b47`。仅在临时提取的 JS 上静态重放 `patches/acp-model-source/autohand/apply.py`，四项输出摘要均匹配现有 v1 契约。原版安装无补丁，受管安装沿用已有 v1 补丁。
- 安装需要 Node.js 22+、npm 和 Python 3。锁定 `node-pty@1.1.0` 包无两种 Linux 预编译目标，受管安装禁用生命周期脚本时，原生 `defaultNodePtyLoader` 会走非 PTY 命令回退。不得把 PTY 计为支持，也无需为此打开未经审计的安装脚本。
- 任务 48 在 `managed_runtime.rs::install` 将两种 Linux GNU 目标接入 `autohand-0.9.11-model-source-v1`，复用 `install.py` 的锁摘要、npm SRI、补丁和四文件摘要校验。沿用 `npm ci --registry=https://registry.npmjs.org --ignore-scripts --prefer-offline --no-audit --no-fund --fetch-timeout=20000 --fetch-retries=2`，保留架构可选依赖。发现和显式覆盖复用 `reviewed_agent_command`、`verify_reviewed_agent_executable`，核对 `node_modules/.bin/autohand` 实际落点。启动参数为 `--acp`，原生 `autohand` 发现和原有 npx 路径仍服务普通 `cli_local`。
- `model_source/drivers::autohand::configure` 已复用 OpenAI Chat Completions 私有回环配置，校验模型工具调用、温度和启动参数。显式直出及关闭总结只接受现有参数组，不作为 Linux 安装默认值。原生文件读取、原生技能扫描及只读技能快照可复用同一 v1 JS。Apple Silicon 仍沿用已有安装和权限契约，其运行证据不计入 Linux。
- 任务 48 核对 `skill_mounts::validate` 和 `request::do_stream_request_acp` 的 AutoHand 分支。当前两处仍限 Darwin arm64。启用受管分支需要固定入口、`session_model`、允许原生工具、禁用 full access 及可用的宿主审批。保留 `matches_managed_entry`、`configure` 对私有快照的绑定、请求结束后的销毁以及挂载轮重新启动 ACP。旧分支拒绝带挂载的恢复或 fork，当前只验证新建请求的撤除对照。
- 具体阻塞。原版外部技能可发现但工作区外读取受原生边界拒绝。原生 stdio MCP 可直接派发，项目钩子或权限模式也可能改变原生工具的审批结果。v1 仅为宿主选择的 HTTP 投影强制逐次审批，未为原生项目 MCP、原生写入和命令建立不可绕过的宿主审批。Linux 的 `RuntimeHost::prepare_stdio_mcp_launch` 最终调用 `mimo_mcp::prepare`，后者仅接受 Darwin arm64，并用 `/usr/bin/sandbox-exec` 隔离。现有通用 Bubblewrap 未提供该投影的私有 scratch、工作目录、环境和执行边界的等价接线。需要 Linux 等价隔离及原生调用的强制审批后才能发布相应能力。任务 48 对所选 Linux MCP 显式拒绝，不能透传原始 stdio 配置替代投影。
- 任务 48 在 `acceptance_support/mod.rs` 按 Linux GNU x64/ARM64 的 `cfg(test)` 注册 `platform_autohand_tests`。该测试通过现有探针调用真实网关的配置、会话、流式聊天、审批及回读入口。回环模型校验实际 Chat 路径及调用 ID，区分原生工具和外挂 MCP、原生技能原路径和选集快照，检查批准一次且实际续轮、撤除和拒绝零次新增派发。现有未接线网关应在挂载轮触发宿主门控。该探针未覆盖原生项目 MCP、原生写入或命令审批、同一 ACP 会话的恢复和取消。
- 两种容器均以非 root 用户设置 `SHIJU_ACP_AUTOHAND_GATEWAY_BIN` 为当次受测网关绝对路径。接线完成后运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_autohand_contract -- --ignored --nocapture`，预期每架构命中一项。追加 `--ignored` 用于显式启用需要实际网关和私有 npm 安装的验收。未执行 Cargo、Docker、npm 安装或上游运行时，未取得红绿结果。
- 验收源码提交为 `bbe136ec2`。Rust 格式检查、Python 语法解析、定向工作区及暂存差异检查均通过。

## Linux 受管连接加固

- 两个 GNU 目标共用 `apply.py PACKAGE BOUNDARY LINUX_BOUNDARY`。额外参数为 `patches/acp-model-source/autohand/linux-mcp-boundary.mjs`。在 ACP 受管会话构造 MCP manager 时即装载连接门，早于原生项目自行初始化连接。该 manager 只连接宿主当次投影的回环 HTTP 服务，核对名称、传输、URL 和唯一的 Authorization 头。沿用原有调用时逐次审批；拒绝同名冲突和原地重选，变更名单须新建会话。普通 `cli_local` 与 macOS v1 继续沿用既有分支。
- 任务 48 可在接通 Linux 隔离投影后将该输出注册为单独修订 `autohand-0.9.11-model-source-v2`。安装前沿用原有归档、锁及原文件校验。Linux 安装素材新增 `linux-mcp-boundary.mjs`，仅该目标传入第三参数。在安装发布、发现、显式覆盖及启动复核时使用下表的 Linux 文件集，未列的 `dist/index.js` 和 `dist/shiju-session-boundary.mjs` 仍用 v1 摘要；Linux 受管能力不能回退 v1。网关验收的私有安装、目录及摘要断言随 v2 注册一起切换。

| Linux v2 文件 | SHA-256 |
| --- | --- |
| `dist/acp-LDTU5B5V.js` | `5faaa93fe02434c76931e73681774b709ea7ada6d38e22943853d47644983734` |
| `dist/chunk-R73FESUM.js` | `fccb6680282df921958877a8cc86cd01b39f63f59002e618a7142c77530d9c7d` |
| `dist/shiju-linux-mcp-boundary.mjs` | `04ac776ad60ffda49d9d6deaefb28c8bed3821c732f1ba3be9b64125f024c9cd` |

- 共享工作区已在 ACP 客户端初始化、恢复路径拒绝未接线的 Linux MCP。供应商内部自己连接的分支仍需上述守卫。已检查现有 `mimo_mcp`、公共 MCP 目录与撤权、通用 Bubblewrap 和 ACP 宿主回调。公共目录提供选集和撤销，`mimo_mcp` 的 Linux 服务隔离仍缺；运行器包装无法直接替换 Darwin 的每服务文件、环境、网络和进程约束。原生项目服务要在保留来源及撤权的前提下交给宿主做隔离投影，现有 AutoHand ACP 入口只接收外部传入的服务列表，未提供独立的原生目录投影协议。
- 原生审批也有独立缺口。固定 JS 的 `ToolManager.execute` 允许项目钩子的 allow 决定影响是否提示，`createPermissionBridge` 受原生模式短路。仅强制该回调无法覆盖 `ActionExecutor` 的内部审批、后台运行和钩子执行；宿主的 ACP 文件、终端回调只约束已委托给宿主的操作。两架构若要开放受管原生写入、命令及原生项目 MCP，需要上游或等价受审实现提供执行前不可绕过的绑定，包含最终工具、参数、会话和提示轮，以及与宿主隔离投影配套的原生目录入口。保留现有门控，状态仍为已阻塞。
- 独立边界断言位于 `linux-mcp-boundary.test.mjs`，后续运行 `node --test patches/acp-model-source/autohand/linux-mcp-boundary.test.mjs`。本轮仅在固定归档临时提取物上静态应用两个补丁模式，核对 v1 四文件完全匹配和 v2 五文件摘要，使用 Node 语法检查、Python AST 和定向差异检查。未运行测试、安装、Docker 或 ACP 进程。
- Linux 连接守卫源码及独立断言提交为 `0f418397d`。
