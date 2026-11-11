# 任务 27: 检测macOS陈旧Codex客户端

批次：【批次 13】 依赖：批次 12 的任务 26。同批实施与资源规则见 prompt.md。

状态：已完成，待回归

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：以代次与进程启动时间检测daemon和其他app-server，保留2秒边界；ps失败返回unverified而非无陈旧。

来源任务：`.plans/2026-11-05-CC能力完整融合/10-聚合目录与客户端特殊能力.md`；源码入口 `services/provider/codex_client_catalog.rs::stale_clients`。

预计会话范围：只完成“检测macOS陈旧Codex客户端”这一验证目标。建议一个20–45分钟聚焦会话；不顺带实现下一层入口、重构依赖或补齐整个母能力。构建耗时另计。

## 依赖与文件

上述直接依赖必须有完成证据；已删除任务文件以提交和正式证据为准，不能仅以文件不存在认定完成。代码实施按任务 1 登记核对本任务实际使用的母能力接口。

- 拟新增/修改业务：`crates/vw-agent-apps/src/aggregation/stale_clients.rs`。
- 最小注册：`crates/vw-agent-apps/src/aggregation/mod.rs`（仅本模块注册，不顺手重构共享文件）。
- 独立测试：`crates/vw-agent-apps/src/aggregation/stale_clients_tests.rs`；新增测试名统一前缀 `aggregation_t27_`，本任务至少一项有效行为测试。

## 上下文

读取来源根 `/Users/xiangmin/worker/code/cc-switch/src-tauri/src/` 下 `services/provider/codex_client_catalog.rs::stale_clients`（TS入口从来源根 `src/` 读取）及相邻测试；参考 `来源映射.json`。以任务2的 `docs/shiju/agent-apps/aggregation/接口契约.md` 为类型/方法/错误准绳。以上是拟新增文件，若前置已实现等价能力，应扩展真实所有者并记录路径，禁止重复体系。

复用母能力2已注册的vw-agent-apps原生lib和默认features，本任务不修改workspace清单或搬迁来源包；配置变更走母能力4的事务、账号走母能力6、代理走母能力8，不引入Tauri依赖。

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

- 行为：以代次与进程启动时间检测daemon和其他app-server，保留2秒边界；ps失败返回unverified而非无陈旧。
- 失败验证测试：在 `crates/vw-agent-apps/src/aggregation/stale_clients_tests.rs` 新增 `aggregation_t27_stale_clients_preserves_contract`；用该前缀为边界用例命名，测试业务输出/实际副作用，不断言源码文本。
- 失败验证命令：`cargo test -p vw-agent-apps --lib aggregation_t27_ -- --nocapture`。
- 预期失败原因：当前实现尚未具备上述行为或缺少接入；先把测试接入真实编译入口，再确认断言失败。若等价实现已通过，则新增此前未覆盖的实际失败边界，不制造人为失败。
- 通过验证命令：`cargo test -p vw-agent-apps --lib aggregation_t27_ -- --nocapture`；使用crate默认features、macOS原生target，不添加臆测feature或交叉平台目标。
- 模拟策略：规则直接传入真实结构；文件用临时目录，事务使用真实前置存储，API走实际router/权限中间件，SDK走本机HTTP测试服务核对线协议。外部模型上游、时钟、进程表及重启可替代，记录输入/输出形状和不能证明的真实客户端行为。

## 步骤

1. 阅读本任务文件与只读plan、前置契约、真实调用者和相邻测试，先确认范围/依赖，检查工作区避免覆盖无关变更。
2. 编写上述独立失败测试并登记测试模块，领取共享槽位后运行过滤命令；确认至少1项有效测试且因预期行为失败，编译错误或零测试不能作为失败行为证据。
3. 仅实现“检测macOS陈旧Codex客户端”的最小变更，明确返回不支持、权限、版本冲突和目标失败，不伪造成功。
4. 执行 `cargo fmt --all` 后审查差异并排除无关格式化；运行相同过滤测试，再运行 `cargo clippy --all-targets --all-features -- -D warnings`。
5. 对真实行为与预期逐项核对，保留脱敏记录；只有测试通过后才能作本任务范围内必要整理。

## 验证

- 聚焦命令：`cargo test -p vw-agent-apps --lib aggregation_t27_ -- --nocapture`，预期至少1项匹配测试、全部通过、退出码0。
- 必需检查：`cargo clippy --all-targets --all-features -- -D warnings`，修复本次引入的问题；既有或其他任务错误不得顺手修复，保留路径/诊断/退出码。
- 不执行无过滤全量单测、`cargo fmt --all -- --check`、wasm检查或Makefile；无需其他平台验收。
- 所需证据：失败/通过的同一过滤命令、有效数量、失败原因、退出码、实际实现路径/符号、权限与失败场景、槽位与绝对CARGO_TARGET_DIR；证据写入 `docs/shiju/agent-apps/aggregation/任务27-验证.md`，不含密钥、原始账号/进程载荷。

## 完成

只有本任务所列新证据齐全才标为 `已完成`。有实际代码、测试、配置或文档变更，使用 `$git-commit` 审查并只提交本任务；仅编号任务状态/删除不调用该技能、不单独提交。最终报告证据并删除已完成任务文件，正式证据文档保留。

无法满足真实前置或必需环境时，只在本任务记录 `已阻塞`、缺失文件/契约、已运行检查及最小解除条件；先尝试在本任务范围解决，不能吞并整个前置母能力或修改只读plan。无关既有编译错误阻止回归时可标 `已完成，待回归` 并保留任务文件，必须说明尚缺证据，不算母任务完成。不修改、不提交其他任务或既有工作区变更。

## 本次执行证据（2026-11-11）

当前检出完成 `aggregation/stale_clients.rs` 原生检测及独立测试；正式证据为 `docs/shiju/agent-apps/aggregation/任务27-验证.md`。同一过滤命令失败验证4项（1过3预期断言失败，退出101），通过验证7项全过（退出0），包含隔离HOME启动真实已安装Codex并由原生ps发现、自有进程回收。未把编译、零测试或缺失客户端当通过。

全仓 `cargo clippy --all-targets --all-features -- -D warnings` 退出101；其他任务未跟踪的 `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_operation_tests.rs:4,22` 发生 E0432/E0433（`super::aggregation_read_snapshot` 导入路径错误）。未修改或提交这些无关文件，不宣称全仓通过。

槽位11：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-11`（失败验证与Clippy）；槽位12：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-12`（7项通过）。正式阶段同时令 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，使用jobs=1、test opt-level=0/debug=0；全部自有进程结束后完整核对owner释放锁并保留缓存。仅格式化本任务源码。

下一步：由导入错误所有者修复后，重新动态领取槽位，设置真实CLI的测试环境 `T27_CODEX_BINARY`，运行上述聚焦命令及全仓Clippy；全仓通过后再删除本任务卡。本次保留卡片供回归，不修改只读plan.md。

## 延后回归核对（2026-11-12）

实现/测试摘要与38860cae4和原7项聚焦通过证据一致；核对直接前置后仍适用，详情见正式证据。本轮无改码、Cargo或槽位。统一Clippy由任务15执行，本轮被其他MCP文件5处column!宏歧义阻断且期间源码变化，不作通过证据。按调度停止轮询，继续“已完成，待回归”，保留卡片待统一结果就绪后核对；不标真实阻塞。
