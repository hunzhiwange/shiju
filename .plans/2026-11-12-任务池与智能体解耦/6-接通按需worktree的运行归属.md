# 任务 6: 接通按需worktree的运行归属

> 验证调整（2026-11-13）：遵守 `.plans/AGENTS.md`；取消额外全量构建、原生验收窗口及截图验证，以聚焦行为测试和差异审查收尾。历史记录保留。

批次：【批次 6】 依赖批次 5；传递依赖批次 1–5

线性顺序：任务 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14；本任务不可与其他批次并行。

状态：已完成，待回归

目的：复用 EnterWorktree/ExitWorktree，只在显式调用时创建隔离目录，并记录所属 run 与产物。

来源任务：无；来源为用户本轮确认的重构目标。

预计会话范围：围绕“普通任务零 worktree；显式工具创建真实隔离目录后，后续文件与命令使用新目录；非 Git 显式调用清晰失败。”一个验收故事。只沿列出的真实调用链修改，必要相邻依赖先调查、记录再修复；不叠加无关重构。

## 文件

- 修改：`crates/vw-agent/src/tools/enter_worktree/mod.rs`
- 修改：`crates/vw-agent/src/tools/exit_worktree/mod.rs`
- 修改：`crates/vw-agent/src/tools/context/mod.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/runner/gateway_executor.rs`
- 测试：`crates/vw-agent/src/tools/enter_worktree/tests.rs`
- 测试：`crates/vw-agent/src/tools/exit_worktree/tests.rs`
- 测试：`crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs`
- 参考：`crates/vw-agent/src/worktree/worktree.rs`

## 上下文

保留现有容量配置、工位池、预热/回收机制、员工交互、列表/像素办公室切换、展开收起、状态统计、确认与操作日志。员工与工位是现有智能体及运行的投射，只替换 worktree 绑定；不得借解耦删除这些产品架构、重置容量值或把容量限制成智能体定义数量。

EnterWorktreeTool 已调用 crate::worktree::create 并绑定工具上下文，ExitWorktreeTool 默认检查脏文件。工作树底层正有其他会话修改，实施先读当前差异，不覆盖 creation_policy。

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

先读根目录 AGENTS.md 与 ./skills/rustcodegraph/SKILL.md，核对当前索引及调用者。现有未提交变更由其他会话持有，不覆盖、不回退、不混入本任务提交。plan.md 只读；关键决定与阻塞证据仅写本文件。

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

中间产物现在默认跟随目标目录；统一入口显式设置两个目录，不再使用旧的独立 build-dir 覆盖。

## 测试计划

- 行为：普通任务零 worktree；显式工具创建真实隔离目录后，后续文件与命令使用新目录；非 Git 显式调用清晰失败。
- 新增测试：`task_pool_worktree_created_only_by_explicit_tool`。
- 新增测试：`task_pool_worktree_binding_is_scoped_to_run`。
- 新增测试：`task_pool_worktree_non_git_rejected_without_side_effects`。
- 新增测试：`task_pool_worktree_failed_run_preserves_changes`。
- 上述测试名为本任务明确要求新增的测试，不是已存在测试；分别落在“文件”列出的对应独立测试文件，并核对模块注册。
- 失败验证命令与通过验证命令相同，先新增测试再运行：

- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_pool_worktree_ -- --nocapture`

- 预期失败原因：现有池资源与会话工具绑定分属两条生命周期，任务归档不能可靠关联动态申请的目录。
- 聚焦测试 features：使用各 crate 默认 features；vw-agent 默认包含 rag-pdf/firecrawl/web-fetch-html2md/web-fetch-plaintext/shell-ast/channel-lark；vw-desktop 默认空。禁止为避开本次错误临时关闭默认 features。
- 模拟策略：真实目录、Git 和 SQLite；仅替换 LLM/ACP 协议对端与外部消息网络。使用现有 global_async_env_lock 等夹具隔离全局状态，不调用真实外部收件人或生产服务。
- 测试命令必须实际命中新增测试；各 crate 分别至少一个非 ignored 测试，合计至少 4 个有效测试。零测试、仅编译或仅 JSON 形状比较不能视为本行为验收成功。

## 步骤

1. 使用真实 Git 临时仓库运行工具，验证调用前后 worktree list；普通任务不触发创建。
2. 沿用现有工具 ID、工厂注册与 crate::worktree 子系统，不新增任务专用 EnterWorktree 或预热池。
3. 传递 tenant/project/task/run 上下文并记录真实路径、分支和所属运行；将创建后的绑定变化同步到当前 run 产物，重试不能读取另一个 run 的绑定。
4. 进入 worktree 时遵守任务 4 的目录占用，不在旧执行仍写入时转移/释放守卫。成功绑定后所有工具使用动态有效目录。
5. 任务失败或取消保留可恢复产物；清理仅通过显式 ExitWorktree/现有管理操作，拒绝无授权 force、reset/clean 或删除用户 worktree。
6. 没有显式清理时报告保留目录，不无限预建空闲目录；验证权限拒绝在文件系统副作用之前发生。

## 验证

1. 在已领取槽位中记录新增测试的预期失败与原因；不把编译环境错误当作预期业务失败。
2. Rust 修改完成先运行 `cargo fmt --all`，检查并排除其对无关文件的格式化，不覆盖别人正在编辑的内容。
3. 运行上述各条通过验证命令，记录测试名称、有效数量、成功/失败/ignored 数和退出码。
4. 运行 `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`，记录退出码；修复本任务引入的问题，无关既有失败仅记录具体文件与错误。
5. 审查本任务差异、安全边界、调用者注册及资源释放，确认没有真实对外消息、发布或自动删除用户目录。



预期：聚焦行为测试有效通过，无本次引入的编译错误或警告；全仓检查如被无关改动阻断，必须提供阻断证据并保留待回归状态。

所需证据：失败前后命令与输出摘要、实际测试数、退出码、相关符号和改动路径；实际槽位和绝对路径 CARGO_TARGET_DIR；涉及 UI 时提供聚焦行为断言和结果摘要。不得运行无过滤 `cargo test`、`cargo fmt --all -- --check`、wasm 构建或 Makefile 命令。

## 完成

本任务只有 `未开始`、`已完成`、`已阻塞`、`已完成，待回归` 四种状态。每个完成结论必须有当前证据；依赖批次完成后先核对代码与提交，不以已删除任务文件单独证明依赖成功。

有实际代码、测试、配置或文档变更时，使用 `$git-commit` 检查 staged/unstaged/untracked，排除其他任务与用户变更后以中文提交；仅本文件状态更新或删除不提交。证据齐全则标记已完成，在最终回复报告证据后删除本文件；无关环境或既有错误阻断验证则标记已完成，待回归并保留本文件；本任务本身仍不满足目标则标记已阻塞，记录调查、替代路径和所需输入。不要修改 plan.md 或把进度写入 prompt.md。

## 本轮执行记录（2026-11-14）

- 已核对依赖提交：59d9db68a、220308e0b、6b0470516、aeaaa91f2、afab6ceca。前序聚焦证据与源码齐备，既有全仓问题留回归；本轮按线性顺序实施。
- 采用已有网关 delegated ToolUseContext 传递可信运行归属，不接受请求 options 中伪造的任务身份；同 session/tenant 继承共享绑定，重试 session 不继承。保留原始执行目录锁，在外部新目录追加已有 WorkspaceClaim。必要相邻调整：增加按领取时规范路径提取锁的入口，支持显式退出已删除目录后的锁释放。
- 红灯：python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_pool_worktree_ -- --nocapture，退出 101；10 个有效测试，9 通过、1 失败、0 ignored。新增 task_pool_worktree_binding_is_scoped_to_run 在同运行绑定继承断言失败（非编译错误）；日志 /tmp/shiju-task6-red.log。
- cargo fmt --all 已执行（槽位 3）；已还原其对非本任务文件的格式化，保留进入任务前其他会话修改。
- 最终验证运行于槽位 2；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-2。命令顺序：cargo test -p vw-agent --lib task_pool_worktree_ -- --nocapture；cargo clippy --all-targets --all-features -- -D warnings。日志 /tmp/shiju-task6-final.log，结果待补充。
- 原生首轮修复验证暴露模块私有路径引用，已通过 crate 内 re-export 修复；随后行为测试 11 通过、2 失败，根因为夹具没有调用真实运行器的 heartbeat_run，run 仍为 Leased。补齐后 12 通过、1 失败：真实 managed worktree 的 Exit 被底层 deletion-preflight/snapshot 安全策略拒绝。
- 必要验收调整依据：worktree_ops::remove_for_tenant 对 managed checkout 强制预检与快照，registry::remove_managed 同样拒绝旁路。保留该已有保护，不为本任务绕过或自动确认。工具测试断言 dirty/force/managed 删除拒绝均保留目录及绑定，用户通过现有管理操作清理。另新增 task_pool_worktree_removed_directory_releases_recorded_claim，独立覆盖删除后锁释放。

- 最终证据：/tmp/shiju-task6-final-proof.log。经 python3 scripts/cargo_slot.py sh -c 顺序执行，cargo test -p vw-agent --lib task_pool_worktree_ -- --nocapture 退出 0，14 passed / 0 failed / 0 ignored，五项新增测试全部有效通过（原要求四项及目录删除后锁释放）。真实 Git/SQLite 验证显式创建、动态文件/命令目录、运行归属产物、非 Git 拒绝、失败后保留变更、dirty/force/managed 清理保护。
- 最终 cargo clippy --all-targets --all-features -- -D warnings 退出 101，仅见既有 vw-desktop 的 CodeEditor::wrap_column / set_wrap_column 缺失：preview_panel/view/editor_options.rs:34、app/message/editor.rs:276 及 editor_options_tests.rs 多处（9 个测试目标错误）。未见本次 vw-agent 新增错误。原生测试链接另有既有大目标 __eh_frame 超过 16MB 的 ld 提示，不属于业务诊断。
- 最终槽位 2，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；入口已结束，自有锁释放、缓存保留。cargo fmt --all 退出 0，后续改动使用聚焦 rustfmt；git diff --check 通过。
- 保留容量/工位/员工 UI 与 creation_policy，未发对外消息、未发布、未自动清理用户目录。因既有全仓错误保留本任务卡，建议编辑器依赖修复后统一回归 Clippy。
