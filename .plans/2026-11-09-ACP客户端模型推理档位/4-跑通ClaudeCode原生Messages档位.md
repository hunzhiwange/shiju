# 任务 4：跑通ClaudeCode原生Messages档位

批次：【批次 4】 依赖批次 1、3

状态：未开始

目的：在通过审查的 Claude Code 运行时上验证两档切换改变客户端模型的原生 Messages 请求。

来源任务：无

预计会话范围：只完成已验证运行时的设档和真实出站参数对照，维持现有 Chat 转换边界。

## 文件

- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/claude.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/claude_tests.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`
- 检查或按行为最小修改：`scripts/acp_model_source_fixture.py`
- 检查或按行为最小修改：`scripts/verify_acp_model_source.py`
- 检查或按行为最小修改：`crates/vw-agent/src/model_relay/messages.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/model_relay/messages_tests.rs`

## 上下文

- 原生 Messages 和 Chat 转换的 thinking 语义不同。Chat 路径的关闭行为有既有测试。以任务 3 公布的配置 ID 和档位为真值，使用支持自适应档位的模型。
- 线性顺序为任务 1 至 6。各批次只执行一个任务。共享验收入口与回归输出串行使用。仅有列明的批次是行为依赖。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

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

## 测试计划

- 失败验证测试：在 `crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs` 增加 Claude 的原生 Messages 两档子场景；同一模型会话设 A、B 后断言实际 `output_config.effort`，并检查模型要求的 `thinking` 形态。
- 失败和通过验证命令：`python3 scripts/verify_acp_model_source.py --agent claude --mode session-model --manifest docs/shiju/acp-model-source/cli-versions.json --output target/acp-effort-evidence/claude.json --offline-fixture --effort-check`。预期旧接线缺少正确设档或出站值保持默认。若已有真实通过证据，保留并按观察修最小缺陷。
- 模拟策略：真实 Claude CLI、ACP、生产路由，仅替换上游 Messages 服务。

## 步骤

1. 使用任务 3 的版本、固定协议和支持档位的客户端模型，先证明真实请求体不能通过两档断言或当前已自然通过。
2. 用原生会话 config option 设档，复用保存及回放。若存在能力或协议落差，按真实模型支持范围限制选项。避免引入统一预算换算。
3. 保留原生 thinking/签名语义与历史，确认 Chat 模型仍遵循已有降级边界。测试无档位时的正常请求与不支持值的明确拒绝。

## 验证

- Rust 源码改动后先运行 `cargo fmt --all`，核对差异并排除无关格式改动。
- 运行：`python3 scripts/verify_acp_model_source.py --agent claude --mode session-model --manifest docs/shiju/acp-model-source/cli-versions.json --output target/acp-effort-evidence/claude.json --offline-fixture --effort-check`
- 运行：`cargo test -p vw-agent --lib --all-features acp_model_source_claude -- --nocapture`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`。
- 预期与所需证据：真实 `/v1/messages` 的两个同会话主请求具有不同且准确的 `output_config.effort`，同时确认原生模型绑定及 Chat 既有保护。
- 保存脱敏证据、实际命令、退出码、有效测试数量、槽位及绝对 `CARGO_TARGET_DIR`。证据不得包含认证头、凭据、提示词、完整请求体或生产用户数据。

## 完成

仅在达到所需证据时标记 `已完成`。外部无关故障导致无法回归时记录为 `已完成，待回归` 并保留本文件。能力或上游契约尚未证明时标记 `已阻塞`，记录已运行的探针、失败位置与仍缺证据。
有实际代码、测试、配置或文档变更时使用 `$git-commit` 只提交本任务改动。仅更新或删除本任务文件时无需提交。
