# 任务 6: MCP唯一管理与分发路径

批次：【批次 2】 依赖：批次 1 的任务 1。清理准入需要母任务 20 及相关替代能力的实际验收。同批实施与资源规则见 prompt.md。

状态：未开始

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：在已融合领域内切到唯一权威路径并删除失去真实调用者的局部副本。

来源任务：`.plans/2026-11-05-CC能力完整融合/21-最终重复体系收敛与清理.md`。仅 macOS 验收，来源母任务及母 plan.md 本轮不修改。

预计会话范围：一个窄领域或一个清理门槛，预计 20–40 分钟有效操作，构建等待另计；不补齐前置业务。

## 文件

- 搜索后按证据限定修改：`crates/vw-agent/src/gateway/api/handlers/mcp.rs`、`crates/vw-gateway-client/src/client/mcp_api.rs`。
- 只读来源：`crates/vw-agent-apps/upstream/src-tauri/` 对应 `services/mcp.rs、database/dao/mcp.rs；src/lib/api/mcp.ts、src/hooks/useMcp.ts`（TS 位于原来源 src）。
- 新建独立行为测试：`crates/vw-agent/tests/cc_convergence_mcp.rs`；仅调用实际公开 API/启动入口，不为测试新增生产后门。
- 新建正式证据：`docs/shiju/cc-convergence/mcp.md`。

## 上下文

- 本域与母任务 2–20 的实际实现衔接；未交付符号不在此计划中臆造。
- 上游原始仓库只读，运行树源码快照只在归档任务整体处理。

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

- 本计划所有源码变更、`cargo fmt --all` 和 workspace Clippy 串行执行，不能与其他会话争写；槽位仅隔离构建缓存。锁记录须覆盖正常、失败、中断退出清理。

## 会话边界与前置检查

- 开始先审查 Git 状态与本任务域的前置交付证据。母批次 20 以及本域 API/SDK/执行端/Iced 未验收时，只在本任务记录缺项，不通过删除来源来掩盖缺项。
- 将本域实际生产文件、符号、全部读写调用者、数据位置、权限上下文、已存在测试以及最小删除集合绑定到本任务证据文件；上述目录是搜索入口，不授权修改整个目录。API/SDK 契约保持稳定。
- 本任务只收敛已等价的路径；若需要新增业务、一次处理多个独立事务或超过约 40 分钟有效实现，先记录具体缺口，停止扩大实现，交回原领域前置任务或请求细分。构建等待不计入有效实现时间。
- 旧用户数据转换由母任务 18 交付；这里只用脱敏夹具核对转换结果、校验和与失败回退，不新增旧结构兼容层或数据迁移，不碰真实用户目录。
- 若本域已只有一个所有者，运行现有行为验收并记录零删除依据；不得为制造红灯破坏代码或编写字符串匹配业务测试。

## 测试计划

- 行为：一次管理更新只更新唯一权威记录，应用分发是可核对的派生结果，失败可重试且无重复配置写入。
- 新增测试名：`cc_convergence_mcp_uses_single_owner`、`cc_convergence_mcp_rejects_cross_scope_or_failure_without_side_effects`，置于上述独立集成测试文件。必要时复用既有公开测试启动方式，禁止将业务逻辑复制到测试。
- 失败验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --test cc_convergence_mcp --all-features cc_convergence_mcp_ -- --nocapture`。
- 预期失败原因：真实 API/执行链仍触发旧读取、第二写入/调度/事件或权限副作用；先断言本任务实际观察到的那一项失败。不存在差异时按零删除路径提供既有通过证据，不虚构红灯。
- 通过验证命令：同上；至少执行上述两条有效行为测试，断言实际响应与落盘/事件结果，不能只检查 mock 调用次数。
- 模拟策略：真实临时存储、实际网关路由和 SDK；仅不确定的外部账号/上游 HTTP 使用本机服务替身，保留相同错误与数据形状。网络凭据及真实用户目录不参与。
- crate/features：`vw-agent`，`--all-features`；测试按本域前缀过滤。新业务包仅在前置已实际注册时由真实调用链链接，不执行未经核对的 `python3 scripts/cargo_slot.py cargo test -p vw-agent-apps`。

## 步骤

1. 读取 `docs/shiju/cc-convergence/准入与所有者.md` 及本域母任务正式证据，列全本域真实调用者，限定一次事务/解析/执行入口的重复路径。若没有等价前置，记录阻塞，不重做领域迁移。
2. 先写真实失败测试并运行命令，记录预期红灯；测试若先通过，核实是否无需修改。
3. 最小切换本域调用者与写入路径，脱敏夹具前后比对字段、条数、哈希；在测试验证失败回滚。
4. 用 `rg -n` 或 RustCodeGraph callers/impact 查本任务旧符号、配置键和直接写入入口；零运行调用者后只删除本域失效副本。允许来源归档/测试引用，逐项解释。
5. Rust 改动后运行 `cargo fmt --all`，审查并排除无关格式变化；运行聚焦测试，再运行 `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`。
6. 在本域证据记录 API/SDK/Iced 调用去向、删除集合、保留边界、数据对照与回退提交；不修改其他领域证据。

## 验证

- 命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --test cc_convergence_mcp --all-features cc_convergence_mcp_ -- --nocapture`、`python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`、`git diff --check`。
- 预期：至少两条本域测试通过，Clippy 和差异检查退出 0；本任务的旧运行调用者为零，无长期双写，SDK/API 保持既有权限与错误语义。
- 记录失败到通过的输出、实际槽位/绝对 `CARGO_TARGET_DIR`；若无需修改，仍须当前行为与调用者证据，不以历史验收代替。

## 完成

- 证据文件保留实际文件/符号、替代路径、调用者清零查询结果、数据对照、恢复点、命令/退出码/有效测试数、macOS 版本与架构；测试不能是零条或静默跳过。敏感数据只保留脱敏摘要。
- 新增代码无警告；无关既有错误只记录，不顺手修复。没有足够证据不得标已完成。
- 状态只使用未开始、已完成、已阻塞、已完成，待回归。完成后有实际变更使用 `$git-commit`，排除无关变更；仅任务状态或任务文件删除不提交。完成并报告证据后删除本编号文件；待回归或阻塞保留。
