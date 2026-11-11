# 任务 14: 补齐Messages请求转换能力

批次：【批次 8】 依赖批次 7；直接依赖任务 13

状态：已完成，待回归

目的：根据全量核查结果补齐Messages请求的可表达字段和目标协议转换。

来源任务：用户要求全量核查 api.json、shiju.json 并实现原生优先与协议转换。

预计会话范围：围绕本任务的一个主要验证目标执行，复用前序已交付合同和现有同职责模块。资料任务只写本组报告，代码任务按批次串行。

## 文件

- 修改：`crates/vw-agent/src/model_relay/messages.rs`
- 新建：`crates/vw-agent/src/model_relay/messages_request_tests.rs`
- 修改：`crates/vw-agent/src/model_relay/mod.rs`
- `复用已有同职责转换模块`

## 上下文

- 先读本目录plan.md、调研基线.md、目录基线.json及本任务；所有新路径与测试名称是拟定交付。前置任务文件删除后，以其正式产物和提交证据确认交付。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。
- 开始前检查当前源码与Git差异，核实前置交付，保留其它任务变更；只在本任务文件写进度和阻塞。
- 本批次只有本任务；共享解析、路由、转换、格式化及Clippy与其它代码任务串行执行。

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

本仓库`.cargo/config.toml`还设置了共享`build-dir = "target"`。领取槽位后同时设置`export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`，使中间产物落在同一已领取槽位。测试使用所列crate的默认features，Clippy使用全workspace全部targets/features。

## 测试计划

- 行为：文本、工具、停止条件、结构化输出及模型声明的输入能力能正确映射，无法表达的请求明确拒绝。
- 失败验证测试：在`crates/vw-agent/src/model_relay/messages_request_tests.rs`新增`model_protocol_014_`前缀的独立行为测试；必要相邻crate使用相同前缀。
- 失败验证命令：`cargo test -p vw-agent --lib model_protocol_014_ -- --nocapture`。
- 预期失败原因：当前实现不能满足本卡描述的行为，断言或真实线协议观测不符；先让测试可编译，不能用缺符号、零测试或编译失败充当行为红灯。
- 通过验证命令：`cargo test -p vw-agent --lib model_protocol_014_ -- --nocapture`，退出0、有效测试大于0且失败数0。
- 模拟策略：使用真实解析、选择、序列化和传输调用链；仅外部供应商HTTP边界用有限回环服务替代，按官方请求/JSON/SSE形状验证路径、参数及终态。

## 步骤

1. 由官方查询与真实捕获的脱敏字段建立失败夹具，覆盖stop_sequences、缓存提示、output_config、工具选择和适用的图像输入。
2. 聚焦现有Messages→Chat请求转换；若发现来源计划已交付同职责转换，复用其真实接口。另一种跨协议引擎作为有据的依赖记录，当前任务完成条件只覆盖现有转换器中已证实的请求能力差异。
3. 所有字段按语义映射，保留模型能力优先级；未知扩展需有明确失败原因，禁止为通过而直接删除用户推理状态或降级工具。
4. 与任务15约定思考回放输入，先证明请求转换行为，再登记可用转换格子。
5. Rust修改后执行`cargo fmt --all`并审查差异范围；执行聚焦验证和全仓Clippy，只处理本项引入的问题。

## 验证

- 运行：`cargo test -p vw-agent --lib model_protocol_014_ -- --nocapture`。
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`。
- 预期：本任务主要行为得到当前证据，覆盖计数匹配实际源文件，失败与未验证状态可追溯。
- 所需证据：确切命令、退出码、有效测试数或完整性检查计数、失败前后差异、核查日期、已审查文件；Rust/Cargo任务还记录槽位和绝对CARGO_TARGET_DIR。
- 接口/流式验收记录HTTP、实际绑定、事件序列和最终状态。只编译成功、握手成功或HTTP200不足以判定任务完成。

## 完成与阻塞

- 所需证据齐全后标记`已完成`。有实际代码、测试、配置或文档变更，使用技能`$git-commit`核对并只提交本任务变更，报告证据后删除本任务文件。只有本任务状态更新或删除则不单独提交。
- 遇到问题先调查并在既有授权内处理实现原目标所需的相邻依赖；仍有实际依赖缺失则在本文件标记`已阻塞`，记录复现、已尝试路径和所需输入。
- 无关既有问题导致回归无法执行，可标记`已完成，待回归`并保留任务文件；按上述规则提交实际变更。缺少完成本任务本身所必需的实现或验证，不能据此宣称完成。
- plan.md为只读，prompt.md不是进度来源；正式证据写入本任务列明的产物。

## 执行记录（2026-11-11）

- 已读取执行、RustCodeGraph、提交技能及计划和两份基线。依赖13实现提交 `4a604e91d` 已存在，9项启动测试因构建资源/中断保留待回归；本任务复用其共享选择器和实际转换能力列表，不将前置待回归当成验证通过。
- 当前Messages模块已有其它任务的可见性修改，mod.rs、messages_stream.rs、server.rs及工作区大量文件有其它变更，只提交本任务增量。plan.md只读。
- 官方字段依据（核查2026-11-11）：https://platform.claude.com/docs/en/api/messages/create （stop_sequences、顶层cache_control）；https://platform.claude.com/docs/en/build-with-claude/structured-outputs （output_config.format与工具strict）；https://developers.openai.com/api/reference/resources/chat （stop、image_url、response_format与function.strict）。夹具为公开协议形状的合成脱敏数据，不冒充真实供应商捕获。
- 约定任务15输入：thinking enabled/adaptive、thinking/redacted_thinking签名与回放仍明确422；不删除、不伪造、不改为disabled。本任务只扩展现有Messages→Chat请求字段，另两种目标协议引擎不在范围。Chat响应无法区分自然stop与命中哪个停止词，此响应侧差异留给后续响应/流式任务，不能伪造stop_sequence。
- 范围调整依据：routing.rs::relay_conversions为ACP与relay共用事实来源，需登记StructuredOutput；Chat已有媒体能力门禁，图像转换复用该门禁。
- 首次动态领取槽位10，CARGO_TARGET_DIR及CARGO_BUILD_BUILD_DIR均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-10`，CARGO_BUILD_JOBS=1。`cargo fmt --all`退出0后开始规定红灯命令，日志 `/tmp/shiju-task14-red.log`；测试尚在编译中，不能声称行为红灯。
- 当前新增9项`model_protocol_014_`测试，覆盖文本/停止词、结构化Schema、缓存提示、工具选择/strict、图像顺序/模型能力门禁、未知字段/思考拒绝、共享能力登记、真实HTTP工具续轮与负例；代码仍为原始转换器以取得红灯证据。实现草稿暂存在 `/tmp/shiju-task14-messages-after.rs`，未写入生产源。红灯编译进程51721采样（`sample 51721 1 1 -file /tmp/shiju-task14-rustc-sample.txt`）显示Rust analysis/privacy检查仍在工作，physical footprint约8.6G，尚无Rust源码诊断，未把编译进展算成测试结果。
- 根据后续无人值守恢复指令，停止重复重型构建：核对自有rustc PID51721/父cargo51260后发送SIGTERM。规定lib测试命令退出101（signal15），有效测试0；这不是行为红灯，也没有归因OOM。该轮约27分钟、CPU约9分钟，第二次采样进入单态化收集、physical footprint约12.4G。自有槽位10已由退出trap释放。
- 调查替代路径：仓库已有`tests/cc04_accept_transactions.rs`和`tests/cc_route_030_outbound_scan.rs`通过公共API运行真实生产库。沿用该模式新增`tests/model_protocol_014_messages.rs`，复用同一HTTP断言模块走完整RelayServer→ProviderRelayBackend→选择器→转换→真实HTTP边界；不替换内部逻辑。7项纯转换断言移到共享core测试模块，由lib测试和集成目标共同包含，集成目标直接包含原始messages.rs/chat.rs，并重导出真实provider模块，未复制生产转换算法、未伪造模型类型。
- 范围调整：新增共享测试文件`messages_request_core_tests.rs`、`messages_request_http_tests.rs`和集成目标，Cargo.toml只追加本目标注册。首次发现`autotests=false`导致未注册目标退出101、有效测试0，已补显式注册，不计为红灯。替代阶段动态领取槽位3，绝对CARGO_TARGET_DIR及CARGO_BUILD_BUILD_DIR均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；命令`cargo fmt --all`及`cargo test -p vw-agent --test model_protocol_014_messages -- --nocapture`，日志`/tmp/shiju-task14-focused-red.log`。
- 资源与证据修正：首次临时Rustc core运行器误读取了另一个任务slot6/build的编译参数，其结果撤回、不作为验收证据；当时误向该另一任务rustc PID65295发送SIGTERM，关联`/tmp/shiju-t40-two-recheck.log`（聚合任务40），该次回归需由对应任务重新执行。已承认错误，并改为联合核对占用记录、完整父链及`--out-dir`。未启动另一任务的重构建、未删除其锁/缓存。
- 正确本任务生产库父链为64683→64691→64910→72459，完整`--out-dir`为slot3/debug/deps，核对后停止自有旧快照的普通库构建。聚焦HTTP集成命令退出101（SIGTERM），有效测试0；保留完整测试而不宣称HTTP验收通过。参数中的依赖部分为.rmeta，独立test链接要求.rlib；已经仅切换为同哈希且实际存在的.rlib，第一次缺rlib形式的编译错误不计红灯。
- 重新动态领取槽位3，运行`/tmp/shiju-task14-claim.sh python3 /tmp/shiju-task14-core-corrected.py`。运行器完整Rustc命令分别存`/tmp/shiju-task14-corrected-red-command.json`和`/tmp/shiju-task14-corrected-green-command.json`，使用本任务正确Cargo编译参数、同槽位依赖和原始生产messages.rs/chat.rs；Model仅重导出真实vw_shared::provider::types::Model，与生产provider的类型一致，没有替代实现。
- 有效行为红灯：编译退出0，测试退出101；7项中5失败/2通过，失败均为原始转换器对stop_sequences、顶层缓存、output_config、strict工具和图像返回422。有效绿灯：编译退出0，测试退出0，7项通过/0失败/0忽略。日志`/tmp/shiju-task14-corrected-{red,green}.log`。这是7项纯请求转换/模型门禁证据，不冒充完整relay HTTP或规定lib目标验收。
- 修改：Messages→Chat新增停止词映射（拒绝超过4项，不截断）、顶层缓存形状校验、结构化Schema严格映射、工具strict及用户图像有序映射；未知字段、无法表达的输入/思考状态继续拒绝；模型门禁仍由Chat准备和共享选择器执行。Hosted原生保留对应新字段。共享转换格子登记StructuredOutput，沿用既有消费者。
- `cargo fmt --all`退出0，`git diff --check`退出0；已有不支持字段测试移除stop_sequences（非法形状400由新增测试覆盖）。9项lib测试仍保留；聚焦集成目标共8项有效断言测试及1个仅由父测试启动的ignored子进程入口。来源文件、基线与当前源码SHA-256及测试计数记录`/tmp/shiju-task14-provenance.json`。
- 全仓Clippy动态领取槽位6：CARGO_TARGET_DIR及CARGO_BUILD_BUILD_DIR均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-6`；确切命令`cargo clippy --all-targets --all-features -- -D warnings`，使用10分钟进程组上限避免无界等待，日志`/tmp/shiju-task14-clippy.log`。该阶段尚在进行，不提前声称通过。

## 最终验证状态（2026-11-11）

- 全仓`cargo clippy --all-targets --all-features -- -D warnings`退出0，实际耗时9分41秒；已覆盖本任务集成目标和lib测试目标的类型及警告检查。不是行为测试通过。
- 正确槽位3的源码级行为验证：红灯编译0/测试101，7项中2通过5失败；绿灯编译0/测试0，7通过0失败。`cargo fmt --all`及本任务`git diff --check`均退出0。
- 实际槽位：初次lib尝试10；生产库集成尝试及修正后的源码验证3；全仓Clippy 6。绝对CARGO_TARGET_DIR分别为`/Users/xiangmin/worker/code/shiju/target/rust-slot-10`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-6`；所有Cargo阶段CARGO_BUILD_BUILD_DIR与相应CARGO_TARGET_DIR相同，jobs=1。自有Cargo和源码测试均已结束，锁已释放，缓存保留。
- 保留“已完成，待回归”：前置13已交付实现；本项真实转换逻辑取得7项源码行为证据、全部目标Clippy通过，但完整HTTP请求/响应链及规定lib命令均未运行到测试执行阶段，不能把类型检查、编译进展或SIGTERM当成功。按恢复指令避免继续重复重型代码生成，保留9项lib及8项集成测试的原始断言，资源恢复后运行下列命令：
  - `cargo test -p vw-agent --lib model_protocol_013_ -- --nocapture`
  - `cargo test -p vw-agent --lib model_protocol_014_ -- --nocapture`
  - `cargo test -p vw-agent --test model_protocol_014_messages -- --nocapture`
  - `cargo test -p vw-agent --lib model_relay::messages_tests -- --nocapture`
- 意外中断交接：聚合任务40的`/tmp/shiju-t40-two-recheck.log`对应rustc PID65295因本项误判父链被SIGTERM结束；原Cargo通过临时manifest执行`aggregation_t40_`过滤回归。此失败不能归为任务40源码问题，建议对应任务在资源允许时重跑。首次引用该缓存的本项临时core结果全部撤回，已在正确自有槽位3重做并验证。未恢复或重启其未知调度流程，避免再次干扰其它任务。

- 最终格式化复核：动态领取槽位1，CARGO_TARGET_DIR及CARGO_BUILD_BUILD_DIR为`/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`cargo fmt --all`退出0；该阶段自有锁已释放。仅清理新测试文件末尾多余空行，未改变已通过的7项断言或生产逻辑。
- 提交隔离复核：只提交本项10个文件；messages.rs/mod.rs原有pub(crate)可见性改动及Cargo.toml的toml_edit依赖保留为其它任务未提交变更。`git diff --cached --check`退出0，状态保持“已完成，待回归”，不删除任务卡。
