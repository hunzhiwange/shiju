# 任务 23 适配 gemini

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 gemini 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/gemini.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/gemini.rs`、`crates/libs/acp-access-policy/src/access_policy/gemini.rs`、`patches/acp-model-source/gemini`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_gemini_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/gemini.md`、`docs/dev/acp/skill-coexistence/gemini.md`。

## 上下文

审计基线版本为 0.66.0。MCP 现状｜原生先受目录信任拦截；获信后旧泛型 Other 在宿主审批处失败。模型来源现状｜原生有选模；受审、私有路由，管理员冲突拒绝；通用。原生工具现状｜默认共存；私有关目录是旧实验；v2 历史 57 次目标及 1 次官方主题调用。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v2 为受审挂载附审批元数据，绑定活动提示及服务工具，另隔离模型、历史及技能），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_gemini_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_gemini_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| 目标 | 固定制品 | 源码结论 |
| --- | --- | --- |
| Linux GNU x64 | `@google/gemini-cli@0.66.0`，v3 七文件摘要 | JS 入口及补丁适用。上游有可选 x64 PTY，候选安装省略可选依赖。 |
| Linux GNU ARM64 | 同一个 npm 归档、版本与摘要 | JS 入口及补丁适用。主包未声明该架构的 PTY 预构建包，候选安装省略可选依赖。 |

- 两架构使用 npm [固定归档](https://registry.npmjs.org/@google/gemini-cli/-/gemini-cli-0.66.0.tgz)，SRI `sha512-EAtsJ9/JxiphHlduzvhoCkb7I0LdOxyugB5NpUtVqKVGZIWjV3JmurvK89ZrvMbV3kwk5SdWINz1M5ZN/2K/Lw==`。包无 OS/CPU 限制、无必需 npm 依赖。Node >=20、npm、Python 3 和 patch 用于受管安装。Linux 候选使用 `--global=false --package-lock=true --ignore-scripts --omit=optional --save-exact @google/gemini-cli@0.66.0`。可选 keytar 和 PTY 均排除。上游 shell 源码有 child process 回退，交互 PTY 及原生命令审批未纳入候选验证。
- 任务 48 调整 `managed_runtime.rs::install` 与 `managed_runtime/install.py` 的 Linux GNU 两架构分支，复用 stable key `gemini`、`gemini-0.66.0-model-source-v3` 目录、`contracts.json`、`gemini.patch` 和 npm 锁 SRI 检查。用现有 `reviewed_agent_command` / `verify_reviewed_agent_executable` 核验七文件和 `node_modules/.bin/gemini` 至 `bundle/gemini.js` 的规范路径，并校验 Linux 不含可选原生依赖。启动、显式覆写及恢复应使用同一校验。入口参数固定为 `--experimental-acp`，`drivers::gemini::configure` 追加 `--shiju-model-source`。其 `supports_managed_linux_target()` 只判定候选目标，任务 48 接入 `drivers/mod.rs::capabilities` 及 `request.rs` 时仍应维持公开门控。通用 `cli_local` 保留现有发现和参数。
- 候选复用 `prepare_profile`、Gemini GenerateContent 中继、私有设置及历史，以及 `skill_mounts::gemini::configure` 的受审追加视图。信任继续由原生目录信任控制，安装器不得写信任条目。Apple Silicon 沿用原 v3 及现有安装参数。
- 任务 48 在 `acceptance_support/mod.rs` 以 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_gemini_tests;` 注册。提供当前源码的 Linux 网关 `SHIJU_ACP_GEMINI_GATEWAY_BIN` 和隔离的 Linux 受审安装 `GEMINI_COEXISTENCE_RUNTIME`。网关探针检查固定入口、实际 GenerateContent 路径、原生和外挂技能目录及读取、静态内容的同 ID 回传、外挂 MCP 单次允许、拒绝和撤除的调用审计。撤除轮使用新会话。旧网关在 `request.rs` 的 Linux `session_model` 门控处失败。
- 两架构的完整合同继续阻塞。v3 的 `newSessionConfig` 将原生配置和挂载服务合并，按名覆盖。宿主 `AcpEventClient::request_permission` 仅接受活动提示中的会话挂载服务名称，无法批准原生配置服务。上游 `settings.mcpServers.*.trust` 和 `settings.mcp.allowed` 可在策略层直接放行，v3 仅为上游已要求确认的调用附加元数据。任务 4 的执行租约尚未接进 Gemini 的实际执行入口。需要保持原生拒绝优先，区分原生和挂载服务、拒绝身份碰撞，并在执行前校验绑定服务、工具、实参及活动调用的宿主授权。现有探针未覆盖原生 MCP、同会话撤权、跨进程恢复和取消；HTTP/SSE 也未获得独立回执。任务 49 按架构登记限制。

## 本轮静态验证

- 核对 npm 元数据、归档 SRI、包名、版本和入口，检查七个原始 SHA-256，通过仓库补丁生成七个 v3 文件并比对 SHA-256；七文件 `node --check`、Python AST、定向 Rust 格式和 Git 差异检查通过。
- 未执行 Cargo、Docker、上游安装或供应商运行时。后续获准且完成接线后，在两架构各自容器运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_gemini_contract -- --ignored`。当前无红绿退出码或测试命中数。

## 续查与接线更新

- 两架构共用 Linux v4 修订（`gemini-0.66.0-model-source-v4`）。保留上节 npm SRI、安装依赖及启动参数。任务 48 验证 v3 全部摘要后，在同一 npm 包根目录 `patch --batch --forward -p1` 叠加 `gemini-cli-0.66.0-linux-mcp.patch`，补丁 SHA-256 为 `109a2e8534470465ce8e81f1818a67e132a7ae4eaa4241e1c4af26ac52664b4d`。运行时摘要用 `linux-hashes.json` 覆盖 v3 对应项并加入 sidecar，继续验证未改的 `bundle/gemini.js`。禁止受管 Linux 回退 v3。Apple Silicon 继续使用 v3；Linux 普通 CLI 入口不启用 v4 审批路径。
- v4 在 `newSessionConfig` 拒绝同名静态配置、管理端强制配置、旧式 serverCommand 和重复挂载。工具调用按本次 config 记录来源，扩展覆盖挂载时拒绝执行。保留供应商拒绝。其余标准 MCP 调用一律请求单次 ACP 宿主批准，载荷含 `shiju.gemini/mcp-v4`（origin、server、tool）及 `toolCall.rawInput`。三套发布 bundle 都在 MCP 实际发送前消费绑定到原 invocation、信号、服务、工具和实参的一次批准。取消、会话 dispose、不同调用对象、改参和重放均失效；`session/load` 重建来源集合。
- 任务 48 在 `AcpEventClient::request_permission` 接入 `acp_access_policy::access_policy::gemini::{MANAGED_MCP_METADATA, validate_managed_mcp_request}`。仅接受 Linux v4 已校验制品、本会话活动提示和已核验挂载集合。将新标记纳入受管工作流分支，调用现有显式宿主确认逻辑，强制 `DenyAll`、无交互、撤销、重复 ID 和过期回复失败关闭。原生来源必须由受审 bundle 提供，客户端拒绝把已挂载名称标成原生。确认界面呈现来源、服务、工具与 rawInput。挂载桥若走宿主中继，须传同一 ACP 调用 ID 并在中继消费任务 4 的租约；现有供应商 MCP 执行帧仅发 progressToken，暂无该身份字段。v4 使用进程内批准消费，旧共享宿主仍拒绝未知 Other 标记。
- 对两架构保留公开门控。绕过标准 `DiscoveredMCPToolInvocation` 的供应商浏览器 MCP 另有 `BrowserManager.callTool` 路径，内部输入拦截和工具调用缺少同一 ACP 身份。Gemini 原生命令和写操作也有供应商内部执行路径，仍缺全覆盖的宿主逐次执行合同。仅改标准 MCP 或调整供应商模式无法覆盖这些路径。完整合同保持已阻塞，所需输入为浏览器独立调用及原生命令/写操作的可审计执行协议，或任务方案指定的等价受控入口。任务 49 可分别记录两架构标准 MCP 的适配源码与余项，不能把它计为完整放行。
- 网关探针升级到 Linux v4，继续校验原生/外挂技能、外挂工具批准、拒绝及新会话撤除。独立 Node 测试检查来源、碰撞、取消、改参和重放；后续 Linux 执行入口为 `GEMINI_REVIEWED_PACKAGE=<v4 包根目录> node --test patches/acp-model-source/gemini/linux-mcp-approval.test.mjs`。原生 MCP、恢复及取消仍无网关端到端回执，HTTP/SSE 仍须单独验收。
- 静态复核通过。v3 上叠加 v4 后八个摘要匹配，八个 bundle 文件与 Node 测试源码通过语法检查，Python AST、定向 Rust 格式及 Git 空白检查通过。未执行测试、编译或安装。
