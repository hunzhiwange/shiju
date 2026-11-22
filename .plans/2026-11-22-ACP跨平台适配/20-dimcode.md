# 任务 20 适配 dimcode

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 dimcode 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/dimcode.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/dimcode.rs`、`patches/acp-model-source/dimcode`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_dimcode_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/dimcode.md`、`docs/dev/acp/skill-coexistence/dimcode.md`。

## 上下文

审计基线版本为 0.5.28。MCP 现状｜原生 HTTP/SSE 已走完；宿主前置拒绝 stdio，故原生 stdio 未知。模型来源现状｜原版私有 Provider 可选模；空挂载可走原版，远程挂载要求受审 session_model；通用。原生工具现状｜无挂载原生目录；有挂载即排除官方工具，无名单亦然；不支持逐服务名单时宿主报错。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（受管补丁在挂载时按服务正向范围裁剪定义并拒绝范围外执行），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_dimcode_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_dimcode_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 静态交付记录

- GNU x64 使用 [dimcode-linux-x64 0.5.28](https://registry.npmjs.org/dimcode-linux-x64/0.5.28)，ELF machine 62，解释器 `/lib64/ld-linux-x86-64.so.2`。GNU ARM64 使用 [dimcode-linux-arm64 0.5.28](https://registry.npmjs.org/dimcode-linux-arm64/0.5.28)，ELF machine 183，解释器 `/lib/ld-linux-aarch64.so.1`。两包各只有 `package.json` 与 `package/bin/dimcode`，npm 元数据没有安装脚本或依赖。ELF 声明依赖 glibc 的 libc、libpthread、libdl、libm 和对应加载器，链接版本最高为 GLIBC_2.17，实际最低运行环境仍待容器验证。修补和解包依赖 Python 3，下载沿用现有 HTTPS curl；受管 ELF 启动不依赖 Node、npm 或独立 Bun。
- 两份 tarball 与 npm SHA-512 相符，归档、原始 ELF 及受管 ELF 的逐架构 SHA-256 固定在 `patches/acp-model-source/dimcode/linux-artifacts.json`。原 v3 六处 bundle 替换都能在两个 Linux 输入唯一匹配并原位写入，内存计算的结果与清单一致，ELF 文件大小及头部保持一致，串用另一架构在写出前因原始摘要不符而拒绝。未运行 ELF。Darwin arm64 继续按原摘要走默认补丁参数和签名流程。旧 v1 正向裁剪曾屏蔽内置工具，现行 v3 已保留原生目录并将默认拒绝下的 HTTP/SSE 审批限定于当次 ACP 注册的服务 ID，沿用 v3。
- 任务 48 安装接线。在 `acp-agent-registry/src/managed_runtime.rs` 的契约选择处读入该 Linux 清单，仅在 Linux GNU x86_64/aarch64 选择对应行。让 `install` 和 `reviewed_agent_command`、`verify_reviewed_agent_executable` 共用该平台契约。将清单与 `dimcode/apply.py` 一并写入安装暂存 assets。让共享 `managed_runtime/install.py` 的 dimcode 分支沿用 HTTPS 下载、归档摘要校验、只提取指定普通 tar 成员和原子发布。Linux 补丁调用为 `python3 assets/dimcode/apply.py <raw> <staging-runtime>/dim --linux-arch <x86_64|aarch64>`，发布前以清单中的 `managed_sha256` 校验最终普通文件，Linux 无签名步骤。绝对受管入口为配置目录下 `acp-runtime/dimcode-0.5.28-model-source-v3/dim`，唯一参数 `acp`。普通 `dim acp`、固定 npx 回退及 npm 安装继续供 `cli_local` 使用。
- 任务 48 运行接线。沿用 `model_source/drivers/dimcode.rs::configure` 的私有 `DIMCODE_HOME`、精确 `session-selected/<alias>` 和 Chat Completions 路由。Linux 的受管 `session_model` 入口包括空挂载时也要验平台和可执行文件摘要，定向检查可接在 `acp-client/src/client/process.rs::spawn_child_at` 的 DimCode managed 分支。保持 `drivers::capabilities` 的对外 Linux 发布门控，待平台证据具备后再按已验安装发布。通过身份校验的受管运行分支在 `skill_mounts/mod.rs::validate` 接入 GNU 两目标，沿用 `request.rs`、`protocol.rs`、`prompt_loop.rs`、`event_client.rs` 的挂载检查和租约、当次 prompt 元数据、宿主技能读取根。保留 `dimcode_access.rs` 的只读同步、拒绝和工作区写入授权。沿用 `session-engine/src/acp_defaults.rs` 的逐服务过滤拒绝及 native stdio 的传输拒绝。
- 未解决限制。两个目标的 ACP `new/load/resume` 均用 `validateAcpMcpServers` 取客户端服务。供应商 `applyAcpMcpRuntime` 对该列表调用 `reloadServers`，而项目 `.mcp.json` 由另一条 `McpManager` 初始化路径读取。没有已审的合并、分别授权及撤除入口。保留普通 CLI 原生行为，两个 Linux 目标的原生 MCP 与会话外挂 MCP 共存继续阻塞，需供应商支持安全的按来源会话目录，或宿主先对已注册原生服务做独立隔离和授权后再投影。禁止自动导入项目配置或把其 stdio 变成隐式执行。现有 v3 只覆盖随 ACP 注册的 HTTP/SSE，逐服务工具名单与 stdio 继续显式拒绝。
- 任务 48 在 `acp/acceptance_support` 增加 `#[cfg(test)] mod platform_dimcode_tests;`。用例走真实网关，核对已安装受管入口的摘要、Chat Completions、原生工具读回、两类技能来源和带回执续轮。HTTP 四轮按基线、挂载、撤除、拒绝检查，实际服务调用计数为 0/1/0/0，批准和拒绝各对应一次。用例使用各阶段的新会话，尚无 Linux 同会话撤权和恢复证据。后续用 `crates/vw-acp/tests/fixtures/dimcode_coexistence.py <reviewed-dim>` 补 HTTP/SSE 的原生 ACP 加载、恢复、取消与撤除对照，再用网关入口复核同会话恢复。原生既有 MCP 无验收入口，不能从外挂结果推定支持。
- 本轮只做静态检查。预期红灯为原探针的 Darwin 平台门控，接通探针后的旧网关会拒绝 Linux 技能挂载或找不到对应的固定受管摘要。后续分别在 Linux amd64 和 arm64 GNU 容器，设置 `SHIJU_ACP_DIMCODE_GATEWAY_BIN` 为测试网关绝对路径、`ACP_DIMCODE_REVIEWED_BIN` 为同架构已安装固定制品的绝对路径，执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_dimcode_contract -- --include-ignored`。测试依赖受管制品和网关，标记 ignored，必须加参数才执行。命令、安装、上游进程及 Cargo 本轮均未运行，暂无退出码和测试数。
- 实现提交 `c0c769a3d`。修改路径的差异空白检查、Python AST 解析及新 Rust 文件的 rustfmt 解析通过。任务卡保留给任务 48 和 49。

## 复查交付记录

- 再核对固定 Linux v3 的上游输入。x64 归档及 ELF 均与清单摘要匹配。源码中 `reloadServersUnqueued` 以入参建立完整 desired map，并逐一断开不在 map 中的连接；同名传输变化会断开并重连。CLI headless 路径调用 `initializeMcpRuntime -> McpManager.initialize` 读取 `.mcp.json`，ACP 则直接调用 `applyAcpMcpRuntime`。保留旧连接会保留已撤除挂载，合并原始项目配置会在审批前建立未受宿主登记的连接，也无法区分同名覆盖和执行时的权属。
- 已逐一排查当前宿主入口。`session-engine::acp_defaults` 只向出站 ACP 发布宿主配置中获选的服务。`agent-app-services::mcp::acp_mount::resolve_mounts` 处理宿主数据库中有作用域和版本的记录，经 `agent-acp::protocol` 进入入站网关的会话池，无法为出站 DimCode 的 `.mcp.json` 生成权限或撤销记录。`vw-agent/.../model_source/mimo_mcp.rs::prepare` 只接收宿主选出的 stdio，固定为 Darwin arm64 的受审供应商和沙箱；其中的 HTTP 路由消费的也是这些 stdio 代理的调用授权。它不能接管两个 Linux 目标的原生配置任务，也不能把供应商本地授权转换为宿主租约。当前出站 DimCode 使用标准 ACP 同会话审批，v3 仅把本次 ACP 注册服务的默认拒绝转入审批；把原生服务名也塞进该集合仍缺独立的宿主授权、冲突检测及执行前撤权。
- Linux GNU `session_model` 驱动现在对工作区 `.mcp.json` 显式报错，在建立 `DIMCODE_HOME` 前停止，文件和链接都不读取。普通 `cli_local` 不走该驱动，Darwin 行为照旧。独占测试 `acp_platform_dimcode_contract_rejects_unmanaged_project_mcp` 校验拒绝和没有发布私有配置。原先只核对项目 provider 覆盖，遇到该文件可以继续，存在静默失去原生项目服务的风险。同一 Cargo 过滤将匹配驱动拒绝测试及网关测试，网关用例要等任务 48 接线后加 `--include-ignored` 执行。
- GNU x64 和 GNU ARM64 的供应商原生项目 MCP 与外挂 MCP 共存仍为已阻塞。解阻至少需要一项范围外能力，即上游提供可独立标识和撤销原生服务且在执行前可向宿主验证授权的 ACP 接口，或扩展共享宿主桥，把用户显式登记的供应商原生服务变成获授权的分源租约，并在实际工具执行前核销。普通原生 CLI、内置工具、两类技能与随 ACP 注册的 HTTP/SSE 保留现有交付边界，Linux 仍待 48 接线和容器回归。没有开放任何 Linux 发布门控。本轮源码和差异仅作静态检查，没有运行测试、Cargo 或上游二进制。
- 复查实现提交 `f97ba5cd5`。相关 Rust 文件的 rustfmt 解析及定向差异空白检查通过。任务 48 可独立接入首轮固定制品和验收，原生 MCP 共存能力继续关闭。
