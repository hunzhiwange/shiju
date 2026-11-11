# 任务 39: 接通连接测试与取消API

批次：【批次 28】 依赖：批次 27 的任务 38。外部前置：核实母任务 1 的快照、母任务 2 的对应网关能力、母任务 3 的应用实例与路径。实际原生写入复用母任务 4 的事务，凭据解析复用母任务 6。

状态：已完成，待回归

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：启动返回作业标识；状态、事件、取消复用底座合同且只能访问所属批次。

来源任务：`.plans/2026-11-05-CC能力完整融合/11-MCP统一管理与应用分发.md`；来源入口：`commands/mcp.rs、gateway/api/handlers/mcp.rs`。路径基准见本目录来源与执行约定.md。

预计会话范围：只处理本任务一个主要验证故事；预计30–60分钟有效修改，编译等待另计。适配器任务仅一个应用的局部生命周期；验收任务仅一个应用或一种传输。不要顺带执行下一项。

## 文件

- 新建或复用：`crates/vw-agent/src/gateway/api/handlers/mcp_management_probe.rs`。
- 新建或复用：`crates/vw-agent/src/gateway/api/handlers/mcp_management_probe_tests.rs`。
- 新建或复用：`docs/vibewindow/agent-apps/mcp/evidence/任务39.md`。
- 修改：所属父模块注册文件与独立测试连线；若入口已被前置计划拆分，使用真实等价入口并在本任务记录，避免双实现。
- 修改：`crates/vw-agent/src/gateway/bootstrap.rs`、`crates/vw-agent/src/gateway/api/handlers/mod.rs`。

## 上下文

- 先阅读根AGENTS.md、plan.md、来源与执行约定.md以及真实调用者/工厂/相邻测试。代码导航使用仓库rustcodegraph技能。
- POST /v1/agent-apps/mcp/checks。本计划新增路由，沿用底座认证/成员授权/执行主机解析，不按Tauri命令名动态调用；集成测试经过真实router与认证中间件，覆盖未认证/跨租户拒绝。
- 实施前核实本卡列出的外部产物及直接前置任务的实际证据；缺失接口不得伪造。目标crate：vw-agent；聚焦使用默认features，不传--all-features；仓库Clippy使用全部features。只验收macOS。

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

## 测试计划

- 行为：启动返回作业标识；状态、事件、取消复用底座合同且只能访问所属批次。
- 失败验证测试：`crates/vw-agent/src/gateway/api/handlers/mcp_management_probe_tests.rs`，新增以`cc_mcp_t39_`为前缀的独立行为测试；测试名后缀按具体输入与预期结果命名。
- 失败验证命令：`cargo test -p vw-agent --lib cc_mcp_t39_ -- --nocapture`。
- 预期失败原因：前置实现未覆盖本任务行为，断言应在生产路径观察到缺失/错误结果；不能以未注册测试、编译错误或零测试冒充红灯。若已等价实现，先运行真实回归给证据，只补缺口。
- 通过验证命令：`cargo test -p vw-agent --lib cc_mcp_t39_ -- --nocapture`；至少1个有效测试通过，失败数0。
- 模拟策略：使用真实临时目录、实际底座存储、loopback HTTP/SSE与无秘密stdio夹具；仅外部账号/不可控网络可在传输边界替代，保留真实响应形状，不mock内部业务结果。

## 步骤

1. 验证依赖、读取入口和相邻测试；新增独立测试并由父模块`#[cfg(test)]`注册。
2. 运行失败验证并记录预期断言失败。
3. POST /v1/agent-apps/mcp/checks。本计划新增路由，沿用底座认证/成员授权/执行主机解析，不按Tauri命令名动态调用；集成测试经过真实router与认证中间件，覆盖未认证/跨租户拒绝。
4. 最小实现后运行`cargo fmt --all`，审查并恢复格式化波及的无关文件，再运行聚焦命令。
5. 运行仓库Clippy；仅在行为保持通过时局部整理，不新增无调用者抽象。将证据写入本任务指定正式文档。

## 验证

- 运行：`cargo test -p vw-agent --lib cc_mcp_t39_ -- --nocapture`；`cargo clippy --all-targets --all-features -- -D warnings`；`git diff --check`。
- 预期：聚焦测试至少1个且失败0，Clippy退出0；本次引入问题必须修复。无关编译错误只记录，不顺手修复。
- 所需证据：失败断言、通过输出、有效测试数、所有命令/退出码、实际槽位与绝对CARGO_TARGET_DIR、已审查差异、接口/实现符号与正式证据路径。

## 完成与阻塞

- 状态只用未开始、已完成、已阻塞、已完成，待回归。完成依赖当前证据，不凭编译成功或旧证据；测试与业务文件分开。
- 获得证据后标记已完成；有实际代码/测试/配置/文档变更时使用`$git-commit`只提交本任务内容；仅编号任务状态或删除无需提交。最终报告证据后删除本任务文件，正式证据保留。
- 尽力突破本任务阻塞；确实无法解决只在本任务标记已阻塞，记录缺失依赖、已运行检查与恢复条件。无关既有错误导致验证受阻可标已完成，待回归，保留任务文件，不能宣称通过。
- 不修改本计划或母计划的plan.md，不更新历史来源清单，不覆盖/提交工作区其它变更。新增需求或范围变化先询问是否记录回来源产物，不静默扩大母需求。

## 本轮执行记录

- 已完成macOS POST /v1/agent-apps/mcp/checks最小接线、独立cc_mcp_t39_测试及正式证据；复用agent_apps_diagnostics::start、network_probe::start_handshakes与DiagnosticJobs，不另造查询/取消接口、不弱化断言。plan.md未修改。
- 必需HTTP依赖为 `.plans/2026-11-05-故障转移连通检测与诊断/38-提供诊断结果与取消API.md`。核实当前正式router仅有POST启动路由，read_job/cancel_job尚未交付；DiagnosticJobs::snapshot/cancel已存在。按最新调度授权将本任务标记“已完成，待回归”，不在本批次等待其交付或重复已知无法通过的构建。
- 实际动态槽位6；CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-6，CARGO_BUILD_BUILD_DIR同值，CARGO_BUILD_JOBS=1。自身Cargo、测试及受控stdio子进程全部结束后已退出持有shell PID13857，确认自身slot-6.lock已释放，缓存保留。
- T=`cargo --config 'profile.test.package.vw-agent.opt-level=0' test -p vw-agent --lib cc_mcp_t39_ -- --nocapture`。红灯退出101，1个有效用例在真实router观察405、预期422。两次cargo fmt --all均退出0。首次绿灯因编译快照缺少上述前置路由而主动停止自身rustc，退出101、0测试，不计行为绿灯。新增HTTP用例尚未获得通过证据。
- `cargo clippy --all-targets --all-features -- -D warnings`已实际执行，自然退出101，被无关agent_apps_high_availability.rs:173/175/194的ProviderId.to_string E0599阻断；未修改该文件。
- 安全补充验证使用本轮红灯实际生成的 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6/debug/deps/vw_agent-7d22e0df937b4289`：`diagnostic_mcp_ --nocapture`退出0、5通过0失败；`cc_mcp_t31_ --nocapture`退出0、5通过0失败。覆盖真实loopback握手授权、取消、进程回收与修订隔离，不冒充任务39的HTTP绿灯。
- git diff --check退出0；固定来源commands/mcp.rs的git show|cmp退出0。正式完整证据：`docs/vibewindow/agent-apps/mcp/evidence/任务39.md`。
- 恢复条件：诊断任务38正式路由落地且高可用模块无关编译错误解决后，动态领取空闲槽位运行T、完整Clippy及git diff --check。至少1个有效任务39用例通过且Clippy通过后补齐证据、删除本任务文件。

## 实现阶段可见性修复

- 按调度要求修复任务42报告的E0603；共享诊断补丁将start退回私有。将正式共用提交边界明确为pub(super) async fn submit，诊断POST路由及MCP适配一并指向它，授权与执行函数体逐字比较不变。未修改诊断查询/取消或其他任务的临时验证补丁。
- 动态原子领取槽位2，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，CARGO_BUILD_BUILD_DIR同值，CARGO_BUILD_JOBS=1，持有脚本PID66165。
- cargo fmt --all -- --emit stdout --config skip_children=true退出0；函数体/适配器一致性断言退出0；git diff --check退出0。cargo clippy --all-targets --all-features -- -D warnings退出0（3分36秒），原私有函数E0603已消除。脚本自然结束并释放自身slot-2.lock，保留缓存。
- 本轮仅恢复实现连线，不提前执行完整延后行为回归；保留本任务“已完成，待回归”状态。

## 管理连接检查结果契约修复

- 按任务43反馈核对任务31和实际DTO：原管理入口错误复用了DiagnosticResult::Mcp握手报告，不携带connectionRevision。新增SubmissionRequest::McpConnectionChecks内部提交种类，复用原整批授权/执行主机解析；原DiagnosticStartRequest::Mcp仍走原握手报告，保留既有诊断调用者。
- 管理结果使用现有DiagnosticResult::McpConnectionCheck {scope, generation, item: McpConnectionCheckItem {id, connectionRevision, outcome}}；connectionRevision来自经当前目录验证的实际授权连接修订，generation为管理请求可选u64、默认0。执行复用任务30/31的McpCatalogStore::check_connection_inner与CheckRevision::Connection、真实工具发现及DiagnosticJobs；不在SDK造修订或成功结果。
- 保留McpProbeAccess的逐项授权、凭据/端点/stdio许可检查与发布前复核。底座ConnectionCheckHost增加可选账户存储（无凭据连接不强制建立账号库）和调用者重新授权回调；既有可信领域调用者保持原授权责任。新增独立connection_probe_tests.rs验证非初始修订、工具发现、旧诊断兼容、撤销权限、取消；原完整HTTP用例更新正式结果断言，仍延后统一回归。
- 本轮动态槽位8，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-8，CARGO_BUILD_BUILD_DIR同值，CARGO_BUILD_JOBS=1，持有脚本PID10481。cargo fmt --all -- --emit stdout --config skip_children=true退出0；仅格式化本次负责文件的rustfmt退出0。
- 聚焦命令cargo --config 'profile.test.package.vw-agent.opt-level=0' test -p vw-agent --lib cc_mcp_t39_revision_ -- --nocapture正在执行，首轮出现无关聚合退出依赖快照错误（AggregationExitRequest/Exited缺失；实际新API源码已有定义），不修改聚合模块，不冒充行为红灯。首轮完整Clippy随后顺序执行退出0（5分30秒）。

- 新增契约复验已执行：动态原子重新领取兼容槽位8，CARGO_TARGET_DIR同上，持有脚本PID55590；同一聚焦命令退出0，2通过0失败（构建40分54秒，运行0.26秒）。实际二进制target/rust-slot-8/debug/deps/vw_agent-7d22e0df937b4289分别执行diagnostic_mcp_和cc_mcp_t31_，均退出0，各5通过0失败，保留旧诊断及取消/修订语义。
- 原延后HTTP用例已按最新调度要求同步强类型解码DiagnosticJobIdentity/DiagnosticJobSnapshot，按McpConnectionCheck断言generation=17和真实connectionRevision；省略代次的独立批次断言0，另从旧诊断HTTP入口断言Mcp握手报告。原授权、批次隔离和取消断言保留，完整HTTP用例未提前运行。
- 最终检查动态原子领取槽位8，CARGO_TARGET_DIR同上，持有脚本PID2182；cargo fmt --all -- --emit stdout --config skip_children=true退出0，最终cargo clippy --all-targets --all-features -- -D warnings退出0（2分32秒），覆盖最新HTTP强类型断言的编译检查。所有自身进程自然结束，slot-8.lock已释放，缓存保留。任务状态仍为已完成，待回归。

- 本轮交接：正式McpConnectionCheck结果带实际scope、generation和item.connectionRevision/outcome；新增2项契约+旧诊断5项+批次5项均通过，最终全仓Clippy退出0。完整延后HTTP行为用例只做必要契约调整和编译检查，保留任务卡供计划末尾统一执行。

- 已隔离提交：25ac2d5a4（共享提交入口可见性）、f358714ed（正式修订结果及测试契约）。最终标准索引已刷新，本任务之外的工作区改动保留，未纳入提交。
