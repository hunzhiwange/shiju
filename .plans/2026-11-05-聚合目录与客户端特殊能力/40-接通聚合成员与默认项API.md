# 任务 40: 接通聚合成员与默认项API

批次：【批次 12】 依赖：批次 6 的任务 7、批次 11 的任务 39。同批实施与资源规则见 prompt.md。

状态：执行中

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：类型化增删/设默认接口校验scope和修订，聚合生效时调用已有重发布事务，失败保留原成员/目录；非法官方成员和移除默认明确报错。

来源任务：`.plans/2026-11-05-CC能力完整融合/10-聚合目录与客户端特殊能力.md`；源码入口 `mode/controller.rs::set_stack_member、set_route`。

预计会话范围：只完成“接通聚合成员与默认项API”这一验证目标。建议一个20–45分钟聚焦会话；不顺带实现下一层入口、重构依赖或补齐整个母能力。构建耗时另计。

## 依赖与文件

上述直接依赖必须有完成证据；已删除任务文件以提交和正式证据为准，不能仅以文件不存在认定完成。代码实施按任务 1 登记核对本任务实际使用的母能力接口。

- 拟新增/修改业务：`crates/vw-agent/src/gateway/api/handlers/aggregation_members.rs`。
- 最小注册：`crates/vw-agent/src/gateway/api/handlers/mod.rs`（仅本模块注册，不顺手重构共享文件）。
- 独立测试：`crates/vw-agent/src/gateway/api/handlers/aggregation_members_tests.rs`；新增测试名统一前缀 `aggregation_t40_`，本任务至少一项有效行为测试。
- API类任务仅增补 `crates/vw-api-types/src/agent_apps/aggregation.rs` 及其独立测试，并在 `crates/vw-agent/src/gateway/api/handlers/mod.rs` 最小挂载；代理任务以gateway/agent_apps为拟新增接线位置，复用母能力8实际常驻路由注册；若已存在对应模块则使用其准确路径。现有src/model_relay是会话绑定能力，不得扩权或改作公共常驻代理。不存在的父模块只作最小声明。

## 上下文

读取来源根 `/Users/xiangmin/worker/code/cc-switch/src-tauri/src/` 下 `mode/controller.rs::set_stack_member、set_route`（TS入口从来源根 `src/` 读取）及相邻测试；参考 `来源映射.json`。以任务2的 `docs/shiju/agent-apps/aggregation/接口契约.md` 为类型/方法/错误准绳。以上是拟新增文件，若前置已实现等价能力，应扩展真实所有者并记录路径，禁止重复体系。

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

- 行为：类型化增删/设默认接口校验scope和修订，聚合生效时调用已有重发布事务，失败保留原成员/目录；非法官方成员和移除默认明确报错。
- 失败验证测试：在 `crates/vw-agent/src/gateway/api/handlers/aggregation_members_tests.rs` 新增 `aggregation_t40_aggregation_members_preserves_contract`；用该前缀为边界用例命名，测试业务输出/实际副作用，不断言源码文本。
- 失败验证命令：`cargo test -p vw-agent --lib aggregation_t40_ -- --nocapture`。
- 预期失败原因：当前实现尚未具备上述行为或缺少接入；先把测试接入真实编译入口，再确认断言失败。若等价实现已通过，则新增此前未覆盖的实际失败边界，不制造人为失败。
- 通过验证命令：`cargo test -p vw-agent --lib aggregation_t40_ -- --nocapture`；使用crate默认features、macOS原生target，不添加臆测feature或交叉平台目标。
- 模拟策略：规则直接传入真实结构；文件用临时目录，事务使用真实前置存储，API走实际router/权限中间件，SDK走本机HTTP测试服务核对线协议。外部模型上游、时钟、进程表及重启可替代，记录输入/输出形状和不能证明的真实客户端行为。

## 步骤

1. 阅读本任务文件与只读plan、前置契约、真实调用者和相邻测试，先确认范围/依赖，检查工作区避免覆盖无关变更。
2. 编写上述独立失败测试并登记测试模块，领取共享槽位后运行过滤命令；确认至少1项有效测试且因预期行为失败，编译错误或零测试不能作为失败行为证据。
3. 仅实现“接通聚合成员与默认项API”的最小变更，明确返回不支持、权限、版本冲突和目标失败，不伪造成功。
4. 执行 `cargo fmt --all` 后审查差异并排除无关格式化；运行相同过滤测试，再运行 `cargo clippy --all-targets --all-features -- -D warnings`。
5. 对真实行为与预期逐项核对，保留脱敏记录；只有测试通过后才能作本任务范围内必要整理。

## 验证

- 聚焦命令：`cargo test -p vw-agent --lib aggregation_t40_ -- --nocapture`，预期至少1项匹配测试、全部通过、退出码0。
- 必需检查：`cargo clippy --all-targets --all-features -- -D warnings`，修复本次引入的问题；既有或其他任务错误不得顺手修复，保留路径/诊断/退出码。
- 不执行无过滤全量单测、`cargo fmt --all -- --check`、wasm检查或Makefile；无需其他平台验收。
- 所需证据：失败/通过的同一过滤命令、有效数量、失败原因、退出码、实际实现路径/符号、权限与失败场景、槽位与绝对CARGO_TARGET_DIR；证据写入 `docs/shiju/agent-apps/aggregation/任务40-验证.md`，不含密钥、原始账号/进程载荷。

## 完成

只有本任务所列新证据齐全才标为 `已完成`。有实际代码、测试、配置或文档变更，使用 `$git-commit` 审查并只提交本任务；仅编号任务状态/删除不调用该技能、不单独提交。最终报告证据并删除已完成任务文件，正式证据文档保留。

无法满足真实前置或必需环境时，只在本任务记录 `已阻塞`、缺失文件/契约、已运行检查及最小解除条件；先尝试在本任务范围解决，不能吞并整个前置母能力或修改只读plan。无关既有编译错误阻止回归时可标 `已完成，待回归` 并保留任务文件，必须说明尚缺证据，不算母任务完成。不修改、不提交其他任务或既有工作区变更。


## 2026-11-11 前置接口核对与阻塞证据

按当前检出执行，没有创建或启动工作树，没有修改只读 plan.md。已读取仓库的 do-task-plans、rustcodegraph 技能、任务卡、只读计划、执行分线、接口契约及任务7/11/14/39正式证据。任务7由 `384e6dd98` 交付，任务39由 `4f0f1b5b9` 交付状态/预览、授权及完整修订接口；任务39当前状态仍为“已完成，待回归”，受控副本11项通过不能改写为正常入口已通过。本次没有把这些前置测试计入任务40。

### 已调查的最小底层补齐路径

用户允许在原所有者补齐必要的最小底层能力。已沿真实调用链检查，而非仅因拟新增文件缺失停止：

1. `aggregation/membership.rs::set_member` 是成员和默认项的现有规则所有者，但对已附着的有效聚合变更明确返回 `UnsupportedTarget`。其最终 `commit_checked` 的 `PendingTarget` 不携带 project_scope；任务39读取的项目绑定状态会被 `providers/live/project_binding.rs::validate_update` 拒绝。因此单纯给该函数增加HTTP调用者不能工作。项目字段适配本身属于可补齐的小缺口，但不能解决以下发布链缺失。
2. 现有生产 `handlers/app_routing/lifecycle_api.rs::RoutingControl::publish_aggregation` 内部固定 `let app = AppId::Codex`，授权和绑定检查均只面向Codex。`app_routing/aggregation_routing.rs::AggregationPublication::validate` 也明确拒绝所有非Codex应用。这不是缺少一次router注册；将Claude/Desktop套入该入口会在配置提交回调前被拒绝，或要求改动真实多协议数据面的能力边界。普通Claude发布 `app_routing/provider_publication.rs::apply` 明确拒绝 `state.stack.enabled`，不能借用普通路由冒充聚合。
3. 原生Claude目录入口 `aggregation/claude_publish.rs::publish` 读取现有成员，而非接受“旧状态+新成员”的事务计划；其客户端凭证固定为 `CLIENT_TOKEN_PLACEHOLDER`，运行时接口仅提供地址及提交后 `publish`，没有消费监听发布回调签发凭证的入口。不能先提交新名单再调用它，否则重发布失败会留下已改变名单，也不能把占位符当作生产授权令牌。
4. Codex `aggregation/codex_publish.rs::publish` 明确只发布目录和指针：`PendingTarget::default()` 不提交新成员、默认路由或完整客户端契约。需要由同一既有事务接收冻结的新成员/模式以及监听凭证，不能先后调用两次事务宣称原子更新。Desktop `providers/claude_desktop/publication.rs::run` 虽接受新stack，但最终 `ClaudeDesktopFiles::commit_profile` 仍走不携带project_scope的旧 `commit_checked`；同样无法发布任务39要求的已绑定槽位。

综上，已找到可复用的规则、作用域及修订所有者，但尚无满足三客户端的现成联合重发布入口。只实现未附着成员写入、附着时恒返回503/422，不能满足本卡要求；去除应用/项目限制、使用占位凭证或分两次提交会削弱授权或原子性。补齐全部缺口需要常驻聚合发布所有者交付Claude/Desktop应用边界及三客户端联合原生提交接口，不能在本API卡中另建并列发布体系或借用model_relay公共化来绕过。

### 最小解除条件

在原有所有者内交付并验证如下生产接口后继续本卡：

- 已授权主机/项目/应用的聚合发布入口支持本卡应用，使用真实监听候选凭证；回调前失败保留旧目录/名单及旧授权，已开始发布失败报告原事务pending恢复，不伪造回滚或成功。
- 原生Claude/Codex/Desktop发布计划在同一 `commit_project_checked` 中提交预像、新成员/永久键、新默认项及该客户端完整目录/契约；接收冻结的输入与提交前修订复核，不先保存名单。保留Codex官方默认项的账号验证关口。
- 用实际监听与临时原生文件证明上述接口行为；正常入口完成任务39回归，或由调度明确接受其当前有界证据作为本任务前置。

### 本次已运行检查

- `rustcodegraph status`、`rustcodegraph explore 'aggregation_read_snapshot ProviderTransactions set_stack_member set_route'`：退出0；新接口未由图完整覆盖，随后用rg及文件读取核对实际所有者。
- `git show --no-patch --format='%h %s' 4f0f1b5b9 cc9543d60`、任务7实现文件历史查询：退出0，核对任务6/7/39实际提交；读取任务7/11/14/39正式证据及来源 `mode/controller.rs::set_route/set_stack_member/publish_stack` 和相邻测试位置。
- `uname -sm`：退出0，Darwin arm64。`git status --short`确认共享检出有大量其他任务未提交修改，本次未覆盖、暂存或还原它们。
- 本次没有Rust/测试/配置/正式文档变更；未执行Cargo、格式化、任务40过滤测试或Clippy。新增有效测试数量为0，没有行为红灯或通过证据，不宣告完成。
- Cargo槽位：未领取；CARGO_TARGET_DIR：未设置（未进入任何Cargo阶段）。未触碰其他会话锁、进程或缓存。
- 仅更新本编号任务文件状态及上述阻塞记录；按用户要求不提交、不删除，保留任务卡待解除前置。


## 协调后恢复
任务16获授权补齐共享原事务候选记录/目录修订/名单/默认项。任务40独立推进DTO、复用项目级写授权和意图校验；未附着路径沿原成员所有者保存，附着重发布等待16接口明确后接线。历史调查不作为永久阻塞，任务未完成、不删除卡片。


### 独立准备进度（未完成验收）

已新增严格DTO `AggregationMemberRequest` / `AggregationDefaultRequest`；网关 `aggregation_members.rs` 注册PUT成员和默认项，复用共享 `authorize_aggregation_write`，按已落定attached状态复核路由权限，使用任务39签名修订，拒绝异项目/未认领/pending。原成员所有者新增 `plan_member` 纯意图规划及 `set_member_project`，租户旧入口不获得项目写能力，未附着状态沿原事务提交；未新增名单存储。附着变更暂保持原所有者明确拒绝，未将这一状态计为任务完成。

任务16当前 `gateway/aggregation_resync.rs::RecordPublicationContext::replace(key,before,after)` 面向记录替换，其candidate由旧state克隆和顺序同步生成。本任务待消费的最小接口需允许同一expected预像下传入 `membership::plan_member` 产生的stack/mode（含候选默认项）并进行真实重发布；不得通过先存名单或无关记录替换伪造该输入。原生规划真实调用者现为本任务网关。

首次正常入口命令 `cargo test -p vw-agent --lib aggregation_t40_ -- --nocapture` 使用槽位4，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4，出现其他任务OMP私有router及Desktop测试Debug等错误；本任务断言使用了不具备Debug的AppState/ScopedProvider，已改为真实相等性断言。该次执行0项，不是行为红灯。收到编译诊断后仅终止自有编译；自有包装器在信号处理中的重入wait阻塞，确认Cargo已结束、无消费者后终止包装器，执行137，精确核对owner后释放槽位4，缓存保留。

受控源码副本 `/tmp/shiju-t40-focused-source` 保留真实生产路由/授权/存储，只把vw-agent无关test条件切至非测试生产分支，保留aggregation_members_tests注册。确切命令 `cargo test --manifest-path /tmp/shiju-t40-focused-source/Cargo.toml -p vw-agent --lib aggregation_t40_ -- --nocapture` 首次在动态槽位1（CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1）取得真实红灯：4项执行，全部因缺少写路由返回404而失败，退出101；已正常释放锁。实现及扩充后的6项同一过滤命令正在动态槽位13验证，未提前记录通过。各Cargo阶段jobs=1、test opt-level=0/debug=0，构建目录为本槽位内build；一次仅一组重型Cargo。
