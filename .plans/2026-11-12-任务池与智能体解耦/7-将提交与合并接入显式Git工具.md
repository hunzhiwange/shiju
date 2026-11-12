# 任务 7: 将提交与合并接入显式Git工具

批次：【批次 7】 依赖批次 6；传递依赖批次 1–6

线性顺序：任务 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14；本任务不可与其他批次并行。

状态：未开始

目的：Git 提交与合并通过受权限约束的工具执行，供任务或工作流显式调用。

来源任务：无；来源为用户本轮确认的重构目标。

预计会话范围：围绕“仅显式且有权限的提交/合并修改 Git；工具遵循当前 worktree 绑定，对冲突和非 Git 返回明确结果。”一个验收故事。只沿列出的真实调用链修改，必要相邻依赖先调查、记录再修复；不叠加无关重构。

## 文件

- 修改：`crates/vw-agent/src/tools/git_operations/mod.rs`
- 测试：`crates/vw-agent/src/tools/git_operations/tests.rs`
- 参考：`crates/vw-agent/src/gateway/api/handlers/git.rs`
- 参考：`crates/vw-agent/src/gateway/api/handlers/git/repository.rs`
- 参考：`crates/vw-agent/src/gateway/api/handlers/agent_scheduler.rs`

## 上下文

GitOperationsTool 支持 add/commit，尚无 merge，且工作目录保存在 workspace_dir。已有网关 git2 commit/merge 可参考；不得通过任意参数 Git HTTP 端点绕过工具权限。

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

先读根目录 AGENTS.md 与 ./skills/rustcodegraph/SKILL.md，核对当前索引及调用者。现有未提交变更由其他会话持有，不覆盖、不回退、不混入本任务提交。plan.md 只读；关键决定与阻塞证据仅写本文件。

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

- 行为：仅显式且有权限的提交/合并修改 Git；工具遵循当前 worktree 绑定，对冲突和非 Git 返回明确结果。
- 新增测试：`task_pool_git_commit_requires_explicit_allowed_call`。
- 新增测试：`task_pool_git_merge_obeys_permissions_and_target_lock`。
- 新增测试：`task_pool_git_uses_active_worktree_directory`。
- 新增测试：`task_pool_git_conflict_keeps_recoverable_state`。
- 上述测试名为本任务明确要求新增的测试，不是已存在测试；分别落在“文件”列出的对应独立测试文件，并核对模块注册。
- 失败验证命令与通过验证命令相同，先新增测试再运行：

- `cargo test -p vw-agent --lib task_pool_git_ -- --nocapture`

- 预期失败原因：现有工具未暴露 merge，且静态 workspace_dir 在动态 worktree 下可能操作错误仓库。
- 聚焦测试 features：使用各 crate 默认 features；vw-agent 默认包含 rag-pdf/firecrawl/web-fetch-html2md/web-fetch-plaintext/shell-ast/channel-lark；vw-desktop 默认空。禁止为避开本次错误临时关闭默认 features。
- 模拟策略：真实目录、Git 和 SQLite；仅替换 LLM/ACP 协议对端与外部消息网络。使用现有 global_async_env_lock 等夹具隔离全局状态，不调用真实外部收件人或生产服务。
- 测试命令必须实际命中新增测试；各 crate 分别至少一个非 ignored 测试，合计至少 4 个有效测试。零测试、仅编译或仅 JSON 形状比较不能视为本行为验收成功。

## 步骤

1. 在真实临时仓库验证显式 add/commit 与两个分支 merge，包含拒绝权限、脏目标、冲突、重复合并和动态绑定。
2. 扩展现有 git_operations 操作契约以支持必要的 merge，source/target 明确校验；复用真实 git2 合并实现，不新增任意命令转发。新增局部模块仅在超过单一文件职责时拆出并显式注册。
3. 将写操作权限分类覆盖 merge，保持路径与分支校验；不得自动 add -A、绕过 hooks 或在失败时强制 reset/clean。沿用现有提交语义，不顺手修改全仓 Git 策略。
4. 同一目标分支合并互斥由 Git 执行边界持有，锁不得依赖常驻任务池或把其他 worktree 变为员工；复用已有锁并迁移实际必要的代码。
5. 工具执行读取当前有效工具根目录；结果返回提交/目标分支/冲突等结构化信息，失败不得标为已合并。
6. 确保工作流 NativeTools 可调用同一工具而不扩权；只修改真实必需的注册与参数协议。

## 验证

1. 在已领取槽位中记录新增测试的预期失败与原因；不把编译环境错误当作预期业务失败。
2. Rust 修改完成先运行 `cargo fmt --all`，检查并排除其对无关文件的格式化，不覆盖别人正在编辑的内容。
3. 运行上述各条通过验证命令，记录测试名称、有效数量、成功/失败/ignored 数和退出码。
4. 运行 `cargo clippy --all-targets --all-features -- -D warnings`，记录退出码；修复本任务引入的问题，无关既有失败仅记录具体文件与错误。
5. 审查本任务差异、安全边界、调用者注册及资源释放，确认没有真实对外消息、发布或自动删除用户目录。



预期：聚焦行为测试有效通过，无本次引入的编译错误或警告；全仓检查如被无关改动阻断，必须提供阻断证据并保留待回归状态。

所需证据：失败前后命令与输出摘要、实际测试数、退出码、相关符号和改动路径；实际槽位和绝对路径 CARGO_TARGET_DIR；涉及 UI 时附实际渲染截图及日志。不得运行无过滤 `cargo test`、`cargo fmt --all -- --check`、wasm 构建或 Makefile 命令。

## 完成

本任务只有 `未开始`、`已完成`、`已阻塞`、`已完成，待回归` 四种状态。每个完成结论必须有当前证据；依赖批次完成后先核对代码与提交，不以已删除任务文件单独证明依赖成功。

有实际代码、测试、配置或文档变更时，使用 `$git-commit` 检查 staged/unstaged/untracked，排除其他任务与用户变更后以中文提交；仅本文件状态更新或删除不提交。证据齐全则标记已完成，在最终回复报告证据后删除本文件；无关环境或既有错误阻断验证则标记已完成，待回归并保留本文件；本任务本身仍不满足目标则标记已阻塞，记录调查、替代路径和所需输入。不要修改 plan.md 或把进度写入 prompt.md。
