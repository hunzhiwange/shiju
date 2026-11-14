# 任务 37: 提供诊断启动API

批次：【批次 6】 依赖：批次 3 的任务 6、28、批次 4 的任务 36、批次 5 的任务 31、34。同批实施与资源规则见 prompt.md。

状态：已完成，待回归

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：每类诊断独立从已授权配置创建任务，越权不会触发网络或进程。

来源任务：`.plans/2026-11-05-CC能力完整融合/9-故障转移连通检测与诊断.md`；具体来源：commands/mcp.rs start_mcp_connectivity_checks；stream_check_provider；test_api_endpoints。来源仓库根目录为 `/Users/xiangmin/worker/code/cc-switch/`，不要修改上游。

预计会话范围：只交付“提供诊断启动API”这一项，通常一个聚焦会话；不执行下一任务、不展开整个母能力。若实际工作需要独立设计或第二个验证故事，记录本任务阻塞与证据，先确认拆分，不越界承接前置计划。

## 验证阶段

Rust 编译资源和会话调度按 `.plans/references/rust.md` 的项目覆盖规则执行；下方必要定向单元测试仍在任务内完成。

- 开发阶段完成本任务和直接受影响行为的必要定向单元测试，优先复用现有测试；确认有效测试数及结果，不积压到最终回归。
- 基础检查限定受影响包和目标；同一目标已被定向测试成功编译时不重复 check。Rust 不能只编译单个 `.rs` 文件，测试过滤也可能编译整个 lib-test 目标。
- 本任务不运行整仓构建、全目标全特性 Clippy、全仓格式检查或全量测试。统一回归只补跨任务集成及公共检查，不机械重跑未受后续改动影响的已通过单元测试。
- 仅同仓库 Rust 重型编译串行排队、复用缓存；其它语言的普通定向单元测试不套用此限制。必要定向测试通过后按原完成规则收尾；仅具体集成待验收或客观外部阻塞保留待回归。

## 文件

- 新建或修改（计划落点，当前不存在者由本任务新建）：`crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics.rs`。
- 独立测试文件：`crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics_tests.rs`；新增用例：`diagnostic_api_start_authorizes_before_execution`。
- 仅允许必要的直接父模块注册；API任务另修改 `crates/vw-agent/src/gateway/bootstrap.rs` 与 `api/handlers/mod.rs`；SDK任务另注册 `client/mod.rs`。已有等价模块则就地接入并在本文件记录真实路径，不复制第二实现。

## 上下文

- 先读根 AGENTS.md、`skills/rustcodegraph/SKILL.md` 和本计划 `来源映射.md`，查真实调用者及相邻测试。
- 实施前置：按任务 1–3 的合同核实本任务使用的真实产物。诊断使用已交付的授权与执行端，故障切换和运行时联动使用已交付的常驻路由接口。缺少本任务必需依赖则记录已阻塞，不把整个前置能力塞入本会话。
- 仅macOS验收；不要求其他系统实机、交叉编译或wasm。实机与原生窗口由用户人工验收，仅记录简短文字结果。
- 新模块与API名称是本计划明确拟新增的设计；下列命令是测试注册完成后的验证契约，不声称当前可运行。

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

- 行为：每类诊断独立从已授权配置创建任务，越权不会触发网络或进程。
- 单元测试：先在 `crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics_tests.rs` 新增 `diagnostic_api_start_authorizes_before_execution`，显式挂到现有测试模块，禁止内联业务文件或只检验mock次数。
- 定向单元测试命令（任务内运行）：`cargo test -j 1 -p vw-agent --lib diagnostic_api_start_authorizes_before_execution -- --nocapture`。
- 覆盖风险（不要求制造红灯）：真实调用链当前缺少本任务行为，观察“每类诊断独立从已授权配置创建任务，越权不会触发网络或进程。”对应断言失败；仅找不到crate/模块或编译失败不是行为红灯，先完成最小测试接线再取得断言失败。
- 通过验证命令（任务内运行）：`cargo test -j 1 -p vw-agent --lib diagnostic_api_start_authorizes_before_execution -- --nocapture`；有效匹配至少1项，0项不能通过。
- 模拟策略：领域状态使用真实结构与事务；HTTP/SSE/MCP使用隔离本地真实协议服务，只有外部供应商/时钟/故障边界受控替代。网络请求数、持久结果和进程退出必须实际观察；标记受控证据，不冒充真实服务商。

## 步骤

1. 按风险复用或补充行为测试，审查预期断言；必要时先确认缺陷测试失败，实现后运行定向测试确认通过。
2. 注册 POST /v1/agent-apps/diagnostics/jobs，强类型kind选择任务30–36执行器；不同kind仅接受必需字段。
3. 复用认证与主机/应用/模型/MCP权限，先验证再入队；只在目标主机执行；覆盖403和无网络/进程副作用、无效模型、并发上限、202响应任务ID。
4. 实现测试证明范围内的最小变更；审查本次差异，记录未运行的定向回归检查。

## 定向验证（任务内完成）

- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 所需证据：红灯断言、通过输出、有效测试数量、确切命令/退出码、实际槽位/绝对CARGO_TARGET_DIR、真实文件/符号及已审查差异；API/SDK另附请求与状态，进程任务附存活/退出证据。

## 完成

- 任务收尾：必要定向验证通过并审查后按原完成规则处理；只有明确集成待验收或客观外部阻塞时保留待回归，不能把必要单元测试统一延后。

- 仅在本任务取得当前验证证据后标记 `已完成`。若无关既有编译错误阻断验证，记录检查后可标 `已完成，待回归` 并保留任务；缺少本任务的必需接口或自动验证证据，标 `已阻塞` 并保留，不能用信心代替证据。
- 阻塞和进度只写本任务文件，不改只读 `plan.md`，不把 `prompt.md` 当进度来源；不静默删减来源需求。发现新需求先询问是否记录回来源产物。
- 有实际代码/测试/配置/正式文档变更时，使用 `$git-commit` 审查并仅提交本任务变更；只有本编号文件状态更新或删除时无需提交。不要提交其他任务或工作区变更。
- 最终回复报告完成证据与推荐下一步；完成且非待回归时按执行技能删除本任务文件，正式证据文档保留。只生成本计划不会完成母任务9。

## 本次执行记录

- 按最新授权直接在原仓库工作，未创建工作树；只读 plan.md 未修改。已读执行技能、来源映射、任务 1–3 合同，核对 DTO、DiagnosticJobs、ReachabilityProbe、speedtest::start、ModelProbe、StreamProbe、network_probe::start_handshakes 真实产物；任务 34/36 的旧待回归状态不作为接口缺失。
- 已接 POST /v1/agent-apps/diagnostics/jobs，强类型 kind 仅接受已有最小字段。供应商诊断沿用现有出站诊断的 routing_manage / routing_apps 与 provider_config_read，模型由保存部署核对；MCP 沿用配置级 mcp_probe。未新增权限配置项或人工批准。
- 独立测试覆盖五类启动、本地真实协议、持久任务身份、越权无网络/stdio 哨兵副作用、无效模型、容量 429，并补充真实 gateway 子进程入口。
- 领取槽位 7，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7，同时 CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR。
- 首次 cargo fmt --all -- --emit stdout 完成；再次 cargo fmt --all -- --emit stdout --config skip_children=true 退出 0，随后只格式化负责文件。
- 红灯尝试 cargo test -p vw-agent --lib diagnostic_api_start_authorizes_before_execution -- --nocapture 在测试前被无关 aggregation_pi.rs 的 PiDirectoryRead/Write/State 缺失与 serde 属性错误阻断。其他所有者随后补齐类型，旧快照无重跑价值，停止自己的 Cargo 与残留 rustc（退出 130；0 项测试），未将其当作行为红灯。最新源码聚焦验证及规定 Clippy 正在执行。

- 新授权要求突破重复 ACP 阻断：复核 `session/llm/acp/model_source/drivers/drivers_tests.rs:57,85` 已有 `.model_api.clone().unwrap()`，直接复用、保留，未写入或纳入诊断提交。旧快照 Clippy 观察到 E0509 后中断退出 130；0 项行为测试。
- 增量测试构建运行约 55 分钟仍在单态化/MIR；一次 1 秒 sample 退出 0，未见依赖图保存栈。随后内存压力 2、逻辑压缩页约 48 GiB、自己 rustc footprint 9.5 GB / 驻留约 1.5 GiB。按新授权仅停止自己的进程（测试与排队旧 Clippy 均 130），保留缓存；19:52:56 起在槽位 7 以 CARGO_INCREMENTAL=0、CARGO_BUILD_JOBS=1 重跑聚焦测试，成功后顺次完整 Clippy。采样与精确检查记录见正式证据草稿 `docs/vibewindow/agent-apps/diagnostics/start-api-37.md`。

- 会话管理接口中断后，20:58 核实原 shell/Cargo/rustc 均已结束、工具会话失效，未取得测试输出和可恢复退出码。校验旧持有进程 54011 已结束后只释放本任务 slot-7.lock，保留缓存；动态顺次重新领取仍为槽位 7。21:00:46 起以 CARGO_INCREMENTAL=0、CARGO_BUILD_JOBS=1、CARGO_PROFILE_TEST_DEBUG=0、CARGO_PROFILE_DEV_DEBUG=0、CARGO_PROFILE_TEST_CODEGEN_UNITS=256 重跑，实际 CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-7。未改仓库配置；当前仍在验证。

- 22:50:59 首次完成聚焦测试：退出 101，2 项执行，网关用例通过，主用例因无效模型 503 != 404 失败。定位为夹具 tempfile 根目录未显式 0700，被 CredentialFileStore 拒绝。原二进制在 umask 077 下两项通过（退出 0，1.55 秒）；只修复独立测试目录权限，未弱化安全检查。该失败不作为行为红灯。
- 原槽位全部进程结束后释放自己的槽位 7 锁并保留缓存；22:53:23 顺次重新原子领取到槽位 2，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-2，继续相同低内存环境。fmt 输出检查退出 0；修复后聚焦测试和规定完整 Clippy 正在顺序执行。

- 槽位 2 聚焦测试退出 101、0 项：无关 ProviderOperationProgress 快照缺少 aggregation_outcome（E0560），当前 DTO 已由其他任务补齐，本任务未改。随后完整规定 Clippy 退出 0，12 分 46 秒完成，含本任务测试源码。
- 用户允许优先复用先前实际领取槽位。槽位 2 重领时已被其他任务占用；短暂进入冷缓存 3 后，原子取得空闲旧槽位 7，仅停止自己的槽位 3 启动进程（143、0 测试）并核实释放。23:23:44 起复用槽位 7 同阶段运行聚焦测试及完整 Clippy；两个绝对目录均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-7，其他低内存变量不变。

- 最终默认 features 聚焦 Cargo 命令退出 0：2 passed、0 failed、12303 filtered，2.05 秒。完整 Clippy（槽位 2）退出 0，覆盖 0700 修复；相关源码未再变更，按用户指示复用并停止重复 Clippy（143）。已确认自己的进程结束、锁释放、缓存保留。历史行为红灯未取得和 __eh_frame 链接警告如实保留在正式证据。

- 收尾补充要求有效行为失败证据仍必需。当前其他任务有 15 个活动 Cargo/编译器进程；本任务变更涉及共享授权、父模块、bootstrap 与生命周期，不能在其构建期间倒退源码影响快照。本轮不伪造前置基线，也不把夹具错误当作行为红灯。状态保留“已完成，待回归”，剩余项是在本会话取得受保护回放窗口，仅精确回放并恢复本任务块，取得真实调用链缺失行为的断言失败（标为基线回放证据），必要绿灯回归后再删除本卡。
