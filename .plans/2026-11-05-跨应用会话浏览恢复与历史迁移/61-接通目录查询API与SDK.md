# 任务 61: 接通目录查询API与SDK

批次：【批次 45】 依赖：批次 44 的任务 60。外部前置：核实母任务 1 的来源快照、母任务 2 的授权/可信主机和母任务 3 的应用实例与路径。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：通过真实 HTTP 查询授权主机的应用/项目过滤、搜索、分组元数据与游标；拒绝越权租户/主机且不回退本机。

来源任务：`.plans/2026-11-05-CC能力完整融合/14-跨应用会话浏览恢复与历史迁移.md`，需求映射见本目录 source-map.md。

预计会话范围：只完成“接通目录查询API与SDK”的实现与下述单一验收故事，先读指定来源和相邻独立测试；不在本会话实现其他任务。若超出一个聚焦会话，记录明确剩余范围并请求补拆，不静默扩大任务。

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

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

## 测试计划

- 行为：通过真实 HTTP 查询授权主机的应用/项目过滤、搜索、分组元数据与游标；拒绝越权租户/主机且不回退本机。
- 失败验证：在 `crates/vw-agent/src/gateway/api/handlers/external_sessions/catalog_tests.rs` 新增前缀 `external_history_t61_` 的测试，至少包含 `external_history_t61_primary_behavior` 与 `external_history_t61_boundary_behavior` 两个确切测试名，分别覆盖上述正常路径与最相关拒绝/失败路径。测试是本项拟新增，必须通过父模块显式注册；单元测试不内联业务文件。
- 失败验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib external_history_t61_ -- --nocapture`。
- 预期失败：当前缺少本项原生行为或尚未接线；先搭好最小可编译接口，再确认真实行为断言失败。仅编译错误、零测试和测试未注册不算红灯证据。若前序已实现该行为，先运行验证，补缺失边界用例并记录已有等价实现，不强造失败。
- 通过验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib external_history_t61_ -- --nocapture`。
- SDK 验证：`python3 scripts/cargo_slot.py cargo test -p vw-gateway-client --lib external_history_t61_ -- --nocapture`，同样要求有效测试大于零。
- features：上述 crate 的 macOS host 默认 features；不加未定义开关。vw-agent-apps 的 Cargo.toml 由母任务2交付，执行前检查真实名称与 lib 测试入口；不存在则不得宣称命令可运行。
- 模拟策略：使用真实 axum Router/认证上下文和临时存储启动回环 HTTP，由真实 GatewayClient 调用；只替代不可控原生终端/应用进程，不 mock 处理器、SDK 或权限检查。

## 步骤

1. 先读来源指定函数与相邻测试、目标模块父级注册、调用者及 contracts.md。确认权限/主机/任务合同已由底座交付。
2. 写并注册失败测试，运行上述命令，保存断言失败而非编译失败的证据。
3. 仅实现本项行为，沿用现有显式 match 和错误类型，不新增无调用者 trait、配置或第二存储。大于约500行按同一职责拆文件，记录实际落点。
4. Rust 修改后运行 `cargo fmt --all`，审查 diff 排除无关格式化，再运行通过验证和 SDK 验证（如适用）。
5. 运行强制 Clippy，记录退出码与有效测试数量；在测试保持通过时才做必要局部整理。更新 `docs/shiju/external-sessions/coverage.md` 本项实现与证据，不改 plan.md。

## 验证

- 聚焦：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib external_history_t61_ -- --nocapture`。
- SDK 验证：`python3 scripts/cargo_slot.py cargo test -p vw-gateway-client --lib external_history_t61_ -- --nocapture`，同样要求有效测试大于零。
- 强制：`python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`；差异：`git diff --check`。
- 预期：聚焦测试有效数量大于零、断言全部通过；本项新错误/警告为零。Clippy 是全 workspace 验证，不等同通过本项行为验收。
- 所需证据：失败/通过的确切测试名、命令、退出码和有效测试数量；槽位与绝对 CARGO_TARGET_DIR；生产/独立测试文件及关键符号、差异审查、API/SDK输出或渲染证据（适用时）。无关既有编译错误只记录，不修无关模块。

## 完成

- 只在本任务文件更新状态/阻塞。完成证据写正式 coverage.md；最终回复报告确切文件、符号、验证结果和后续依赖。
- 状态仅用 `未开始`、`已完成`、`已阻塞`、`已完成，待回归`。缺真实依赖标阻塞；无关既有错误导致自动验证不齐，保留待回归，不能假报通过。
- 有实际代码、测试、配置或文档变更，使用 `$git-commit` 检查并仅提交本任务差异；只有任务卡状态更新/删除时不调用该技能、不单独提交。取得全部证据后报告并删除本任务卡；待回归/阻塞保留。
- 不运行无过滤全量 cargo test、fmt check、wasm 检查或 Makefile；不覆盖、暂存或提交其他会话变更。不修改只读母/子 plan.md，不把母任务14标记完成。
