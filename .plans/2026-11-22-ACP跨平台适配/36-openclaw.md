# 任务 36 适配 openclaw

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 openclaw 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/openclaw.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/openclaw.rs`、`patches/acp-model-source/openclaw`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_openclaw_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/openclaw.md`、`docs/dev/acp/skill-coexistence/openclaw.md`。

## 上下文

审计基线版本为 2026.10.10。MCP 现状｜原版 ACP 拒绝非空声明，原生私有网关配置可完整执行和回传。模型来源现状｜原生 Provider 可选模；空挂载受审 v1 也可，挂载验证 v2；通用。原生工具现状｜普通和挂载沿原权限；挂载限完全访问、绝对 stdio，拒绝受限审批和 cli_local；无逐工具名单投影。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 现有挂载只在全访问时开放。实现逐次授权需接到专有网关的实际执行边界，不能以强制 full_access 达成。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_openclaw_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_openclaw_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 固定上游入口 | 原生依赖身份 |
| --- | --- | --- |
| x86_64 | OpenClaw 2026.10.10 v3，`node_modules/.bin/openclaw acp` → `openclaw.mjs` | Node 24.22.3 linux-x64，fs-safe 0.26.5、proc-safe 0.4.2、sqlite-vec 0.1.9 的 x64 ELF machine 62 |
| ARM64 | 同一 npm 归档和 v3 入口 | Node 24.22.3 linux-arm64，同版三项原生依赖的 ARM64 ELF machine 183 |

- 实际核对固定 npm 归档 SHA-512 与 `contracts.json` 一致，并在临时源码副本应用现有 v3 补丁。入口、技能扫描器和 stdio 回收传输的三个产出摘要均与合约一致。Node 上游要求 `>=24.16.0 <25 || >=26.1.0`。两个目标的 Node 归档官方 SHA-256，以及实际下载核验的六个 npm 平台归档 SRI、ELF 类型和文件 SHA-256，均登记在 `patches/acp-model-source/openclaw/linux.json`。三项受审原生依赖只是网关相关子集，不能作为传递依赖锁或权限证据。 [固定 npm 归档](https://registry.npmjs.org/openclaw/-/openclaw-2026.10.10.tgz) · [Node 24.22.3 校验表](https://nodejs.org/dist/v24.22.3/SHASUMS256.txt)
- 任务 48 安装输入。复用 `acp_agent_registry::install_builtin_agent` → `managed_runtime::install` → `managed_runtime/install.py` 的 `openclaw` v3 分支。依赖 Node 24.22.3、npm、Python 3 和对应架构 glibc。作用域为私有 stage，显式设 `npm_config_global=false`、`npm_config_package_lock=true`、`npm_config_include=optional`。现有参数为 `npm install --registry=https://registry.npmjs.org --ignore-scripts --prefer-offline --fetch-timeout=20000 --fetch-retries=2 --fetch-retry-mintimeout=1000 --fetch-retry-maxtimeout=5000 --no-audit --no-fund --save-exact openclaw@2026.10.10`。核对主包 lock SRI，依序用 `openclaw/apply.py` 应用原技能、MCP 回收与共存补丁，验证 v3 产物。Linux 验收增加对应锁项、包版本、ELF 与摘要校验。生产安装如采纳附加校验，将 `linux.json` 加到 `managed_runtime.rs::install` 的素材清单，并在 `install.py` 发布前对 Linux GNU 目标执行与 `scripts/acp_skill_coexistence/openclaw.py::install_linux` 相同的检查。要发布固定 Linux 能力，仍需固定传递依赖图和确认实际模块解析位置。
- 任务 48 注册及能力。稳定 key `openclaw`，`managed_runtime::reviewed_agent_command` 选绝对 `node_modules/.bin/openclaw`，固定参数 `["acp"]`。普通 `cli_local` 保留原生账号和发现逻辑，不写用户配置。普通 `session_model` 继续经 `model_source::prepare_profile` 的受审入口检查、`drivers::launch_route` 的 Chat 协议判定和 `drivers::openclaw::configure_with_mcp` 的独占 Gateway。模型走回环 `/v1/chat/completions` 与明确的模型别名，实际本机协议由宿主 relay 路由选择。保留全访问路径原有权限与选项拒绝条件。
- 两种 GNU 目标均缺等价受限执行边界。`skill_mounts/openclaw.rs::restricted_launch`、`drivers/openclaw.rs::configure_with_mcp` 及 `acp-client::OpenClawSandbox::command` 只接受 Darwin arm64。现有 `launcher.py` 依赖 Darwin 的文件、进程及精确环回限制，OMP 中继使用的 Cortex stdio 执行器也直接运行 `/usr/bin/sandbox-exec`。单用 Linux Landlock 的端口规则无法约束连接地址，也无法在允许整个工作区后排除其中的敏感子路径。需要 Linux Gateway、桥和下游 stdio 的等价隔离，以及同一会话和轮次的批准、拒绝、取消与回收语义。现行中继只审批其转发的 MCP 调用。上游 ACP 对 exec 审批是部分支持，原生读写和 Gateway 静态 MCP 缺同一宿主授权约束，ACP 还会拒绝非空 `mcpServers`。禁止为绕过限制授予全访问。Apple Silicon 和 Linux 对外门控保留。 [固定版 ACP 文档](https://github.com/openclaw/openclaw/blob/v2026.10.10/docs/cli/acp.md) · [Linux Landlock 接口](https://docs.kernel.org/userspace-api/landlock.html)
- 任务 48 测试接线。在 `acceptance_support/mod.rs` 以 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_openclaw_tests;` 注册。旧 Linux 路由在基线轮命中宿主桌面原生工具审批门控，预期没有模型续轮，挂载轮同样不能靠 ACP 透传 MCP。四轮复用真实网关、OpenClaw 私有安装、回环 Chat Completions 模型替身和只读 stdio 服务，检查两类技能来源和原生读调用、外挂 MCP 工具调用关联、一次授权与拒绝，以及服务审计增量 0/1/0/0。原生静态 MCP、原生写入/命令逐次授权、同一会话恢复和撤权仍缺 Linux 实现及验收；四轮的撤除为新会话对照。
- 后续获准后，在 `linux/amd64` 和 `linux/arm64` GNU 非 root 容器各自提供当前源码构建的网关绝对路径 `SHIJU_ACP_OPENCLAW_GATEWAY_BIN` 与固定 Node/npm，执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_openclaw_contract -- --ignored`。预期注册后恰好命中 1 项。当前仅核对归档、补丁、源文件及断言。未执行安装器、运行时、Cargo 或 Docker，退出码与测试数未取得。

## 续审交付

代码提交：`c1933e41b`。

- 两个 GNU 目标的既有全访问会话模型加 stdio MCP 组合可复用 v3 共存扫描器。驱动仅在成功验证 v3 且宿主已配置会话技能扫描器的情况下，将规范化工作区写入私有 Gateway 配置，并仅向该 Gateway 注入 `SHIJU_OPENCLAW_NATIVE_SKILL_WORKSPACE`。扫描器只追加工作区物理 `skills` 目录，会话投影仍由宿主生成。无挂载、旧受管版本、`cli_local` 与 Darwin 路线沿原选择规则。此组合属于既有授权范围，静态完成，待双架构回归。
- 现有 Gateway stdio 转发可实际启动调用，但 OpenClaw ACP 不消费会话工具名单。Linux `drivers::openclaw::project_mcp` 现在在生成 Gateway MCP 配置前拒绝带名单的挂载，即使已有全访问授权。无全访问、非交互、`cli_local` 和非 stdio 仍按既有规则拒绝。独立 `acp_platform_openclaw_contract_rejects_unenforceable_mcp_tool_lists` 覆盖名单拒绝与原投影行为。注册点在独占 driver 子模块，无需共享接线。任务 48 已在 `acceptance_support/linux_platform_tests.rs` 注册原 ignored 网关验收。
- 重新核对共享进程隔离与可复用原语。Codex 的已固定 bwrap 资源只有包装器身份，现有代码无可复用的 Linux Gateway + ACP + 受控 stdio 启动器、私有回环端口代理或工作区排除策略。OMP 的 HTTP 会话中继能保留调用审批，但其现有 Cortex stdio 在建立工具目录时已经执行 Darwin 沙箱。OpenClaw 的 Docker、Podman 等原生沙箱仅迁移工具执行，网关留在宿主，无法代替这两层进程隔离。原生 exec 部分审批也无法覆盖读取及 Gateway 静态 MCP。 [固定版 OpenClaw 沙箱](https://github.com/openclaw/openclaw/blob/v2026.10.10/docs/gateway/sandboxing.md) · [固定版 ACP 协议限制](https://github.com/openclaw/openclaw/blob/v2026.10.10/docs/cli/acp.md)
- 两架构受限会话及其逐次审批、原生静态 MCP、恢复与当前会话撤权继续阻塞。所需输入为可审计并固定到两架构的宿主隔离启动器及网络约束，以及供应商原生执行的调用身份/审批协议或等价受管执行桥。接线需将每次批准绑定到作用域、调用 ID 和参数，在 Gateway 实际分发时消费，并让取消/撤除与 Gateway、ACP、下游执行器共同失效。当前保持 `restricted_launch`、`OpenClawSandbox::command` 和桌面工具拒绝，未将全访问子集计作受限支持。
- 本轮定向差异与 Rust 解析检查通过。后续普通过滤命令命中新增 1 项非 ignored 投影测试，使用 `-- --ignored` 命中 1 项原真实网关验收。Linux Docker 两架构均未执行，无测试计数或运行证据。
