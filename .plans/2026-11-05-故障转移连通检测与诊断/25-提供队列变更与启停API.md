# 任务 25: 提供队列变更与启停API

批次：【批次 7】 依赖：批次 6 的任务 10、11、24。同批实施与资源规则见 prompt.md。

状态：已完成，待回归

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：增删、重排和开关写入必须通过资格、版本和模式校验。

来源任务：`.plans/2026-11-05-CC能力完整融合/9-故障转移连通检测与诊断.md`；具体来源：commands/failover.rs add/remove/set_auto_failover_enabled。来源仓库根目录为 `/Users/xiangmin/worker/code/cc-switch/`，不要修改上游。

预计会话范围：只交付“提供队列变更与启停API”这一项，通常一个聚焦会话；不执行下一任务、不展开整个母能力。若实际工作需要独立设计或第二个验证故事，记录本任务阻塞与证据，先确认拆分，不越界承接前置计划。

## 验证阶段

Rust 编译资源和会话调度按 `.plans/references/rust.md` 的项目覆盖规则执行；下方必要定向单元测试仍在任务内完成。

- 开发阶段完成本任务和直接受影响行为的必要定向单元测试，优先复用现有测试；确认有效测试数及结果，不积压到最终回归。
- 基础检查限定受影响包和目标；同一目标已被定向测试成功编译时不重复 check。Rust 不能只编译单个 `.rs` 文件，测试过滤也可能编译整个 lib-test 目标。
- 本任务不运行整仓构建、全目标全特性 Clippy、全仓格式检查或全量测试。统一回归只补跨任务集成及公共检查，不机械重跑未受后续改动影响的已通过单元测试。
- 仅同仓库 Rust 重型编译串行排队、复用缓存；其它语言的普通定向单元测试不套用此限制。必要定向测试通过后按原完成规则收尾；仅具体集成待验收或客观外部阻塞保留待回归。

## 文件

- 新建或修改（计划落点，当前不存在者由本任务新建）：`crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability.rs`。
- 独立测试文件：`crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability_tests.rs`；新增用例：`ha_api_queue_write_rejects_conflicts`。
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

- 行为：增删、重排和开关写入必须通过资格、版本和模式校验。
- 单元测试：先在 `crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability_tests.rs` 新增 `ha_api_queue_write_rejects_conflicts`，显式挂到现有测试模块，禁止内联业务文件或只检验mock次数。
- 定向单元测试命令（任务内运行）：`cargo test -j 1 -p vw-agent --lib ha_api_queue_write_rejects_conflicts -- --nocapture`。
- 覆盖风险（不要求制造红灯）：真实调用链当前缺少本任务行为，观察“增删、重排和开关写入必须通过资格、版本和模式校验。”对应断言失败；仅找不到crate/模块或编译失败不是行为红灯，先完成最小测试接线再取得断言失败。
- 通过验证命令（任务内运行）：`cargo test -j 1 -p vw-agent --lib ha_api_queue_write_rejects_conflicts -- --nocapture`；有效匹配至少1项，0项不能通过。
- 模拟策略：领域状态使用真实结构与事务；HTTP/SSE/MCP使用隔离本地真实协议服务，只有外部供应商/时钟/故障边界受控替代。网络请求数、持久结果和进程退出必须实际观察；标记受控证据，不冒充真实服务商。

## 步骤

1. 按风险复用或补充行为测试，审查预期断言；必要时先确认缺陷测试失败，实现后运行定向测试确认通过。
2. 提供 POST/DELETE /queue/members、PUT /queue/order、PUT /enabled，固定方法和强类型请求；分别调用领域服务，不暴露任意数据库操作。
3. 验证缺权限无写入、重复添加、官方账号拒绝、并发版本冲突、开启失败原子回滚；返回最新版本供SDK刷新。
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


## 本会话执行记录

- 直接依赖：任务10 `944a3e897`、任务11 `ae2e7d82f`、任务24 `96b1c174a`；已核对生产服务、队列事务与现有网关注册，原工作区直接实施，未创建工作树。
- 动态原子领取槽位7；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7`，同时设置 `CARGO_BUILD_BUILD_DIR` 为同一绝对路径；`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_CODEGEN_UNITS=256 CARGO_PROFILE_DEV_DEBUG=0`。
- 已注册独立真实网关红灯测试；`cargo fmt --all -- --emit stdout` 退出0，仅回写自己的测试文件。红灯命令 `cargo test -p vw-agent --lib ha_api_queue_write_rejects_conflicts -- --nocapture` 仍在编译，尚无测试结果。
- 首次构建遗漏仓库 build-dir 覆盖，已只停止本任务Cargo/OpenSSL进程树（退出143、0项），补齐绝对 `CARGO_BUILD_BUILD_DIR` 后复用槽位缓存继续；该次中断不视作行为红灯。

- 行为红灯已取得：槽位7，标准聚焦命令退出101，实际1项失败、12316项过滤；授权POST返回401而预期200，断言位于测试文件214行；编译62m56s，运行0.58s。既有macOS紧凑展开表链接警告未影响断言运行。红灯进程及网关子进程已结束，原槽位锁释放。
- 已写入四类强类型写入路由与最小队列领域服务扩展；绿灯阶段重新原子领取槽位7，额外设置 `CARGO_PROFILE_TEST_LTO=off`（保留opt-level=1、默认features与调试断言）。格式化仍只回写自己的三个Rust文件。

- 按追加核对要求，401已用前实现二进制复核归因：同一身份/租户/项目/主机/应用GET queue=200/version0；POST未交付写路由401，与确认不存在路径的响应相同；再GET仍200且无写入。独立真实协议探针退出0、网关退出-9。前实现router源码仅有两个GET；非凭据夹具错误。当前验收测试已增加“先读成功”的前置断言，未弱化任何认证。

- 首次LTO=off绿灯编译退出101、0项测试，发现本任务三处ProviderId错误使用to_string（类型未实现Display）；已改为取出已有新类型中的String，不新增Display实现、不改共有DTO。旧日志保存在本机会话临时文件，后续复用槽位缓存重新验证。

- ProviderId修复后的全仓 `cargo fmt --all -- --emit stdout` 退出1：无关 `crates/vw-gateway-client/src/client/mcp_management_write_tests.rs` 被父模块引用但文件尚不存在。未修补他人模块；本任务三个Rust文件单独rustfmt退出0，继续原聚焦测试，不将格式化失败当行为红灯。

- 无关缺失模块由对应任务创建后，最终 `cargo fmt --all -- --emit stdout` 退出0；逐个比对本任务三个文件与全仓格式化输出一致，未写回他人文件。当前聚焦测试继续执行，仍未生成绿灯结论。

- 聚焦绿灯：槽位7，`cargo test -p vw-agent --lib ha_api_queue_write_rejects_conflicts -- --nocapture` 退出0；2项通过、0失败、12360项过滤，运行0.55s，编译31m20s。真实网关与持久状态覆盖全部计划写入行为；仅既有macOS紧凑展开表链接警告。未使用opt-level=0，实际仍opt-level=1、LTO=off。槽位已正常释放、缓存保留，正在继续规定完整Clippy。

- 首轮完整Clippy退出101，阻于无关聚合退出DTO的4处E0425/E0599（aggregation_exit.rs:71/113/275、app_providers/operations/aggregation.rs:121）。进程结束后核对共享DTO，原任务已补回AggregationExitRequest和AggregationOutcome::Exited，属于真实外部状态变化；未修改其文件，保留失败日志，并在新状态上复跑一次原完整Clippy。首轮槽位7锁正常释放、缓存保留。

- 第二轮完整 `cargo clippy --all-targets --all-features -- -D warnings` 退出101，已越过聚合类型问题，最终阻于无关 `crates/vw-desktop/src/app/voice/acceptance_fixture.rs:57/82/92` 的3处E0433：未声明tokio_tungstenite依赖。本任务文件没有该依赖，也未修改语音夹具/Cargo清单；不处理他人改动，不声称Clippy通过。按用户约定转为“已完成，待回归”，保留本任务卡，推荐原任务修复依赖后重跑完整Clippy。
- 正式证据：`docs/shiju/agent-apps/high-availability-evidence/任务25.md`。实际Cargo槽位均为7，绝对 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7`，`CARGO_BUILD_BUILD_DIR`同路径；所有本任务验证进程已退出、槽位锁已释放、缓存保留。只读plan.md未修改。
