# 任务 41: 接通聚合发布API

批次：【批次 13】 依赖：批次 12 的任务 16、40。同批实施与资源规则见 prompt.md。

状态：已完成，待回归

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：发布经过主机授权与预览修订校验；长任务使用既有进度/取消契约，不重复提交，取消边界准确。

来源任务：`.plans/2026-11-05-CC能力完整融合/10-聚合目录与客户端特殊能力.md`；源码入口 `mode/controller.rs::enter_with_route`。

预计会话范围：只完成“接通聚合发布API”这一验证目标。建议一个20–45分钟聚焦会话；不顺带实现下一层入口、重构依赖或补齐整个母能力。构建耗时另计。

## 依赖与文件

上述直接依赖必须有完成证据；已删除任务文件以提交和正式证据为准，不能仅以文件不存在认定完成。代码实施按任务 1 登记核对本任务实际使用的母能力接口。

- 拟新增/修改业务：`crates/vw-agent/src/gateway/api/handlers/aggregation_publish.rs`。
- 最小注册：`crates/vw-agent/src/gateway/api/handlers/mod.rs`（仅本模块注册，不顺手重构共享文件）。
- 独立测试：`crates/vw-agent/src/gateway/api/handlers/aggregation_publish_tests.rs`；新增测试名统一前缀 `aggregation_t41_`，本任务至少一项有效行为测试。
- API类任务仅增补 `crates/vw-api-types/src/agent_apps/aggregation.rs` 及其独立测试，并在 `crates/vw-agent/src/gateway/api/handlers/mod.rs` 最小挂载；代理任务以gateway/agent_apps为拟新增接线位置，复用母能力8实际常驻路由注册；若已存在对应模块则使用其准确路径。现有src/model_relay是会话绑定能力，不得扩权或改作公共常驻代理。不存在的父模块只作最小声明。

## 上下文

读取来源根 `/Users/xiangmin/worker/code/cc-switch/src-tauri/src/` 下 `mode/controller.rs::enter_with_route`（TS入口从来源根 `src/` 读取）及相邻测试；参考 `来源映射.json`。以任务2的 `docs/shiju/agent-apps/aggregation/接口契约.md` 为类型/方法/错误准绳。以上是拟新增文件，若前置已实现等价能力，应扩展真实所有者并记录路径，禁止重复体系。

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

- 行为：发布经过主机授权与预览修订校验；长任务使用既有进度/取消契约，不重复提交，取消边界准确。
- 失败验证测试：在 `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_tests.rs` 新增 `aggregation_t41_aggregation_publish_preserves_contract`；用该前缀为边界用例命名，测试业务输出/实际副作用，不断言源码文本。
- 失败验证命令：`cargo test -p vw-agent --lib aggregation_t41_ -- --nocapture`。
- 预期失败原因：当前实现尚未具备上述行为或缺少接入；先把测试接入真实编译入口，再确认断言失败。若等价实现已通过，则新增此前未覆盖的实际失败边界，不制造人为失败。
- 通过验证命令：`cargo test -p vw-agent --lib aggregation_t41_ -- --nocapture`；使用crate默认features、macOS原生target，不添加臆测feature或交叉平台目标。
- 模拟策略：规则直接传入真实结构；文件用临时目录，事务使用真实前置存储，API走实际router/权限中间件，SDK走本机HTTP测试服务核对线协议。外部模型上游、时钟、进程表及重启可替代，记录输入/输出形状和不能证明的真实客户端行为。

## 步骤

1. 阅读本任务文件与只读plan、前置契约、真实调用者和相邻测试，先确认范围/依赖，检查工作区避免覆盖无关变更。
2. 编写上述独立失败测试并登记测试模块，领取共享槽位后运行过滤命令；确认至少1项有效测试且因预期行为失败，编译错误或零测试不能作为失败行为证据。
3. 仅实现“接通聚合发布API”的最小变更，明确返回不支持、权限、版本冲突和目标失败，不伪造成功。
4. 执行 `cargo fmt --all` 后审查差异并排除无关格式化；运行相同过滤测试，再运行 `cargo clippy --all-targets --all-features -- -D warnings`。
5. 对真实行为与预期逐项核对，保留脱敏记录；只有测试通过后才能作本任务范围内必要整理。

## 验证

- 聚焦命令：`cargo test -p vw-agent --lib aggregation_t41_ -- --nocapture`，预期至少1项匹配测试、全部通过、退出码0。
- 必需检查：`cargo clippy --all-targets --all-features -- -D warnings`，修复本次引入的问题；既有或其他任务错误不得顺手修复，保留路径/诊断/退出码。
- 不执行无过滤全量单测、`cargo fmt --all -- --check`、wasm检查或Makefile；无需其他平台验收。
- 所需证据：失败/通过的同一过滤命令、有效数量、失败原因、退出码、实际实现路径/符号、权限与失败场景、槽位与绝对CARGO_TARGET_DIR；证据写入 `docs/shiju/agent-apps/aggregation/任务41-验证.md`，不含密钥、原始账号/进程载荷。

## 完成

只有本任务所列新证据齐全才标为 `已完成`。有实际代码、测试、配置或文档变更，使用 `$git-commit` 审查并只提交本任务；仅编号任务状态/删除不调用该技能、不单独提交。最终报告证据并删除已完成任务文件，正式证据文档保留。

无法满足真实前置或必需环境时，只在本任务记录 `已阻塞`、缺失文件/契约、已运行检查及最小解除条件；先尝试在本任务范围解决，不能吞并整个前置母能力或修改只读plan。无关既有编译错误阻止回归时可标 `已完成，待回归` 并保留任务文件，必须说明尚缺证据，不算母任务完成。不修改、不提交其他任务或既有工作区变更。

## 本次执行记录（2026-11-11）

- 发布API、原共享操作registry与类型化aggregationOutcome已接通。正式证据：`docs/shiju/agent-apps/aggregation/任务41-验证.md`。
- 真正失败证据：同一受控manifest/过滤命令执行1项，publish缺路由404与422断言不符，退出101。最终同一过滤执行11项全部通过、0过滤，退出0；正常API types过滤2项通过，退出0。
- 最终 `cargo clippy --all-targets --all-features -- -D warnings` 退出101，仅其他任务 `crates/vw-desktop/src/app/voice/hotkeys.rs` 的Config(7)、Shortcut(14)、Action(19)、Machine(21)、press/release(23/27)共5项dead_code。未修改或抑制该文件诊断，不宣告Clippy通过或母能力完成。
- 最终槽位2：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。全部自有Cargo消费者退出，完整owner核对后已释放锁，保留缓存；未创建工作树，未改plan.md。
- 解除条件：语音热键所有者修复上述生产接线后，重新动态领取共享槽位并执行该确切Clippy命令；通过后补记新槽位/绝对路径/退出码，标记已完成，再删除本卡。无需重复无过滤全量测试，也不把本机HTTP/隔离文件验收称为真实客户端实机验收。

## 延后回归进度（2026-11-12）

本轮共享操作鉴权已由后续退出/重启任务扩展，旧聚焦证据不能覆盖认证顺序回退。当前源码受控副本执行11项，10通过1失败，退出101：匿名且缺projectId时返回404，既有断言要求401。本轮仅修改原operations/aggregation.rs的认证/选择器核验顺序及原发布测试的匿名异域GET/DELETE断言；保留Codex重启独立权限，不重建进度或配置授权。

最终代码及源码SHA-256、红/绿日志交接位于 `/tmp/shiju-t41-deferred-handoff.json`，原15及调度可直接读取。聚焦通过前不宣告本轮完成。全仓Clippy由原任务15统一执行，必须覆盖该交接文件的最终源码散列；本会话不重复启动全仓Clippy。


本轮聚焦结果：修复后同一过滤11项全部通过（21过滤）、退出0；相邻独立重启权限1项通过（31过滤）、退出0。槽位3，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3，build目录同路径，所有自有消费者已退出并核对完整owner释放。最终散列和日志结果已写入 `/tmp/shiju-t41-deferred-handoff.json`，原15已确认读取交接。尚待原15统一Clippy覆盖最终修复散列的通过结果；收到并核对后更新正式证据、删除本卡。本会话不追加全仓Clippy。
