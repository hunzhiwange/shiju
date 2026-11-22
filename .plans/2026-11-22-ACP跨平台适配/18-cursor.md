# 任务 18 适配 cursor

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 cursor 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/cursor.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/cursor.rs`、`crates/libs/acp-access-policy/src/access_policy/cursor.rs`、`patches/acp-model-source/cursor`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_cursor_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/cursor.md`、`docs/dev/acp/skill-coexistence/cursor.md`。

## 上下文

审计基线版本为 2026.11.11-b912975。MCP 现状｜原包闭环未知；历史“原版”实为受管 v1，断于私有 prompt。模型来源现状｜受审 v5，私有 Chat；通用，其他平台有官方安装路线。原生工具现状｜私有路径普通仅 shell，挂载再加 MCP，冲突拒绝；v5 历史 9 次 MCP 和 1 次官方读取。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v2 修复模型目录到原生执行器及结果映射；v4 恢复共存，v5 有界续轮），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_cursor_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_cursor_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

### 固定 Linux 制品

- 供应商的 Unix 安装入口列出 Linux x64 和 ARM64，当前安装脚本版本会浮动。两种目标均固定至审计基线 `2026.11.11-b912975`。分别下载并静态审查 [Linux x64 官方归档](https://downloads.cursor.com/lab/2026.11.11-b912975/linux/x64/agent-cli-package.tar.gz) 和 [Linux ARM64 官方归档](https://downloads.cursor.com/lab/2026.11.11-b912975/linux/arm64/agent-cli-package.tar.gz)。[官方安装脚本](https://cursor.com/install)仅作为发行路径依据。
- x86_64 归档 176703341 字节，SHA-256 为 `be01f42ad334f81d05a9992fcf060a52cc746f52cc451e849eb652146a6e3c91`。内置 Node 是 ELF64 x86-64，GNU 加载器为 `/lib64/ld-linux-x86-64.so.2`。
- aarch64 归档 175384019 字节，SHA-256 为 `f068af64ed91b5304605976586f690579679f740a41f388091a6b3a2e776987c`。内置 Node 是 ELF64 AArch64，GNU 加载器为 `/lib/ld-linux-aarch64.so.1`。
- 两者都使用 `dist-package/cursor-agent acp`，入口为 Bash 包装器，引用归档同级 Node。依赖 Bash、常规 coreutils、glibc（静态符号最高为 2.28）、libstdc++ 与 libgcc_s。归档含所需的 Node 模块、本机扩展和沙箱辅助程序，无需安装外部 Node 或 npm 包。本机扩展实际加载、沙箱及系统权限仍待运行验证。归档内路径均在 `dist-package`，未见重复路径或链接。每架构 353 个原文件，加桥后验证 354 个文件。原始锚点、供应商归档和全部产物摘要见 [Linux 制品清单](../../patches/acp-model-source/cursor/linux-artifacts.json)。
- Linux `index.js` 分别校验原始摘要，共用 `6136.index.js`。受管补丁移植现有 v6，保留 v2 的原生执行及结果映射、v4 共存和 v5 的有界续轮，适配 Linux 打包的分块、导出及技能扫描。继续使用现有 Chat Completions 回环桥，实际端点为 `/v1/chat/completions`。宿主的 `drivers::cursor::configure` 写入 0600 私有配置，复用会话历史、内存凭据及 `acp --model <alias>`。Apple Silicon 使用原始单参数补丁调用，重新静态应用所得三个文件摘要与现行合约一致。普通 `cli_local` 仍是独立的供应商入口。

### 任务 48 接线

- 在 `acp_agent_registry::managed_runtime::{contract, installed_directory, reviewed_agent_command, verify_reviewed_agent_executable, install_builtin_agent}` 的 Cursor 分支只为 Linux GNU x86_64/aarch64 选择 `linux-artifacts.json` 的对应 `targets` 合约。使用现有固定版本的 `cursor-2026.11.11-b912975-model-source-v6` 目录；显式覆写路径必须解析到宿主受管入口。启动和恢复复用安装身份校验，检查普通文件、精确文件集合及目录边界，拒绝链接和额外运行模块。两 Linux 架构的包装器摘要相同，身份必须包括内置 Node 和全量文件摘要。保留 Darwin arm64 分支。
- 在安装素材中写入 `cursor/linux-artifacts.json`。共享 `managed_runtime/install.py` 按 `platform.machine()` 选择同一目标合约和固定 URL，先校验压缩包并验证归档成员，再复用解包和全量摘要检查。调用 `python3 <assets>/cursor/apply.py <runtime>/dist-package x86_64` 或以 `aarch64` 为末参数。Darwin 调用参数保持原样。继续由宿主以私有临时安装和原子发布管理目录。
- 生产驱动、`skill_mounts::cursor::configure` 和 `acp-client::client::process::spawn_child_at` 已复用选模、挂载视图、工作目录和运行时校验。已审源码有原生 shell/技能读取、原生与选集技能发现、会话 MCP 工具目录、工具同 ID 续轮和 ACP 拒绝状态。旧 Linux 路径首先在共享安装器的 Darwin 门控失败。任务 48 可以接入固定制品和验收入口，完整能力继续保持现有门控。
- 在 `acceptance_support/mod.rs` 下以 `cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))` 注册 `platform_cursor_tests`。拒绝未审入口的测试经过 `prepare_profile`。ignored 网关测试通过 Cursor 专属夹具调用生产安装器，从正常 `/chat/stream` 走四轮，核对制品身份、认证后的 Chat 请求、原生与选集技能的不同来源、同 ID 续轮、外挂 MCP 仅批准时有执行，以及拒绝或撤除后服务审计零新增。它用新会话控制撤除场景，不能覆盖同会话加载后的撤权。

### 两架构共通阻塞与验证

- 受管共享服务用空 `mcpLease` 配置替代供应商原生配置装载，避免依赖远端团队设置。ACP 传入的外挂 MCP 有执行路径，但私有模型缺供应商项目/原生 MCP 与外挂 MCP 的完整并行验证和来源隔离。普通 cli_local 的原生配置路径可保留。
- Darwin 历史只验证指定工具的 ACP 按次批准。Linux 仍需证明 Cursor 原生权限规则不会跳过当前宿主批准，以及供应商启动 stdio MCP 后任务 4 的执行授权可与当前提示和调用绑定。Linux 同会话撤除后加载、旧服务或审批失效、取消和进程回收仍无用例结果。任务 49 应对 x64、ARM64 分别记录该限制。
- 两 Linux 归档、ELF 架构、安装布局、补丁前后摘要与分块均已静态核对，生成 JS 经 `node --check`。Apple Silicon 对照通过补丁摘要复现。未执行供应商程序、安装、Cargo 或 Docker。后续两架构分别运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_cursor_contract`，预期命中 1 项。接线完成并具备 Linux 网关后二次设置 `SHIJU_ACP_CURSOR_GATEWAY_BIN` 为其绝对路径，追加 `-- --ignored` 跑网关契约，预期再命中 1 项。当前无退出码、命中数和 Linux 红绿运行结果。
