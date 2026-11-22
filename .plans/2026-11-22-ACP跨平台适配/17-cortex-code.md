# 任务 17 适配 cortex-code

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 cortex-code 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/skill_mounts/cortex_code.rs`、`patches/acp-model-source/cortex-code`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_cortex_code_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/cortex-code.md`、`docs/dev/acp/skill-coexistence/cortex-code.md`。

## 上下文

审计基线版本为 1.2.46+175654.6f8561fd271f。MCP 现状｜new 成功但忽略会话声明；原生有 clientTools 扩展。模型来源现状｜原生有 --model，外部端点未证；受审 Mach-O 和私有模型；通用。原生工具现状｜空挂载仅 read/write/edit；登记 MCP 即替换官方目录并拒绝官方调用。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v3 接扩展、按服务过滤、会话重绑及单次审批），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_cortex_code_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_cortex_code_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 上游固定版制品及入口 | 受管运行 |
| --- | --- | --- |
| x86_64 | `coco-1.2.46+175654.6f8561fd271f-linux-amd64.tar.gz`，归档根内的 `cortex acp serve`，ELF 解释器 `/lib64/ld-linux-x86-64.so.2` | 安装、模型、技能、授权和工具验收均未运行。已生成 Linux SEA 受管引擎，缺等价隔离和原生 MCP/工具审批 |
| aarch64 | `coco-1.2.46+175654.6f8561fd271f-linux-arm64.tar.gz`，归档根内的 `cortex acp serve`，ELF 解释器 `/lib/ld-linux-aarch64.so.1` | 安装、模型、技能、授权和工具验收均未运行。已生成 Linux SEA 受管引擎，缺等价隔离和原生 MCP/工具审批 |

- 从 [Snowflake 固定版清单](https://sfc-repo.snowflakecomputing.com/cortex-code-cli/a4643c4278/1.2.46%2B175654.6f8561fd271f/manifest.json) 确认架构和版本，下载两个完整归档到临时目录核对 SHA-256，检查内部版本标记、归档根、ELF 架构和入口摘要。固定 URL、两个摘要和入口参数见 `patches/acp-model-source/cortex-code/linux-artifacts.json`。两边均为 Node SEA ELF，入口的最高 GLIBC 符号版本为 2.28，同时依赖系统 libstdc++、libgcc_s。供应商归档含对应架构的 SQLite、Koffi、keyring 和 fs-native-extensions 原生模块，完整安装应保留同级模块及随附助手，不抽取单个 ELF 作为完整运行时。两个 core 归档的下载摘要也匹配清单，同样分发二进制和原生模块，不能替代受管隔离。
- 固定版两个 ELF 内嵌脚本均为 31,443,739 字节，SHA-256 为 `b1fc56584cd5ed8e1f013980dd6049bf36f87eab9891184668216576997e39a7`。现有 `apply.py` 的 20 个 JS 替换锚点在两个脚本中各命中一次。原版存在私有端点变量、连接检查和 `clientTools`，因此可复用 JS 修改思路，端点变量本身未建立绕过供应商连接检查后的明确选模、技能目录和工具限制。现行二进制写入器依赖 Mach-O SEA 段、对齐空间及 codesign，不能用于 Linux ELF 的 SEA note。首轮尚未生成 Linux 受管补丁产物，续查已完成 ELF 引擎移植。
- 已审 Darwin v5 由 `model_source::drivers::cortex::configure` 设置选定模型及私有 Chat 回环，`skill_mounts::cortex_code::publish` 挂载选中技能，供应商原生扫描提供另一类技能。桥把显式 stdio 服务注册成 Cortex `clientTools`，按所属服务筛选，按会话重绑并关联调用 ID、名称和参数进行单次审批。受管 launcher 与桥的 stdio worker 分别依赖 `/usr/bin/sandbox-exec`。现有通用 Bubblewrap 包装共享宿主临时目录，且没有此适配器需要的指定回环端口和子进程执行限制，直接复用无法保留隔离合同。既有宿主逐调用中继也未给 Cortex 建立隔离 stdio 端和原生工具执行边界。Apple Silicon v5 及其摘要保持原样。
- 两架构保持当前受管能力拒绝。原生 `read/write/edit` 之外的目录及上游静态 MCP 缺 Linux 安全接入。两类技能、本机模型及外挂 MCP 依赖受管入口，均不能据原版 ELF 或 Darwin 证据发布。保留独立的普通 `cli_local` 发现契约 `cortex acp serve`，其 Linux 运行仍待回归。
- 任务 48 可将固定归档作为普通 CLI 安装的候选素材。现行 `config_acp_install::acp_install_supported` 和 `acp_agent_registry::managed_runtime::{installed_directory, reviewed_agent_command, verify_reviewed_agent_executable, install}` 不应把该素材识别为可受管入口。若后续实现普通 CLI 安装，按架构选归档、校验固定 SHA-256、以当前解包边界校验条目、保留同级文件并安装到单独的普通 CLI 目录，参数为 `acp serve`。受管安装需先补固定 ELF 补丁产物、启动器和双进程隔离。届时还须联动 `model_source::drivers::capabilities`、`skill_mounts::validate`、`request::do_stream_request_acp`、`AcpClient::managed_cortex` 及 spawn 校验，避免重新发现、恢复或覆盖命令穿过身份门控。任务 49 按两架构分别登记阻塞。
- 任务 48 在 `acceptance_support/mod.rs` 增加 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_cortex_code_tests;`。测试通过实际网关及已有模型和 MCP 夹具走四个会话，检查选中制品、Chat 协议、两类技能目录与正文、按调用 ID 续轮、单次批准一次落盘，以及拒绝和撤除轮零新增服务审计。当前可用上游归档目录作为红灯 fixture，现有网关应拒绝其受管身份或 Linux 门控。绿灯需将 fixture 换成完整受管目录。四轮使用独立会话，未覆盖获得审批后的同会话取消、恢复及撤除，也未覆盖原生写入的宿主审批或供应商静态 MCP；这些仍是完整适配的缺口。
- 本轮仅做归档与 ELF 检查、Python AST、JSON、目标 Rust 格式和 Git 空白检查。未安装、编译或执行测试。后续分别在两个 Linux GNU Docker 容器配置 `SHIJU_ACP_CORTEX_GATEWAY_BIN` 为当前构建的绝对网关路径，`ACP_CORTEX_REVIEWED_DIR` 为私有 fixture 目录，接线后执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_cortex_code_contract -- --ignored`。预期命中 1 项，当前无退出码或命中数。

## 续查交付

- `apply.py --linux x86_64|aarch64 SOURCE DESTINATION` 已能从固定供应商 ELF 生成 `cortex-local`。仅扩展最后的 SEA note，受现有只读映射和零填充空隙约束，保留内嵌资产和其余已加载内容。两个结果的摘要记为 `linux-artifacts.json` 中的 `local_sha256`。任务 48 可在现有安装暂存区先将 `cortex` 重命名为 `cortex-native`，再用 Python 3 执行上述命令，验证两个引擎摘要。此步骤仅生成引擎，受管 `cortex` 入口和发布仍须等隔离及审批接口齐备。原有两参数 Mach-O 命令及签名流程沿用。
- 已核对已生成 ELF 的 NOTE 大小、只读 LOAD 边界、原生资产及布局，提取的两份补后脚本通过 `node --check`。网关验收要求通过正向路径后同时记录原生和补后摘要。上游原包可用于现有门控的红灯。Python AST、JSON、Rust 格式及定向空白检查通过。未启动引擎，未执行 Cargo、测试或安装。
- 已复核通用 Linux 包装。Bubblewrap 只提供通用挂载及命名空间，当前仓库包装不建立专用回环代理，丢弃原命令环境、工作目录并共享宿主临时目录。Landlock 的 `wrap_command` 明确返回 Unsupported。Snowflake 的 Linux Bubblewrap/socat 路线只包裹供应商终端命令，文档还允许无法沙箱化的命令进入普通审批流程。固定发布清单亦没有 Linux cocobox 辅助制品。Docker 级的普通沙箱包装不提供单一模型中继出口及模型、外挂 MCP 间相互隔离的两个执行配置。任一路线都缺可接入的 Linux 双进程执行边界。[固定版发布清单](https://sfc-repo.snowflakecomputing.com/cortex-code-cli/a4643c4278/1.2.46%2B175654.6f8561fd271f/manifest.json) · [Snowflake Linux 终端沙箱](https://docs.snowflake.com/en/user-guide/cortex-code/sandbox)
- 固定版 `clientTools` 扩展可复用现有 per-session 注册和单次审批，但执行回调只携带工具名称和参数，桥由当前轮的名称和实参推断调用 ID。并发同名同参的真实调用及绑定撤销后无法借该回调证明原始执行身份。上游原生权限标准模式只对敏感动作发请求，文档描述的编辑 diff 为结果事件。现有 Chat 白名单只有 `read/write/edit` 和显式会话工具，缺供应商静态 MCP 的带身份调用、凭据交接与执行前宿主审批证据。终端 shell hook 还需要启动子进程，不能替代主进程和外围 MCP 的隔离。[Snowflake ACP 扩展与模式](https://docs.snowflake.com/en/user-guide/cortex-code/cortex-code-acp#extending-the-agent-with-editor-tools) · [Snowflake hooks](https://docs.snowflake.com/en/user-guide/cortex-code/extensibility#hooks)
- 状态保持已阻塞。两种 Linux 架构的 `cli_local` 可继续由任务 48 评估独立固定安装。受管模型和技能的共同输入为独立 Linux confinement helper 或等价受审启动边界，须只读挂载安装目录，按当前项目和私有状态授权文件，限定模型进程只能连接其指定回环中继，且为 MCP worker 提供独立无网络、只读工作区和进程清理边界。完整组合另需上游执行回调传递稳定调用身份，或在固定 JS 中实现等价的执行前关联，以及原生/静态 MCP 的可拦截执行协议。当前可交付补丁、摘要和验收入口均不开放生产能力。任务 48、49 继续按两架构登记上述在制品之外的缺口。
