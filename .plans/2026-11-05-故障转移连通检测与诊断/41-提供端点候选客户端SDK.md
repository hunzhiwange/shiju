# 任务 41: 提供端点候选客户端SDK

批次：【批次 9】 依赖：批次 8 的任务 39、40。同批实施与资源规则见 prompt.md。

状态：已完成，待回归

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：桌面可以读取规范化候选并保留来源，草稿不通过URL查询字符串发送。

来源任务：`.plans/2026-11-05-CC能力完整融合/9-故障转移连通检测与诊断.md`；具体来源：任务40端点候选API；useSpeedTestEndpoints.ts。来源仓库根目录为 `/Users/xiangmin/worker/code/cc-switch/`，不要修改上游。

预计会话范围：只交付“提供端点候选客户端SDK”这一项，通常一个聚焦会话；不执行下一任务、不展开整个母能力。若实际工作需要独立设计或第二个验证故事，记录本任务阻塞与证据，先确认拆分，不越界承接前置计划。

## 验证阶段

Rust 编译资源和会话调度按 `.plans/references/rust.md` 的项目覆盖规则执行；下方必要定向单元测试仍在任务内完成。

- 开发阶段完成本任务和直接受影响行为的必要定向单元测试，优先复用现有测试；确认有效测试数及结果，不积压到最终回归。
- 基础检查限定受影响包和目标；同一目标已被定向测试成功编译时不重复 check。Rust 不能只编译单个 `.rs` 文件，测试过滤也可能编译整个 lib-test 目标。
- 本任务不运行整仓构建、全目标全特性 Clippy、全仓格式检查或全量测试。统一回归只补跨任务集成及公共检查，不机械重跑未受后续改动影响的已通过单元测试。
- 仅同仓库 Rust 重型编译串行排队、复用缓存；其它语言的普通定向单元测试不套用此限制。必要定向测试通过后按原完成规则收尾；仅具体集成待验收或客观外部阻塞保留待回归。

## 文件

- 新建或修改（计划落点，当前不存在者由本任务新建）：`crates/vw-gateway-client/src/client/agent_apps_diagnostics_api.rs`。
- 独立测试文件：`crates/vw-gateway-client/src/client/agent_apps_diagnostics_api_tests.rs`；新增用例：`diagnostic_endpoints_sdk_uses_scoped_body`。
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

- 行为：桌面可以读取规范化候选并保留来源，草稿不通过URL查询字符串发送。
- 单元测试：先在 `crates/vw-gateway-client/src/client/agent_apps_diagnostics_api_tests.rs` 新增 `diagnostic_endpoints_sdk_uses_scoped_body`，显式挂到现有测试模块，禁止内联业务文件或只检验mock次数。
- 定向单元测试命令（任务内运行）：`cargo test -j 1 -p vw-gateway-client --lib diagnostic_endpoints_sdk_uses_scoped_body -- --nocapture`。
- 覆盖风险（不要求制造红灯）：真实调用链当前缺少本任务行为，观察“桌面可以读取规范化候选并保留来源，草稿不通过URL查询字符串发送。”对应断言失败；仅找不到crate/模块或编译失败不是行为红灯，先完成最小测试接线再取得断言失败。
- 通过验证命令（任务内运行）：`cargo test -j 1 -p vw-gateway-client --lib diagnostic_endpoints_sdk_uses_scoped_body -- --nocapture`；有效匹配至少1项，0项不能通过。
- 模拟策略：领域状态使用真实结构与事务；HTTP/SSE/MCP使用隔离本地真实协议服务，只有外部供应商/时钟/故障边界受控替代。网络请求数、持久结果和进程退出必须实际观察；标记受控证据，不冒充真实服务商。

## 步骤

1. 按风险复用或补充行为测试，审查预期断言；必要时先确认缺陷测试失败，实现后运行定向测试确认通过。
2. 为任务40候选查询API添加GatewayClient强类型方法，与任务39诊断方法共享已注册模块；复用任务6公开类型，若需候选DTO只补真实调用必需字段。
3. 本地真实HTTP服务验证POST路径、正文、供应商作用域、认证、响应来源标识和403，断言只发一次请求且无自动测速/保存副作用。
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

## 本次执行记录（2026-11-12）

- 直接依赖任务 39 `2b5362db5`、任务 40 `3619f2233` 和公开 DTO/API 已核实。直接在原仓库完成，无新工作树，无修改 plan.md。
- 实现：既有 `agent_apps_diagnostics_api.rs::GatewayClient::diagnostic_endpoint_candidates`；独立测试复用原注册，不改共享 client/mod.rs。请求正文携带供应商、草稿和预设，三项作用域留在查询；保留候选 ID、URL、全部来源和 403 错误。
- 红灯：`cargo test -p vw-gateway-client --lib diagnostic_endpoints_sdk_uses_scoped_body -- --nocapture` 退出 101，编译成功后 1 项行为断言失败，实际路径 /diagnostics/jobs 而非候选查询。
- 绿灯：同一命令退出 0，1 passed、0 failed、460 filtered。真实本地 HTTP 观察 200/403 各一次请求，无额外探测、保存或重试，服务线程 join 退出。
- 共享 helper 回归：`cargo test -p vw-gateway-client --lib diagnostic_sdk_preserves_job_identity -- --nocapture` 退出 0，2 passed、0 failed、459 filtered。
- `cargo fmt --all -- --emit stdout --config skip_children=true` 退出 0；仅 rustfmt 回写本任务两文件，未格式化其他会话文件。
- `cargo clippy --all-targets --all-features -- -D warnings` 退出 101；其他会话未跟踪桌面文件 `crates/vw-desktop/src/app/message/agent_apps_diagnostics.rs:24–28` 的 Handshake/Succeeded/Failed/Cancelled 从未构造触发 dead_code。未修改无关文件或降低检查。
- 实际原子领取槽位 2；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；按要求应用低内存环境参数，未改变 Cargo 配置或 features。
- 正式证据见 `docs/vibewindow/agent-apps/diagnostics/endpoint-candidates-sdk-41.md`。全仓检查缺少通过证据，保留本卡等待桌面所有者接线后回归。

- 局部补充：`cargo clippy -p vw-gateway-client --all-targets --all-features -- -D warnings` 退出 0，32.35s；不替代全仓检查。验证进程结束，已释放自己的槽位 2 锁，缓存保留。
