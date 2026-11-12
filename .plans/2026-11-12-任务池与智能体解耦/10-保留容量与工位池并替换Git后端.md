# 任务 10: 保留容量与工位池并替换Git后端

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

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-20`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–20。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 20 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

仓库 `.cargo/config.toml` 当前设置 `target-dir = "target-dir"`、`build-dir = "target"`；必须通过上述环境覆盖目标目录，并将 `CARGO_BUILD_BUILD_DIR` 设为 `"$CARGO_TARGET_DIR/build"`，使中间构建产物也落在已领取槽位内。不要修改全仓 Cargo 配置。领取锁后立即安排退出清理，遇到子进程仍运行先等待其结束；不清理其他人的锁或缓存。

## 测试计划

- 行为：现有容量配置及维护/分配/释放/回收/快照功能正常，零 worktree 也能运行；容量调整不会创建磁盘副本或修改智能体定义。
- 新增测试：`task_pool_capacity_maintain_preserves_target_without_creating_git_directories`。
- 新增测试：`task_pool_capacity_slots_reuse_existing_agents_and_independent_runs`。
- 新增测试：`task_pool_capacity_recycle_preserves_files_and_other_active_runs`。
- 新增测试：`task_pool_capacity_api_client_and_cli_keep_management_operations`。
- 测试名称为本任务要新增的行为测试，分别放入上列池、路由、客户端与 CLI 独立测试文件，并保留其模块注册。
- 失败验证命令与通过验证命令相同，先写测试再运行：

- `cargo test -p vw-agent --lib task_pool_capacity_ -- --nocapture`
- `cargo test -p vw-gateway-client --lib task_pool_capacity_ -- --nocapture`
- `cargo test -p vw-cli --lib task_pool_capacity_ -- --nocapture`

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
4. 运行 `cargo clippy --all-targets --all-features -- -D warnings`，记录退出码；修复本任务引入的问题，无关既有失败仅记录具体文件与错误。
5. 审查本任务差异、安全边界、调用者注册及资源释放，确认没有真实对外消息、发布或自动删除用户目录。

涉及多个 crate 的修改属于同一工位池数据链，保持端到端可编译。保留容量及工位的现有设置与交互，不删除通用 outbox、资源租约或按需 worktree 能力。

预期：聚焦行为测试有效通过，无本次引入的编译错误或警告；全仓检查如被无关改动阻断，必须提供阻断证据并保留待回归状态。

所需证据：失败前后命令与输出摘要、实际测试数、退出码、相关符号和改动路径；实际槽位和绝对路径 CARGO_TARGET_DIR；涉及 UI 时附实际渲染截图及日志。不得运行无过滤 `cargo test`、`cargo fmt --all -- --check`、wasm 构建或 Makefile 命令。

## 完成

本任务只有 `未开始`、`已完成`、`已阻塞`、`已完成，待回归` 四种状态。每个完成结论必须有当前证据；依赖批次完成后先核对代码与提交，不以已删除任务文件单独证明依赖成功。

有实际代码、测试、配置或文档变更时，使用 `$git-commit` 检查 staged/unstaged/untracked，排除其他任务与用户变更后以中文提交；仅本文件状态更新或删除不提交。证据齐全则标记已完成，在最终回复报告证据后删除本文件；无关环境或既有错误阻断验证则标记已完成，待回归并保留本文件；本任务本身仍不满足目标则标记已阻塞，记录调查、替代路径和所需输入。不要修改 plan.md 或把进度写入 prompt.md。
