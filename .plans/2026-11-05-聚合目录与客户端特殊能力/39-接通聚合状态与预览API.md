# 任务 39: 接通聚合状态与预览API

批次：【批次 11】 依赖：批次 10 的任务 13。同批实施与资源规则见 prompt.md。

状态：实施中

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：读取当前状态与预览走类型化API，拒绝无认证及异主机/项目访问；预览请求不能触发发布副作用。

来源任务：`.plans/2026-11-05-CC能力完整融合/10-聚合目录与客户端特殊能力.md`；源码入口 `mode/controller.rs::stack_view_with_clients`。

预计会话范围：只完成“接通聚合状态与预览API”这一验证目标。建议一个20–45分钟聚焦会话；不顺带实现下一层入口、重构依赖或补齐整个母能力。构建耗时另计。

## 依赖与文件

上述直接依赖必须有完成证据；已删除任务文件以提交和正式证据为准，不能仅以文件不存在认定完成。代码实施按任务 1 登记核对本任务实际使用的母能力接口。

- 拟新增/修改业务：`crates/vw-agent/src/gateway/api/handlers/aggregation_read.rs`。
- 最小注册：`crates/vw-agent/src/gateway/api/handlers/mod.rs`（仅本模块注册，不顺手重构共享文件）。
- 独立测试：`crates/vw-agent/src/gateway/api/handlers/aggregation_read_tests.rs`；新增测试名统一前缀 `aggregation_t39_`，本任务至少一项有效行为测试。
- API类任务仅增补 `crates/vw-api-types/src/agent_apps/aggregation.rs` 及其独立测试，并在 `crates/vw-agent/src/gateway/api/handlers/mod.rs` 最小挂载；代理任务以gateway/agent_apps为拟新增接线位置，复用母能力8实际常驻路由注册；若已存在对应模块则使用其准确路径。现有src/model_relay是会话绑定能力，不得扩权或改作公共常驻代理。不存在的父模块只作最小声明。

## 上下文

读取来源根 `/Users/xiangmin/worker/code/cc-switch/src-tauri/src/` 下 `mode/controller.rs::stack_view_with_clients`（TS入口从来源根 `src/` 读取）及相邻测试；参考 `来源映射.json`。以任务2的 `docs/shiju/agent-apps/aggregation/接口契约.md` 为类型/方法/错误准绳。以上是拟新增文件，若前置已实现等价能力，应扩展真实所有者并记录路径，禁止重复体系。

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

## 测试计划

- 行为：读取当前状态与预览走类型化API，拒绝无认证及异主机/项目访问；预览请求不能触发发布副作用。
- 失败验证测试：在 `crates/vw-agent/src/gateway/api/handlers/aggregation_read_tests.rs` 新增 `aggregation_t39_aggregation_read_preserves_contract`；用该前缀为边界用例命名，测试业务输出/实际副作用，不断言源码文本。
- 失败验证命令：`cargo test -p vw-agent --lib aggregation_t39_ -- --nocapture`。
- 预期失败原因：当前实现尚未具备上述行为或缺少接入；先把测试接入真实编译入口，再确认断言失败。若等价实现已通过，则新增此前未覆盖的实际失败边界，不制造人为失败。
- 通过验证命令：`cargo test -p vw-agent --lib aggregation_t39_ -- --nocapture`；使用crate默认features、macOS原生target，不添加臆测feature或交叉平台目标。
- 模拟策略：规则直接传入真实结构；文件用临时目录，事务使用真实前置存储，API走实际router/权限中间件，SDK走本机HTTP测试服务核对线协议。外部模型上游、时钟、进程表及重启可替代，记录输入/输出形状和不能证明的真实客户端行为。

## 步骤

1. 阅读本任务文件与只读plan、前置契约、真实调用者和相邻测试，先确认范围/依赖，检查工作区避免覆盖无关变更。
2. 编写上述独立失败测试并登记测试模块，领取共享槽位后运行过滤命令；确认至少1项有效测试且因预期行为失败，编译错误或零测试不能作为失败行为证据。
3. 仅实现“接通聚合状态与预览API”的最小变更，明确返回不支持、权限、版本冲突和目标失败，不伪造成功。
4. 执行 `cargo fmt --all` 后审查差异并排除无关格式化；运行相同过滤测试，再运行 `cargo clippy --all-targets --all-features -- -D warnings`。
5. 对真实行为与预期逐项核对，保留脱敏记录；只有测试通过后才能作本任务范围内必要整理。

## 验证

- 聚焦命令：`cargo test -p vw-agent --lib aggregation_t39_ -- --nocapture`，预期至少1项匹配测试、全部通过、退出码0。
- 必需检查：`cargo clippy --all-targets --all-features -- -D warnings`，修复本次引入的问题；既有或其他任务错误不得顺手修复，保留路径/诊断/退出码。
- 不执行无过滤全量单测、`cargo fmt --all -- --check`、wasm检查或Makefile；无需其他平台验收。
- 所需证据：失败/通过的同一过滤命令、有效数量、失败原因、退出码、实际实现路径/符号、权限与失败场景、槽位与绝对CARGO_TARGET_DIR；证据写入 `docs/shiju/agent-apps/aggregation/任务39-验证.md`，不含密钥、原始账号/进程载荷。

## 完成

只有本任务所列新证据齐全才标为 `已完成`。有实际代码、测试、配置或文档变更，使用 `$git-commit` 审查并只提交本任务；仅编号任务状态/删除不调用该技能、不单独提交。最终报告证据并删除已完成任务文件，正式证据文档保留。

无法满足真实前置或必需环境时，只在本任务记录 `已阻塞`、缺失文件/契约、已运行检查及最小解除条件；先尝试在本任务范围解决，不能吞并整个前置母能力或修改只读plan。无关既有编译错误阻止回归时可标 `已完成，待回归` 并保留任务文件，必须说明尚缺证据，不算母任务完成。不修改、不提交其他任务或既有工作区变更。


## 2026-11-11 前置核对与阻塞记录

在项目当前检出执行，未创建或启动工作树；只读 `plan.md` 未修改。使用 `skills/do-task-plans/SKILL.md` 与 `skills/rustcodegraph/SKILL.md` 核对。直接依赖任务 13 已由提交 `1ee788f05` 和 `docs/shiju/agent-apps/aggregation/任务13-验证.md` 交付，其 7 项原生预览测试属于前置证据，未作为本任务通过证据。

### 具体缺口

接口契约要求原生槽位在同一事务内绑定项目，并禁止仅凭租户一致推断既有配置的项目归属。本次检查实际生产所有者：

- `crates/vw-agent-apps/src/providers/live/commit.rs::AppState` 仅有 `stack_tenant`、`mode_tenant` 和租户范围 `pointer`；`PendingTarget`、`Pending` 亦未保存项目归属。`ProviderTransactions::open_existing` 可以无副作用读取，但不能证明所读物理槽位属于请求项目。
- `crates/vw-agent/src/app_providers.rs::AppProviderStore` 的供应商档案按租户保存，没有原生槽位到项目的绑定。
- `crates/vw-agent/src/app_routing/route_auth.rs::VerifiedRouteBinding` 有项目和账号代次，但仅用于监听内的已验证发布；现有 `RoutingListener::snapshot` 只返回监听生命周期、地址和启动时间，不提供原生配置槽位的归属证明，未附着时也不能从运行时推断持久槽位的项目。
- `crates/vw-agent-apps/src/aggregation/preview.rs::PreviewInput` 明确要求调用方事先完成 host/project/read 授权；任务 13 是纯规则计算，不交付项目槽位归属。现有 `agent_apps/access.rs` 的项目路由权限只能证明主体允许操作某项目，无法证明既有配置属于该项目。新增 `aggregation_read` 权限本身也不能补齐资源归属。

如果直接把租户状态与请求 `projectId` 拼装，会把同租户另一项目的配置作为本项目状态读取并签发预览令牌；若所有非空状态一律拒绝，则无法满足本任务“读取当前状态与预览”的成功行为。只读 API 不能在读取时补写归属，因为这违反禁止持久化及发布副作用的要求。此次未添加仅空状态成功或恒返回拒绝的伪接线。

最小解除条件：由现有配置事务所有者提供可验证、持久的主机/应用原生槽位项目归属及受控认领/写入路径，并明确旧的仅租户状态如何拒绝或经授权认领；提交正式接口和行为证据后，本任务消费该接口接通读权限、类型化 DTO、修订和短时完整性令牌。该前置属于共享事务及写入所有者，不能在本只读 API 卡中另建旁路所有者或未经确认重定义既有状态归属。

### 已运行检查与未运行验证

- `rustcodegraph status`：退出 0，索引存在；`rustcodegraph explore 'aggregation preview AggregationPreviewInput mode_state authorized_app'`：退出 0；新任务 13 代码及索引未覆盖的接口用 `rg`、文件读取核对。
- 读取只读计划、任务卡、执行分线、来源核对、接口契约和任务 11/12/13 正式证据；`git show --no-patch --format='%h %s' 1ee788f05`：退出 0。
- 读取来源 `mode/controller.rs::stack_view_with_clients`、相邻测试定位与 `src/lib/api/proxy.ts::getProxyStack`，确认客户端进程探测不属于本次状态读取。
- `rg` 检查上述事务、供应商和路由所有者；`uname -sm`：退出 0，Darwin arm64。
- 未修改 Rust、测试、配置或正式文档；未执行 Cargo、格式化、聚焦测试或 Clippy。有效新增测试数量为 0，未取得行为红灯或通过证据，不宣告完成。
- Cargo 槽位：未领取；`CARGO_TARGET_DIR`：未设置（本次没有 Cargo 阶段）。未触碰其他会话的锁、进程或缓存。
- 本次仅更新本编号任务文件状态及阻塞记录，依要求不提交、不删除任务卡。


## 恢复安排
2026-11-11 收到协调指令：任务14负责现有事务所有者的持久项目绑定与授权首次认领，本任务先交付独立 DTO、授权及只读接线；待接口实际落地后接入并验证，不新建归属所有者。旧状态缺归属返回冲突，读取不补写。


## 共享锁事故核对（2026-11-11）

任务39一次共享写入命令在 `mkdir merge.lock` 失败后，因为后续 Python 和清理未处于同一成功分支，仍追加代码并误删原占用记录。已停止共享写入，未隐瞒事故。核对其他会话历史与进程树，原持有者为任务15会话 `01a1c8cc-d1ed-7c92-850c-8ce691aad463` 的 `cargo fmt --all`，持有 shell PID 63665，cargo-fmt/rustfmt 为 63672/63681；其命令写入 `task15 formatting PID=63665`（含换行28字节），与任务36领取失败时看到的原 owner 文件大小一致。原会话已自行终止这组进程；`ps` 确认三者全部不存在。

现锁随后由任务36会话 `01a1c8b4-040b-77e0-8541-99bea2ae7126` 的 PID 64008 重新原子领取，owner 为 `task=36 session=01a1c8b4 pid=64008`，操作为持锁等待提交。核查时该进程仍活跃。恢复检查程序遇到该新锁后未写入、未覆盖或释放它；没有把原任务15记录覆盖回新锁。任务36暂存的10个MCP路径不含任务39注册，随后由其自行提交；任务39未执行任何暂存或提交命令。

共享差异检查：对开始前保存的5个文件快照，`handlers/agent_apps/access.rs`、`handlers/agent_apps/mod.rs`、`handlers/mod.rs`、API类型父模块和 `model_catalog.rs` 均无既有非空行删除。聚合3个模块与MCP update两个模块注册分别仅1次；`gateway/router.rs` 对 HEAD 无差异。`git diff --check`、`git diff --cached --check` 退出0，`git ls-files -u` 无冲突。未恢复或回滚其他会话内容。

后续共享操作使用单一 Python 控制块：mkdir 失败立即退出75、不发生写入；领取成功后记录任务/完整会话/PID及随机nonce；finally仅在owner精确匹配自有记录时释放。Cargo自有锁清理也加入精确所有权验证。事故核对完成后恢复本任务。


## 最新格式化约束
2026-11-11 主调度要求覆盖原全仓格式化步骤：当前共享检出后续仅格式化本任务文件并skip_children，或临时副本全仓格式化后仅取回本任务文件；不在共享检出运行全仓格式化，不批量还原，提交前复核共享差异和暂存范围。本任务采用本任务文件的聚焦rustfmt流程。
