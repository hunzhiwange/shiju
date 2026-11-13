# 任务 7: 将提交与合并接入显式Git工具

> 验证调整（2026-11-13）：遵守 `.plans/AGENTS.md`；取消额外全量构建、原生验收窗口及截图验证，以聚焦行为测试和差异审查收尾。历史记录保留。

批次：【批次 7】 依赖批次 6；传递依赖批次 1–6

线性顺序：任务 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14；本任务不可与其他批次并行。

状态：已完成，待回归

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

保留现有容量配置、工位池、预热/回收机制、员工交互、列表/像素办公室切换、展开收起、状态统计、确认与操作日志。员工与工位是现有智能体及运行的投射，只替换 worktree 绑定；不得借解耦删除这些产品架构、重置容量值或把容量限制成智能体定义数量。

GitOperationsTool 支持 add/commit，尚无 merge，且工作目录保存在 workspace_dir。已有网关 git2 commit/merge 可参考；不得通过任意参数 Git HTTP 端点绕过工具权限。

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

- 行为：仅显式且有权限的提交/合并修改 Git；工具遵循当前 worktree 绑定，对冲突和非 Git 返回明确结果。
- 新增测试：`task_pool_git_commit_requires_explicit_allowed_call`。
- 新增测试：`task_pool_git_merge_obeys_permissions_and_target_lock`。
- 新增测试：`task_pool_git_uses_active_worktree_directory`。
- 新增测试：`task_pool_git_conflict_keeps_recoverable_state`。
- 上述测试名为本任务明确要求新增的测试，不是已存在测试；分别落在“文件”列出的对应独立测试文件，并核对模块注册。
- 失败验证命令与通过验证命令相同，先新增测试再运行：

- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_pool_git_ -- --nocapture`

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
4. 运行 `python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`，记录退出码；修复本任务引入的问题，无关既有失败仅记录具体文件与错误。
5. 审查本任务差异、安全边界、调用者注册及资源释放，确认没有真实对外消息、发布或自动删除用户目录。



预期：聚焦行为测试有效通过，无本次引入的编译错误或警告；全仓检查如被无关改动阻断，必须提供阻断证据并保留待回归状态。

所需证据：失败前后命令与输出摘要、实际测试数、退出码、相关符号和改动路径；实际槽位和绝对路径 CARGO_TARGET_DIR；涉及 UI 时提供聚焦行为断言和结果摘要。不得运行无过滤 `cargo test`、`cargo fmt --all -- --check`、wasm 构建或 Makefile 命令。

## 完成

本任务只有 `未开始`、`已完成`、`已阻塞`、`已完成，待回归` 四种状态。每个完成结论必须有当前证据；依赖批次完成后先核对代码与提交，不以已删除任务文件单独证明依赖成功。

有实际代码、测试、配置或文档变更时，使用 `$git-commit` 检查 staged/unstaged/untracked，排除其他任务与用户变更后以中文提交；仅本文件状态更新或删除不提交。证据齐全则标记已完成，在最终回复报告证据后删除本文件；无关环境或既有错误阻断验证则标记已完成，待回归并保留本文件；本任务本身仍不满足目标则标记已阻塞，记录调查、替代路径和所需输入。不要修改 plan.md 或把进度写入 prompt.md。

## 本轮执行证据（2026-11-14）

- 依赖核对：批次 6 提交 aa85b3699，任务文件已有 14 passed / 0 failed / 0 ignored 的聚焦证据；批次 1–5 对应代码与提交已存在，保留既有待回归记录。
- 红灯：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_pool_git_ -- --nocapture`，退出 101，4 项有效测试，1 passed / 3 failed / 0 ignored。失败为未支持 merge 与静态目录导致 active 分支断言失败，非编译错误；日志 `/tmp/shiju-task7-red.log`。
- 实现调整依据：网关 merge_branch_blocking 会自动提交脏目标并在失败后 reset，不符合本任务安全边界。因此沿用 git2 原语，增加独立 merge.rs，以 Git 引用事务锁持有目标（跨进程、RAII 释放），不依赖任务池。仅允许当前有效目录已检出的目标分支；冲突用内存 merge_commits 检测，保持 HEAD、索引和用户文件。未新增 HTTP 旁路。NativeTools 已通过 execute_tool_call 与现有注册执行同一工具，无需更改注册。
- 同阶段原生入口：`python3 scripts/cargo_slot.py sh -c 'cargo fmt --all && cargo test -p vw-agent --lib tools::git_operations::tests:: -- --nocapture; result=$?; echo FOCUSED_EXIT=$result; cargo clippy --all-targets --all-features -- -D warnings; result=$?; echo CLIPPY_EXIT=$result; exit $result'`，日志 `/tmp/shiju-task7-final.log`。
- 红灯与最终验证均为槽位 2，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。所有入口已结束并释放自有槽位锁，缓存保留。

- 绿灯：完整 Git 工具聚焦 13 passed / 0 failed / 0 ignored，退出 0；任务要求的同一过滤器复验 4 passed / 0 failed / 0 ignored，退出 0（`/tmp/shiju-task7-proof.log`），四项新增测试全部命中。另有既有 macOS 链接器 unwind 大小提示，不影响行为测试。
- 整仓 Clippy 退出 101（`/tmp/shiju-task7-final.log`）：无关并发桌面编辑器改动 `crates/vw-desktop/src/app/components/preview_panel/view/editor_options.rs:34,47` 以及 `editor_options_tests.rs` 使用依赖不存在的 wrap_column/set_wrap_column API，共 9 项 E0599。遵守不修改其他会话变更的约定，保留任务供回归；未将该失败当作通过。
- 已运行 cargo fmt --all；差异审查未产生额外无关 Rust 改动，未触及容量、工位池或员工产品面；未运行 GUI、全量测试或额外全量构建。新增合并不自动 add、不清理用户目录、不强制 reset，不发送外部消息。

- 补充隔离检查：同槽位运行 `cargo clippy -p vw-agent --all-targets --all-features -- -D warnings` 退出 0（`/tmp/shiju-task7-proof.log`），本任务未引入编译错误或 Clippy 警告。后续推荐：修复无关桌面编辑器 API 错误后重跑整仓 Clippy；保留本任务文件，不删除。
