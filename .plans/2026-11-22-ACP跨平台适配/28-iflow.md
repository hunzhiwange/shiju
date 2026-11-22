# 任务 28 适配 iflow

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：待回归

来源任务：无

目的：完成 iflow 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/iflow.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/iflow.rs`、`crates/libs/acp-access-policy/src/access_policy/iflow.rs`、`patches/acp-model-source/iflow`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_iflow_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/iflow.md`、`docs/dev/acp/skill-coexistence/iflow.md`。

## 上下文

审计基线版本为 0.5.19。MCP 现状｜初次缺凭据；授权后原生合并静态/会话服务、目录 21/29/21，目标执行并回传。模型来源现状｜原生配置可选模；受审；通用。原生工具现状｜目录已证共存，官方同轮执行待证；三项限制来自验收转发器，原生免确认覆盖有限。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（固定 lock、私有模型及技能/历史隔离），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_iflow_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_iflow_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

- 两目标共用 npm `@iflow-ai/iflow-cli@0.5.19`、私有 `iflow-lock.json`、现有 `iflow/apply.py`。Darwin 保留 v2，Linux 批次使用 v3 叠加层。官方包和两个可选 PTY 归档已按锁文件 SRI 回读校验；上游两份 bundle 输入匹配原补丁摘要。目标内文件的路径及 SHA-256 见 [Linux 制品清单](../../patches/acp-model-source/iflow/linux-artifacts.json)。
- `x86_64-unknown-linux-gnu` 对应 npm `x64`，实查 `vendors/ripgrep/x64-linux/rg` 与 `@lydell/node-pty-linux-x64@1.1.0/pty.node` 均为 64 位小端 ELF、机器号 62。 `aarch64-unknown-linux-gnu` 对应 npm `arm64`，实查 `vendors/ripgrep/arm64-linux/rg` 与 `@lydell/node-pty-linux-arm64@1.1.0/pty.node` 为同格式、机器号 183。两个目标均无安装或加载证据。
- 任务 48 接在 `acp_agent_registry::install_builtin_agent` 的 `managed_runtime::install` 平台分支，以及 `reviewed_agent_command`／`verify_reviewed_agent_executable` 共用的 `verify`。Linux GNU 两目标由清单选文件，校验锁、主包、两份补后 bundle、原生依赖和入口，拒绝不匹配制品。安装用目标架构 Node.js（上游当前建议 22+，固定包声明 >=20）、npm 和 Python 3，沿用私有 `npm ci --ignore-scripts` 并保留匹配架构的可选依赖，无需执行上游 postinstall（非 Darwin 分支仅输出跳过提示）。`node_modules/.bin/iflow` 必须指向 `bundle/entry.js`。普通 `cli_local` 用 `--experimental-acp`，受管由现有驱动追加 `--shiju-model-source`；保留普通安装回退和 Darwin arm64 分支。
- 模型由 `model_source::drivers::capabilities` 和现有 `drivers::iflow::configure` 接线。使用拥有者回环 Chat Completions、锁定选中模型、临时 `IFLOW_HOME` 及按租户／项目／会话隔离的持久历史。技能复用 `skill_mounts::iflow::configure`，项目原生目录保持优先，显式空视图屏蔽旧外挂。权限沿用 `AcpClient::sync_iflow_access` 的 new/load/prompt 同步和 `session/request_permission`。受管能力仅在相应 Linux 目标实测后发布。原生默认模式仍可能自行执行部分读工具，完整原生 I/O 强制宿主审批暂无证据。
- 两目标均保留 stdio 验收候选和原生静态/会话合并路径。宿主 `session-engine/src/acp_defaults.rs` 当前遗漏 iFlow 的逐服务 `allowedTools`／`deniedTools`，ACP 类型也未把这两项直接翻译成原生 `includeTools`／`excludeTools`。Linux v3 在 new/load 的共用 session 配置中拒绝缺失、非空名单及非 stdio 挂载；任务 48 只向没有名单的选中服务投影明确空名单。HTTP/SSE 未声明能力。静态原生 MCP 同轮执行、同 ID load 后撤除、在途撤销及完整 native I/O 审批仍需目标验证；当前探针的撤除是新会话对照。
- 任务 48 在 `llm-runtime/src/acp/acceptance_support` 中注册 Linux 测试模块 `platform_iflow_tests`。测试用 `SHIJU_ACP_IFLOW_GATEWAY_BIN` 指向目标网关，`IFLOW_COEXISTENCE_RUNTIME` 指向隔离的受审 Linux v3 安装根。探针复制到临时网关配置，通过生产 `/chat/stream`、选模和审批 API 驱动真实 iFlow。回环仅接受认证后的 `/v1/chat/completions`，按调用 ID 核对原生/外挂技能与附件、MCP 结果和续轮，核对单次允许、单次拒绝及撤除阶段服务执行审计。旧 Linux 源码在 session_model 平台选择处拒绝。Linux 容器执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_iflow_contract -- --ignored`，附加 `--ignored` 显式运行依赖外部制品的验收，须命中 1 项且零失败。
- 静态核对通过：上游归档 SRI、两目标 ELF 文件摘要、锁文件及补后合约一致性、Python 语法和定向差异。未执行红绿命令，无退出码、命中数或 Linux 行为结论。

## 续审交付

- 两个 Linux GNU 架构共用 Linux v3 JS 叠加层。任务 48 从现有 `iflow/apply.py` 生成 v2 后，先校验 `linux-artifacts.json` 中的 `overlay_sha256`，再以包根运行 `iflow/apply-linux.py`，校验 v3 的 `patched_files` 和分架构 ELF 文件。已有 Linux 验收模块声明。待接 v3 私有安装、身份与能力分支，保留现有 Linux 公开门控、普通 CLI 路径及 Darwin v2。
- 向受管 iFlow 的 stdio `McpServer` 投影 `_meta["shiju.iflow.toolPolicy"] = {"allow": [], "deny": []}`，仅限宿主确认没有逐服务、会话或项目过滤限制的选中挂载。缺失、错误、非空名单、重名、可信直通和远端挂载均在供应商会话配置拒绝。v3 仍保留原生静态配置入口及独立名字；stdio 启动丢弃原有全量继承环境。
- 主工具分派在调用实际执行函数前使用当前调用 ID、原始参数及单次接受/拒绝选项，拒绝异常、取消、无匹配单次回复和覆盖调用 ID。宿主 `AcpEventClient::request_permission` 对 `shiju.iflow/tool-v3` 接 `access_policy::iflow::valid_linux_tool_approval`，仅接受已验证 Linux v3 进程的有效当前提示，并要求显式单次宿主批准；缺少批准函数、取消、过期及未知进程须返回拒绝。复用现有 active-prompt 所有权和单次调用 ID 防重；不得让通用自动批准路径吞掉该标识。验收已要求独立的 Skill 与原生读取回执。
- v3 隔离范围为主分派和 stdio 环境。命令执行、子代理、斜杠/美元前缀及隐式附件走不到同一受管边界，当前拒绝这些路径。静态 MCP 的发现可在提示之前启动原生进程；环境清理不提供进程隔离，目录接入仍需共享宿主租约与执行边界才能发布。两个架构均未验证 shell 或供应商钩子、静态 MCP、同会话恢复或即时撤权。
- 从固定 npm 归档在临时目录生成 v2 和 v3，两个补丁的上下文与清单摘要匹配，Node 仅进行语法解析。Rust 格式及 Python AST、定向差异通过。未运行 Cargo、网关或上游进程。按首轮记录运行 Docker ignored 验收。
