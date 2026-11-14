# 任务 61: 接通目录查询API与SDK

批次：【批次 45】 依赖：批次 44 的任务 60。外部前置：核实母任务 1 的来源快照、母任务 2 的授权/可信主机和母任务 3 的应用实例与路径。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：通过真实 HTTP 查询授权主机的应用/项目过滤、搜索、分组元数据与游标；拒绝越权租户/主机且不回退本机。

来源任务：`.plans/2026-11-05-CC能力完整融合/14-跨应用会话浏览恢复与历史迁移.md`，需求映射见本目录 source-map.md。

预计会话范围：只完成“接通目录查询API与SDK”的实现与下述单一验收故事，先读指定来源和相邻独立测试；不在本会话实现其他任务。若超出一个聚焦会话，记录明确剩余范围并请求补拆，不静默扩大任务。

## 验证阶段

Rust 编译资源和会话调度按 `.plans/references/rust.md` 的项目覆盖规则执行；下方必要定向单元测试仍在任务内完成。

- 开发阶段完成本任务和直接受影响行为的必要定向单元测试，优先复用现有测试；确认有效测试数及结果，不积压到最终回归。
- 基础检查限定受影响包和目标；同一目标已被定向测试成功编译时不重复 check。Rust 不能只编译单个 `.rs` 文件，测试过滤也可能编译整个 lib-test 目标。
- 本任务不运行整仓构建、全目标全特性 Clippy、全仓格式检查或全量测试。统一回归只补跨任务集成及公共检查，不机械重跑未受后续改动影响的已通过单元测试。
- 仅同仓库 Rust 重型编译串行排队、复用缓存；其它语言的普通定向单元测试不套用此限制。必要定向测试通过后按原完成规则收尾；仅具体集成待验收或客观外部阻塞保留待回归。

## 文件

- 新建（本计划拟定）：`crates/vw-agent/src/gateway/api/handlers/external_sessions/catalog.rs`。若前序已建立此文件，本项仅修改对应行为。
- 独立测试：`crates/vw-agent/src/gateway/api/handlers/external_sessions/catalog_tests.rs`，测试名前缀 `external_history_t61_`。
- 新建注册父模块：`crates/vw-agent/src/gateway/api/handlers/external_sessions/mod.rs`、`crates/vw-gateway-client/src/client/external_sessions/mod.rs`，只公开已实现的目录API。
- 新建：`crates/vw-gateway-client/src/client/external_sessions/catalog.rs` 及同目录 `catalog_tests.rs`。
- 修改：`crates/vw-agent/src/gateway/api/handlers/mod.rs`、`crates/vw-gateway-client/src/client/mod.rs`；复用母任务2网关注册/执行主机上下文，仅注册本项。

## 上下文

- 来源目录为只读 `/Users/xiangmin/worker/code/cc-switch/`；实施优先读已验收的 `crates/vw-agent-apps/upstream/` 对应快照，不运行 Tauri。
- 来源简写：`session_manager/`、`codex_*.rs`、`services/`、`commands/` 位于源仓 `src-tauri/src/`；`components/`、`hooks/` 位于源仓 `src/`，`tests/` 位于源仓根。
- 本项来源：`commands/session_manager.rs::list_sessions`。
- 执行前逐项核实本卡的外部前置，缺产物时只登记具体阻塞，不补建整个底座。
- 新增行为以任务卡为合同，开始前检查 git status 和最新代码；不得改写其他会话的现有变更。

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

- 行为：通过真实 HTTP 查询授权主机的应用/项目过滤、搜索、分组元数据与游标；拒绝越权租户/主机且不回退本机。
- 失败验证：在 `crates/vw-agent/src/gateway/api/handlers/external_sessions/catalog_tests.rs` 新增前缀 `external_history_t61_` 的测试，至少包含 `external_history_t61_primary_behavior` 与 `external_history_t61_boundary_behavior` 两个确切测试名，分别覆盖上述正常路径与最相关拒绝/失败路径。测试是本项拟新增，必须通过父模块显式注册；单元测试不内联业务文件。
- 定向单元测试命令（任务内运行）：`cargo test -j 1 -p vw-agent --lib external_history_t61_ -- --nocapture`。
- 预期失败：当前缺少本项原生行为或尚未接线；先搭好最小可编译接口，再确认真实行为断言失败。仅编译错误、零测试和测试未注册不算红灯证据。若前序已实现该行为，先运行验证，补缺失边界用例并记录已有等价实现，不强造失败。
- 通过验证命令（任务内运行）：`cargo test -j 1 -p vw-agent --lib external_history_t61_ -- --nocapture`。
- SDK 验证：`cargo test -j 1 -p vw-gateway-client --lib external_history_t61_ -- --nocapture`，同样要求有效测试大于零。
- features：上述 crate 的 macOS host 默认 features；不加未定义开关。vw-agent-apps 的 Cargo.toml 由母任务2交付，执行前检查真实名称与 lib 测试入口；不存在则不得宣称命令可运行。
- 模拟策略：使用真实 axum Router/认证上下文和临时存储启动回环 HTTP，由真实 GatewayClient 调用；只替代不可控原生终端/应用进程，不 mock 处理器、SDK 或权限检查。

## 步骤

1. 先读来源指定函数与相邻测试、目标模块父级注册、调用者及 contracts.md。确认权限/主机/任务合同已由底座交付。
2. 按需补充行为测试并审查断言；实现后运行定向测试确认通过，不强制每次编辑制造红灯。
3. 仅实现本项行为，沿用现有显式 match 和错误类型，不新增无调用者 trait、配置或第二存储。大于约500行按同一职责拆文件，记录实际落点。
4. 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
5. 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。

## 定向验证（任务内完成）

- 聚焦：`cargo test -j 1 -p vw-agent --lib external_history_t61_ -- --nocapture`。
- SDK 验证：`cargo test -j 1 -p vw-gateway-client --lib external_history_t61_ -- --nocapture`，同样要求有效测试大于零。
- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 所需证据：失败/通过的确切测试名、命令、退出码和有效测试数量；槽位与绝对 CARGO_TARGET_DIR；生产/独立测试文件及关键符号、差异审查、API/SDK输出或渲染证据（适用时）。无关既有编译错误只记录，不修无关模块。

## 完成

- 任务收尾：必要定向验证通过并审查后按原完成规则处理；只有明确集成待验收或客观外部阻塞时保留待回归，不能把必要单元测试统一延后。

- 只在本任务文件更新状态/阻塞。完成证据写正式 coverage.md；最终回复报告确切文件、符号、验证结果和后续依赖。
- 状态仅用 `未开始`、`已完成`、`已阻塞`、`已完成，待回归`。缺真实依赖标阻塞；无关既有错误导致自动验证不齐，保留待回归，不能假报通过。
- 有实际代码、测试、配置或文档变更，使用 `$git-commit` 检查并仅提交本任务差异；只有任务卡状态更新/删除时不调用该技能、不单独提交。取得全部证据后报告并删除本任务卡；待回归/阻塞保留。
- 不运行无过滤全量 cargo test -j 1、fmt check、wasm 检查或 Makefile；不覆盖、暂存或提交其他会话变更。不修改只读母/子 plan.md，不把母任务14标记完成。
