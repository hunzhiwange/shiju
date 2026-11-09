# 任务 2　覆盖所有ACP后端并验收权限

批次：【批次 2】依赖批次 1

状态：已阻塞

目的：逐项实现并证明已注册 ACP 后端遵守两档权限契约，记录可验证的回退和无法验证的边界。

来源任务：无。本对话的 ACP 双档权限需求。

预计会话范围：只做后端权限适配和行为验证。基于批次 1 的共用策略，优先复用现有后端同步分支；各后端存在差异时逐项验证实际工具路径，不为它们引入用户选项。

## 文件

- 核对：`crates/vw-acp/src/agent_registry.rs`、`crates/vw-agent/src/gateway/api/handlers/global.rs`、`crates/vw-agent/src/session/llm/acp/config.rs`、`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs`、`crates/vw-acp/src/access_policy.rs`、`crates/vw-acp/src/managed_runtime/contracts.json`。
- 修改：`crates/vw-acp/src/client/access_policy.rs`、`crates/vw-acp/src/client/event_client.rs` 与实际需要变更的 `crates/vw-acp/src/access_policy/` 后端文件及其测试。
- 参考：`crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`、`scripts/verify_acp_model_source.py`。新建专用 `scripts/verify_acp_approval.py` 驱动真实桌面网关权限链。
- 新建：`docs/shiju/acp-model-source/approval-coverage.md`，记录各后端覆盖和证据。

## 上下文

结构化内置注册表与模型来源 dispatch 当前各 22 项且集合一致。网关和运行时还合并自定义命令与同名覆盖项。旧别名按实际解析归到同一后端；仅旧接口支持的命令随该入口验收。

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-10`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–10。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 10 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

仓库另设 Cargo 中间产物目录。领取槽位后令 `CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`，与现有验收脚本一致。格式化前后审查差异，仅保留本任务需要的格式变更。

## 测试计划

- 行为：每个已注册后端都进入同一两档策略；普通编辑可自动完成，敏感动作及授权撤销可验证。
- 失败验证测试：新增 `acp_desktop_approval_backend_matrix`，至少以一个当前还逐次弹窗的非 Claude 后端验证失效，并让测试从结构化注册表枚举 key，新增内置项漏覆盖时失败。另测未知自定义 key、同名替换可执行文件及兼容别名。
- 失败验证命令：`cargo test -p vw-acp --lib acp_desktop_approval_backend_matrix -- --test-threads=1`。预期非 Claude 至少有一条映射或回退行为不满足契约。
- 通过验证命令：同上，加 `cargo test -p vw-acp --lib acp_full_access_ -- --test-threads=1` 和 `cargo build -p vw-cli --bin shiju`，均使用默认 features。真实端到端运行 `python3 scripts/verify_acp_approval.py --gateway "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-approval-evidence/matrix.json`。该脚本为本任务交付，负责遍历实测矩阵并生成结构化证据，支持复用现有验收脚本。
- 模拟策略：只模拟上游模型输出，定向生成连续写文件、敏感动作、终端和拒绝/撤销场景。适配器、ACP 传输、宿主与实际文件操作使用固定版真实程序和临时项目，不调用用户项目、真实凭据或已有会话。

## 步骤

1. 枚举并对照生产注册和启动路径。当前起点为 `auggie`、`claude`、`codex`、`codebuddy`、`copilot`、`cursor`、`droid`、`gemini`、`goose`、`hermes`、`iflow`、`kilocode`、`kimi`、`kiro`、`openclaw`、`opencode`、`pi`、`qoder`、`qwen`、`snow`、`trae`、`vibe`。以后端注册与运行时解析为准。用可控的自定义 ACP fixture 验证通用策略，同名覆盖按真实可执行文件和版本重新核验，供应商名字不能赋予权限。
2. 对每个 key 记录原生能力证据和选择。采用经证实的原生限制及协议回读，或采用宿主侧可验证的编辑。只支持 CLI 原生模型的条目也必须进入权限矩阵。外部说明用于取证，版本行为以实测为准。
3. 逐项验证最宽松的安全行为。现有默认模式已经满足时只补证据。后端可在进程内执行且宿主无法约束时，只有经验证的原生控制或已有隔离能够证明边界。其余情况保留能执行的宿主审批；无法强制边界时，明确报告不可验证并拒绝无法约束的权限路径。
4. 对新建、加载、恢复、完全访问切回审批、后端不回读、并行会话隔离跑行为测试。测试脚本可证明写入确实发生或被拒绝，以及同一工具未因权限应答被重放。
5. 未安装或需账号授权的适配器先完成可独立验证的共享路径，记录阻塞及复现方式。缺失的实机行不能计作验收通过。

## 验证

矩阵必须覆盖生产名单，每行记录模型来源、平台、版本、回退及验证结果。可运行条目须有真实文件变化与审批事件证据。运行 `cargo clippy --all-targets --all-features -- -D warnings`。有缺失验收行时按阻塞或待回归规则处理，不标记全部完成。

## 完成

仅在全部完成证据齐全时标记为 `已完成`。遇到未获授权的真实运行时或其他实质阻塞，记录具体后端、平台、版本、已尝试命令和缺失证据。实际变更按 `$git-commit` 提交，排除其他会话变更。

## 实施记录

批次 1 的宿主写入边界和 Claude 安全回退由 `42689b4a0`、`640bceea0` 交付。批次 2 提交为 `9f932a8c1`、`e7a454ef3`、`7a4398a24`。实机网关使用本工作区已有的桌面单次审批桥接。

- 宿主工具和终端要求当前显式完全访问授权。无授权的 `ApproveAll` 回落到审批档。红测在 Auggie 收到 `AllowOnce` 时失败。Codex 审批档确认 `read-only`，兼容名称使用同一权限守卫。
- 固定版 OpenCode 两种来源均复现审批档的原生 Bash 直接创建和删除文件。原生代理专属规则可以覆盖全局规则，桌面审批档改为在启动前拒绝该路径。两种来源均实测显式完全访问可完成五项原生操作，撤销后在模型请求前拒绝。Rust 启动守卫先红后绿。
- [逐后端覆盖](../../docs/shiju/acp-model-source/approval-coverage.md)列出全部 22 个后端的固定版本和模型来源。Darwin-arm64 的真实网关矩阵在 `target/acp-approval-evidence/matrix.json`。
- Claude 0.26.0 两种来源均验证两次原生编辑在单次批准后落盘、敏感操作拒绝和撤销后重新拒绝。Codex 2.5.0 两种来源均验证只读回退。Copilot 1.0.105 两种来源均验证单次审批与实际命令副作用。Gemini 0.66.0 两种来源均验证确认后覆盖写入、敏感操作拒绝与撤销。Gemini 保持原生工作区限制，完全访问仍拒绝越界 Write。客户端模型使用原生 Google 中继，Chat 转换的写入轮次曾返回 400，未计为独立验收路径。
- 同名 Codex 替换夹具证明结构化宿主创建和编辑无需批准，完全访问可越界写入，撤销并加载后重新拒绝。两个并行工作区同时等待审批，六次跨工作区答复、六次重复答复均被拒绝。真实供应商审批事件也检查了跨工作区答复、永久批准和重复答复。
- 44 条矩阵行中 10 条真实通过、5 条为生产不支持的客户端模型路径、29 条阻塞。27 条缺已安装固定版，Pi 的两条路径缺通过摘要核验的补丁适配器与 0.73.1 引擎。已检查 PATH、管理运行时目录与已展开的本地缓存。Gemini 以已有本地包复制到临时目录，应用仓库已审核补丁并校验前后摘要。后续补齐获授权的固定运行时及所需隔离模型配置，再重跑同一矩阵。

### 验证记录

- 槽位 1 使用 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，用于通过测试、网关构建与真实矩阵。
- 槽位 2 使用 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，用于前一阶段的复验、提交格式核对和定向 Clippy。
- 槽位 3 使用 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，用于 OpenCode 启动守卫红测。
- `CARGO_BUILD_BUILD_DIR` 与对应槽位绝对路径一致。格式化在隔离源码快照运行。

| 命令 | 退出码 | 有效证据 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 本任务 Rust 文件与格式化输出一致 |
| `cargo test -p vw-acp --lib acp_desktop_approval_backend_matrix -- --test-threads=1` | 红测 101，修复后 0 | 1 项通过，含 22 个注册 key 与兼容及覆盖场景 |
| `cargo test -p vw-acp --lib acp_full_access_opencode_desktop_requires_current_grant -- --test-threads=1` | 红测 101，修复后 0 | 1 项通过 |
| `cargo test -p vw-acp --lib acp_full_access_ -- --test-threads=1` | 0 | 84 项通过，含 OpenCode 启动守卫 |
| `cargo test -p vw-acp --lib acp_desktop_approval_policy_real_claude_falls_back -- --ignored --test-threads=1` | 0 | 前一阶段固定版 Claude 回退 1 项通过 |
| `cargo build -p vw-cli --bin shiju` | 0 | 默认 features 网关构建成功 |
| `python3 -m unittest discover -s scripts -p acp_approval_model_tests.py` | 0 | 3 项通过 |
| `python3 scripts/verify_acp_approval.py --gateway "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-approval-evidence/matrix.json` | 1 | 10 条真实通过，29 条缺固定运行时，5 条生产不支持。并行宿主夹具通过 |
| `cargo clippy -p vw-acp --all-targets --all-features -- -D warnings` | 0 | ACP 定向检查通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 101 | 无关的 `vw-desktop/src/app/message/project/helpers.rs:668` 调用 `session_list_for_project` 触发 E0599 |

### 固定运行时补验

- `39f18f856` 修复 npm 11 从暂存目录外执行时生成错误锁文件路径的问题。CodeBuddy、iFlow、Kilocode、OpenClaw、Pi、Qwen、Snow 的固定 npm 包由仓库安装器核对摘要并在独立临时目录安装。Hermes、Vibe 使用已核验的 wheel 和仓库补丁，Goose 核对官方源码和补丁后构建，Kiro 和 Kimi 核对官方固定版产物。验收仅使用回环模型和临时配置，未使用真实账号凭据。
- `5c633a07e` 收紧 Hermes、Kilocode、OpenClaw、Pi 的桌面原生执行路径。实测发现审批档及撤销后仍存在原生命令或文件副作用，两种来源现均在模型调用前拒绝未授权路径。Goose 保持原生 `approve`，允许切换时刷新候选列表，继续核验实际选择的模型、供应商和思考配置。
- 夹具处理 Kiro 回环控制面、Qwen 覆盖前读取、Snow 的工具名审批、Copilot 的原生 Bash 与 CodeBuddy 的辅助模型绑定。iFlow 和 Gemini 在完全访问档仍保持原生越界写入限制。OpenClaw 的部分工具 ID 由适配器重写，证据使用五个模型工具请求和实际文件状态。
- 五个外部行均为 Darwin arm64 原生路径。Auggie 0.36.0 和 Qoder 1.1.82 已核验 npm 包摘要，分别缺离线模型注入和有权账号。Droid 1.3.0 已核验官方 SHA-256，`initialize` 成功，`session/new` 返回登录错误 `-32000`，回环模型请求为零。Cursor 2026.10.22-27dcf5f 的固定包 GET 403。Trae 0.121.3 的固定包 HEAD 200，缺可核验摘要与离线配方。获取方式与配置入口见[逐后端覆盖](../../docs/shiju/acp-model-source/approval-coverage.md)。

本轮实际槽位为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` 至 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。槽位 1 用于 Goose 构建和 CodeBuddy、Copilot 定向复验，槽位 2 用于启动守卫红测和最终网关矩阵，槽位 3 用于 Rust 验证与 Clippy，槽位 4 仅用于旧网关报文诊断。两个 Cargo 目录变量均指向各自的槽位绝对路径。

| 命令 | 槽位 | 退出码 | 有效证据 |
| --- | --- | --- | --- |
| `cargo fmt --all` | 3 | 0 | 在隔离源码快照格式化，仅采用本任务文件的格式差异 |
| `cargo test -p vw-acp --lib acp_full_access_unconfined_desktop_adapters_require_current_grant -- --test-threads=1` | 2 | 101 | 红测复现 Kilocode 在未授权时被允许 |
| `cargo test -p vw-acp --lib acp_full_access_ -- --test-threads=1` | 3 | 0 | 86 项通过，含启动守卫和 Goose 配置检查 |
| `cargo test -p vw-acp --lib acp_desktop_approval_backend_matrix -- --test-threads=1` | 3 | 0 | 1 项通过，覆盖 22 个注册后端 |
| `cargo build -p vw-cli --bin shiju` | 2 | 0 | 默认 features 的隔离验收网关构建成功 |
| `cargo clippy -p vw-acp --all-targets --all-features -- -D warnings` | 3 | 0 | ACP 定向检查通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 3 | 0 | 全量 Clippy 通过 |
| `python3 -m unittest discover -s scripts -p acp_approval_model_tests.py` | — | 0 | 6 项通过 |
| `python3 scripts/verify_acp_approval.py --gateway "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-approval-evidence/matrix-final.json` | 2 | 1 | 34 条真实通过，5 条生产不支持，5 条外部阻塞。宿主夹具拒绝六次跨工作区和六次重复答复 |
| `python3 scripts/verify_acp_approval.py --gateway "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-approval-evidence/round3-goose-exact.json --agent goose` | 1 | 1 | 收紧固定版版本比较后 Goose 两条来源再次通过。定向模式不计为全矩阵通过 |

完整证据核对后已归档至 `target/acp-approval-evidence/matrix.json`。本会话的槽位锁均已释放。待上述五条路径具备外部条件后补跑。

### 共享工作区回归

2026-11-09 使用当前共享源码和已核验固定运行时复跑 Darwin arm64 完整矩阵。44 条供应商路径中 34 条通过、5 条生产不支持、5 条受外部条件阻塞，失败为 0。宿主夹具通过，拒绝六次跨工作区答复及六次重复答复。证据在 `target/acp-approval-evidence/matrix-regression.json` 和 `target/acp-approval-evidence/upstream-regression.json`。

| 命令 | 槽位 | 退出码 | 有效证据 |
| --- | --- | --- | --- |
| `cargo build -p vw-cli --bin shiju` | 3 | 0 | 当前共享源码的默认网关构建成功 |
| `python3 scripts/verify_acp_approval.py --gateway "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-approval-evidence/matrix-regression.json` | 3 | 1 | 34 条通过，5 条生产不支持，5 条外部阻塞，宿主夹具通过 |
| `cargo test -p vw-acp --lib acp_full_access_ -- --test-threads=1` | 4 | 0 | 86 项通过 |
| `cargo test -p vw-acp --lib acp_desktop_approval_backend_matrix -- --test-threads=1` | 4 | 0 | 1 项通过，覆盖全部 22 个注册后端 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 4 | 0 | 全量 Clippy 通过 |
| `python3 -m unittest discover -s scripts -p acp_approval_model_tests.py` | — | 0 | 6 项通过 |

两个 Cargo 目录变量使用对应的 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` 和 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`，本轮槽位锁已释放。

Auggie 固定版仍缺离线注入，Cursor 仍缺可下载的固定包及隔离模型路径，Droid 和 Qoder 的实机探针均在创建会话时要求平台认证，Trae 缺固定包核验摘要及同版本 ACP 配方。具体探针及补验前提见[逐后端覆盖](../../docs/shiju/acp-model-source/approval-coverage.md)。这些行仍缺实际工具权限证据，保留本任务待补验。
