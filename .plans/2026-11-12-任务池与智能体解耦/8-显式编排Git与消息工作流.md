# 任务 8: 显式编排Git与消息工作流

> 验证调整（2026-11-13）：遵守 `.plans/AGENTS.md`；取消额外全量构建、原生验收窗口及截图验证，以聚焦行为测试和差异审查收尾。历史记录保留。

批次：【批次 8】 依赖批次 7；传递依赖批次 1–7

线性顺序：任务 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14；本任务不可与其他批次并行。

状态：未开始

目的：以现有工作流工具节点完成提交、合并与消息动作；没有步骤就不产生业务副作用。

来源任务：无；来源为用户本轮确认的重构目标。

预计会话范围：围绕“工作流仅执行显式 Git/消息节点并遵守依赖；普通任务与失败上游不自动提交、合并或通知。”一个验收故事。只沿列出的真实调用链修改，必要相邻依赖先调查、记录再修复；不叠加无关重构。

## 文件

- 修改：`crates/vw-agent/src/workflow/native_agent.rs`
- 修改：`crates/vw-agent/src/workflow/native_tests.rs`
- 修改：`crates/vw-agent/src/workflow/runner_tests.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/workflow.rs`
- 测试：`crates/vw-agent/src/gateway/api/handlers/workflow_tests.rs`
- 参考：`crates/vw-agent/src/tools/thread_tool/mod.rs`
- 参考：`crates/vw-agent/src/tools/agent_tool/communication.rs`
- 参考：`crates/vw-agent/src/plugins/compat/mcp_app_execution.rs`
- 新建：`docs/dev/task-pool/显式动作示例.md`

## 上下文

保留现有容量配置、工位池、预热/回收机制、员工交互、列表/像素办公室切换、展开收起、状态统计、确认与操作日志。员工与工位是现有智能体及运行的投射，只替换 worktree 绑定；不得借解耦删除这些产品架构、重置容量值或把容量限制成智能体定义数量。

工作流已有 tool/agent 节点与 NativeTools 权限检查；消息有同项目线程/团队工具及插件 MCP 执行入口。消息渠道必须取自用户已配置工具，不能假设已有通用跨渠道发送工具。

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

- 行为：工作流仅执行显式 Git/消息节点并遵守依赖；普通任务与失败上游不自动提交、合并或通知。
- 新增测试：`task_pool_actions_absent_steps_have_no_side_effects`。
- 新增测试：`task_pool_actions_explicit_git_steps_use_native_tool_permissions`。
- 新增测试：`task_pool_actions_explicit_message_requires_authorized_target`。
- 新增测试：`task_pool_actions_upstream_failure_skips_downstream_send`。
- 上述测试名为本任务明确要求新增的测试，不是已存在测试；分别落在“文件”列出的对应独立测试文件，并核对模块注册。
- 失败验证命令与通过验证命令相同，先新增测试再运行：

- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_pool_actions_ -- --nocapture`

- 预期失败原因：尚无这组显式动作链的可执行验收，现有任务池依靠内建收尾与状态推进执行 Git。
- 聚焦测试 features：使用各 crate 默认 features；vw-agent 默认包含 rag-pdf/firecrawl/web-fetch-html2md/web-fetch-plaintext/shell-ast/channel-lark；vw-desktop 默认空。禁止为避开本次错误临时关闭默认 features。
- 模拟策略：真实目录、Git 和 SQLite；仅替换 LLM/ACP 协议对端与外部消息网络。使用现有 global_async_env_lock 等夹具隔离全局状态，不调用真实外部收件人或生产服务。
- 测试命令必须实际命中新增测试；各 crate 分别至少一个非 ignored 测试，合计至少 4 个有效测试。零测试、仅编译或仅 JSON 形状比较不能视为本行为验收成功。

## 步骤

1. 用现有 WorkflowRuntime 解析并运行真实图，真实 Git 操作落临时仓库；外部消息发送只接本地录制传输边界。
2. 提供可直接使用的节点参数示例：普通非 Git 任务、显式 add/commit/merge、显式发送消息；遵循现有 YAML 解析器，不新增专用 workflow DSL 或全局模板注册。
3. 原生工作流调用已有工具与插件 MCP 执行链；如边界遗漏当前会话工具能力，最小补齐上下文传递，工具权限取交集，不把工作流输入的 full_access 当授权。
4. 同项目消息复用 send_message_to_thread 或团队消息语义；外部渠道通过已配置插件工具明确选择目标。未配置渠道/收件人、越租户/项目、工具被禁用均在发送前失败；不添加新的渠道实现。
5. 已成功副作用节点的重试/恢复不默认重放；用工作流既有运行状态记录结果。外部工具无幂等能力且投递结果未知时，保留待处理状态与证据，禁止声称 exactly-once 或自动重复发送。
6. 示例文档写明动作需显式授权与绑定工具，测试不得访问真实收件人；不导入用户现有工作流或替其发布。

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
