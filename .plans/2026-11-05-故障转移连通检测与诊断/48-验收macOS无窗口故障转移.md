# 任务 48: 验收macOS无窗口故障转移

批次：【批次 19】 依赖：批次 15 的任务 23、批次 18 的任务 42、43、44。同批实施与资源规则见 prompt.md。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：关闭桌面后API/路由仍完成切换、熔断恢复和单预算执行。

来源任务：`.plans/2026-11-05-CC能力完整融合/9-故障转移连通检测与诊断.md`；具体来源：母任务9能力1/2/4。来源仓库根目录为 `/Users/xiangmin/worker/code/cc-switch/`，不要修改上游。

预计会话范围：只交付“验收macOS无窗口故障转移”这一项，通常一个聚焦会话；不执行下一任务、不展开整个母能力。若实际工作需要独立设计或第二个验证故事，记录本任务阻塞与证据，先确认拆分，不越界承接前置计划。

## 文件

- 新建或修改（计划落点，当前不存在者由本任务新建）：`crates/vw-agent/src/gateway/api/handlers/agent_apps_ha_acceptance_tests.rs`。
- 新建或修改（计划落点，当前不存在者由本任务新建）：`docs/vibewindow/agent-apps/macOS高可用验收.md`。
- 独立测试文件：`crates/vw-agent/src/gateway/api/handlers/agent_apps_ha_acceptance_tests.rs`；新增用例：`ha_macos_headless_routing_acceptance`。
- 仅允许必要的直接父模块注册；API任务另修改 `crates/vw-agent/src/gateway/bootstrap.rs` 与 `api/handlers/mod.rs`；SDK任务另注册 `client/mod.rs`。已有等价模块则就地接入并在本文件记录真实路径，不复制第二实现。

## 上下文

- 实机验收由用户按`.plans/2026-11-05-CC能力完整融合/人工验收.md`人工完成。自动任务以聚焦测试、隔离夹具和命令输出为证据，无需启动真实桌面、使用真实账号、截图或录屏。 交付文档将用户人工验收单列为待验；它不阻塞本自动任务，也不计为已通过。

- 先读根 AGENTS.md、`skills/rustcodegraph/SKILL.md` 和本计划 `来源映射.md`，查真实调用者及相邻测试。
- 实施前置：按任务 1–3 的合同核实本任务使用的真实产物。诊断使用已交付的授权与执行端，故障切换和运行时联动使用已交付的常驻路由接口。缺少本任务必需依赖则记录已阻塞，不把整个前置能力塞入本会话。
- 仅macOS验收；不要求其他系统实机、交叉编译或wasm。实机与原生窗口由用户人工验收，仅记录简短文字结果。
- 新模块与API名称是本计划明确拟新增的设计；下列命令是测试注册完成后的验证契约，不声称当前可运行。

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

适用 crate：`vw-agent`；聚焦测试使用该 crate 当前默认 features（不传 `--no-default-features`），Clippy使用workspace全部features。`vw-agent-apps`须先由母任务2注册，禁止在未注册时声称测试已运行。

## 测试计划

- 行为：关闭桌面后API/路由仍完成切换、熔断恢复和单预算执行。
- 失败验证测试：先在 `crates/vw-agent/src/gateway/api/handlers/agent_apps_ha_acceptance_tests.rs` 新增 `ha_macos_headless_routing_acceptance`，显式挂到现有测试模块，禁止内联业务文件或只检验mock次数。
- 失败验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib ha_macos_headless_routing_acceptance -- --nocapture`。
- 预期失败原因：真实调用链当前缺少本任务行为，观察“关闭桌面后API/路由仍完成切换、熔断恢复和单预算执行。”对应断言失败；仅找不到crate/模块或编译失败不是行为红灯，先完成最小测试接线再取得断言失败。
- 通过验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib ha_macos_headless_routing_acceptance -- --nocapture`；有效匹配至少1项，0项不能通过。
- 模拟策略：领域状态使用真实结构与事务；HTTP/SSE/MCP使用隔离本地真实协议服务，只有外部供应商/时钟/故障边界受控替代。网络请求数、持久结果和进程退出必须实际观察；标记受控证据，不冒充真实服务商。

## 步骤

1. 阅读真实调用者与相邻测试，新增上述失败测试并执行失败验证，保存非零测试的预期断言失败。
2. 编写隔离受控上游及配置fixture，真实网关HTTP/SDK/长期路由链完成队列启停、P1失败P2成功、全部熔断和半开恢复；记录OS/架构/应用版本与实际命中。
3. 复用任务17、20–22计数场景，验证重试不相乘、已输出流不重放、官方不跨账号和工具计数1；执行相关聚焦用例而非全量测试。
4. 在文档仅记录本任务当前命令/退出码/有效数量/网络计数与权限结果，不标真实服务商通过，不更改母任务状态。
5. 实现测试证明范围内的最小变更；运行 `cargo fmt --all`，审查并排除无关格式化差异，再执行通过验证与规定Clippy。仅在保持通过时重构。

## 验证

- 运行：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib ha_macos_headless_routing_acceptance -- --nocapture`，再运行 `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`。
- 预期：聚焦测试至少1项通过、无失败；新行为的断言覆盖本文件目的，Clippy无本任务新增错误/警告。
- 所需证据：红灯断言、通过输出、有效测试数量、确切命令/退出码、实际槽位/绝对CARGO_TARGET_DIR、真实文件/符号及已审查差异；API/SDK另附请求与状态，进程任务附存活/退出证据。

## 完成

- 仅在本任务取得当前验证证据后标记 `已完成`。若无关既有编译错误阻断验证，记录检查后可标 `已完成，待回归` 并保留任务；缺少本任务的必需接口或自动验证证据，标 `已阻塞` 并保留，不能用信心代替证据。
- 阻塞和进度只写本任务文件，不改只读 `plan.md`，不把 `prompt.md` 当进度来源；不静默删减来源需求。发现新需求先询问是否记录回来源产物。
- 有实际代码/测试/配置/正式文档变更时，使用 `$git-commit` 审查并仅提交本任务变更；只有本编号文件状态更新或删除时无需提交。不要提交其他任务或工作区变更。
- 最终回复报告完成证据与推荐下一步；完成且非待回归时按执行技能删除本任务文件，正式证据文档保留。只生成本计划不会完成母任务9。
