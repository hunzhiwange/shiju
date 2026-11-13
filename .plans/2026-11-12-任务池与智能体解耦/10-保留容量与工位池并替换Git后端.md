# 任务 10: 保留容量与工位池并替换Git后端

> 验证调整（2026-11-13）：遵守 `.plans/AGENTS.md`；取消额外全量构建、原生验收窗口及截图验证，以聚焦行为测试和差异审查收尾。历史记录保留。

批次：【批次 10】 依赖批次 9；传递依赖批次 1–9

线性顺序：任务 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14；本任务不可与其他批次并行。

状态：未开始

目的：保留现有容量、工位池与维护/回收调用结构，把池槽位从 Git 目录改成现有智能体运行的逻辑投射。

来源任务：用户本轮明确要求保留容量与员工交互架构。

预计会话范围：只替换容量/工位池的底层资源语义与其 DTO/client/CLI 连线，验证容量不再创建 worktree，维持现有调用结构和可编译性。

## 文件

- 修改：`crates/vw-agent/src/task_worktree_pool/mod.rs`
- 测试：`crates/vw-agent/src/task_worktree_pool/tests.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/resources/worktree.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/resources/mod.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/task_pool.rs`
- 测试：`crates/vw-agent/src/gateway/api/handlers/task_pool_tests.rs`
- 修改：`crates/vw-gateway-client/src/client/task_pool_api.rs`
- 测试：`crates/vw-gateway-client/src/client/task_pool_api_tests.rs`
- 修改：`crates/vw-api-types/src/task.rs`
- 修改：`crates/vw-cli/src/handlers/api/task_pool.rs`
- 测试：`crates/vw-cli/src/handlers/api/task_pool_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/task_board/helpers/execution.rs`
- 修改：`crates/vw-desktop/src/app/message/task_board/message.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/fields_navigation.rs`

## 上下文

保留现有容量配置、工位池、预热/回收机制、员工交互、列表/像素办公室切换、展开收起、状态统计、确认与操作日志。员工与工位是现有智能体及运行的投射，只替换 worktree 绑定；不得借解耦删除这些产品架构、重置容量值或把容量限制成智能体定义数量。

现有 TaskWorktreePoolService、maintain/acquire/release/recycle/snapshot 与 DTO/client/CLI 已形成完整调用链。保留这条产品调用架构，只把 Git 资源替换为逻辑工位/运行上下文；不得以无调用者为由先删掉 UI 再删除池。

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

先读根目录 AGENTS.md 与 ./skills/rustcodegraph/SKILL.md；核对当前调用者、相邻测试与未提交变更。plan.md 供实施会话只读，阻塞与决定只记录本文件。

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

中间产物现在默认跟随目标目录；统一入口显式设置两个目录，不再使用旧的独立 build-dir 覆盖。

## 测试计划

- 行为：现有容量配置及维护/分配/释放/回收/快照功能正常，零 worktree 也能运行；容量调整不会创建磁盘副本或修改智能体定义。
- 新增测试：`task_pool_capacity_maintain_preserves_target_without_creating_git_directories`。
- 新增测试：`task_pool_capacity_slots_reuse_existing_agents_and_independent_runs`。
- 新增测试：`task_pool_capacity_recycle_preserves_files_and_other_active_runs`。
- 新增测试：`task_pool_capacity_api_client_and_cli_keep_management_operations`。
- 测试名称为本任务要新增的行为测试，分别放入上列池、路由、客户端与 CLI 独立测试文件，并保留其模块注册。
- 失败验证命令与通过验证命令相同，先写测试再运行：

- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_pool_capacity_ -- --nocapture`
- `python3 scripts/cargo_slot.py cargo test -p vw-gateway-client --lib task_pool_capacity_ -- --nocapture`
- `python3 scripts/cargo_slot.py cargo test -p vw-cli --lib task_pool_capacity_ -- --nocapture`

- 预期失败原因：现有 maintain/acquire 依赖真实 Git 仓库并创建 worktree，容量与目录复制绑定，非 Git 无法提供池管理。
- features：各 crate 默认 features；vw-agent 默认 rag-pdf/firecrawl/web-fetch-html2md/web-fetch-plaintext/shell-ast/channel-lark，vw-cli 默认 agent-defaults/memory-postgres/memory-mariadb/channel-lark，vw-gateway-client 无 features 表。全仓 clippy 使用 all-features。
- 模拟策略：真实临时目录、现有持久化和本地 HTTP 夹具，只模拟模型/外部网络；用无 .git 的目录证明池管理不依赖 Git。
- 所需测试数量：每个 crate 至少一个有效非 ignored 测试，合计至少 4 个；零测试和仅编译不算通过。

## 步骤

1. 先记录现有容量数值、上下限、目标空闲量/并发量关系、分配与回收规则及管理 API/CLI 调用基线，再新增上述失败测试。
2. 沿现有服务结构实现逻辑工位容量和所有权：槽位绑定现有智能体及 run/session；同一定义可服务多个工位，槽位不等于员工账号或 Git 目录。不新增一套容量配置或员工注册表。
3. maintain/预热按原容量规则准备逻辑槽位，snapshot 返回原状态分区与统计，acquire/release/recycle 操作实际运行上下文；完整保留调用入口、管理操作、确认反馈与日志，不整体删除池服务或相关 DTO/client/CLI。
4. Git 分支/路径只在显式绑定真实 worktree 时携带；逻辑工位操作不调用 Git、不创建目录副本。若内部类型含 worktree 命名，仅在真实必要范围调整类型与连线，不能借重命名重做整个架构。
5. 逻辑回收与重置遵守运行归属和任务 4/5 的退出顺序，不能抢占其他活跃运行；减少容量沿原有忙碌工位保护规则处理。工位清理/删除只处理逻辑上下文，真实 Git 合并/目录删除经显式任务或工作流及既有确认执行。
6. 只清除被替换后确无调用者的 Git 自动预建/自动清理实现；保留工位池与用户配置值，不自动 prune/reset/clean、不删除既有磁盘目录。运行 API/client/CLI 聚焦测试及 clippy，记录容量和文件系统前后证据。

## 验证

1. 在已领取槽位中记录新增测试的预期失败与原因；不把编译环境错误当作预期业务失败。
2. Rust 修改完成先运行 `cargo fmt --all`，检查并排除其对无关文件的格式化，不覆盖别人正在编辑的内容。
3. 运行上述各条通过验证命令，记录测试名称、有效数量、成功/失败/ignored 数和退出码。
4. 运行 `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`，记录退出码；修复本任务引入的问题，无关既有失败仅记录具体文件与错误。
5. 审查本任务差异、安全边界、调用者注册及资源释放，确认没有真实对外消息、发布或自动删除用户目录。

涉及多个 crate 的修改属于同一工位池数据链，保持端到端可编译。保留容量及工位的现有设置与交互，不删除通用 outbox、资源租约或按需 worktree 能力。

预期：聚焦行为测试有效通过，无本次引入的编译错误或警告；全仓检查如被无关改动阻断，必须提供阻断证据并保留待回归状态。

所需证据：失败前后命令与输出摘要、实际测试数、退出码、相关符号和改动路径；实际槽位和绝对路径 CARGO_TARGET_DIR；涉及 UI 时提供聚焦行为断言和结果摘要。不得运行无过滤 `cargo test`、`cargo fmt --all -- --check`、wasm 构建或 Makefile 命令。

## 完成

本任务只有 `未开始`、`已完成`、`已阻塞`、`已完成，待回归` 四种状态。每个完成结论必须有当前证据；依赖批次完成后先核对代码与提交，不以已删除任务文件单独证明依赖成功。

有实际代码、测试、配置或文档变更时，使用 `$git-commit` 检查 staged/unstaged/untracked，排除其他任务与用户变更后以中文提交；仅本文件状态更新或删除不提交。证据齐全则标记已完成，在最终回复报告证据后删除本文件；无关环境或既有错误阻断验证则标记已完成，待回归并保留本文件；本任务本身仍不满足目标则标记已阻塞，记录调查、替代路径和所需输入。不要修改 plan.md 或把进度写入 prompt.md。
