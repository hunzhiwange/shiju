# 任务 4　统一 ACP 后台入口授权

批次：【批次 4】依赖批次 1、2、3

状态：未开始

目的：让计划运行、任务调度和定时任务的 ACP 执行继承已确认的权限档，覆盖绕过聊天入口的自动化请求。

来源任务：无。本对话要求全部 ACP 使用同一两档权限。

预计会话范围：检查后台入口的授权传递。复用前置任务的权限策略和证据，聚焦 ACP 执行，不扩展普通模型工具的权限。

## 文件

- 修改：`crates/vw-agent/src/task_plan_run/runner.rs`、`crates/vw-agent/src/agent_scheduler/runner/gateway_executor.rs`。
- 核对：`crates/vw-agent/src/cron/scheduler.rs`、`crates/vw-agent/src/session/title.rs`、`crates/vw-acp/src/session_runtime/runtime/prompt.rs`、`crates/vw-cli/src/acp.rs`。
- 按可信授权来源补接：`crates/vw-api-types/src/task.rs`、`crates/vw-desktop/src/app/impls/agent_stream.rs`、`crates/vw-agent/src/gateway/api/handlers/task_plan_run.rs`、`crates/vw-agent/src/agent_scheduler/runner/local.rs`。
- 测试：`crates/vw-agent/src/task_plan_run/runner_tests.rs`、`crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs`、`crates/vw-agent/src/cron/tests/scheduler.rs`。

## 上下文

计划运行的 `gateway_session_options` 和本地调度的 `gateway_local_agent_stream_request` 固定为 ACP 完全访问。定时任务读取 `job.full_access`，标题生成显式拒绝工具。区分 ShiJu 发起的外部 ACP 与 `shiju acp` 对外服务角色。

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

- 行为：后台 ACP 缺少可信完全访问授权时使用审批档。有明确授权才按完全访问执行，显式拒绝及无交互策略继续生效。
- 失败验证测试：在计划运行、调度各添加 `acp_background_approval_` 测试，断言未授权入口不会送出 `full_access=true` 或 `approve-all`。
- 失败验证命令：`cargo test -p vw-agent --lib acp_background_approval_ -- --test-threads=1`。预期两个入口固定提权的断言失败。
- 通过验证命令：同上，加 `cargo test -p vw-agent --lib gateway_session_options -- --test-threads=1`。使用默认 features。通过批次 2 的验收入口增加计划运行或调度的无授权实际写入与敏感操作拒绝场景。
- 模拟策略：保留真实网关和 ACP 流程。仅替换上游模型及需要可复现的后台任务输入，使用临时工作区。

## 步骤

1. 列出聊天、计划运行、本地调度、定时任务的可信授权来源和当前选项。审查工作流子轮、模型切换、任务恢复和隔离工作区使用同一个授权来源。
2. 先更新锁定固定提权的旧断言，再移除 ACP 分支的隐式完全访问。父会话或任务已有可校验授权时传递当前值。缺少来源时默认审批，工作区存在或任务已开始不能代替授权。
3. 接入批次 1 的安全编辑策略。按有无交互能力分流，不能向无人应答的审批通道无限等待。无交互时保留现有显式拒绝或失败语义。
4. 回归定时任务显式授权、标题生成的拒绝、独立 ACP CLI 的显式安全选项。对 `shiju acp` 入站服务及其他 CLI 路径记录实际控制权和验证结果，禁止把外部客户端授权当作 ShiJu 内部授权。
5. 将各入口及已验证行为加入批次 2 的覆盖矩阵，记录需人工授权或未提供边界证据的运行时。

## 验证

保存失败和通过的命令结果。至少证明计划运行及调度不隐式提权，后台普通工作区写入仍能完成，敏感操作的拒绝有明确终态。运行 `cargo clippy --all-targets --all-features -- -D warnings`，记录测试数量和退出码。

## 完成

证据完整后标记为 `已完成`。记录剩余限制及对应入口。存在实际变更时按 `$git-commit` 提交，排除无关改动。
