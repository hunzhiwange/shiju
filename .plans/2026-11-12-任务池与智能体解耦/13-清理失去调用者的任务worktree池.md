# 任务 13: 清理失去调用者的任务worktree池

批次：【批次 13】 依赖批次 12；传递依赖批次 1–12

线性顺序：任务 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14；本任务不可与其他批次并行。

状态：未开始

目的：移除任务调度对常驻 worktree 池的剩余接口与容量维护，保留按需 Git 工具。

来源任务：无；来源为用户本轮确认的重构目标。

预计会话范围：围绕“任务调度与员工展示不再预建/维护 worktree；旧池入口不再能被当作员工容量，按需工具正常工作且旧目录不被自动删除。”一个验收故事。只沿列出的真实调用链修改，必要相邻依赖先调查、记录再修复；不叠加无关重构。

## 文件

- 修改或删除：`crates/vw-agent/src/task_worktree_pool/mod.rs`
- 修改或删除：`crates/vw-agent/src/task_worktree_pool/tests.rs`
- 修改或删除：`crates/vw-agent/src/agent_scheduler/resources/worktree.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/resources/mod.rs`
- 修改或删除：`crates/vw-agent/src/gateway/api/handlers/task_pool.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/mod.rs`
- 修改：`crates/vw-agent/src/lib.rs`
- 修改：`crates/vw-gateway-client/src/client/task_pool_api.rs`
- 修改：`crates/vw-api-types/src/task.rs`
- 修改：`crates/vw-cli/src/handlers/api/task_pool.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 修改：`crates/vw-desktop/src/app/message/task_board/helpers/execution.rs`
- 修改：`crates/vw-desktop/src/app/message/task_board/message.rs`
- 修改：`crates/vw-desktop/src/app/state/app_state/fields_navigation.rs`
- 新建测试：`crates/vw-agent/src/agent_scheduler/pool_cleanup_tests.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/mod.rs`
- 测试：`crates/vw-gateway-client/src/client/project_api_tests.rs`
- 测试：`crates/vw-cli/src/handlers/api/project_tests.rs`
- 测试：`crates/vw-cli/src/handlers/api/tests.rs`

## 上下文

前序任务已迁出执行、Git 锁与员工 UI 调用者。maintain 当前 target_idle 至少 1，acquire 还会预热，CLI/API 暴露池维护。必须先做 callers/impact，再按真实调用关系删代码。

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

- 行为：任务调度与员工展示不再预建/维护 worktree；旧池入口不再能被当作员工容量，按需工具正常工作且旧目录不被自动删除。
- 新增测试：`task_pool_cleanup_scheduler_never_calls_pool_routes`。
- 新增测试：`task_pool_cleanup_retired_maintenance_is_not_registered`。
- 新增测试：`task_pool_cleanup_explicit_worktree_tool_still_operates`。
- 上述测试名为本任务明确要求新增的测试，不是已存在测试；分别落在“文件”列出的对应独立测试文件，并核对模块注册。
- 失败验证命令与通过验证命令相同，先新增测试再运行：

- `cargo test -p vw-agent --lib task_pool_cleanup_ -- --nocapture`
- `cargo test -p vw-gateway-client --lib task_pool_cleanup_ -- --nocapture`
- `cargo test -p vw-cli --lib task_pool_cleanup_ -- --nocapture`

- 预期失败原因：旧 maintain 路由、CLI 命令与桌面容量字段仍保留常驻池业务语义。
- 具体测试归属：scheduler_never_calls_pool_routes 与 explicit_worktree_tool_still_operates 放入新建 pool_cleanup_tests.rs 并在 agent_scheduler/mod.rs 注册；retired_maintenance_is_not_registered 在 CLI api/tests.rs 验证参数解析拒绝，在 client/project_api_tests.rs 增加同前缀的普通 worktree 管理请求回归。退役模块移除时一并移除其旧测试注册，不能把新测试放进将被删除的测试模块。
- 聚焦测试 features：使用各 crate 默认 features；vw-agent 默认包含 rag-pdf/firecrawl/web-fetch-html2md/web-fetch-plaintext/shell-ast/channel-lark；vw-desktop 默认空。vw-cli 默认 agent-defaults/memory-postgres/memory-mariadb/channel-lark；vw-gateway-client 无 features 表。禁止为避开本次错误临时关闭默认 features。
- 模拟策略：真实目录、Git 和 SQLite；仅替换 LLM/ACP 协议对端与外部消息网络。使用现有 global_async_env_lock 等夹具隔离全局状态，不调用真实外部收件人或生产服务。
- 测试命令必须实际命中新增测试；各 crate 分别至少一个非 ignored 测试，合计至少 3 个有效测试。零测试、仅编译或仅 JSON 形状比较不能视为本行为验收成功。

## 步骤

1. 重新检查所有池服务、DTO、客户端与 CLI 的实际调用者，确认任务 6/7 的工具和目标锁已不依赖池。
2. 先新增黑盒路由/CLI 测试，验证任务启动不访问池、退役维护入口不可达，以及 EnterWorktree 仍然创建真实资源。
3. 删除无人使用的预热、池容量、槽位员工模型和旧路由注册；同步 DTO/client/CLI 与桌面消息/状态字段。确有独立普通 Git 调用者的能力留在现有 Git 子系统，禁止保留任务池兼容旁路。
4. 若原有池专属测试文件随服务删除，删除前把仍适用的真实工作树安全行为迁入现有 enter/exit_worktree 测试；新增验收放入本任务定义的 pool_cleanup_tests.rs，并在 agent_scheduler/mod.rs 注册，客户端/CLI 测试放入仍保留的 project_api_tests.rs、project_tests.rs 和 api/tests.rs。
5. 不自动执行 prune/reset/clean、不删除磁盘已有池目录；在最终文档列出现有显式清理入口与保留产物检查方法。
6. 清除只服务于本次退役链的死代码与注释；对普通聊天、Git 面板、项目 worktree 管理保持聚焦回归。

## 验证

1. 在已领取槽位中记录新增测试的预期失败与原因；不把编译环境错误当作预期业务失败。
2. Rust 修改完成先运行 `cargo fmt --all`，检查并排除其对无关文件的格式化，不覆盖别人正在编辑的内容。
3. 运行上述各条通过验证命令，记录测试名称、有效数量、成功/失败/ignored 数和退出码。
4. 运行 `cargo clippy --all-targets --all-features -- -D warnings`，记录退出码；修复本任务引入的问题，无关既有失败仅记录具体文件与错误。
5. 审查本任务差异、安全边界、调用者注册及资源释放，确认没有真实对外消息、发布或自动删除用户目录。

涉及多个 crate 的修改是同一退役调用链，不能拆成单独提交后留下无法编译的客户端。不要顺手删除通用 outbox、资源租约或非任务池工作树能力。

预期：聚焦行为测试有效通过，无本次引入的编译错误或警告；全仓检查如被无关改动阻断，必须提供阻断证据并保留待回归状态。

所需证据：失败前后命令与输出摘要、实际测试数、退出码、相关符号和改动路径；实际槽位和绝对路径 CARGO_TARGET_DIR；涉及 UI 时附实际渲染截图及日志。不得运行无过滤 `cargo test`、`cargo fmt --all -- --check`、wasm 构建或 Makefile 命令。

## 完成

本任务只有 `未开始`、`已完成`、`已阻塞`、`已完成，待回归` 四种状态。每个完成结论必须有当前证据；依赖批次完成后先核对代码与提交，不以已删除任务文件单独证明依赖成功。

有实际代码、测试、配置或文档变更时，使用 `$git-commit` 检查 staged/unstaged/untracked，排除其他任务与用户变更后以中文提交；仅本文件状态更新或删除不提交。证据齐全则标记已完成，在最终回复报告证据后删除本文件；无关环境或既有错误阻断验证则标记已完成，待回归并保留本文件；本任务本身仍不满足目标则标记已阻塞，记录调查、替代路径和所需输入。不要修改 plan.md 或把进度写入 prompt.md。
