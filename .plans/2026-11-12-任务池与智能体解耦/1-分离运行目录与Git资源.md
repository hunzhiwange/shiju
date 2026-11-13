# 任务 1: 分离运行目录与Git资源

> 验证调整（2026-11-13）：遵守 `.plans/AGENTS.md`；取消额外全量构建、原生验收窗口及截图验证，以聚焦行为测试和差异审查收尾。历史记录保留。

批次：【批次 1】 无

线性顺序：任务 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14；本任务不可与其他批次并行。

状态：已完成，待回归

目的：让默认工作区准备只依赖已授权项目目录，运行目录不再要求 Git 分支或池槽位。

来源任务：无；来源为用户本轮确认的重构目标。

预计会话范围：围绕“已注册非 Git 目录可准备工作区；不存在目录、跨项目目录必须失败，准备期间不访问池或初始化 Git。”一个验收故事。只沿列出的真实调用链修改，必要相邻依赖先调查、记录再修复；不叠加无关重构。

## 文件

- 修改：`crates/vw-agent/src/agent_scheduler/runner/local.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/runner/gateway_executor.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/agent_scheduler.rs`
- 修改：`crates/vw-api-types/src/agent_scheduler.rs`
- 测试：`crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs`
- 测试：`crates/vw-api-types/src/agent_scheduler_tests.rs`
- 参考：`crates/vw-agent/src/agent_scheduler/store/artifacts.rs`
- 参考：`crates/vw-agent/src/agent_scheduler/store/artifacts_tests.rs`
- 参考：`crates/vw-agent/src/agent_scheduler/store/runs.rs`

## 上下文

保留现有容量配置、工位池、预热/回收机制、员工交互、列表/像素办公室切换、展开收起、状态统计、确认与操作日志。员工与工位是现有智能体及运行的投射，只替换 worktree 绑定；不得借解耦删除这些产品架构、重置容量值或把容量限制成智能体定义数量。

LocalAgentWorkspace、lease_gateway_local_agent_workspace_for_project 和 workspace_context_from_value 当前强制槽位及分支。默认 lease 改为目录上下文，真正 worktree 保持可选资源；不创建新员工模型。StoredAgentRun 没有 metadata 字段，运行目录使用现有按 run_id 关联的任务产物存储。

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

- 行为：已注册非 Git 目录可准备工作区；不存在目录、跨项目目录必须失败，准备期间不访问池或初始化 Git。
- 新增测试：`task_pool_workspace_non_git_uses_registered_directory`。
- 新增测试：`task_pool_workspace_rejects_missing_or_foreign_directory`。
- 新增测试：`task_pool_workspace_serializes_without_git_metadata`。
- 上述测试名为本任务明确要求新增的测试，不是已存在测试；分别落在“文件”列出的对应独立测试文件，并核对模块注册。
- 失败验证命令与通过验证命令相同，先新增测试再运行：

- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_pool_workspace_ -- --nocapture`
- `python3 scripts/cargo_slot.py cargo test -p vw-api-types --lib task_pool_workspace_ -- --nocapture`

- 预期失败原因：现有准备链调用 acquire_and_prepare，非 Git 项目失败；DTO 无法表达仅有目录的工作区。
- 聚焦测试 features：使用各 crate 默认 features；vw-agent 默认包含 rag-pdf/firecrawl/web-fetch-html2md/web-fetch-plaintext/shell-ast/channel-lark；vw-desktop 默认空。禁止为避开本次错误临时关闭默认 features。
- 模拟策略：真实目录、Git 和 SQLite；仅替换 LLM/ACP 协议对端与外部消息网络。使用现有 global_async_env_lock 等夹具隔离全局状态，不调用真实外部收件人或生产服务。
- 测试命令必须实际命中新增测试；各 crate 分别至少一个非 ignored 测试，合计至少 3 个有效测试。零测试、仅编译或仅 JSON 形状比较不能视为本行为验收成功。

## 步骤

1. 为非 Git、Git 普通目录、缺失目录与越界路径编写准备阶段测试，不在本任务断言完整执行成功。
2. 调整 LocalAgentWorkspace 为必需路径与可选 Git 资源信息；具体可选字段在现有类型中定义，不用空字符串或虚构分支表达无 Git。更新确有调用关系的构造者和测试夹具，保持全仓可编译。
3. 默认准备返回项目规范化目录；不按 vcs 标记、任务标题或执行器身份自动申请 worktree。继续从注册项目与租户上下文验证路径。
4. 在准备成功时通过现有 record_task_artifact 写入 kind 为 execution_workspace 的按 run_id 关联产物，载荷包含必需 path 和可选 worktree 信息；API 从当前 run 的该产物读取运行目录。原 run_workspaces 表只记录真正的 Git 槽位，避免为普通目录修改旧表约束或添加迁移。统一读写映射，不伪造 selected_worktree 字段，不添加不存在的 run metadata 属性。
5. 旧 Git 收尾在需要实际 Git 资源的分支中明确校验，完整普通任务成功由任务 2 完成；不能通过跳过错误假称成功。仅修改必需构造者、DTO 和运行产物读取以保持可编译，不在此阶段改业务终态。
6. 运行测试、API 序列化测试和 clippy；审查未触碰数据库结构与现有 worktree 目录。

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

## 本轮执行记录（2026-11-13）

- 无前置批次；执行技能的常规阻碍停止规则按用户授权覆盖。
- LocalAgentWorkspace 与 API DTO 保留必需 path，将 slot_id/branch 改为 Option；默认准备校验实际目录与注册项目目录/沙箱的规范化路径相等，不再申请池资源。
- execution_workspace 使用现有 SQLite/Postgres 产物 API，按 task/run 关联；旧表仅记录真实 Git 槽位，未修改数据库结构。旧 Git 收尾显式拒绝缺失资源，完整普通任务收尾留给任务 2。
- 必要相邻适配：local_e2e_tests、local_tests 和桌面 task_board/update 的构造者/可选字段读取。保留容量、工位、预热回收和员工交互全部架构。
- 红灯：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_pool_workspace_ -- --nocapture`（入口中顺序调用），槽位 2，退出 101，2 failed / 0 passed / 0 ignored；真实业务失败为 unexpected acquire call，以及非法目录触发 acquire 计数断言。日志 `/tmp/shiju-task1-red-agent.log`。API 首次运行发生在类型调整后，通过 1 个测试，未记录其独立修改前红灯。
- 已通过整仓 Clippy（`cargo clippy --all-targets --all-features -- -D warnings`，入口槽位 6，退出 0）；新增 SQLite 产物测试之后重跑完整验证中。
- 已执行 cargo fmt --all、git diff --check；其它会话变更不暂存，不覆盖。
- 最终验证入口使用槽位 2，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。命令顺序为 cargo fmt --all、两 crate 的 task_pool_workspace_ 聚焦测试、vw-agent 的 agent_scheduler::runner:: 聚焦回归、整仓 Clippy。

### 最终证据（2026-11-14）

- 默认 profile 原生验收（槽位 2）：`cargo test -p vw-agent --lib task_pool_workspace_ -- --nocapture` 3 passed / 0 failed / 0 ignored，退出 0；`cargo test -p vw-api-types --lib task_pool_workspace_ -- --nocapture` 1 passed / 0 failed / 0 ignored，退出 0。日志 `/tmp/shiju-task1-final-agent.log`、`/tmp/shiju-task1-final-api.log`。
- 扩展回归复用已生成的原生测试二进制 `target/rust-slot-2/debug/deps/vw_agent-346fac923f6b2351`：`agent_scheduler::runner::` 20 passed / 0 failed / 1 ignored；`gateway_local_executor_prepares_git_project_without_leasing_worktree` 1 passed / 0 failed / 0 ignored，均退出 0。原 `cargo test ... agent_scheduler::runner::` 被其它会话依赖更新触发重编译后主动停止（退出 101），不记为测试通过。
- 本地运行器扩展回归曾发现 Git 日志缺失 slot/branch，已恢复原日志内容，普通目录改用 execution workspace 日志。
- 修复后原生验证使用同一入口槽位 7，两个目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-7`。为避免优化巨型 lib-test 再次耗时，采用仅本 crate 的命令行 profile 覆盖（未修改任何配置、未关闭默认 features）：`cargo test --config profile.test.package.vw-agent.opt-level=0 -p vw-agent --lib task_pool_workspace_ -- --nocapture`，3 passed / 0 failed / 0 ignored，退出 0；`cargo test --config profile.test.package.vw-agent.opt-level=0 -p vw-agent --lib agent_scheduler::local_runner_tests:: -- --nocapture`，14 passed / 0 failed / 0 ignored，退出 0。日志 `/tmp/shiju-task1-repaired-agent.log`、`/tmp/shiju-task1-repaired-local.log`。
- 修复后标准整仓 `cargo clippy --all-targets --all-features -- -D warnings` 退出 101，因无关的编辑器 wrap_column 接口编译失败：`crates/vw-desktop/src/app/components/preview_panel/view/editor_options.rs:34` 缺少 `wrap_column`；`crates/vw-desktop/src/app/message/editor.rs:276` 缺少 `set_wrap_column`；`editor_options_tests.rs:153` 等共 9 个测试编译错误。日志 `/tmp/shiju-task1-repaired-clippy.log`。这些来自其它会话编辑器改动，本任务不修改。早期槽位 6 整仓 Clippy 曾退出 0，但不能替代最终回归。
- 测试链接器报告既有巨型测试目标 `__eh_frame section too large` 提示，未阻止测试运行；未将其当作 Rust/Clippy 成功证据。
- git diff --check 通过；未调整容量、工位、交互架构、数据库结构，不创建/删除用户 worktree，不发外部消息。其余会话改动不提交。
- 当前状态保留为已完成，待回归。下一步：编辑器依赖接口修复后运行标准整仓 Clippy，成功后再删除本任务文件并进入任务 2。

- 本任务入口进程均已结束，自有槽位锁已释放，缓存保留。
