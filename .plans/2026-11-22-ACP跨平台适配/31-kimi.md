# 任务 31 适配 kimi

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 kimi 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/kimi.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/kimi.rs`、`crates/libs/acp-access-policy/src/access_policy/kimi.rs`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_kimi_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/kimi.md`、`docs/dev/acp/skill-coexistence/kimi.md`。

## 上下文

审计基线版本为 2.5.0。MCP 现状｜原生 stdio 可挂载；旧宿主 TOML 类型错误使 new 认证失败。模型来源现状｜原版可选模；宿主校验启动及私有配置，无额外摘要；通用，非 Unix 权限路径另有限制。原生工具现状｜普通 Bash 有实测，默认挂载无目录裁剪；历史 101 次 MCP，官方同轮及完整目录未知。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（无安装补丁；驱动修正整数限额并隔离私有配置），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_kimi_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_kimi_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

两种架构均固定原生 Kimi Code 2.5.0，裸二进制参数为一个 `acp`，不需 npm、Node 或安装补丁。上游标签解引用为 `41c341434ae24e45bfe0f2232ec11b80531ac717`。从[官方发布清单](https://code.kimi.com/kimi-code/binaries/2.5.0/manifest.json)下载两个归档，分别核对压缩、解包摘要及 ELF 头和动态依赖，证据固化在 [linux.json](../../patches/acp-model-source/kimi/linux.json)。两个制品均引用至 GLIBC_2.28 和 GLIBCXX_3.4.21，依赖目标架构的动态加载器、glibc、libstdc++ 和 libgcc_s。取包端需要 zstd 解码，原生 Shell 还需目标环境的兼容 Shell。未安装或执行供应商运行时。

| Linux GNU 目标 | 固定上游制品 | ELF 加载器 | 运行验收 |
| --- | --- | --- | --- |
| x86_64 | `kimi-code-linux-x64.zst` → `kimi-code-linux-x64` | `/lib64/ld-linux-x86-64.so.2` | `linux/amd64` 未执行 |
| aarch64 | `kimi-code-linux-arm64.zst` → `kimi-code-linux-arm64` | `/lib/ld-linux-aarch64.so.1` | `linux/arm64` 未执行 |

- 任务 48 可以在 `acp_agent_registry::managed_runtime::{install_builtin_agent, reviewed_agent_command, verify_reviewed_agent_executable}` 消费清单。按 Linux GNU 目标选归档，先校验压缩摘要，再在私有临时路径解包并核验原始摘要，确认后发布为私有安装目录中的绝对路径 `kimi`，保留单参数 `acp`。安装、发现、显式覆盖、启动及重建复用同一身份验证。保留普通 `cli_local` 的 `kimi acp` 与原 npm 安装路径。Apple Silicon 私有技能入口仍使用现有 `d4dd02d4…` 摘要。
- `model_source::prepare_profile`、`drivers::launch_route` 和 `drivers::kimi::configure` 可复用整数限额、按租户/项目/会话隔离的 0700 私有目录、选定模型与回环协议。Chat Completions、Responses、Anthropic 和 Google 有现有路线分支，本任务夹具覆盖 Chat Completions。防止回落到默认账户或模型，保留参数、环境和回环地址约束，持续执行现有 Unix 权限校验。
- `skill_mounts::kimi::configure` 将租约视图追加到私有 TOML 的 `extra_skill_dirs`。任务 48 在取得 Linux 受审安全入口及目标证据后，将 `verify_entry` 的 Linux 校验与受管注册的入口摘要共用，保留 `request.rs` 的 `session_model`、禁用工具和 `DenyAll` 限制及 `acp-client` 技能租约读边界。原生 Skill/Read 和会话外挂 stdio 在既有 Darwin 证据中可区分，Linux 运行证据独立。
- 上游 `WorkspaceMcpService` 分别构造工作区基线与会话 overlay，`session/new` 注入临时服务，`load/resume` 仅在重新物化时采用新服务集合，已有 live session 会沿用旧集，`fork` 忽略同请求的 MCP 变更。宿主使用 `acp/session.rs` 的服务配置、技能指纹与租约参与连接复用判定。Linux 发布前仍需验证同会话撤除后停止旧进程并重新物化、恢复只读当前绑定。当前四轮夹具创建独立会话，不能证明同会话撤权。
- 两种架构保持完整能力拒绝。固定上游的 `mcpCore/client-stdio.ts::mergeStdioEnv` 与 `os/backends/node-local/hostProcessService.ts` 继承父进程环境。宿主私有中继令牌经 `VW_KIMI_RELAY_TOKEN` 注入进程，原生 Shell 与静态或外挂 stdio 子进程均可继承。现有宿主 MCP 转发无法收紧供应商内部的本机 Shell 和静态服务进程边界。默认模式及 ACP 审批不能撤回已经由进程继承的令牌，`yolo` 还会减少 ACP 审批回调。Kimi 的历史 MCP 权限回调由工具标题关联，缺少任务 4 要求的每次调用执行前实参与租约绑定的证据。在具备受审的子进程隔离、完整逐调用授权与双架构回执前，继续保留 `drivers::capabilities` 的 Linux 对外门控及 `skill_mounts::kimi::verify_entry` 的平台限制。任务 49 对两种架构登记相同实现缺口。
- 任务 48 在 `acceptance_support/mod.rs` 按 Linux GNU x86_64/aarch64 注册 `platform_kimi_tests`。测试调用 `verify_acp_skill_coexistence.py --agent kimi`，用绝对路径固定真实 Kimi 制品，发真实网关聊天请求。夹具在实际模型请求验证协议、认证、选模、原生及外挂技能和 Read 附件、外挂 MCP 调用 ID 与真实随机返回值。批准一次产生一次服务审计，拒绝、未挂载和撤除对照零新增。旧路由会在 Linux model source 或技能平台门禁报错。仍缺原生静态 MCP、Shell 凭证隔离和同会话取消/恢复的可执行正例。

### 本轮静态验证与复跑

- 校验两种架构的发行摘要、ELF 动态依赖、Python AST、Rust 定向格式与差异空白。未运行 Cargo、Docker、网关或 Kimi。
- 后续在计划规定的各自非 root Linux Docker 容器中，将目标架构解包后的固定制品放至 `target/acp-skill-coexistence/kimi-artifact/<linux-x64 或 linux-arm64>/kimi` 并赋予执行权限，设置 `SHIJU_ACP_KIMI_GATEWAY_BIN` 为当前构建网关的绝对路径。测试使用 ignored，避免常规单测执行外部供应商制品。接线后命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_kimi_contract -- --ignored`，须实际匹配 1 项并取得 0 失败。当前没有测试数或退出码。

### 继续排查与 Linux 受管替代路径

- 两架构的官方原生制品是 Node SEA，现有 Kilocode 补丁处理的是 Bun ELF 模块表，不能复用其应用器。固定的同版 npm 归档提供可修改的 `dist/main.mjs`，适合沿 GLM 私有 npm 路线提交局部修复。归档 SRI、原入口摘要、补丁摘要和补丁后摘要见 [linux-managed-process.json](../../patches/acp-model-source/kimi/linux-managed-process.json)。本轮在临时解包副本使用 `git apply --unidiff-zero`，核对四层摘要及修改后 JavaScript 静态语法。现有普通入口、Apple Silicon 路径和产品门控保持。
- [受管进程补丁](../../patches/acp-model-source/kimi/linux-managed-process.patch)只在 Linux 且存在宿主启动引用时生效。它在本机进程后端合并环境后移除中继令牌和启动引用，包括没有传入环境覆写时。已识别的原生 Bash 只能走上游 `AcpProcessService` 的 `terminal/create`，能力未声明或宿主返回拒绝时失败，预留宿主按会话、当前轮次和确切命令检查的入口。这个边界覆盖审查的 Bash 与本机进程后端的直接环境继承，不能约束同 UID 进程读取父进程、原生 MCP 在父进程中的远端环境引用、或供应商其他本机写入后端。
- 任务 48 可暂存受管 npm 素材。用与普通 CLI 分离的私有前缀安装 `@moonshot-ai/kimi-code@2.5.0`，固定 Node `>=22.19.0`、核对归档和锁文件并使用 `--ignore-scripts --save-exact`。校验原入口、补丁及结果后以受审绝对 Node 路径执行 `dist/main.mjs acp`。发布前须将传递依赖图与实际 Node 身份纳入安装和启动验证，并在 `managed_runtime::{install_builtin_agent, reviewed_agent_command, verify_reviewed_agent_executable}`、`acp-client::spawn_child_at` 和技能入口使用同一验证。驱动仍只接收 `["acp"]`，由宿主添加固定 JS 入口；不能令普通 native 覆盖回退受管入口。Bash 接线必须启用 `client/protocol.rs` 的终端能力且在 `event_client.rs::create_terminal` 确认有当前轮次的显式宿主审批，禁止沿普通 `TerminalManager` 的工作区自动执行路径。SDK 适配需改写原生制品身份断言，使受管网关测试检查实际选中的已补丁 npm 路径。
- 宿主现有 `stop_incompatible_cached_clients` 会先关闭旧技能视图对应的进程，受管启动按请求独占，具备重新物化 overlay 的入口。尚无 Linux 证据证明撤除、取消和重新恢复，继续使用同会话 ID 的服务审计门槛。供应商项目/用户静态 MCP 在工具审批前连接并可启动 stdio；原有 ACP `requestPermission` 有带轮次的工具调用 ID，`ApprovalRequest` 和 `buildPermissionToolCallUpdate` 只传展示数据，不含可与执行绑定的原实参或服务来源。把先前的 `tool_call_update.rawInput` 当批准凭据无法解决执行端消费。现有 `mimo_mcp::prepare` 仅接受指定适配器且使用 Darwin `sandbox-exec`，执行租约只消费工具名和实参；普通会话 MCP 直传也没有这些约束。禁止项目 MCP 会删掉所要求的原生能力。
- 两架构仍属实现阻塞。完成原生静态或外挂 MCP 需要宿主批准静态服务启动、按作用域投影目录、在隔离执行器中按来源/同一调用 ID/实参消费一次性许可，以及供应商在审批与执行边界携带这些字段。完整原生写入及命令还需审查 ACP 文件系统的本地回落，并提供能隔离中继凭据和工作区写入的 Linux 执行边界。单纯过滤进程环境、启用终端能力或打开受管路由均无法覆盖这些行为。任务 49 应分两架构登记。普通 `cli_local` 可沿用已有入口，受管 npm 层和双架构原生安装清单可独立交付给任务 48。
- 新增定向测试源码 [linux-managed-process.test.mjs](../../patches/acp-model-source/kimi/linux-managed-process.test.mjs)，断言没有环境覆写和显式覆写两种本机路径都移除内部变量，保持其他环境，Bash 缺终端或终端拒绝时没有本机执行。后续用 `SHIJU_KIMI_PATCHED_ENTRY=/绝对路径/dist/main.mjs node --test patches/acp-model-source/kimi/linux-managed-process.test.mjs` 验证。当前只运行 `node --check` 和静态补丁核对，未运行此测试、Cargo、Docker 或 Kimi。
