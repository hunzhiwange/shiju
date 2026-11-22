# 任务 29 适配 junie

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 junie 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/junie.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/junie.rs`、`patches/acp-model-source/junie`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_junie_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/junie.md`、`docs/dev/acp/skill-coexistence/junie.md`。

## 上下文

审计基线版本为 4100.5（26.11.2）。MCP 现状｜原生接声明，受管 stdio 卡 posix_spawn；同模型端口 HTTP 可完整调用。模型来源现状｜原版 custom 模型入口；受审签名 app；通用，不公布 cli_local。原生工具现状｜模型目录沿原生；私有配置与沙箱限制扩展及进程；历史挂载 6 次目标与 4 次官方读取。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 核实 JetBrains 各架构可取得的发行物和合法安装路径，保留 Safe 权限与私有扩展限制。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_junie_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_junie_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

两份 JetBrains 26.11.2（4100.5）官方 ZIP 均已仅作静态读取，整包 SHA-256 匹配 JetBrains 发布资产元数据，ZIP CRC、ELF 架构、入口执行位、jpackage 配置和应用 JAR 摘要均匹配。固定值见 `patches/acp-model-source/junie/linux-artifacts.json`。入口均为 `junie-app/bin/junie --acp=true`，必须完整保留 `junie-app` 相对布局。已确认内置 Java 21.0.11，所审 JVM 核心引用 GLIBC_2.28，jpackage 启动库还依赖 libstdc++ 和 libgcc_s。运行镜像的其余系统库依赖待执行确认。发行包受 JetBrains 条款约束，由官方发布地址获取。 [固定发行版](https://github.com/JetBrains/junie/releases/tag/4100.5) · [发行资产元数据](https://api.github.com/repos/JetBrains/junie/releases/tags/4100.5) · [ACP 参数](https://junie.jetbrains.com/docs/junie-cli-acp.html)

| GNU 目标 | 发行物及解释器 | 受管能力判定 |
| --- | --- | --- |
| x86_64 | `junie-release-4100.5-linux-amd64.zip`，ELF machine 62，`/lib64/ld-linux-x86-64.so.2` | 阻塞，待等价 Linux 进程与 MCP 隔离 |
| ARM64 | `junie-release-4100.5-linux-aarch64.zip`，ELF machine 183，`/lib/ld-linux-aarch64.so.1` | 阻塞，待等价 Linux 进程与 MCP 隔离 |

- 任务 48 的安装输入为上述 ZIP 与固定参数。若接独立原版安装，须先验整包，安全解包到宿主私有目录并保留入口执行位，原版入口不得注册成已审 `session_model`。清单的单文件摘要用于识别入口、配置和应用 JAR，不能代替保护和验证完整 JRE 树。保留 `agent_registry.rs` 的既有 `junie` PATH 发现及参数，普通账户和模型路径保持既有行为。本轮仍保留原有安装和能力发布门控。
- 同标签 `junie.sh` 默认选择 EAP，且其 Linux 入口指向 `junie/bin/junie`。本次以已回读的固定正式版 ZIP 为安装输入。
- Linux 受管首个缺口在 `patches/acp-model-source/junie/launcher.py` 的签名 app 布局和 `sandbox-exec`。同端口 HTTP 桥可复用，但桥的 stdio 子进程仍经 `patches/acp-model-source/cortex-code/mcp_bridge.py::Stdio` 使用 Seatbelt。通用 Bubblewrap 默认隔断全部网络并绑定宿主 `/tmp`，不能同时提供唯一回环模型/MCP 入口、技能只读视图和禁止任意派生程序。上游文档描述的沙箱作用于终端和原生编辑，未覆盖 MCP、钩子及引擎直接执行，4100.5 两份 ZIP 也没有文档所述的 srt 配套入口。当前驱动在写入中继凭据或设置模型前拒绝 Linux，不能用 Safe 读回代替这两层隔离。 [JetBrains 沙箱范围](https://junie.jetbrains.com/docs/junie-cli.html)
- 任务 48 保留 `managed_runtime.rs`、`managed_runtime/install.py` 和 `contracts.json` 的 Apple Silicon v3 接线。Linux 维持 `model_source/drivers/mod.rs::capabilities` 和 `skill_mounts/mod.rs::validate` 的现有拒绝。完成等价隔离后，接 `model_source/drivers/junie.rs::configure` 的 Chat 路由和私有绑定、`request.rs` 的项目/会话技能投影及 `acp-client/src/client/session_control.rs` 的 Safe 与模型读回，沿用带工具名单、单次审批和撤销的桥接。两个 Linux 目标的原生工具、原生与外挂 MCP、项目和外挂技能、审批、拒绝、撤除及恢复均待该边界落地及真实验证。Apple Silicon 的启动器与合约文件保持原样。
- 任务 48 在 `acceptance_support/mod.rs` 注册 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_junie_tests;`。现有 `acp_platform_junie_contract` 断言固定目标、生产能力、Chat 协议选择及驱动在发布私有配置前拒绝。静态红灯位置为旧驱动先返回制品不可用，无法命中新隔离错误。正向网关行为还缺 Linux 受管入口，既有 `scripts/verify_acp_skill_coexistence.py --agent junie` 只在 Darwin ARM64 选制品，后续可复用其真实网关、模型续轮与独立 MCP 审计夹具。
- 双架构制品回读、测试源码的限定 rustfmt 和限定 `git diff --check` 已通过。计划限制本轮为静态验证。两个 Linux Docker 目标的 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_junie_contract` 均未执行，无运行测试数。
- Junie 专属变更已提交为 `ba7bc9e70`。

## 续查交付

- GNU x64 和 ARM64 的原版安装及会话组合待回归。新增 `managed_runtime/junie.rs` 及 `patches/acp-model-source/junie/install_linux_native.py`。从固定官方 HTTPS URL 下载，先验证整包摘要，再仅投影原版 `junie-app` 树，忽略外层自动更新 shim。入口、配置、JAR、JRE 等 129 个常规文件全部参与整树摘要，包含路径和执行位。宿主私有暂存后原子发布，发现和启动回读整树，拒绝路径重定向与被更改的配置。
- 任务 48 在 GNU x64/ARM64 的 `managed_runtime.rs` 声明 `#[path = "managed_runtime/junie.rs"] mod junie;`，将 Linux `junie` 加到 `linux_native_only`，并在既有固定原版分支的 `linux_cli_command`、`verify_fixed_linux_cli_launch` 和 `install` 分别调用 `junie::discover(&root(agent)?)`、`junie::verify_launch(&root(agent)?, Path::new(command), args)` 和 `junie::install().await`，保持本机 PATH 回退。原版安装需要 Python 3、curl、GNU 运行库。原版安装入口验证完成前保留现有对外安装门控。保持 `session_model`、技能挂载及外挂 MCP 门控，不从原版发现推断受管能力。Apple Silicon 仍走现有签名包路径。
- 新增 `acp_platform_junie_contract_native`，经生产安装、注册发现及 ACP 客户端在临时用户目录创建普通会话，审批策略为 DenyAll，并断言篡改 JAR 路径配置后固定启动校验拒绝。现有过滤器不带 `--include-ignored` 时覆盖隔离拒绝断言，后续获准且任务 48 接线后在两架构容器追加 `-- --include-ignored`，覆盖两个用例。无真实账号、原版模型调用或受管续轮的运行证据。
- 受管组合仍阻塞。已核对当前工作区新增的 `linux_native_only`、固定制品启动校验和新增的供应商专用 MCP 路由。这些设施可复用原版安装合同和挂载拒绝，未提供适用于 Junie JVM 本体及任意外挂 stdio 的同等隔离。直接改用 ACP 原生 stdio 会让 Junie 自行起进程，Safe 只能覆盖其协议审批，且已有同端口桥对外挂服务的筛选与撤销不能由原生声明替代。通用 Bubblewrap 的完全断网模式又无法访问模型桥。两个目标的受管 Chat、原生工具与私有扩展、原生及外挂 MCP、技能和授权恢复组合需要经过审查的 Linux 引擎/stdio 隔离入口，分别确保唯一回环模型/MCP 端口、只读技能、禁任意执行及外挂进程无网络，或上游提供等价受审能力。当前没有这种可接入的运行入口，继续保持失败关闭。
- 本轮重新读取及校验双架构官方 ZIP，并静态计算了整树摘要。未运行安装器、Junie、Cargo 或容器。Junie 专属续查变更已提交为 `56436bf9b`。
