# 任务 33 适配 mimo-code

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 mimo-code 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/mimo_code.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/mimo_code.rs`、`patches/acp-model-source/mimo-code`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_mimo_code_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/mimo-code.md`、`docs/dev/acp/skill-coexistence/mimo-code.md`。

## 上下文

审计基线版本为 0.1.17。MCP 现状｜受管 stdio 子进程被挡，早期同端口发现仍须精确工具名；原包全链待查。模型来源现状｜受审 --pure、私有模型和沙箱；通用。原生工具现状｜普通/挂载固定保留 build/compose 的 read/edit/write 与 plan 的 read；挂载追加，仍按默认拒绝或显式审批执行。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（进程内 fetch 补丁；挂载时用同端口桥、服务名单和调用复核），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_mimo_code_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_mimo_code_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 普通原生制品与受管候选 | 阻塞 |
| --- | --- | --- |
| x86_64 | 上游 0.1.17 的 `mimocode-linux-x64.tar.gz`，入口 `mimo`，ELF 62，GNU loader `/lib64/ld-linux-x86-64.so.2`。受管候选由 Bun 1.3.14 x64 构建，使用其标准 x64 目标，要求 AVX2。 | 仍缺受审的 Linux 启动及 stdio 隔离、受管构建产物摘要、原生 MCP 和端到端证据。无 AVX2 机器需另核对 baseline 受管构建。 |
| aarch64 | 上游 0.1.17 的 `mimocode-linux-arm64.tar.gz`，入口 `mimo`，ELF 183，GNU loader `/lib/ld-linux-aarch64.so.1`。受管候选由 Bun 1.3.14 aarch64 构建。 | 仍缺受审的 Linux 启动及 stdio 隔离、受管构建产物摘要、原生 MCP 和端到端证据。 |

- [Linux 制品清单](../../patches/acp-model-source/mimo-code/linux.json) 记录上游原包及 ELF 摘要、固定源码提交及归档、`bun.lock`、Bun 工具依赖、受审补丁和逐目标构建入口。两份上游归档仅含 `mimo` 且字节匹配官方摘要。普通 `cli_local` 继续走 `agent_registry.rs` 中 `mimo acp` 本地优先及 `npx -y @mimo-ai/cli@0.1.17 acp` 回退。允许任务 48 使用清单辨识原生制品，原生身份只适用于普通路径。[上游 v0.1.17](https://github.com/XiaomiMiMo/MiMo-Code/releases/tag/v0.1.17)
- 固定版原包的 `cli/cmd/acp.ts` 用 `Server.listen` 建立 SDK 私有 HTTP 监听。沿用已受审的 `stdio-only.patch` 改为 `Server.Default().app.fetch`，并允许构建使用空模型快照。未来受管安装在对应 GNU 架构校验清单和安全解包，应用 `patch -p1`。在私有 HOME 下设置 `CI=true`、`MIMOCODE_VERSION=0.1.17`、`MIMOCODE_CHANNEL=latest`、`MIMOCODE_COMMIT_SHA=3704f8d` 和指向 `{}` 的 `MODELS_DEV_API_JSON`。在源码根运行固定 Bun 的 `install --registry https://registry.npmjs.org --frozen-lockfile --ignore-scripts`，再在 `packages/cli` 运行 `bun run script/build.ts --single --skip-install`。候选输出由清单的 `build_entry` 决定，生产安装需要 Python 3、curl、patch、GNU 运行库及锁定的 Bun 和 Linux 原生可选依赖。产物未构建，`build-receipt.json` 的自报摘要仅用于私有夹具的复制校验，生产启用前须固定受审 ELF 摘要并核对构建输入。
- 任务 48 的安装接线点为 `managed_runtime::install_builtin_agent` 和 `managed_runtime/install.py`。发现和显式路径验证使用 `installed_directory`、`reviewed_agent_command` 与 `verify_reviewed_agent_executable`。后续 Linux 受管目录为 `mimo-code-0.1.17-model-source-linux-v1`，固定绝对 `mimo` 和参数 `["acp"]`，受管启动由 `acp-client::spawn_child_at` 的 `managed_workspace_io` 分支追加 `--pure`。安装、发现、显式覆写、启动及恢复均须使用同一逐目标身份验证，不接受通用 native 收据作为 Linux 受审证明。Apple Silicon 使用现有 v1 目录、构建收据、沙箱、技能配置和协议。
- 模型继续复用 MiMo 驱动的私有 `MIMOCODE_CONFIG`、选中模型名、回环 Chat Completions 中继和工具白名单。Linux 驱动会在下发凭据前显式拒绝。`acp-client/src/client/process.rs` 的受管执行依靠 Seatbelt 限制进程、文件读写和模型端口。现有 Bubblewrap 和 Docker 默认隔离网络，阻断宿主回环，通用命令包装也没有每请求单端口例外和 MiMo 进程执行限制。不能用不受限的共享网络或裸启动代替。保留 `drivers::capabilities` 和 `skill_mounts::validate` 的门控。可复用的技能逻辑是 `skill_mounts::mimo_code::configure` 的受信 `skills.paths` 选集与原生 `.agents/skills` 目录，Linux 需把选集及链接目标加入等价的只读策略。
- 已审外挂路径是 `request.rs::do_stream_request_acp` → `RuntimeHost::prepare_stdio_mcp_launch` → `mimo_mcp::prepare`，按 `shiju.mimo.toolPolicy` 和当前调用复核目录，MCP 与模型各用独立回环凭证。该桥自身也只允许 Darwin arm64 并使用 Seatbelt 启动 stdio。后续需把 Linux 进程、文件、端口隔离接到启动器与桥，再接逐调用审批消费和撤权。受管私有配置只发布桥生成的 `mcp`，隔离项目和用户配置。原生静态 MCP 因此仍缺一个可区分来源且能约束审批的入口。两架构都未证明同一会话在取消、撤除、恢复后的权限和审计，任务 49 按阻塞登记。
- 测试复用已注册的独占文件 `model_source/drivers/mimo_code_tests.rs`，以 Linux GNU x86_64/aarch64 条件编译，任务 48 无须另注册测试模块。该测试启动当前生产网关，复用原生及外挂技能正文与附件、真实 stdio 审计及同调用 ID 续轮检查。四轮分别是普通、挂载获准、撤除、拒绝。拒绝允许 ACP 按原行为终止并提供匹配的失败调用 ID。旧路由会在网关门控或启动验证失败。四轮使用新会话，内建静态 MCP 和同会话恢复需等对应安全接线后增加正例。
- 静态验证覆盖源码与锁摘要、现有补丁对固定源码可应用、两份上游 GNU ELF 及两个目标 Bun 归档、JSON/Python 解析、独立 Rust 测试格式及限定 Git 差异。未运行安装器、上游程序、Cargo 或 Docker。后续在两个非 root GNU 容器设置 `SHIJU_ACP_MIMO_GATEWAY_BIN` 和 `ACP_MIMO_REVIEWED_DIR`，运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_mimo_code_contract -- --ignored`。预期每目标命中 1 项，接线前红灯、接线后零失败。暂未取得退出码或命中数。
