# 任务 19 适配 deepagents

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 deepagents 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/deepagents.rs`、`patches/acp-model-source/deepagents`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_deepagents_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/deepagents.md`、`docs/dev/acp/skill-coexistence/deepagents.md`。

## 上下文

审计基线版本为 0.1.42。MCP 现状｜v2 拒绝非空 new/load；固定原包缺会话接线。模型来源现状｜原版有 --model；普遍核验受审入口；Linux 可安装但挂载拒绝，Windows 不可装，能力通用。原生工具现状｜配置工具保持构建；普通/挂载实际仅审批后 read_file/write_file 及获选 MCP，其他本地回退拒绝。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 检查 Linux 受管安装与挂载拒绝的前因，核对 Python 运行时、入口及现有权限边界。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_deepagents_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_deepagents_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 源码与制品证据 | 能力限制 |
| --- | --- | --- |
| x86_64 | 同一 npm `deepagents-acp@0.1.42`、`deepagents@1.16.5`，Node/shebang 启动 v5 的 `dist/cli.js` | 指定模型和原生文件能力沿用受管实现。技能视图候选待 48 接线及 amd64 回归。外挂 MCP 缺隔离执行路径 |
| aarch64 | 同一组 npm 归档与 JS 入口，现有 56 包锁无 `os`、`cpu`、`libc` 或安装脚本字段 | 同一实现缺口。单独等待 arm64 容器回归 |

- 稳定 key 为 `deepagents`。已校验 npm [ACP 0.1.42](https://registry.npmjs.org/deepagents-acp/-/deepagents-acp-0.1.42.tgz) 和 [引擎 1.16.5](https://registry.npmjs.org/deepagents/-/deepagents-1.16.5.tgz) 两份归档的 SHA-512 与 `deepagents-lock.json` 一致。锁 SHA-256 为 `747124f753586b59e0af1229cad39b62790c258350ead8024f2f63cd386ae925`。对临时副本应用现有 `deepagents/apply.py` 后，六个文件 SHA-256 全部匹配 `contracts.json::deepagents.files`。固定入口 SHA-256 为 `55aace3a7b5eb077e8cb6314465f6fb965d8ee844d134e7f361bbca6f5396d4c`。
- 任务 48 安装与发现。两个目标复用 `acp_agent_registry::install_builtin_agent`、`managed_runtime::install`、`reviewed_agent_command` 与 `verify_reviewed_agent_executable`。需要 Python 3、Node ≥22（锁内最高 Node 最低版本要求，既有旁证 v24.22.3）及 npm。生产安装用固定锁执行 `npm ci --registry=https://registry.npmjs.org --ignore-scripts --prefer-offline --no-audit --no-fund --fetch-timeout=20000 --fetch-retries=2`，补丁阶段用同一 Python 解释器。受管目录 `deepagents-0.1.42-model-source-v5`，入口为其下绝对 `node_modules/.bin/deepagents-acp`，空参数且必须解析到 `node_modules/deepagents-acp/dist/cli.js`。当前 Linux `spawn_child_at` 在验证后解析该入口再启动。普通 `cli_local` 保留当前受管启动与原生选模。现有未经受管校验的本机或 npx 回退仍被启动校验拒绝。
- 任务 48 模型与技能接线。继续复用 `model_source::prepare_profile` 和 `drivers::deepagents::configure` 的私有中继。驱动要求工具模型及明确模型 ID，追加 `--model openai:<id> --workspace <path>`，发送 Chat Completions 并绑定现有撤权生命周期。独占的 `deepagents::supports_session_view_target()` 只判定 v5 技能视图候选目标。在 `skill_mounts/mod.rs::validate` 接入时，同时保留 `request.rs::do_stream_request_acp` 的 `session_model`、工具启用与非 `DenyAll` 条件、受管入口校验以及当前会话视图租约。现有 `event_client` 仅读取活动受管会话选中的技能根。v5 从工作区目录和 `shiju.skillMounts` 生成按来源区分的目录，以宿主文件 RPC 读取正文和附件。恢复原选集可重用，选集变化由宿主新建执行上下文，旧进程的 `session/load` 拒绝变化。Linux 对外门控待目标回执，Apple Silicon 原分支和 v5 文件保持原样。
- 两架构共有阻塞。v5 原生工具目录沿用供应商配置，守卫只让 `read_file`、`write_file` 走宿主文件 RPC，其他本地工具拒绝。固定上游无可独立挂载并逐调用授权的原生 MCP 入口。外挂 MCP 的 `mcpSelection` 拒绝 Linux，`mcp.py` 复用的 Cortex stdio 只通过 `/usr/bin/sandbox-exec` 执行。它限制网络、派生子进程、工作区敏感路径和写入。宿主 `mimo_mcp::prepare` 同样依赖 Darwin 沙箱。通用 Bubblewrap 包装将主机 `/tmp` 绑定到子进程，并无该工作区敏感路径及逐次派生限制，不能直接替代。需要保留这些边界的 Linux stdio 执行器、受管入口及宿主逐次授权接线，才能启用外挂 MCP。两架构均维持拒绝。HTTP/SSE 无受审实现。
- 验收入口已经通过 `model_source/drivers/mod.rs::deepagents_tests` 注册，无需新增父模块。测试使用实际生产网关和既有 DeepAgents 回环夹具，检查 v5 固定入口、所选模型、Chat Completions 路径、原生与挂载技能的正文/附件、实际批准后按同 ID 续轮、拒绝与撤除时 FIFO 审计零新增。四阶段使用独立会话，当前 Linux 在挂载阶段预期被宿主技能门控拒绝，放行技能后非空 MCP 还会在供应商选择检查被拒绝。同会话取消、恢复与撤权未取得 Linux 回执。原生写入也尚无 Linux 运行证据。
- 后续在非 root 的 `linux/amd64` 和 `linux/arm64` 容器分别设置 `SHIJU_ACP_DEEPAGENTS_GATEWAY_BIN` 为当前源码构建的 shiju 绝对路径，`ACP_COEXISTENCE_DEEPAGENTS_RUNTIME` 为容器内私有 v5 安装目录，再执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_deepagents_contract -- --ignored`。测试显式忽略以避免常规单测依赖外部安装，完整验收预期每架构命中一项且零失败。当前已核对归档和补丁产出、Rust 格式、Python AST 及定向 Git 差异。未执行安装、Cargo、测试或 Docker。

- 代码及验收入口提交为 `c135d1687`。

### 继续排查与 Linux v6 候选

- GNU x64 与 ARM64 共用独占的 `apply-linux.py`、`linux-mcp.mjs` 和 `linux-contract.json`。v6 复用 v5 的模型、技能、原生读写及审批守卫。在 Linux，非空 MCP 选集只接受当前模型回环 URL 同源的 `/shiju-vtcode-mcp/<service>` 和独立的投影令牌。拒绝原始 stdio、SSE、外部 HTTP、重定向与模型令牌复用。空选集仍走原路径。Apple Silicon 按原 v5 清单，不消费 v6。
- 选中工具在模型和 ACP 审批中使用 `mcp__<service>__<tool>`。调用先经现有提示轮守卫及宿主逐次审批，拒绝持久批准。通过后才向投影传送原参数以及 `params._meta["shiju.deepagents.toolCallId"]`。服务端错误、超时或取消不回退到本地子进程。仍以同调用 ID 向模型回传结果。独占 JS 断言覆盖选集边界、原 ID 转发及服务端拒绝。
- 任务 48 的安装接线输入。仅对 Linux GNU x64/ARM64 将版本目录设为 `deepagents-0.1.42-model-source-v6`。按前述锁安装，应用 v5 并校验其六个文件，再将 `apply-linux.py` 和 `linux-mcp.mjs` 作为同目录素材应用叠加层。安装、发现、启动和恢复统一校验 `linux-contract.json` 的七个产物摘要，入口位置与空参数契约不变。副本已实际应用两层补丁并核对摘要。v6 不需要 Python 子进程执行 MCP。
- 任务 48 的宿主接线输入。在 `request.rs` 的受管来源分支只给已审 v6 和显式选集调用 `Host::prepare_stdio_mcp_launch`。`RuntimeHost::prepare_stdio_mcp_launch` 需将已有 `mimo_mcp::Projection` 接到 DeepAgents，按 `shiju.deepagents.toolPolicy` 过滤静态目录，使用投影的独立 bearer 和上述工具名。仅转发宿主投影后的 HTTP 定义。宿主须在发放与消费时匹配同一启动租约、提示轮、原调用 ID、目录身份和完整实参。现有 `ManagedMcpApproval` 和 `VtcodeApproval::consume` 仅接收工具名、参数及有效性检查，尚未消费该 ID。丢失、重复、失效、来源不符或拒绝均保持失败关闭。恢复重新核验当前选集与授权，不复用旧投影。
- 仍为已阻塞。受管 `mimo_mcp::prepare` 只接受 Darwin arm64，固定依赖 `sandbox-exec`，并且未注册 DeepAgents。需共享的 Linux 隔离执行器及上述授权消费契约才能开放两架构的外挂 stdio。现有 Landlock 文件规则、通用 Bubblewrap 和直接 stdio/HTTP 均不能单独提供既有网络、敏感目录和子进程限制。固定供应商仍缺独立原生 MCP 与非文件工具的宿主执行协议。技能与模型候选、原生受管文件路径可独立接线，取得对应 Linux 运行证据前保留门控。普通 cli_local 与 Apple Silicon 路由沿用现状。
- 后续网关验收沿用上文命令，Linux 私有安装目录改为 v6，实际模型断言不变。MCP 由宿主投影并要求原调用回传；真实服务仍通过原有 FIFO 记录执行次数。独立 JS 源码测试入口为 `node --test patches/acp-model-source/deepagents/linux-mcp.test.mjs`。本轮只做定向 Rust 格式、Python AST、Node 语法、固定归档和两层补丁摘要、局部 Git 差异检查，未运行测试或安装运行时。

- Linux v6 候选及验收更新提交为 `aaa150023`。
