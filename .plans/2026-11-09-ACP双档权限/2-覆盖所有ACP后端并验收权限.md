# 任务 2　覆盖所有ACP后端并验收权限

批次：【批次 2】依赖批次 1

状态：未开始

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
