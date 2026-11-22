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

## 最终静态验收

- 复用任务 49 的 `df77249c6` 双架构静态取证及任务 48 的 `c5c3af94b1` 接线。Linux GNU x86_64、aarch64 都已接 v3 固定锁安装、对应 PTY/ripgrep 与入口身份校验及普通发现。宿主候选审批要求受审入口、当前提示、未消费调用 ID 和显式单次回复。网关探针已按宿主实际输出的 `Skill`、`read_file`、`read_probe` 名称和原始参数匹配 Linux 批准。Darwin 探针沿用供应商描述字段。
- 两目标完整受管均有实现缺口。`model_source::drivers::capabilities` 保持 Linux 公开门控；`AcpClient::spawn_child_at` 在 v3 身份复核后无条件拒绝受管进程，无法进入模型、工具或技能回合。供应商在 `newSessionConfig` 合并项目 MCP 后会通过初始化发现服务，可能先于有效提示及工具批准启动进程。v3 清理 stdio 环境和核对单次批准，仍缺启动前隔离与拥有者绑定的执行及撤销边界。需要任务 48 接入经审查的 Linux 启动与 MCP 隔离，并接通实际的名单投影和恢复撤除；保持现有拒绝，禁止绕过以取得绿灯。
- 已静态核对制品入口、测试注册、宿主审批标题、参数及回执断言。未执行编译、测试、安装或容器。获准后可先在两个目标分别运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p acp-agent-registry --lib acp_platform_registry_contract_uses_separate_iflow_and_amp_linux_revisions` 和 `python3 scripts/cargo_slot.py cargo test -j 1 -p acp-client --lib acp_platform_iflow_contract_requires_current_prompt_and_explicit_once`，各应命中 1 项。隔离接线完成后，使用 `SHIJU_ACP_IFLOW_GATEWAY_BIN`、`IFLOW_COEXISTENCE_RUNTIME` 和前述 ignored 网关命令验证完整路径。当前无运行退出码或命中数，保留编号文件。

## v4 启动前 MCP 隔离候选

- 两架构共用 [v4 制品清单](../../patches/acp-model-source/iflow/linux-mcp-artifacts.json)。先用现有 `apply.py` 生成固定 v2，再校验并应用 [v4 补丁](../../patches/acp-model-source/iflow/apply-linux-mcp.py)。已接生产的 v3 文件及摘要保持原样。任务 48 采用 v4 清单、脚本和修订目录，并同步固定目录测试。普通 CLI、Darwin 与现有 Linux 发布门控沿用。
- 启动构建、扩展合并和无投影恢复的可执行 MCP 目录为空。受管 Linux 单独读取项目 `.iflow/settings.json` 中 `mcpServers` 的名称，使用目录描述符、无跟随打开和 1 MiB 上限，仅展示项目来源名称。供应商配置中的命令、扩展 discovery、`mcpServerCommand`、tool 命令、dotenv 和 hooks 均不得构建受管执行目录。原生 stdin/stdio 和直连远端在 vendor transport 层拒绝。
- 任务 48 在 `drivers::iflow::configure` 结束验证后，把当前受管 profile 的 `SHIJU_IFLOW_MCP_RELAY`（精确 `http://127.0.0.1:<port>` 或 `http://[::1]:<port>`，无路径及尾斜杠）和 `SHIJU_IFLOW_MCP_TOKEN`（该 launch 的 scoped bearer）从宿主注入，供应商在启动时锁定。new/load 只传宿主选择的 `McpServer::Http`，URL 为同一 origin 的 `/shiju-vtcode-mcp/<name>`，ACP Authorization 数组内只含同一 bearer；`_meta["shiju.iflow.mcp-v4"]` 为 `project` 或 `mounted`。项目来源须命中原生目录，外挂同名拒绝。服务与工具名单先由宿主 relay 审定和过滤，适配器不接受直接 stdio 或由客户端字段覆盖名单。无 relay 时仍可读项目目录，执行目录为空。
- 受管 MCP 批准仍用 `shiju.iflow/tool-v3`，其中 relay 身份为 `mcp__<service>__<tool>`。`tools/call` 发送原始参数及 `params._meta.shijuCallId`，ID 沿用同一 ACP 批准帧。任务 4/48 从所选 relay catalog 绑定服务、参数、ID、提示和启动租约，在 stdio 执行前原子消费。服务器连接仅允许实际投影对象与固定的同源 URL，拒绝重定向。相同 ACP ID 的 load 在启动替代前删除旧会话、取消提示、使旧 transport 失效并停止旧 MCP manager，拒绝并行的新建/恢复。宿主撤除或轮次失效须同步撤销 relay 租约；选择变化需轮换或按新目录收紧宿主执行范围。
- 合并宿主 Linux 启动隔离和 stdio relay 前仍标记已阻塞，两个目标都必须保持现有生产拒绝。当前适配器可交付的 MCP 子集是宿主审定、原生项目来源或外挂来源、由 relay 执行的单次调用。运行前需任务 2/4 保护原生声明读取范围、进程启动与可撤销 relay，任务 48 在正确的网关路径接线。不能根据本轮静态证据宣布即时撤销、安装、协议兼容或完整原生工具行为通过。
- 复用上游 SRI 和架构证据，静态重新生成 v4，校验补丁上下文、摘要、Python AST、改动 Rust 文件格式以及 bundle JS 语法。未启动供应商入口，未跑测试、安装、编译或容器。后续两目标运行 `python3 patches/acp-model-source/iflow/linux_mcp_contract_test.py`（预期 4 项）及原 ignored 网关命令（令 `IFLOW_COEXISTENCE_RUNTIME` 指向 v4 安装根）。网关用例按 relay 审批名、两类技能、工具回执及拒绝/移除副作用验收。


## v4 生产接线复核

- 复核 `bb4129467` 和所含前置接线。Linux GNU x86_64、aarch64 已从正常网关请求进入受管选模和交互审批。宿主把经过审定的项目及会话服务投影到同一回合，按来源、工具、参数和调用 ID 消费许可。撤除任一引用可撤销旧 batch 并中止当前回合。生产 Host 夹具覆盖双来源独立执行、交叉许可及重放拒绝、项目撤权与会话撤除；此前“受管启动一律拒绝”和“待 MCP 接线”的结论已被该窄路径取代。
- iFlow 项目原生 Skill、工作区文件读取及外挂技能扫描使用已有受管路径。补齐新建历史持久化与 Linux 恢复校验，缺失、格式错误或 ID 不匹配的历史会报错。新增网关夹具在同一会话先调用原生及外挂技能并读取附件，撤除外挂后恢复两轮，分开确认历史连续、当前目录撤除及当轮批准和工具结果。历史中已经批准的可见内容保留，后续测试核验当前技能目录及新的访问。MCP 并存及撤权由生产 Host 夹具覆盖，暂时没有合并为一次真实供应商 MCP 网关回合的收据。
- 已接子集为固定 v4 制品、Chat Completions 选模、原生 Skill/读工具与选中技能、受审项目及会话 MCP、交互单次批准、原 ID load 和撤除。独立 shell/终端及子代理执行、斜杠自动化、隐式附件、供应商直连 MCP 与 headless 仍受显式限制；本轮未放开这些能力。普通 `cli_local` 沿用原生账户，Linux 会话外挂 MCP 仅开放受管模式。
- 已从锁定归档静态重建 v4，更新补丁和 bundle 摘要，核对补丁上下文、JS 构建文本语法、Python AST、Rust 格式及定向差异。未执行测试、安装、网关、编译或容器。双目标仍使用非 root、目标架构 Node.js/npm、Bubblewrap、dash、Python 3 和当前网关。获准后分别运行 `python3 patches/acp-model-source/iflow/linux_mcp_contract_test.py`（预期 5 项）、`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib acp_platform_iflow_contract_host_project_mcp_revision_and_removal -- --ignored`（预期 1 项）及传入 `SHIJU_ACP_IFLOW_GATEWAY_BIN` 的 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_iflow_contract_native_approval -- --ignored`（预期 1 项，含同会话技能恢复）。保留任务卡至实际运行验收完成。

## ARM64 失败复盘与复跑

- 旧 ARM64 网关报告的 `calls.native` 对应模型续轮中的同 ID 错误结果。只读诊断记录为 `{"error":"iFlow tool execution unavailable in this prompt"}`，宿主技能批准为零，未观察到实际 Skill 执行。固定 bundle 把用于读取 Markdown 的 `U4e` 原生 Skill 标为 `execute`。v4 只豁免经固定摘要验证的该类及其精确名称，使其通过原有单次审批。普通执行和 task 仍拒绝。缺失模型原始调用 ID、实参覆盖 `callId`、取消或失效会话继续拒绝。探针仅在验明返回内容或预期拒绝后记同 ID 回执。
- 旧 MCP 独立报告停在 `session/new` 的 scoped project 检查，未有模型工具批准或 relay 执行。该次夹具只建 `.vwacprc.json`，缺少供应商原生 `.iflow/settings.json` 项目目录。共享夹具 `a7d7efa468` 已为项目模式写入原生名称，session-mount 模式维持独立来源。v4 保留严格投影检查，单独报告来源与目录不一致。两模式仍待真实复跑确认。
- 用原始 bundle 固定输入摘要重建 v4 并更新 manifest，Node 只做语法解析，Python AST 和定向差异通过。本次未运行测试、安装、Cargo、网关或容器。由任务 49 串行复跑，确保网关嵌入本次 manifest，使用新安装根，避免旧 v4 目录命中旧摘要。
- ARM64 在任务 49 现有槽位执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_iflow_contract_native_approval -- --ignored`，令 `SHIJU_ACP_IFLOW_GATEWAY_BIN` 指向含本次 v4 摘要的网关。入口应命中 1 项，同时覆盖原生批准、拒绝、取消和同会话技能撤除。模型/MCP 分别用新根 `IFLOW_COEXISTENCE_RUNTIME` 执行 `python3 scripts/verify_acp_linux_model_mounts.py --agent iflow --gateway-bin <网关> --output <项目模式.json>`，再增加 `--session-mount` 输出至独立报告，核对同 ID 宿主批准、真实 MCP 挑战值、原会话恢复及撤除后的零调用。
