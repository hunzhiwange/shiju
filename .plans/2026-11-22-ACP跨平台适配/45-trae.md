# 任务 45 适配 trae

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 trae 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/trae.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/trae.rs`、`patches/acp-model-source/trae`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_trae_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/trae.md`、`docs/dev/acp/skill-coexistence/trae.md`。

## 上下文

审计基线版本为 0.121.3（72fe00ff）。MCP 现状｜受管本地组合可 stdio 发现及真实执行；官方无绑定入口缺完整轨迹。模型来源现状｜受审本地组合；通用，其他平台有官方入口。原生工具现状｜普通/挂载无额外目录裁剪，执行受原生审批与沙箱；三工具是试验转发器。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（私有模型及配置装配，沿用供应商工具和 ACP 循环），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_trae_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_trae_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

固定版本为 `0.121.3`，官方 Go 构建记录为 `72fe00ff52f56b0eb11a37bd561d4f10a4a44354`，且标记 `vcs.modified=true`，以归档及文件摘要固定制品身份。静态读取了两份归档、ELF 头和 Go 构建信息。归档各只有一个常规文件 `trae-cli`。两个二进制均为小端 ELF64，使用 Go 1.26.1、`CGO_ENABLED=0`，静态链接且没有解释器或共享库依赖。上游安装脚本确认 CDN 命名和 Linux 架构映射。各自原生启动及账户交互还需回归。[官方安装脚本](https://trae.cn/trae-cli/install.sh)

| Linux GNU 目标 | 官方归档 | 归档 SHA-256 | 原生入口 SHA-256 | ELF machine |
| --- | --- | --- | --- | --- |
| x86_64 | [linux_amd64](https://lf-cdn.trae.com.cn/obj/trae-com-cn/trae-cli/trae-cli_0.121.3_linux_amd64.tar.gz) | `87251d035f6b182adeecff7b5a5cca5ea729c3ef10c264c2561ac84b5c935126` | `9045da1f00a8387084510f7e63b088049da996236065b8352c6783339a3db16f` | 62 |
| aarch64 | [linux_arm64](https://lf-cdn.trae.com.cn/obj/trae-com-cn/trae-cli/trae-cli_0.121.3_linux_arm64.tar.gz) | `d1c3fe8ff0281d2db866615dea162e2831bcf7988b2e82adfbd56207a23dbdd9` | `2865b8330eb545f2132a4e5e49a75d54fb0e41d377269ec4ffc6dc78881e5abf` | 183 |

- **任务 48 原生接线。** 稳定 key 为 `trae`。`patches/acp-model-source/trae/linux.json` 是两架构制品合同。Linux GNU 分支在 `acp_agent_registry::install_builtin_agent` 的安装链路中，按本机架构下载到私有暂存区并运行 `python3 <assets>/trae/install_linux_native.py <archive> <stage-runtime>`，将同目录的 `linux.json` 随脚本写入暂存区。脚本要求目的目录不存在，校验归档摘要、唯一成员、二进制摘要和 ELF 架构，在暂存目录写入原样的 `traecli`。通过注册表现有暂存及原子发布机制落到 `acp-runtime/trae-0.121.3-linux-native-v1`，复用 Linux 上现有的 Python 3 和 HTTPS 下载入口。旧安装位置和用户命令由现有路径解析。固定安装的回读须按架构重新验证 `traecli` 摘要，交给 `agent_registry.rs::built_in_agent_command` 的 Linux 原生分支使用绝对入口和参数 `acp serve`，保留未安装时的原有 `traecli` 路径。原生安装不能进入 `reviewed_agent_command` 或 `verify_reviewed_agent_executable` 的 `session_model` 认证。Apple Silicon 继续使用原 `model-source-v1` 合同。
- **指定模型和隔离。** 原生 Linux 制品保留平台前置检查、用量报告及默认用户状态。现有受管组合使用 Mach-O 固定地址、Go ARM64 指令、Darwin 签名及 `sandbox-exec`，按绑定的精确回环 TCP 端口约束进程和后代。任务 45 通过源码确认这些逻辑及入口限制，也核对了两份 Linux ELF 保留的平台前置检查和用量报告符号。当前宿主通用 Linux 沙箱没有只允许指定回环中继的 TRAE 入口。Docker 的无网络模式也会切断宿主模型中继。新的官方 CLI 2.0 文档有 ACP 初始选模参数，无法证明这一固定 1.0 版在独立于供应商账号前置检查时能受限连接私有中继。缺少安全等价入口时，驱动在创建绑定或装入令牌前显式拒绝 Linux。继续保留 `drivers::capabilities` 的对外门控、`process.rs` 的权限参数检查及 `skill_mounts::trae` 的 Apple Silicon 技能判定。[官方 ACP 说明](https://docs.trae.cn/cli_agent-client-protocol)
- **工具、技能与审批。** 现有 Apple Silicon 证据覆盖 `Read`、用户级原生及宿主外挂技能、会话外挂 stdio MCP、批准和拒绝。项目级 `.trae/skills` 在私有启动目录语义下没有发现证据。受管绑定只装配选定模型，既有账号的原生 MCP 没有等价的会话隔离接入。两个 Linux 架构的独立本机模型、原生 MCP、外挂 MCP、两类技能、宿主审批及同会话取消、撤销和恢复均未验收。继续实施需要受审的逐架构私有组合、仅绑定中继可达且覆盖子进程的 Linux 路径，以及原生 MCP 的会话级授权来源。不能用未绑定的官方原生入口代替。
- **测试接线。** 复用 `model_source/drivers/trae.rs` 已注册的 `trae_tests.rs`，其网关验收只在 Linux GNU x86_64 和 aarch64 注册。测试通过仓库已有的 TRAE 供应商探针和真实网关，原方案在正式安装器的 Darwin 门控即失败。接通受管实现后，Linux 链路由同一安装器保留 `assets/trae.tar.gz` 并安装至探针目录 `trae-0.121.3-linux-model-source-v1`。探针核对实测归档摘要、原始 ELF 及启动参数。启动器和本地组合也须具有各架构合同的受审摘要，探针从对应目标的 `session_model_files` 读取全套摘要，当前未提供。通过安装后，探针要求模型端真实收到流式 `POST /v1/chat/completions`，按随机样本和调用 ID 校验两类技能及原生读取和外挂 MCP 续轮，批准后恰好执行一次，拒绝和移除轮零新增下游审计。四轮使用不同会话，尚需补充既有原生 MCP 及同会话恢复、撤权和取消对照。驱动已有定向单测同时检查 Linux 隔离拒绝、请求目录为空及没有中继令牌。

### 验证

- 双架构归档及 ELF 逐项比对。Python 语法解析、定向 Rust 格式和 Git 空白检查。未运行安装器、供应商进程、Cargo 或 Docker。
- 后续在 `linux/amd64` 和 `linux/arm64` 的非 root 容器中分别设定 `SHIJU_ACP_TRAE_GATEWAY_BIN` 为当前源码构建的网关绝对路径。测试入口 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_trae_contract -- --ignored`，每种架构预期恰好命中 1 项。当前没有退出码、命中数或绿灯记录。
