# 任务 23: 完成去重后API与SDK聚焦回归

批次：【批次 7】 依赖：批次 6 的任务 22。清理准入需要母任务 20 及相关替代能力的实际验收。同批实施与资源规则见 prompt.md。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：只运行已有的逐域 API/SDK 验证，不扩大实现。

来源任务：`.plans/2026-11-05-CC能力完整融合/21-最终重复体系收敛与清理.md`。仅 macOS 验收，来源母任务及母 plan.md 本轮不修改。

预计会话范围：一个窄领域或一个清理门槛，预计 20–40 分钟有效操作，构建等待另计；不补齐前置业务。

## 文件

- 操作范围：`crates/vw-agent/tests/cc_convergence_*.rs；crates/vw-gateway-client/src/client/*_api_tests.rs`。
- 新建正式证据：`docs/shiju/cc-convergence/api-sdk-regression.md`。

## 上下文

- 列出的直接依赖通过后执行；只操作本任务指定清理面。前置运行细节以实际已交付证据为准，不使用猜测配置启动用户环境。

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

- 本计划所有源码变更、`cargo fmt --all` 和 workspace Clippy 串行执行，不能与其他会话争写；槽位仅隔离构建缓存。锁记录须覆盖正常、失败、中断退出清理。

## 验证计划与步骤

这是当前版本回归，不新增生产行为，测试驱动开发不适用。

1. 审阅前批单域测试确实通过 SDK/真实网关而非复制生产实现；核对每个域记录的生产 API 与 SDK 映射。
2. 逐项原样运行各域证据中的精确 Cargo 测试命令，不使用无过滤 `cargo test`；不存在的测试文件只能凭该域已确认零删除与当前替代行为测试证据跳过，并明确记录原因。
3. SDK 现有覆盖补充运行 `python3 scripts/cargo_slot.py cargo test -p vw-gateway-client --lib --all-features provider_api_tests -- --nocapture`、`python3 scripts/cargo_slot.py cargo test -p vw-gateway-client --lib --all-features desktop_settings_api_tests -- --nocapture`、`python3 scripts/cargo_slot.py cargo test -p vw-gateway-client --lib --all-features mcp_api_tests -- --nocapture`、`python3 scripts/cargo_slot.py cargo test -p vw-gateway-client --lib --all-features session_api_tests -- --nocapture`。
4. 核对错误码、租户/项目/主机隔离、取消与进度保持原契约；每条命令记录测试数量和退出码，零条不算成功。
5. `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`。本项不修复无关故障；发现本计划回归则在本任务记录并交由对应领域任务修正后复验。

预期：所有适用域的当前测试与上述四组 SDK 测试均通过，无来源树依赖。

所需证据：确切命令、退出码、有效测试数（非行为检查注明不适用）、实际槽位、绝对 `CARGO_TARGET_DIR` 和本批要求的图/操作记录；任何 Rust 修改先 `cargo fmt --all` 再聚焦验证和 `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`。

## 完成

- 证据文件保留实际文件/符号、替代路径、调用者清零查询结果、数据对照、恢复点、命令/退出码/有效测试数、macOS 版本与架构；测试不能是零条或静默跳过。敏感数据只保留脱敏摘要。
- 新增代码无警告；无关既有错误只记录，不顺手修复。没有足够证据不得标已完成。
- 状态只使用未开始、已完成、已阻塞、已完成，待回归。完成后有实际变更使用 `$git-commit`，排除无关变更；仅任务状态或任务文件删除不提交。完成并报告证据后删除本编号文件；待回归或阻塞保留。
