# 任务 26 适配 harn

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 harn 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/harn.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/harn.rs`、`patches/acp-model-source/harn`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_harn_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/harn.md`、`docs/dev/acp/skill-coexistence/harn.md`。

## 上下文

审计基线版本为 0.10.201。MCP 现状｜new 拒绝非空；替代配置受 read/mutate 权限上限拦截。模型来源现状｜原生 overlay；私有 pipeline；`session_model`；Darwin arm64。原生工具现状｜固定源码补丁绑定 ACP 会话的原生延迟技能注册表；只追加请求选集。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v5 沿用 v4 的 cwd 约束、stdio 沙箱、名单及单次审批），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_harn_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_harn_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

| 目标 | 发行物与源码 | 受管模式 |
| --- | --- | --- |
| Linux GNU x64 | 官方 v0.10.201 的 `harn-x86_64-unknown-linux-gnu.tar.xz`，SHA-256 见 `linux.json`。用同版固定源码和 v5 技能补丁构建，ELF machine 62。| 本机模型 Chat、宿主文件读取、原生与挂载技能的源码待接线回归。原生 MCP 与外挂 MCP 保持阻塞。 |
| Linux GNU ARM64 | 官方 v0.10.201 的 `harn-aarch64-unknown-linux-gnu.tar.xz`，SHA-256 见 `linux.json`。用同版固定源码和 v5 技能补丁构建，ELF machine 183。| 本机模型 Chat、宿主文件读取、原生与挂载技能的源码待接线回归。原生 MCP 与外挂 MCP 保持阻塞。 |

- [官方发行](https://github.com/burin-labs/harn/releases/tag/v0.10.201) 的两个归档 SHA-256 来自发行资产元数据。受管安装消费 [固定源码和产物清单](../../patches/acp-model-source/harn/linux.json)。已下载并核对固定源码摘要，对源码试应用 `skills.patch`。两个官方二进制归档仅用于识别原生发行，未下载和执行。既有 Darwin arm64 v5 的入口、文件摘要和校验路径沿用。
- 任务 48 在 `acp_agent_registry::managed_runtime` 的 `contract`、`installed_directory`、`install`、`verify` 和 `reviewed_agent_command` 接入该清单。仅允许 `target_os = "linux"`、`target_env = "gnu"` 及 x86_64 或 aarch64。目录为 `harn-0.10.201-model-source-v5-linux`，固定入口为其中的绝对 `harn`，参数为空。声明现有目录下的 `mod harn`，在既有摘要与构建收据校验之外调用 `harn::verify_linux_engine`，同时用于安装发布前、发现、显式覆盖验证及恢复。对失败的受管入口保持拒绝，保留注册表独立原生命令 `harn serve acp --session-environment isolated`。现有宿主仍把 Harn 的 `cli_local` 标为不支持，不能把 v5 launcher 当作原生模式。若后续接通原生 CLI，需按来源单独发现原生入口并保留原生账号配置。
- 安装复用 `managed_runtime/install.py` 中现有 Harn 分支的安全解包、v5 `skills.patch`、锁文件和许可证复制。Linux 原生构建依赖 Python 3、Rustup/Rust 1.99.1、GNU 链接器与 glibc 开发环境、`/usr/bin/patch`，按目标使用 `cargo +1.99.1 build --locked -j 1 --target <清单目标> -p harn-cli --bin harn`。从同一槽位的 `<target>/debug/harn` 安装为 `harn-native`，构建收据追加 `target`。暂存 `linux.json` 与 `linux-launcher.patch`，核对启动补丁摘要，在运行目录复制 launcher 为 `harn` 后执行 `/usr/bin/patch --batch --forward -p1 -i <linux-launcher.patch 的绝对路径>`，再核对三项固定文件摘要，最后按既有流程原子发布。
- 对无 MCP 的候选组合，继续使用 `model_source::drivers::harn::configure` 和 `skill_mounts::harn::configure`。受管入口只向请求私有回环发送 OpenAI Chat，保留原始模型选择和文件回调。Linux launcher 使原生账户查询使用请求私有 HOME，并在 `session/new/load/resume` 发现服务之前拒绝所有非空 ACP MCP 列表。任务 48 若接线做容器验证，只放行固定制品的无 MCP 候选组合，宿主应在启动或投影前对 Linux Harn MCP 明确拒绝。对外能力仍按公共门控等待运行证据。
- 两架构的外挂机制都有同一断点。v4/v5 的 `shiju_stdio.py::Stdio.start` 依赖 `/usr/bin/sandbox-exec`，禁止联网、派生进程和写工作区，只允许启动指定可执行文件。现有通用进程沙箱和任务 4 的执行租约未提供经 Harn 入口验证的等价组合。Harn 0.10.201 原生 ACP 拒绝非空列表，原生项目 MCP 的 read/mutate 入口被普通 ACP 权限拒绝。后续需要 Linux 上等价的只读 stdio 沙箱，以及保持原调用 ID、实参和提示轮的审批消费路径。原生 MCP 与会话外挂 MCP 的批准执行和恢复后隔离暂无 Linux 正例。宿主的通用 `fs/write_text_file` 在 `ApproveReads` 且没有 `confirm_write` 时允许工作区内写入，现有源码不能证明 Harn Linux 的每个写入经过逐次审批。任务 48 应保持公开门控，受限模式写入需补接按调用 ID 与提示轮绑定的宿主确认。任务 49 对两架构分别记录。
- 任务 48 在 `acceptance_support/mod.rs` 仅对 Linux GNU x64/ARM64 测试注册 `platform_harn_tests`。其生产网关探针要求 `SHIJU_ACP_HARN_GATEWAY_BIN` 和 `SHIJU_ACP_HARN_RUNTIME` 指向隔离容器里的待测网关和受管安装目录，使用 Python 3 和非 root 用户。旧路由在受管制品发现或平台门控处失败。探针校验发现的入口及摘要、Chat model 请求、仅选定的两种技能视图及同调用 ID 文件读取续轮，验证挂载被拒绝且服务未启动及撤除后仍可聊天。它使用新会话区分挂载状态，同会话恢复和 MCP 获准执行待补。
- 静态验证覆盖源码摘要、源码补丁、Linux launcher 补丁与产物摘要、Python 语法、Rust 格式解析和定向差异。未安装、编译或运行测试。获运行授权后分别在两架构容器运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_harn_contract -- --ignored --nocapture`，预期每架构运行一项。当前无红绿退出码或命中数。
