# 任务 46: 接通Desktop专属配置API

批次：【批次 12】 依赖：批次 4 的任务 31、批次 11 的任务 39。同批实施与资源规则见 prompt.md。

状态：实施中

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

目的：Desktop映射、发布与恢复通过专属类型化入口，不伪装成Claude Code；错误与安装/模式状态可查询。

来源任务：`.plans/2026-11-05-CC能力完整融合/10-聚合目录与客户端特殊能力.md`；源码入口 `claude_desktop_config.rs`。

预计会话范围：只完成“接通Desktop专属配置API”这一验证目标。建议一个20–45分钟聚焦会话；不顺带实现下一层入口、重构依赖或补齐整个母能力。构建耗时另计。

## 依赖与文件

上述直接依赖必须有完成证据；已删除任务文件以提交和正式证据为准，不能仅以文件不存在认定完成。代码实施按任务 1 登记核对本任务实际使用的母能力接口。

- 拟新增/修改业务：`crates/vw-agent/src/gateway/api/handlers/aggregation_desktop.rs`。
- 最小注册：`crates/vw-agent/src/gateway/api/handlers/mod.rs`（仅本模块注册，不顺手重构共享文件）。
- 独立测试：`crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_tests.rs`；新增测试名统一前缀 `aggregation_t46_`，本任务至少一项有效行为测试。
- API类任务仅增补 `crates/vw-api-types/src/agent_apps/aggregation.rs` 及其独立测试，并在 `crates/vw-agent/src/gateway/api/handlers/mod.rs` 最小挂载；代理任务以gateway/agent_apps为拟新增接线位置，复用母能力8实际常驻路由注册；若已存在对应模块则使用其准确路径。现有src/model_relay是会话绑定能力，不得扩权或改作公共常驻代理。不存在的父模块只作最小声明。

## 上下文

读取来源根 `/Users/xiangmin/worker/code/cc-switch/src-tauri/src/` 下 `claude_desktop_config.rs`（TS入口从来源根 `src/` 读取）及相邻测试；参考 `来源映射.json`。以任务2的 `docs/shiju/agent-apps/aggregation/接口契约.md` 为类型/方法/错误准绳。以上是拟新增文件，若前置已实现等价能力，应扩展真实所有者并记录路径，禁止重复体系。

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

- 行为：Desktop映射、发布与恢复通过专属类型化入口，不伪装成Claude Code；错误与安装/模式状态可查询。
- 失败验证测试：在 `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_tests.rs` 新增 `aggregation_t46_aggregation_desktop_preserves_contract`；用该前缀为边界用例命名，测试业务输出/实际副作用，不断言源码文本。
- 失败验证命令：`cargo test -p vw-agent --lib aggregation_t46_ -- --nocapture`。
- 预期失败原因：当前实现尚未具备上述行为或缺少接入；先把测试接入真实编译入口，再确认断言失败。若等价实现已通过，则新增此前未覆盖的实际失败边界，不制造人为失败。
- 通过验证命令：`cargo test -p vw-agent --lib aggregation_t46_ -- --nocapture`；使用crate默认features、macOS原生target，不添加臆测feature或交叉平台目标。
- 模拟策略：规则直接传入真实结构；文件用临时目录，事务使用真实前置存储，API走实际router/权限中间件，SDK走本机HTTP测试服务核对线协议。外部模型上游、时钟、进程表及重启可替代，记录输入/输出形状和不能证明的真实客户端行为。

## 步骤

1. 阅读本任务文件与只读plan、前置契约、真实调用者和相邻测试，先确认范围/依赖，检查工作区避免覆盖无关变更。
2. 编写上述独立失败测试并登记测试模块，领取共享槽位后运行过滤命令；确认至少1项有效测试且因预期行为失败，编译错误或零测试不能作为失败行为证据。
3. 仅实现“接通Desktop专属配置API”的最小变更，明确返回不支持、权限、版本冲突和目标失败，不伪造成功。
4. 执行 `cargo fmt --all` 后审查差异并排除无关格式化；运行相同过滤测试，再运行 `cargo clippy --all-targets --all-features -- -D warnings`。
5. 对真实行为与预期逐项核对，保留脱敏记录；只有测试通过后才能作本任务范围内必要整理。

## 验证

- 聚焦命令：`cargo test -p vw-agent --lib aggregation_t46_ -- --nocapture`，预期至少1项匹配测试、全部通过、退出码0。
- 必需检查：`cargo clippy --all-targets --all-features -- -D warnings`，修复本次引入的问题；既有或其他任务错误不得顺手修复，保留路径/诊断/退出码。
- 不执行无过滤全量单测、`cargo fmt --all -- --check`、wasm检查或Makefile；无需其他平台验收。
- 所需证据：失败/通过的同一过滤命令、有效数量、失败原因、退出码、实际实现路径/符号、权限与失败场景、槽位与绝对CARGO_TARGET_DIR；证据写入 `docs/shiju/agent-apps/aggregation/任务46-验证.md`，不含密钥、原始账号/进程载荷。

## 完成

只有本任务所列新证据齐全才标为 `已完成`。有实际代码、测试、配置或文档变更，使用 `$git-commit` 审查并只提交本任务；仅编号任务状态/删除不调用该技能、不单独提交。最终报告证据并删除已完成任务文件，正式证据文档保留。

无法满足真实前置或必需环境时，只在本任务记录 `已阻塞`、缺失文件/契约、已运行检查及最小解除条件；先尝试在本任务范围解决，不能吞并整个前置母能力或修改只读plan。无关既有编译错误阻止回归时可标 `已完成，待回归` 并保留任务文件，必须说明尚缺证据，不算母任务完成。不修改、不提交其他任务或既有工作区变更。


## 2026-11-11 当前检出前置核对

在原检出 `/Users/xiangmin/worker/code/shiju` 执行，未创建工作树，未修改只读 plan.md。读取仓内 `skills/do-task-plans/SKILL.md`、`skills/rustcodegraph/SKILL.md`、执行分线、任务卡、接口契约与来源 `cc-switch/src-tauri/src/claude_desktop_config.rs` 的 run/get_status、发布及恢复测试定位。直接依赖任务31由 `b6aaab1c2` 和《任务31-验证.md》交付；任务39的生产接口由 `4f0f1b5b9` 交付，正式证据记录受控副本11项通过，但原任务仍为“已完成，待回归”，不能当作全仓回归通过。读取二者证据，不将前置测试计入本任务。

### 实际生产缺口

契约同时要求独立 Desktop 状态、带修订的映射，以及 `POST /v1/agent-apps/claude-desktop/aggregation/publish` 和 `/exit` 的真实发布/恢复结果。当前不具备完整接通所需的执行链：

- `crates/vw-agent/src/app_routing/aggregation_routing.rs::AggregationPublication::validate` 显式仅接受 `AppId::Codex`，其他应用返回422。纯规则 `aggregation::resolve` 已支持 Desktop 数字目录，但常驻发布器尚不消费其 Desktop 目录和目标。
- `crates/vw-agent/src/app_routing/dispatch.rs::forward_json` 仅接受 Codex Chat/Responses 和 Claude `/v1/messages`，Desktop 走不支持分支；尚无 Desktop `/claude-desktop/v1/models` 的目录响应实现。`protocol_errors.rs` 对路径识别和 `route_auth.rs` 的 Desktop Origin 限制不等于对应业务转发与发现已经实现。将 Desktop 绑定改为 Claude 会违反独立应用作用域。
- `crates/vw-agent-apps/src/providers/claude_desktop/publication.rs::run` 使用 `ClaudeDesktopFiles::commit_profile`，其 PendingTarget 不带 project_scope。`live/project_binding.rs::validate_update` 会拒绝已绑定项目的槽位使用无项目目标；未绑定槽位直接调用该路径又不会持久认领项目。此处需要在原 Desktop 文件所有者上接入 commit_project_checked/作用域恢复，不能靠 API 先后分开写状态和文件。
- 既有 `handlers/app_providers/desktop_mapping.rs` 仅目录保存，接收无 projectId/expectedRevision 的旧 DTO，不能代替本任务要求的项目授权、有修订的映射与生效重发布。任务39只提供读授权、快照、修订及令牌签发，未交付 aggregation_write 或发布/退出执行器。
- 契约的发布与退出 API 所有者为后续任务41/42（卡片仍在，分别批次13/15）；目前 handlers 下仅有 aggregation_read、aggregation_read_snapshot 及测试。为本卡另写一个异步发布/退出执行体系会重复该所有者。

### 最小解除条件与范围判断

已经调查原有事务、原生 Desktop、常驻路由和现有 Gateway 映射所有者。项目事务接线本身可以是小修，但仅修它仍无法验证真正发布后的请求和目录发现；只加状态/表单或返回 unsupported 的发布占位也不满足本卡完整目标。补齐整条链需要同时实现 Desktop 数据面、发布/退出授权及操作生命周期，超出本卡“一个窄接口”和“不能吞并整个前置母能力”的限制。本轮未添加重复执行器、拒绝占位路由或无法使用的客户端配置。

解除条件：在原有所有者交付 Desktop 常驻目录发现/消息转发和项目事务发布/恢复接口，并由任务41/42交付或明确归并通用发布/退出生命周期；随后本卡接状态、映射修订与专属类型、运行真实 router/权限/事务聚焦验收。任务39既有接口优先复用，无需重建其项目绑定、HMAC修订或令牌体系。若调度将这些生产缺口明确归入本卡，需要相应扩大实施范围和验收集合，不能只以编译通过认定完成。

### 已执行与未执行的检查

- `rustcodegraph status`：退出0，索引存在（7522文件）；`rustcodegraph explore 'aggregation_read desktop_mapping claude_desktop publication'`：退出0，返回原生 Desktop 的 run/get_status 调用者和映射所有者。当前未索引新增/改动代码及文档以 rg/文件读取复核。
- `git show --no-patch --format='%h %s' b6aaab1c2 4f0f1b5b9`：退出0，确认依赖提交；`uname -sm`：退出0，Darwin arm64。
- `rg` 核对上述常驻发布/转发、项目事务、API注册与卡片职责，退出0；未以源码搜索代替行为测试。
- 未修改 Rust、测试、配置或正式文档；未运行 Cargo、格式化、聚焦测试或 Clippy。新增有效测试0，无红灯/绿灯证据，不声明完成或待回归。
- Cargo槽位未领取，CARGO_TARGET_DIR未设置（无Cargo阶段）；未触碰其他任务锁、进程或缓存。仅本编号卡状态和阻塞记录有变更，无需 git-commit，不删除任务文件。


## 2026-11-11 扩展范围后恢复

协调明确将 Desktop 独立数据面、原文件所有者项目事务/恢复及专属状态映射纳入本卡；通用发布/退出生命周期仍由41/42接续。任务16拥有共同候选记录恢复及通用监听/校验核心，本卡以独立Desktop职责模块接入并保留其改动；不另建监听、操作生命周期或存储。原阻塞记录为历史调查，不代表恢复后的最终状态。
