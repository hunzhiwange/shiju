# 任务 21: 对接 aionrs

批次：【批次 21】 依赖批次 20

线性顺序：任务 21；本批只有此任务，与其他批次串行，公共注册与分派文件不可并行写入。

状态：未开始

目的：将 aionrs Rust CLI 通过最小 JSON Stream→ACP 桥接接入现有智能体目录，显示名称和稳定 backend key 均为 aionrs。

来源任务：用户明确追加 aionrs 接入；替代此前排除 aionrs 的范围约定，不使用 shiju-agent-cli 名称。它不同于任务 20 的 Pi/Node.js 包。

预计会话范围：一个引擎的外部进程协议桥接；复用现有 SessionBackend/serve_with_io 和 ACP 客户端，不引入 aionrs crate 到 workspace、不移植其引擎、不新增运行引擎枚举。先通过常驻进程、审批与取消的闭环验证，之后才开放目录。复杂可选能力保持明确支持边界。

## 验证阶段

Rust 编译资源和会话调度按 `.plans/references/rust.md` 的项目覆盖规则执行；下方必要定向单元测试仍在任务内完成。

- 开发阶段完成本任务和直接受影响行为的必要定向单元测试，优先复用现有测试；确认有效测试数及结果，不积压到最终回归。
- 基础检查限定受影响包和目标；同一目标已被定向测试成功编译时不重复 check。Rust 不能只编译单个 `.rs` 文件，测试过滤也可能编译整个 lib-test 目标。
- 本任务不运行整仓构建、全目标全特性 Clippy、全仓格式检查或全量测试。统一回归只补跨任务集成及公共检查，不机械重跑未受后续改动影响的已通过单元测试。
- 仅同仓库 Rust 重型编译串行排队、复用缓存；其它语言的普通定向单元测试不套用此限制。必要定向测试通过后按原完成规则收尾；仅具体集成待验收或客观外部阻塞保留待回归。

## 来源与可行性

- 官方仓库：https://github.com/iOfficeAI/aionrs 。版本发行：https://github.com/iOfficeAI/aionrs/releases 。协议：https://github.com/iOfficeAI/aionrs/blob/main/docs/json-stream-protocol.md 。
- 可复现基线 v0.2.11，revision `8e61a90329fa9f67c4fdf7e97fe02c24dba33f75`，来自参考项目 `packages/desktop/shiju/backend/manifest.json`；对应本地源码包 `packages/desktop/shiju/sources/aionrs-8e61a90329fa9f67c4fdf7e97fe02c24dba33f75.tar.gz`。
- 已读固定源码包 `crates/aion-cli/src/cli.rs`、`crates/aion-cli/Cargo.toml`、`docs/json-stream-protocol.md`。二进制名 aionrs，启动参数 `--json-stream`，使用独立 NDJSON 协议；不能伪造 `aionrs acp` 或将原生 stdout 直接交给 ACP JSON-RPC 客户端。
- 来源协议覆盖 ready、message、text_delta、stream_end、tool_request/tool_approve/tool_deny、cancel、set_mode、set_config、动态 MCP 及会话恢复。协议说明证明可设计桥接，不代表当前 ShiJu 已真实运行成功。
- 优先固定上游版本和独立子进程；参考项目补丁仅作调查材料，不能假设未打补丁的官方二进制具备补丁能力。若官方行为不足，明确记录证据和必要最小修补，不引入整套 AionCore。

## 文件

- 新建：`crates/vw-acp/src/bin/aionrs_acp.rs`，只负责参数、标准输入输出与桥接服务启动；`crates/vw-acp/Cargo.toml` 显式注册 `aionrs-acp` 二进制，`test = false`。
- 新建：`crates/vw-acp/src/aionrs/mod.rs`、`protocol.rs`、`process.rs`、`backend.rs`，分别负责组织、强类型 NDJSON、进程/读写和 SessionBackend 实现；对应测试独立在 `protocol_tests.rs`、`process_tests.rs`、`backend_tests.rs`。
- 修改：`crates/vw-acp/src/lib.rs`，导出该桥接二进制实际所需的最小入口；复用 `server::SessionBackend`、`BackendEvent`、`BackendPermissionRequest`、`serve_with_io`。
- 修改：`crates/vw-acp/src/agent_registry.rs`、`crates/vw-config-types/src/acp_install.rs` 和对应独立测试，注册桥接入口与固定安装契约；本地 aionrs 主 CLI 与桥接程序分别探测。
- 修改：`crates/vw-acp/src/install.rs`、`crates/vw-acp/src/managed_runtime.rs` 仅在确需受控来源构建/部署分支时修改；安装源码或脚本调用 Cargo 也遵守共享槽位规则。
- 新建：`crates/vw-acp/tests/onboarding_aionrs.rs`；修改：`crates/vw-agent/src/agent/catalog_tests.rs`，验证 `acp-aionrs` 官方预设、runtime backend 和显示名称。
- 按已验证能力修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs`，新增 `aionrs.rs`、`aionrs_tests.rs`；模型选择通过既有中继和会话私有环境配置。
- 新建证据：`docs/dev/evidence/acp-onboarding-aionrs.md`，记录固定版本、来源、协议帧形状、拒绝与取消、真实两轮任务、平台和未支持能力。

## 上下文

- 先读现有 Codex/Claude 启动、模型来源与审批链路，以及 vw-acp/src/server 下的后端 trait、事件和权限测试。aionrs 是新增外部引擎，通过 ACP 桥接复用产品链路，不替换当前 Rust 原生引擎。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## ACP 与内置智能体双重交付

- 必须同时交付 `aionrs` 的 ACP 运行链路和系统内置智能体 `acp-aionrs`，与现有 Codex、Claude Code 的内置预设一致；只添加命令/安装项不算完成。
- 内置目录通过 `crates/vw-agent/src/agent/catalog.rs` 的 `merged_agent_catalog` 派生，要求 `builtin=true`、`runtime=AgentRuntime::Acp { backend: "aionrs" }`，补齐显示名称、用途说明、推荐提示词和默认模型/权限/技能/MCP 继承。沿用现有默认启用规则及子代理限制，不自动扩大权限。
- 新增或修复必要的内置预设、配置持久化和会话选择连线：官方智能体目录可搜索，启用后出现在已启用目录及会话智能体选择器，可编辑保存默认项；重开设置/应用后保持配置；通过该预设新建会话确实启动 `aionrs`，不能落回 main、其他 ACP 或另一同名预设。
- 必读/按需修改：`crates/vw-agent/src/agent/catalog.rs`、`crates/vw-agent/src/config/schema/config_validate.rs`、`crates/vw-desktop/src/app/state/settings/agents.rs`、`crates/vw-desktop/src/app/message/settings/agents_profile.rs`；沿用已有配置和 UI 链路，不维护第二份手写目录。
- 新增失败测试：在 `crates/vw-agent/src/agent/catalog_tests.rs` 添加 `acp_onboarding_aionrs_builtin_preset_metadata`，覆盖内置标识、backend、名称/说明/推荐提示词及默认选择；在 `crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs` 添加 `acp_onboarding_aionrs_builtin_profile_persists_and_runs`，通过生产配置与流入口验证保存/重读、启用/停用、新会话实际路由和身份固定。
- 失败/通过验证命令均为 `cargo test -j 1 -p vw-agent --lib acp_onboarding_aionrs -- --test-threads=1`；先确认新增用例因内置预设缺失或路由不完整而失败，完成后至少上述 2 个新测试实际通过，零测试无效。只替代外部 CLI/模型服务边界，不 mock 目录或持久化实现。
- 完成门槛：ACP 适配与内置智能体任一缺失、配置保存失败或会话选择未打通，均不得标记 `已完成`；证据文档分别列出这两项交付结果。

## Cargo 共享槽位规则（最多 10 个）

本任务保留动态领取、继承复用、双缓存路径、所有权核验及退出清理能力；直接运行原生 Cargo，共享池最多 10 个槽位。

1. 同一仓库所有计划和任务共用 `target/rust-slot-1` 至 `target/rust-slot-10`；槽位编号仅为 1–10，不新增第 11 个槽位、不按任务编号固定分配、不创建任务专属缓存。
2. 从仓库根目录以原子 `mkdir target/rust-slot-locks/slot-N.lock` 领取空闲槽位；仅创建成功者持有。占用时尝试下一个，全部占用时等待，不删除其它活跃任务的锁。
3. 锁中记录真实持有进程 PID 以及任务/会话标识；持有进程须覆盖整个构建、验证及产物使用阶段，不能记录即将退出的临时 shell PID。遗留锁仅在核实持有进程及使用者都已退出后清理；归属或存活状态不明则保持占用。
4. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 都设置为同一槽位的绝对路径；原生 Cargo 和所有子进程继承，不通过配置、别名或 `--target-dir` 覆盖。二进制等产物也从该槽位定位。
5. 同阶段顺序命令和嵌套调用复用已持有槽位；继承前核验锁的持有 PID 属于当前进程或仍存活的祖先进程，路径与锁对应，不重复领取，不借用其它任务的槽位。无需包装脚本或额外测试运行器。
6. 正常、失败与取消时先结束自有 Cargo、rustc、链接器及仍使用槽位产物的子进程，确认退出后才删除自有占用记录、释放自有槽位锁；保留缓存。不删除其它任务的锁、进程或缓存。
7. 记录实际槽位、两个绝对缓存路径、确切验证命令、退出码和有效测试数；编译成功或零测试不能替代行为验收。
8. 10 是缓存容量，不是编译并发数。会话并发按 `.plans/references/rust.md` 的项目覆盖规则执行，槽位隔离不解除源码、依赖及共享资源约束；同仓库高内存编译仍串行排队，不利用空闲槽位绕过内存限制。

取消或超时后确认自有 Cargo/rustc、链接器及产物使用者退出，再释放槽位或重试；不能将已发信号视为已退出。顺序验证前项成功才继续，禁止失败后无条件启动后续重型检查。

## 测试计划

- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 行为：ACP 客户端可通过桥接控制一个长期存活的 aionrs 会话，流式输出归属正确、拒绝阻止副作用、取消后无越轮事件。
- 失败测试：`onboarding_aionrs.rs` 中 `acp_onboarding_aionrs_registry_contract`、`acp_onboarding_aionrs_two_turn_session`、`acp_onboarding_aionrs_denial_prevents_write`、`acp_onboarding_aionrs_cancel_keeps_turn_identity`；配置/目录测试也用 `acp_onboarding_aionrs` 前缀。
- 失败验证：`cargo test -j 1 -p vw-acp --test onboarding_aionrs -- --test-threads=1`；先让注册缺失断言失败，再逐项添加桥接行为测试，不能把未实现 API 的编译错误当红灯证据。
- 通过验证：上述命令至少 4 个有效测试；`cargo test -j 1 -p vw-acp --lib aionrs -- --test-threads=1`；`cargo test -j 1 -p vw-config-types --lib acp_onboarding_aionrs -- --test-threads=1`；`cargo test -j 1 -p vw-agent --lib acp_onboarding_aionrs -- --test-threads=1`。每个目标有效测试至少 1 个，不接受零测试。
- 模拟边界：生产桥接+真实 ACP 编解码对接可控的外部 NDJSON 子进程，覆盖分片、错误、审批与取消；模型网络使用受控本地 HTTP 服务。最后使用实际 aionrs 固定二进制和同一本地模型服务证明真实工具写文件与拒绝，无需借用用户第三方账号。

## 步骤

1. 阅读当前任务、只读 plan.md、差异清单、前一批次结果与现有工作区差异。重新核对固定源码和官方地址，分清 aionrs、AionCore、shiju-pi-cli，记录版本和许可证。
2. 先添加注册失败测试。审阅 aionrs 真实 NDJSON 输入循环、取消处理、配置合并和权限 allow-list；利用实际二进制与本地受控模型做最小两轮/审批/取消协议探针。若官方实现不能安全关闭权限旁路，先解决必要相邻问题并保留证据，不能先发布目录入口。
3. 实现最小桥接：一个宿主会话绑定一个子进程；等待 ready 后接收消息；映射文本、思考、工具状态、usage、终止和错误。严格检查 session/msg/call id，处理超时、异常帧、EOF 和 stderr；不把 stderr 原文或工具敏感输入落日志。
4. 权限从宿主当前授权解析；tool_request 转为一次性允许/拒绝，拒绝和取消发 tool_deny，关闭通道默认拒绝。核查原配置 allow-list、auto_approve、set_mode 可能绕过审批的路径；会话私有配置和启动策略不能继承更宽权限。默认不传 auto-approve、不设置 yolo；未经版本与协议回读证明不宣称 Native 完全访问。
5. 映射 cancel 到当前 aionrs 轮次并等终止，必要时结束子进程树；重复取消幂等，后续轮次不能收到旧帧。新/恢复会话检查租户、项目、工作目录和会话归属。恢复历史只能来自真实保存会话，不能把摘要注入伪装为完整恢复；不能安全实现的可选能力明确拒绝并不广告支持。
6. 安装与发现分开处理：本地发现 aionrs 主 CLI；桥接程序从当前发行包/受控安装目录解析，不写死开发机路径。官方发行页未确认预编译包，先核对实际资产；不存在则采用固定 revision 的受控源码构建，目标目录继承领取的 CARGO_TARGET_DIR，用户级部署保留许可证与摘要。禁止猜测 npm 包或任意下载 URL；没有 Rust 构建环境必须给出可操作错误，不能悄然安装其他引擎。
7. 注册稳定 key/display name `aionrs`，ACP 启动指向 `aionrs-acp`，由桥接启动 `aionrs --json-stream`；参数类型化构造。只在探针证明基础闭环安全可用后开放默认停用的官方预设，不新增 shiju-agent-cli 别名，不改现有原生引擎。
8. 按实际支持接入模型来源与 MCP：私有环境和既有模型中继传递所选 provider/model，密钥不进入 argv；不能修改用户全局 .aionrs.toml。MCP 只在 ready 后、首条 message 前注入，等待 mcp_ready；更新时明确处理协议窗口限制。技能只挂载选中隔离视图，未经证实不广告任意目录支持。被拒绝的已选能力必须在发 prompt 前报错。
9. 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
10. 审查注册仍保留原有与前 20 项，证据记录来源、版本、构建槽位、二进制摘要、确切命令、退出码、测试数量、真实请求/文件副作用、限制与平台覆盖。只在单一引擎端到端证据齐全后完成。

## 定向验证（任务内完成）

- 按测试计划执行所有聚焦命令；额外执行 `cargo test -j 1 -p vw-acp --lib agent_registry_tests -- --test-threads=1` 保护注册契约。
- 必须执行 公共检查由统一回归会话执行一次（本任务不运行）；修复本次错误，既有无关失败保留上下文；不执行全量测试、fmt check、wasm check 或 Makefile。
- 实际 aionrs + 桥接 + ShiJu ACP 客户端证明 ready、首轮/第二轮、真实写入、拒绝无写入、取消与进程退出清理；协议帧映射测试成功不能替代此证据。
- 安全证据包括预先配置的 allow-list/auto-approve 不能越过宿主拒绝，异常审批 ID 不被接受，密钥不进入 argv/日志/持久化；未证实前不宣称完全访问支持。
- 记录安装/构建、二进制定位、Unix/Windows 契约测试；没有实际跨平台运行就保留明确待回归项，不把本机编译当全平台可用。
- 最终证据包含实际槽位、绝对 CARGO_TARGET_DIR、确切命令、退出码和有效测试数，以及真实受控模型和文件副作用的脱敏回执。

## 完成

- 任务收尾：必要定向验证通过并审查后按原完成规则处理；只有明确集成待验收或客观外部阻塞时保留待回归，不能把必要单元测试统一延后。

- 标记 `已完成` 前须证明实际 aionrs 可通过现有产品入口完成上述安全会话闭环；可选能力上游不支持则证明明确拒绝。
- 先调查并解决必要依赖；仍缺关键外部条件则只在本文件标记 `已阻塞`，记录已尝试路径和所需输入。只有无关既有问题阻止完成验证时可标记 `已完成，待回归`，不得伪称接入通过。
- 有实际变更则使用 `$git-commit` 只提交本任务，排除其他任务改动。成功后报告证据并删除本文件；阻塞/待回归保留。只有任务文件状态或删除不单独提交；plan.md 只读。
