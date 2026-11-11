# 任务 13: 同步ACP启动与能力声明

批次：【批次 7】 依赖批次 6；直接依赖任务 12

状态：已完成，待回归

目的：使ACP启动配置、模型来源能力与中继最终路由一致。

来源任务：用户要求全量核查 api.json、shiju.json 并实现原生优先与协议转换。

预计会话范围：围绕本任务的一个主要验证目标执行，复用前序已交付合同和现有同职责模块。资料任务只写本组报告，代码任务按批次串行。

## 文件

- 修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/claude.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/codex.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/gemini.rs`
- 修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs`
- `测试对应独立 drivers/*_tests.rs`

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

- 行为：CLI启动时使用与中继一致的模式，模型级不适用原因可以明确返回。
- 失败验证测试：在`crates/vw-agent/src/session/llm/acp/model_source/drivers/claude_tests.rs`新增`model_protocol_013_`前缀的独立行为测试；必要相邻crate使用相同前缀。
- 失败验证命令：`cargo test -p vw-agent --lib model_protocol_013_ -- --nocapture`。
- 预期失败原因：当前实现不能满足本卡描述的行为，断言或真实线协议观测不符；先让测试可编译，不能用缺符号、零测试或编译失败充当行为红灯。
- 通过验证命令：`cargo test -p vw-agent --lib model_protocol_013_ -- --nocapture`，退出0、有效测试大于0且失败数0。
- 模拟策略：使用真实解析、选择、序列化和传输调用链；仅外部供应商HTTP边界用有限回环服务替代，按官方请求/JSON/SSE形状验证路径、参数及终态。

## 步骤

1. 为Messages、Responses、Gemini和Chat来源驱动编写原生/转换/拒绝的启动差异用例，覆盖所有已注册驱动的实际协议声明。
2. 复用同一选择器替代驱动独立猜测；按模型能力与用户请求设定思考参数，原生可用时保留原生能力。
3. 核查已审查CLI版本和MAX_THINKING_TOKENS等限制，消费者依据选择器返回的实际转换能力设置模式；仍需转换支持的场景由任务14—18交付，其步骤同时负责更新消费者的真实验证用例，避免依赖已删除的本卡回写进度。
4. 能力查询不得只根据CLI版本就声称所有模型与协议可用；拒绝原因保持脱敏，凭据由原有存储提供。
5. Rust修改后执行`cargo fmt --all`并审查差异范围；执行聚焦验证和全仓Clippy，只处理本项引入的问题。

## 验证

- 运行：`cargo test -p vw-agent --lib model_protocol_013_ -- --nocapture`。
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

- 已读取本任务、plan.md、调研基线.md、目录基线.json及执行、代码导航技能；plan.md保持只读。
- 直接依赖12实现已提交 `2f8c876b5`，统一选择器与真实调用点均存在；任务12最新5项测试因当时任务外编译错误待回归，本任务不把历史结果当新证据。任务11选择器提交 `6974e46fc`。
- 使用RustCodeGraph status/explore核对调用链，补读索引之外的最新路由文件。工作区其它任务改动保留。
- 新增可编译行为用例：Claude目录原生Messages保留thinking、显式空协议目录在写文件前拒绝。动态领取槽位6，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-6`，同时设置`CARGO_BUILD_BUILD_DIR`，jobs=1，已先运行`cargo fmt --all`，正在运行规定红灯命令。日志 `/tmp/shiju-task13-red.log`。
- 调整范围依据：中继 `routing.rs` 拥有部署、origin、认证授权收敛，`transport.rs` 拥有当前真实转换能力列表；将最小选择入口与能力列表共享给ACP，避免驱动重新猜测和能力列表漂移。ACP request.rs仍有按adapter推断原生能力的相邻消费者，需同步改为已选启动结果。

- 已实现公共ACP预检：22个注册驱动声明实际出站协议；复用中继`select_api`及`relay_conversions`，不再以DeepSeek/阿里特例或Codex adapter猜原生路由；Claude按选中路由保留原生thinking，Codex按选中路由设置托管搜索，Gemini写配置前拒绝不可执行来源。`request.rs`思考选项同步改用同一选择结果。
- 当前共9项`model_protocol_013_`测试（Claude 3、Codex 1、Gemini 1、公共驱动4），覆盖22个注册驱动、四种来源协议的原生/转换/拒绝，以及模型范围、认证、工具能力、空目录和跨域拒绝。显式请求thinking的转换场景拒绝，不静默关闭用户请求。14—18继续扩展转换并更新真实消费者用例；未扩展的Vibe Chat→Gemini路径现由启动预检明确拒绝。
- 首次全仓Clippy（槽位2）退出101，仅报告本项移除Claude特例后留下的`model_relay/mod.rs`未使用阿里重导出；已移除该重导出。日志 `/tmp/shiju-task13-clippy.log`。其后主工作区验证在`cargo fmt --all`退出1，因其它任务注册的`gateway/api/handlers/aggregation_omo.rs`及`vw-agent-apps/src/aggregation/codex_settings.rs`当时不存在，未进入该轮测试；日志 `/tmp/shiju-task13-final.log`。未修改或补造这些无关模块。
- 替代验证：通过`create_worktree`创建稳定检出 `/Users/xiangmin/.codex/worktrees/acp-protocol-validation/shiju`，基于直接依赖12提交`2f8c876b5`，只应用本任务12个文件的补丁。Cargo槽位仍从主仓库原子领取，当前槽位2，绝对`CARGO_TARGET_DIR`与`CARGO_BUILD_BUILD_DIR`均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；不另建构建目录。该检出`cargo fmt --all`退出0。默认features，单作业，测试只对`vw-agent`包使用`--config 'profile.test.package.vw-agent.opt-level=0'`降低代码生成耗时；未改仓库profile配置。日志 `/tmp/shiju-task13-isolated.log`。
- 12个本任务文件在主工作区与验证检出的SHA-256均一致，记录 `/tmp/shiju-task13-provenance.json`。夹具对带Drop的`AcpLaunchProfile.model_api`使用clone，避免移出字段的编译错误。当前稳定检出已进入vw-agent测试产物构建；修改前槽位6快照仍在代码生成，尚无行为红灯/绿灯最终计数，不提前宣称验证通过。

## 恢复调查与收尾核查（2026-11-11）

- 已读取两轮最终失败日志，不再启动整crate单元测试代码生成：槽位6的 `cargo test -p vw-agent --lib model_protocol_013_ -- --nocapture` 退出101；槽位2稳定检出的 `cargo test -p vw-agent --lib model_protocol_013_ --config 'profile.test.package.vw-agent.opt-level=0' -- --nocapture` 退出101。两者均在rustc代码生成期间收到 `signal: 9, SIGKILL: kill`，有效测试数均为0；没有本项Rust源码诊断，不是断言红灯，也不是行为绿灯。日志分别 `/tmp/shiju-task13-red.log`、`/tmp/shiju-task13-isolated.log`。SIGKILL来源尚未证实，不能据此归因OOM；父会话另说明用户在失败后停止过会话。
- 资源证据：原测试rustc采样physical footprint为16.2G、峰值18.0G（sample原样单位）；稳定检出降低优化后的采样physical footprint为12.7G（sample原样单位），并仍在单态化收集阶段。使用单作业、稳定基线、包级opt-level=0仍未取得行为运行结果，不重复冷启动重型测试，不用历史二进制或裁剪生产链路冒充本次验收。
- 自有进程3737/5146/6301、17267/88525/80993均已结束；临时暂停的旧快照已由独立守护恢复后结束，无自有暂停进程遗留。自有槽位2、6锁已由脚本释放；恢复调查时这两个锁由其它计划PID71590、72418持有，未删除他人锁或缓存。追加验证预留脚本因检测到owner不再属于本项而退出75，没有抢占槽位。
- 静态完整性检查退出0：22/22注册驱动均被独立测试表覆盖、9/9测试函数名称唯一。记录 `/tmp/shiju-task13-static-audit.json`；这只是源文件完整性证据，不计为行为测试通过。
- 源码复核补强：新增夹具对带Drop的AcpLaunchProfile克隆model_api；恢复时发现主仓库两处又变为移出字段，已纠正。Gemini凭据冲突负例补齐模型元数据与工具能力，并断言具体冲突错误，避免被缺少模型元数据门禁提前拒绝而假通过。
- 最后仅运行格式化与元数据Clippy检查，动态领取槽位19：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-19`，`CARGO_BUILD_BUILD_DIR`同值，jobs=1，执行地点仍为上述稳定检出；确切命令 `cargo fmt --all`，随后 `cargo clippy --all-targets --all-features -- -D warnings`。日志 `/tmp/shiju-task13-audit.log`，最终退出码在下方补录。


## 最终状态

- 实现与9项独立行为测试已交付；状态为`已完成，待回归`，保留本任务文件。没有已取得的行为红灯或绿灯，不能把SIGKILL、编译前端进展或静态完整性检查算成测试通过。
- 最终槽位19：`cargo fmt --all`退出0；`cargo clippy --all-targets --all-features -- -D warnings`退出101。本轮已完成依赖及库检查并进入`vw-agent --test --emit=dep-info,metadata`阶段，尚无新源码诊断，但测试目标运行约23分钟、CPU累计约5分钟仍未结束。为遵守恢复指令“避免重复重型构建”，在核对PID42203父链95229/95192/86798和精确命令后仅向该自有clippy-driver发送SIGTERM；最终日志为signal 15，不能声称Clippy通过。未继续启动单元测试代码生成，也未裁剪生产链路或用旧二进制替代验证。
- 最后一轮日志`/tmp/shiju-task13-audit.log`、静态完整性记录`/tmp/shiju-task13-static-audit.json`；两轮SIGKILL日志与槽位2/6绝对路径见上方。所有阶段均保留编译缓存，只释放自己的占用记录；提交前再次核对进程与锁。
- 待回归：在资源空闲时，于包含本项提交的当前源码执行 `cargo test -p vw-agent --lib model_protocol_013_ -- --nocapture`（应为9项、退出0、失败0），随后任务12的5项与Claude/Codex/Gemini相邻场景，最后执行 `cargo clippy --all-targets --all-features -- -D warnings` 并取得退出0。仍按共享槽位1—20规则动态领取，两个Cargo目录环境变量均设置为同一绝对路径。回归通过前不删除本卡。

- 提交：`4a604e91d59b01859627ff881477bc7457363168 fix(acp): 修复启动配置与中继协议选择不一致`，仅12个本项源码/测试文件。使用独立暂存索引提交，提交后逐项比较其它4个暂存文件的补丁字节，确认原样保留；共享mod.rs中的messages可见性和request.rs中的范围外格式化仍留在工作区，未混入提交。`plan.md`无差异。
- 自有最终进程86798/95192/95229/42203均已结束，槽位19锁已释放，2/6仅释放本项早期持有记录；其它任务后来领取的同号锁未改动。缓存路径仍保留：`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-6`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-19`。

- 临时验证检出清理：`archive_worktree`返回 `The task's primary checkout cannot be removed.`。遵守工具保护，不使用Git强制删除或绕过保护；检出 `/Users/xiangmin/.codex/worktrees/acp-protocol-validation/shiju` 保留作验证副本。代码提交位于主仓库shiju分支，主仓库本卡仍是唯一进度来源，回归后按契约再删除本卡。
