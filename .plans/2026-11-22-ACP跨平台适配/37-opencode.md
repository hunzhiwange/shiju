# 任务 37 适配 opencode

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 opencode 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/opencode.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/opencode.rs`、`crates/libs/acp-access-policy/src/access_policy/opencode.rs`、`patches/acp-model-source/opencode`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_opencode_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/opencode.md`、`docs/dev/acp/skill-coexistence/opencode.md`。

## 上下文

审计基线版本为 1.19.5。MCP 现状｜受管沿原生 stdio，回环 0/8/0 且真实结果续轮；同版原包对照缺。模型来源现状｜原版可用 opencode.json；受审重签产物；通用。原生工具现状｜官方工具继续沿原权限，验收转发器只缩服务；同轮官方目录与执行缺证。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（重签原生制品仅改 Skill 扫描，兼做宿主模型及状态隔离），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_opencode_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_opencode_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

- 已固定 OpenCode 1.19.5。Linux GNU x64 选用 npm `opencode-linux-x64-baseline@1.19.5`，Linux GNU ARM64 选用 `opencode-linux-arm64@1.19.5`。二者均为 `opencode-ai@1.19.5` 的固定可选依赖，原生文件位于 `bin/opencode`。已核对包元数据、压缩包 SRI、ELF 机器类型与 glibc 加载器。该版 x64 baseline 与标准包的 ELF 逐字节相同，选 baseline 入口以明确对应上游对无 AVX2 主机的选择，实际 CPU 兼容范围待目标机运行。安装用 Node.js、npm 和 Python 3，原生 ACP 入口使用目标系统的 glibc。SRI、原/补丁摘要和逐架构定长锚点见 [Linux 清单](../../patches/acp-model-source/opencode/linux.json)。[上游版本](https://github.com/anomalyco/opencode/releases/tag/v1.19.5)。
- 移植 Apple Silicon 已审扫描方式。保留原生项目/配置/内置技能，追加宿主验证的选集，同名原生优先，去除受管扫描中的任意配置路径和 URL。附件枚举用 Bun Glob，受限模式跳过 Git 子进程，无需为这两个路径放行 Git 或 rg。上游二进制与 Apple Silicon 文件未改。Linux 补丁验源、逐锚点和验成品三步，通过后才发布新 ELF。当前 Darwin v2 启动、签名与能力保持原状，普通 `cli_local` 继续使用原有本机发现。
- 任务 48 在 `acp_agent_registry::managed_runtime` 的 `contract`、`installed_directory`、`reviewed_agent_command`、`verify_reviewed_agent_executable` 接入 Linux 清单。仅选 `target_os=linux,target_env=gnu` 的 x86_64/aarch64。使用清单中的独立安装修订 `opencode-1.19.5-model-source-linux-v1` 和目标补丁摘要，保持现有包装包名、版本与 SRI 校验。显式覆写仍需相同目标制品校验，不接受原版 ELF、Darwin 回退或旧修订。生产验证宜流式校验大文件，并检查入口类型及执行位。安装、发现、启动及恢复均用同一制品选择。
- 在管理安装素材增加 `opencode/linux.json` 与 `opencode/apply-linux-skills.py`，在 `managed_runtime/install.py` 的 OpenCode 分支分离 GNU 与 Darwin。GNU 在临时安装目录中用 `npm install --global=false --package-lock=true --include=optional --ignore-scripts --save-exact`，固定 `opencode-ai@1.19.5` 和清单当前架构的原生包 `@1.19.5`。核对锁中两包的 SRI 和元数据，建 `node_modules/opencode-ai/reviewed/`，以 `python3 apply-linux-skills.py <原生包>/bin/opencode <包装包>/reviewed/opencode` 生成成品，再接 `node_modules/.bin/opencode -> ../opencode-ai/reviewed/opencode`。生产注册参数保持 `acp`。Darwin 继续使用同一 v2 安装与签名分支。
- 两架构的本机 Chat Completions 延用 `model_source::prepare_profile`、`drivers::opencode::configure` 的私有配置与按会话绑定的 XDG 目录、统一选模和回环令牌。两架构的受限运行仍缺 `request.rs` 与 `acp-client::client::opencode_sandbox` 两处 Darwin 门控之后的 Linux 等价执行边界。现有 `launcher.py` 依赖 Seatbelt，限制原生读写、进程与认证回环端口。现有 Landlock 后端只覆盖文件系统，不能单独兑现网络与进程契约。不得以普通原生 stdio、全访问或仅 ACP 提示替代。已有回环 MCP 桥可保留经批准的 stdio 下游、服务 allow/deny 和短时授权，但不约束 OpenCode 进程内的原生 I/O 或未受宿主投影的静态 MCP。两个 GNU 架构先保持门控。恢复还需验证相同会话切换选集时退役旧进程与桥授权，上游或补丁直接复用进程会保留旧服务。非 Chat 的原生协议配置存在，受限 launcher 目前只接受 Chat。原生静态 MCP、写/命令审批、同会话撤权后的恢复都缺 Linux 正向证据。
- 任务 48 在 `acceptance_support/mod.rs` 以 `cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))` 注册 `platform_opencode_tests`。过滤 `acp_platform_opencode_contract` 默认选中拒绝非受审入口断言。授权后，在两台对应 GNU 容器用隔离 `VIBEWINDOW_CONFIG_DIR` 准备上述受审安装，并设 `SHIJU_ACP_OPENCODE_GATEWAY_BIN` 指向当前源码的网关绝对路径，原定 Cargo 过滤命令追加 `-- --include-ignored --nocapture`，计划命中 3 项。制品断言走真实宿主发现、校验与协议配置，网关断言复用既有 OpenCode Chat 层和 stdio 审计。四轮独立会话断言原生/选定技能及附件读取、MCP 实际调用按 ID 续轮、拒绝/撤除后服务零新增。安装接好而 Linux 受限路由尚未接通时，网关首次提示应在模型请求及服务调用前报权限错误。后续验收需补原生静态 MCP 与同会话恢复轮。
- 静态检查已核对两架构归档与摘要、定长补丁及成品摘要、内嵌修改片段 JavaScript 语法、Python 语法、Rust 格式和定向差异。未安装或启动上游进程，未执行 Cargo、网关、Docker 或红绿测试，测试退出码和命中数尚未取得。

实现提交 `289c226395`。

## 续查与失败关闭

- 共享接线已在 `acceptance_support/linux_platform_tests.rs` 注册本任务的三个验收用例。OpenCode Linux 受审修订尚未在共享安装器、发现或启动校验中注册。普通 `OPENCODE_BIN`、用户目录、PATH 和 npm 回退仍可供现有原生账号入口使用。待任务 48 接通的受审安装信息见上文。
- 两架构均在适配器 `configure_native_scanner` 将非空视图写进环境前拒绝。受审 ELF 只能证明扫描器身份，不能约束 OpenCode 本机读取。此处之前只有制品校验，后续注册 Linux 摘要会使未隔离的投影进入该原生路径。空选集和 Apple Silicon 行为保持原样。将来应只在已构造并验证 Linux 受限进程的路径释放视图，与客户端和产品门控一起接通。现有独占用例现在经真实分派检查拒绝原因及两个环境变量均未写入。待授权定向命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_skill_mount_opencode`。
- 对照固定版 1.19.5。原生 read 使用 OpenCode 进程自己的文件系统，宿主 ACP 文件回调无法接管。原生 stdio 的 `connectLocal` 在连接期启动服务，并把供应商进程环境传给子进程。MCP 工具包装随后才调用 `ctx.ask`，传给权限层的元数据也不含原始实参。`registerMcpServers` 只提交新增配置，没有为空表执行移除。这解释了为何强制 `ask`、去掉持久批准或仅替换 ACP 权限回调，均无法兑现启动前隔离、按实参授权及同进程撤权。[原生读取](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/read.ts#L211) · [MCP 启动](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/mcp/index.ts#L316) · [工具审批](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/session/tools.ts#L376) · [ACP 挂载](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/acp/service.ts#L944)。
- 共享宿主已有 `stop_incompatible_cached_clients` 与受管请求收尾，能复用来退役旧进程并重新生成选集。OMP 桥已有按本轮绑定批准与参数、撤销后不分发的实现，也可让 OpenCode 从认证回环访问同一 stdio 下游。但 Cortex stdio 的实现仍硬编码 Seatbelt，MiMo 宿主投影同样只允许 Apple Silicon。通用 Bubblewrap 的断网配置无法保留指定宿主回环，改为共享网络又无法提供端口边界。现有 Landlock 后端只实现文件规则。使用命名空间另建双向回环代理或实现 Linux 执行器需要公共宿主/进程接线和隔离契约，超过本编号可写范围。
- 两个 GNU 目标保持已阻塞，原因是缺等价的宿主执行能力及原生 MCP 审批/路由协议，双架构制品已齐。最小后续依赖是受审的 Linux 运行和每个 stdio 服务启动边界，能保护工作区私密路径、限制写入和派生、仅暴露本轮回环；原生静态 MCP 如要启用，应进入同一宿主绑定的执行边界，或由上游提供等价能力。受影响组合包括受管 Chat 原生工具及技能、原生静态/外挂 MCP 和撤除后恢复。静态检查含本次 Rust 源文件格式与限定差异，未编译、启动或运行测试。

续查提交 `7383349655`。
