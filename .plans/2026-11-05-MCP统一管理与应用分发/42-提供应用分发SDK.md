# 任务 42: 提供应用分发SDK

批次：【批次 31】 依赖：批次 30 的任务 41。外部前置：核实母任务 1 的快照、母任务 2 的对应网关能力、母任务 3 的应用实例与路径。实际原生写入复用母任务 4 的事务，凭据解析复用母任务 6。

状态：已完成，待回归

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：SDK能类型化传递导入、绑定、批量启停、重新同步及底座任务引用。

来源任务：`.plans/2026-11-05-CC能力完整融合/11-MCP统一管理与应用分发.md`；来源入口：`src/lib/api/mcp.ts、client/mcp_api.rs`。路径基准见本目录来源与执行约定.md。

预计会话范围：只处理本任务一个主要验证故事；预计30–60分钟有效修改，编译等待另计。适配器任务仅一个应用的局部生命周期；验收任务仅一个应用或一种传输。不要顺带执行下一项。

## 文件

- 新建或复用：`crates/vw-gateway-client/src/client/mcp_management_distribution.rs`。
- 新建或复用：`crates/vw-gateway-client/src/client/mcp_management_distribution_tests.rs`。
- 新建或复用：`docs/vibewindow/agent-apps/mcp/evidence/任务42.md`。
- 修改：所属父模块注册文件与独立测试连线；若入口已被前置计划拆分，使用真实等价入口并在本任务记录，避免双实现。

## 上下文

- 先阅读根AGENTS.md、plan.md、来源与执行约定.md以及真实调用者/工厂/相邻测试。代码导航使用仓库rustcodegraph技能。
- 复用GatewayClient传输、认证、目标上下文和错误合同；通过真实loopback测试服务器记录路径/方法/JSON/响应解码，既有mcp_inspect/mcp_app_call不退化。
- 实施前核实本卡列出的外部产物及直接前置任务的实际证据；缺失接口不得伪造。目标crate：vw-gateway-client；聚焦使用默认features，不传--all-features；仓库Clippy使用全部features。只验收macOS。

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

- 行为：SDK能类型化传递导入、绑定、批量启停、重新同步及底座任务引用。
- 失败验证测试：`crates/vw-gateway-client/src/client/mcp_management_distribution_tests.rs`，新增以`cc_mcp_t42_`为前缀的独立行为测试；测试名后缀按具体输入与预期结果命名。
- 失败验证命令：`cargo test -p vw-gateway-client --lib cc_mcp_t42_ -- --nocapture`。
- 预期失败原因：前置实现未覆盖本任务行为，断言应在生产路径观察到缺失/错误结果；不能以未注册测试、编译错误或零测试冒充红灯。若已等价实现，先运行真实回归给证据，只补缺口。
- 通过验证命令：`cargo test -p vw-gateway-client --lib cc_mcp_t42_ -- --nocapture`；至少1个有效测试通过，失败数0。
- 模拟策略：使用真实临时目录、实际底座存储、loopback HTTP/SSE与无秘密stdio夹具；仅外部账号/不可控网络可在传输边界替代，保留真实响应形状，不mock内部业务结果。

## 步骤

1. 验证依赖、读取入口和相邻测试；新增独立测试并由父模块`#[cfg(test)]`注册。
2. 运行失败验证并记录预期断言失败。
3. 复用GatewayClient传输、认证、目标上下文和错误合同；通过真实loopback测试服务器记录路径/方法/JSON/响应解码，既有mcp_inspect/mcp_app_call不退化。
4. 最小实现后运行`cargo fmt --all`，审查并恢复格式化波及的无关文件，再运行聚焦命令。
5. 运行仓库Clippy；仅在行为保持通过时局部整理，不新增无调用者抽象。将证据写入本任务指定正式文档。

## 验证

- 运行：`cargo test -p vw-gateway-client --lib cc_mcp_t42_ -- --nocapture`；`cargo clippy --all-targets --all-features -- -D warnings`；`git diff --check`。
- 预期：聚焦测试至少1个且失败0，Clippy退出0；本次引入问题必须修复。无关编译错误只记录，不顺手修复。
- 所需证据：失败断言、通过输出、有效测试数、所有命令/退出码、实际槽位与绝对CARGO_TARGET_DIR、已审查差异、接口/实现符号与正式证据路径。

## 完成与阻塞

- 状态只用未开始、已完成、已阻塞、已完成，待回归。完成依赖当前证据，不凭编译成功或旧证据；测试与业务文件分开。
- 获得证据后标记已完成；有实际代码/测试/配置/文档变更时使用`$git-commit`只提交本任务内容；仅编号任务状态或删除无需提交。最终报告证据后删除本任务文件，正式证据保留。
- 尽力突破本任务阻塞；确实无法解决只在本任务标记已阻塞，记录缺失依赖、已运行检查与恢复条件。无关既有错误导致验证受阻可标已完成，待回归，保留任务文件，不能宣称通过。
- 不修改本计划或母计划的plan.md，不更新历史来源清单，不覆盖/提交工作区其它变更。新增需求或范围变化先询问是否记录回来源产物，不静默扩大母需求。

## 本轮执行记录（2026-11-12）

- 实现类型化导入、绑定、批量启停及重同步 SDK；底座任务ID可复用既有授权查询/取消，公开DTO与真实网关合同对应。
- 动态原子领取槽位2，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，CARGO_BUILD_BUILD_DIR同值，CARGO_BUILD_JOBS=1。阶段结束确认自身子进程退出后释放自己的锁，保留缓存。
- cargo fmt --all 三次退出0。首轮夹具编译E0277退出101不计红灯；修正后 cargo test -p vw-gateway-client --lib cc_mcp_t42_ -- --nocapture 真实红灯4通过、1绑定方法POST/PUT断言失败（退出101）；最终6通过、0失败（退出0）。
- cargo test -p vw-gateway-client --lib cc_mcp_t41_ -- --nocapture 退出0，5通过，包含既有mcp_inspect/mcp_app_call生产路径。git diff --check 退出0。
- cargo clippy --all-targets --all-features -- -D warnings 已执行，范围外 mcp_management_probe.rs:34 调用被并发改为私有的 agent_apps_diagnostics::start，E0603；诊断后停止本任务自身clippy-driver，Cargo退出101，不宣称全仓检查通过。
- 保留本卡供统一回归；恢复条件是诊断模块所有者修复可见性后，重跑全仓Clippy及本任务聚焦命令。正式证据：docs/vibewindow/agent-apps/mcp/evidence/任务42.md。未修改只读plan.md或其它任务内容。

## 桌面48反馈：完整作业快照契约修复（2026-11-12）

- 新增 `mcp_management_resync_status` / `mcp_management_resync_cancel`，返回正式 `vw_api_types::mcp_management::McpResyncOperationStatus`（operation/targets/outcome），复用原生命周期传输、认证及AgentAppsLifecycleError；旧通用SDK行为不变。网关Job::response使用同一DTO，保持GET与取消后的真实完整快照形状。
- 分别审查生命周期read和cancel：前者直接返回OwnedOperation::Mcp快照，后者cancel_and_wait并重新授权后调用同一响应函数；终态取消保留已有结果。正式证据记录了实际来源函数及现有后端router断言，不以手写成功夹具推测合同。
- 动态原子领取槽位16，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-16，CARGO_BUILD_BUILD_DIR同值，CARGO_BUILD_JOBS=1；同槽连续验证，保留缓存，仅阶段结束后释放自有锁。
- 两次cargo fmt --all退出0；新增真实loopback红灯执行1项、targets解码后丢失断言失败（退出101）。修复后cargo test -p vw-gateway-client --lib cc_mcp_t42_ -- --nocapture退出0，8通过；cargo test -p vw-gateway-client --lib agent_apps_lifecycle_api_tests -- --nocapture退出0，4通过。
- 按追加指令，原卡继续保留已完成，待回归；完整回归留到计划末尾。正式证据仍为docs/vibewindow/agent-apps/mcp/evidence/任务42.md。
- 本轮全仓Clippy已实际执行，推进到vw-agent后逾16分钟未完成，无Rust错误/警告诊断；共享环境交换区使用63430.31MB，自身driver处于I/O等待。按完整回归延后的追加要求，确认PID链和槽位16输出路径后，仅SIGTERM停止本任务自身driver，Cargo退出101（signal15），不计通过，不冒称源码错误。自身子进程结束后已释放自有锁并保留缓存。
- 统一回归需重跑cargo clippy --all-targets --all-features -- -D warnings、cc_mcp_t42_及已有cc_mcp_t38_生产router场景；本轮SDK证据为8项分发测试及4项通用兼容性回归通过，网关序列化改动未取得当前全仓检查完成证据。git diff --check退出0；完整记录见正式证据。
