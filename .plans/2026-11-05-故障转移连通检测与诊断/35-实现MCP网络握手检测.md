# 任务 35: 实现MCP网络握手检测

批次：【批次 3】 依赖：批次 2 的任务 3。同批实施与资源规则见 prompt.md。

状态：已阻塞

## 本次核查

- 直接依赖任务 3 的契约提交 `b6fb3a2fc` 已合入当前分支，任务 1、2 的证据提交 `862fdb4d2`、`1cd32bb00` 也已合入。通过 `git merge-base --is-ancestor <提交> HEAD` 逐一核对，均退出 0。`Cargo.toml` 已注册 `vw-agent-apps`，`c9ebc4319` 已交付 `DiagnosticJobs::start` 和跨任务并发许可。其调用契约要求调用者先解析已授权配置，再提交探测。
- 用 `rustcodegraph status`、`rustcodegraph explore` 和源码搜索核对授权、配置存储及传输。诊断请求的 `server_ids` 使用 `McpRecordId`。当前 `McpCatalogStore` 只有新增和更新入口，私有连接定义未提供执行端读取或按应用和主机授权的解析入口，`McpCredentialRef` 也尚未接到该目录的执行链。通用主机授权没有把 MCP 记录绑定到本次诊断作用域。
- 已核对 `tools/mcp_common`、`tools/mcp_inspect.rs`、相邻测试及上游 `SameOriginSseClient`。现有工作区加载器按运行时服务名查找，检查入口还会枚举服务能力。这些入口无法证明目录 ID 在指定租户、项目、主机和应用下获准执行。
- 恢复条件是母任务 11 提供已有记录在指定执行主机上的授权解析入口，能按 `McpRecordId` 返回当前连接配置并在同一边界解析凭据引用。具备该最小接口后即可恢复，无需等待配置分发全部完成。
- 因前置接口缺失，在写测试前停止。未运行 `cargo fmt --all`、`cargo test -p vw-agent --lib diagnostic_mcp_http_handshake_is_scoped -- --nocapture` 或 `cargo clippy --all-targets --all-features -- -D warnings`。有效行为测试 0 项，Cargo 槽位及 `CARGO_TARGET_DIR` 不适用。

## 恢复复核

- 基线 `e2910aad5`。执行 `rustcodegraph status`、`rustcodegraph query McpCatalogStore --limit 12` 和 `rustcodegraph query authorize_mcp --limit 12`，再用 `git log`、`git status`、`rg` 和源码读取核对提交及共享工作区。目录所有者已合入任务 6 的创建接口。工作区中的任务 7 更新接线只校验目录作用域和修订，保留凭据引用。它没有供诊断使用的授权读取。通用 `access::authorize_scoped` 验证成员和目标主机，现有能力分支没有 MCP 记录执行许可。已有 MCP 插件 OAuth 许可仅适用于插件来源与会话。
- 缺少的授权产物由母任务 11 的连接作业入口提供（子任务 39）。任务 35 需要它与目录所有者共享的执行端边界：按已验证的成员、租户、项目、主机和应用批准具体 `McpRecordId`，由 `McpCatalogStore` 在获准的目录作用域读取该记录的当前连接定义及修订。目录作用域隔离只约束存储键，不能据此授予探测许可。公共连接作业路由和完整配置分发无需先落地。
- 缺少的凭据产物由母任务 11 子任务 9 的 `crates/vw-agent/src/agent_apps/mcp/credentials.rs` 提供。需要由该边界在目标主机按获准记录和目标端点解析 `McpCredentialRef`，再交付传输所需的 headers。母任务 6 的 `AuthAccountStore::{copilot_session,codex_access_token,xai_access_token}` 可读取对应服务的账号凭据，但没有授权将其发往 MCP 记录 URL。直接调用账号访问器或以通用主机读权限代替记录许可都会扩大权限，当前无法用读取接线安全补齐。
- 本轮只更新本卡。未运行 Cargo，未领槽位。待记录执行许可和端点约束的凭据解析就绪，再补目录读取接线和红绿握手验证。

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：MCP HTTP/SSE只做短期握手，取消关闭连接且不调用工具。

来源任务：`.plans/2026-11-05-CC能力完整融合/9-故障转移连通检测与诊断.md`；具体来源：services/mcp_connectivity.rs SameOriginSseClient；现有 tools/mcp_common。来源仓库根目录为 `/Users/xiangmin/worker/code/cc-switch/`，不要修改上游。

预计会话范围：只交付“实现MCP网络握手检测”这一项，通常一个聚焦会话；不执行下一任务、不展开整个母能力。若实际工作需要独立设计或第二个验证故事，记录本任务阻塞与证据，先确认拆分，不越界承接前置计划。

## 文件

- 新建或修改（计划落点，当前不存在者由本任务新建）：`crates/vw-agent/src/gateway/agent_apps_mcp_probe.rs`。
- 独立测试文件：`crates/vw-agent/src/gateway/agent_apps_mcp_probe_tests.rs`；新增用例：`diagnostic_mcp_http_handshake_is_scoped`。
- 仅允许必要的直接父模块注册；API任务另修改 `crates/vw-agent/src/gateway/bootstrap.rs` 与 `api/handlers/mod.rs`；SDK任务另注册 `client/mod.rs`。已有等价模块则就地接入并在本文件记录真实路径，不复制第二实现。

## 上下文

- 先读根 AGENTS.md、`skills/rustcodegraph/SKILL.md` 和本计划 `来源映射.md`，查真实调用者及相邻测试。
- 实施前置：按任务 1–3 的合同核实本任务使用的真实产物。诊断使用已交付的授权与执行端，故障切换和运行时联动使用已交付的常驻路由接口。缺少本任务必需依赖则记录已阻塞，不把整个前置能力塞入本会话。 MCP配置管理仍由母任务11负责；本任务仅消费现有可用配置，不等待或重做全部分发能力。
- 仅macOS验收；不要求其他系统实机、交叉编译或wasm。实机与原生窗口由用户人工验收，仅记录简短文字结果。
- 新模块与API名称是本计划明确拟新增的设计；下列命令是测试注册完成后的验证契约，不声称当前可运行。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用原仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-20`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建原仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–20。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从原仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 20 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从原仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

适用 crate：`vw-agent`；聚焦测试使用该 crate 当前默认 features（不传 `--no-default-features`），Clippy使用workspace全部features。`vw-agent-apps`须先由母任务2注册，禁止在未注册时声称测试已运行。

## 测试计划

- 行为：MCP HTTP/SSE只做短期握手，取消关闭连接且不调用工具。
- 失败验证测试：先在 `crates/vw-agent/src/gateway/agent_apps_mcp_probe_tests.rs` 新增 `diagnostic_mcp_http_handshake_is_scoped`，显式挂到现有测试模块，禁止内联业务文件或只检验mock次数。
- 失败验证命令：`cargo test -p vw-agent --lib diagnostic_mcp_http_handshake_is_scoped -- --nocapture`。
- 预期失败原因：真实调用链当前缺少本任务行为，观察“MCP HTTP/SSE只做短期握手，取消关闭连接且不调用工具。”对应断言失败；仅找不到crate/模块或编译失败不是行为红灯，先完成最小测试接线再取得断言失败。
- 通过验证命令：`cargo test -p vw-agent --lib diagnostic_mcp_http_handshake_is_scoped -- --nocapture`；有效匹配至少1项，0项不能通过。
- 模拟策略：领域状态使用真实结构与事务；HTTP/SSE/MCP使用隔离本地真实协议服务，只有外部供应商/时钟/故障边界受控替代。网络请求数、持久结果和进程退出必须实际观察；标记受控证据，不冒充真实服务商。

## 步骤

1. 阅读真实调用者与相邻测试，新增上述失败测试并执行失败验证，保存非零测试的预期断言失败。
2. 复用现有 rmcp 与 tools/mcp_common 传输底层，已授权MCP配置ID在执行主机解析；母任务11拥有配置分发，本任务只消费其最终配置。
3. 覆盖streamable HTTP和SSE、同源重定向与凭据隔离、30秒单项超时、跨批次并发3；外部协议兼容以当前rmcp实际能力为准，不原样复制上游不同版本API。
4. 真实本地MCP服务记录 initialize 和 tools/call，后者必须0；失败结果不暴露URL查询、headers与原始错误。
5. 实现测试证明范围内的最小变更；运行 `cargo fmt --all`，审查并排除无关格式化差异，再执行通过验证与规定Clippy。仅在保持通过时重构。

## 验证

- 运行：`cargo test -p vw-agent --lib diagnostic_mcp_http_handshake_is_scoped -- --nocapture`，再运行 `cargo clippy --all-targets --all-features -- -D warnings`。
- 预期：聚焦测试至少1项通过、无失败；新行为的断言覆盖本文件目的，Clippy无本任务新增错误/警告。
- 所需证据：红灯断言、通过输出、有效测试数量、确切命令/退出码、实际槽位/绝对CARGO_TARGET_DIR、真实文件/符号及已审查差异；API/SDK另附请求与状态，进程任务附存活/退出证据。

## 完成

- 仅在本任务取得当前验证证据后标记 `已完成`。若无关既有编译错误阻断验证，记录检查后可标 `已完成，待回归` 并保留任务；缺少本任务的必需接口或自动验证证据，标 `已阻塞` 并保留，不能用信心代替证据。
- 阻塞和进度只写本任务文件，不改只读 `plan.md`，不把 `prompt.md` 当进度来源；不静默删减来源需求。发现新需求先询问是否记录回来源产物。
- 有实际代码/测试/配置/正式文档变更时，使用 `$git-commit` 审查并仅提交本任务变更；只有本编号文件状态更新或删除时无需提交。不要提交其他任务或工作区变更。
- 最终回复报告完成证据与推荐下一步；完成且非待回归时按执行技能删除本任务文件，正式证据文档保留。只生成本计划不会完成母任务9。
