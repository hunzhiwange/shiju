# 任务 43 适配 snow

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 snow 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/snow.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/snow.rs`、`patches/acp-model-source/snow`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_snow_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/snow.md`、`docs/dev/acp/skill-coexistence/snow.md`。

## 上下文

审计基线版本为 0.9.14。MCP 现状｜原版保存 stdio，prompt 工具收集忽略会话列表。模型来源现状｜原版私有配置可选模；驱动无 v3 摘要前置，原包可能重现断点；通用。原生工具现状｜普通保留原生；挂载追加且冲突失败；无通用名单投影。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v3 加发现、单次审批、结果入历史和清理，Chat 提示使用真实目录），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_snow_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_snow_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

- Linux GNU x64 和 ARM64 共用 [snow-ai 0.9.14 npm 归档](https://registry.npmjs.org/snow-ai/-/snow-ai-0.9.14.tgz)。主归档已按现行合约 SHA-512 验证，清单无 os/cpu 限制，入口为 `bundle/cli.mjs`，原始摘要为 `bcc0a6cf468a245729605bcf75fd762fd13cb48553d342b31121cafc3153d905`。上游原件忽略 ACP 会话 MCP，必须使用受管补丁。Apple Silicon 原 v4 摘要 `b3fb29de76fe1e89ab229b0954bca728589948940e2f203d0ca7ab56ea8702dd` 和无参数补丁行为保留。Linux 目录 `snow-0.9.14-model-source-v5` 使用 `apply.py <package-root> --linux`，从相同原件先应用 v4 再应用 `linux.json`，最终入口摘要 `e041f175238895b5cfb109af8da186de61770537b811d8e6414c7e1e2f6515eb`，配套 `bundle/shiju-mcp.mjs` 摘要 `a9618c48e45601211f3d2be62bed8d4aee7e12e6a2b3925174c61950eface66d`。
- 安装依赖为 Python 3、Node >=22、npm >=8.3。复用现有 npm 私有前缀、精确版本、归档完整性校验和 `--ignore-scripts`，Linux 追加精确 `@vscode/ripgrep@1.18.0`，不执行供应商 postinstall。x64 配对 `@vscode/ripgrep-linux-x64@1.18.0`，归档 SRI、ELF machine 62 和 `bin/rg` 摘要已核实。ARM64 配对 `@vscode/ripgrep-linux-arm64@1.18.0`，归档 SRI、ELF machine 183 和 `bin/rg` 摘要已核实。两组固定数据见 `linux.json`。任务 48 在安装时校验所选 helper 包的 lock SRI 及文件摘要，在发现和启动时校验入口、回环模块及同架构 helper，入口参数固定 `--acp`，使用受管目录的 `node_modules/.bin/snow`。可选的 sharp/canvas 与其余传递依赖未获得逐文件身份及运行验证。
- 复用 `model_source/drivers/snow.rs::configure` 的私有 `SNOW_CONFIG_DIR`、会话选模及 Chat、Responses、Anthropic、Gemini 请求方式。Chat 仍要求支持 temperature，仍使用真实工具目录。复用 `skill_mounts/snow.rs::configure` 的 `SHIJU_SNOW_SKILL_ROOT` 校验，原生内置、用户、项目技能与选中会话目录共存，原生同名优先。Linux 增量将原生 ACP 工具先交给宿主单次审批，原生确认 hook 只保留批准后的否决能力。重新生成过的工具参数需再申请。Linux 外挂只消费同一模型中继上的宿主回环目录，沿用同名冲突拒绝、单次审批及同 ID 结果入历史。取消检查覆盖目录请求、宿主审批和调用前，取消映射为 ACP cancelled。
- 任务 48 接线。注册 key `snow` 不变。`managed_runtime.rs` 在两个 Linux GNU 目标使用 v5 合约、嵌入 `snow/linux.json` 和 `snow/linux-mcp.mjs`，`install.py` 选择 `--linux` 和对应 helper；`reviewed_agent_command` 及 `verify_reviewed_agent_executable` 仅信任该 Linux 安装身份。普通 `cli_local` 保留本机 `snow --acp`。在 `request.rs`、`skill_mounts/mod.rs` 和 `acp-client` 的创建、恢复及进程选择入口对受管技能和会话 MCP 组合验证固定入口、参数、来源、有效宿主权限及工具开关。保留 Snow 的 transient approvals、ClientOnly、host DenyAll 和进程更替撤权。未知目标、未审制品、未获宿主投影的挂载及无有效审批均拒绝。对外 Linux 托管能力保持现有门控，直至解除下列阻塞并取得各架构运行证据。
- x64 与 ARM64 的会话外挂适配器消费端已具备。Linux `discoverProjectedSnowMcp` 仅接受同一模型回环 origin 的 `/shiju-vtcode-mcp/<service>` 请求私有凭证，拒绝原始 stdio、SSE、任意地址、模型令牌与名称冲突。工具别名为 `mcp__<service>__<tool>`，命中现有宿主受管工具识别。批准后的调用参数携带 `_meta.shiju.snow.toolCallId`，保留 ACP 原始调用 ID。无挂载时无需目录凭证。
- 任务 48 的具体协议缺口。在 `session-engine::acp_defaults` 和宿主 `mimo_mcp::prepare` 接通 `shiju.snow.toolPolicy`、每服务允许与显式拒绝名单、`mcp__<service>__<tool>` 和 Snow 的调用 ID 令牌消费；在 `RuntimeHost::prepare_stdio_mcp_launch` 仅为 Linux 受审 Snow 创建 projection，并绑定当前提示、请求和取消。现有宿主桥只在 Darwin arm64 创建 stdio 沙箱，现有授予只比对工具和参数，不消费 Snow 调用 ID。通用 Bubblewrap 缺该宿主桥需要的 workspace/scratch 与拒绝访问合同，Landlock 逐命令包装不可用。Nova 的 Linux 模块也只有适配器消费端。不能靠复用这些入口启用实际启动。
- 原生项目 MCP 仍通过 Snow 自身设置在发现阶段启动，原生 hook、团队与子代理也有供应商内执行边界。全能力组合继续受阻，需宿主提供受审 Linux stdio 隔离和调用 ID 授予消费，并将原生项目服务发现及供应商自动执行入口纳入等价权限边界。直接原生项目服务与原始会话 stdio 不得作为替代。GNU x64、ARM64 各自保留门控。纯选模、原生文件工具和技能候选已提供，可在仅配置受信任的测试工作区、完成固定入口接线后回归。
- 测试接线由任务 48 在 `acceptance_support/mod.rs` 按 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_snow_tests;` 注册。夹具从生产安装及默认入口核对制品，经网关正常聊天验证 Chat、原生与外挂技能、原生文件工具、独立的原生 MCP 与会话 MCP、宿主批准及同 ID 续轮，并校验外挂审计增量 `[0, 1, 0, 0]`。补充原生协议探针通过回环投影的协议替身验证原始 stdio 无进程启动、迟到批准下的取消终态及同 ID load 后零调用。网关路径仍使用真实宿主投影和独立 stdio 审计。旧 Linux 源码最早在生产安装选择不到固定入口；仅用 v4 还会因工作区预批跳过宿主请求以及取消返回 ACP 内部错误而不符合夹具预期。
- 归档与补丁锚点已核对。临时源码副本重建的 v4 与 v5 摘要匹配，均通过 `node --check`。Python AST、定向 rustfmt 和 Git 空白检查通过。获授权并完成接线后，在两个非 root GNU Linux Docker 目标分别提供当前源码构建的网关，设置 `SHIJU_ACP_SNOW_GATEWAY_BIN`，运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_snow_contract -- --ignored --nocapture`。预期恰好命中一项。未安装、编译或实际运行测试。Responses、Anthropic、Gemini 的 Linux 续轮、普通账号 `cli_local` 和真实模型另需目标证据。
- 本任务实现和验收源码提交为 `f10767c82f`。编号文件保留供任务 48 接线及任务 49 入账。
- 宿主回环消费和验收增量提交为 `baf23d565`。复建 Darwin v4 与 Linux v5 入口及配套模块，摘要、Node 语法、Python AST 和定向 Git 空白检查通过。未执行 Linux 测试。
