# 任务 5：跑通GitHubCopilot客户端Chat档位

批次：【批次 5】 依赖批次 1

状态：未开始

目的：验证 GitHub Copilot 的会话档位或启动档位确实改变同一客户端模型的 Chat 请求参数。

来源任务：无

预计会话范围：聚焦已审查 Copilot 1.0.105 的 BYOK Chat。动态入口可证实时优先复用；只有启动入口时只交付新会话初始档位。

## 文件

- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/copilot.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/copilot_tests.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/request.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`
- 检查或按行为最小修改：`scripts/acp_model_source_fixture.py`
- 检查或按行为最小修改：`docs/shiju/acp-model-source/evidence/copilot/verify.py`
- 检查或按行为最小修改：`docs/shiju/acp-model-source/evidence/copilot.md`

## 上下文

- 官方 ACP 文档列出 `--effort` 启动参数。当前托管驱动精确限制传入 argv，并在私有 providers.json 绑定客户端模型。动态设档及启动参数的 BYOK 效果均需探针确认。
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

- 失败验证测试：在 `crates/vw-agent/src/session/llm/acp/model_source/drivers/copilot_tests.rs` 添加 `acp_model_source_copilot_selected_effort_preserves_bound_model`，同时扩展专用 Copilot runner 比较两档真实主请求。
- 失败和通过验证命令：`cargo test -p vw-agent --lib --all-features acp_model_source_copilot -- --nocapture` 与 `python3 docs/shiju/acp-model-source/evidence/copilot/verify.py --agent copilot --mode session-model --manifest docs/shiju/acp-model-source/evidence/copilot/cli-versions.json --output target/acp-effort-evidence/copilot.json --offline-fixture --effort-check`。预期旧托管路径未显式应用设定档位，或上游参数相同/缺失。
- 模拟策略：真实固定版 CLI，BYOK Chat 回环上游。旧版若只有启动参数，两次从同一模型创建隔离的新会话，以两种显式档位作为变量。

## 步骤

1. 探测固定版是否公布可用的动态档位并在 BYOK 路由上生效。若只有启动参数，先独立验证两个值实际改变请求体。
2. 优先采用已验证的会话选项。启动参数路径只能由托管驱动追加已校验的档位，拒绝任意 argv，保留私有配置、账号回退防护和历史隔离。固定版若两种路径均无法验证，记录阻塞，不增加对外支持。
3. 验证同一客户端模型收到两个不同档位请求，核对协议字段和响应。若只支持启动档位，明确运行中修改需要新会话，并向任务 6 提供这项限制。

## 验证

- Rust 源码改动后先运行 `cargo fmt --all`，核对差异并排除无关格式改动。
- 运行：`cargo test -p vw-agent --lib --all-features acp_model_source_copilot -- --nocapture`
- 运行：`python3 docs/shiju/acp-model-source/evidence/copilot/verify.py --agent copilot --mode session-model --manifest docs/shiju/acp-model-source/evidence/copilot/cli-versions.json --output target/acp-effort-evidence/copilot.json --offline-fixture --effort-check`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`。
- 预期与所需证据：两个真实 BYOK Chat 请求实际参数不同且模型相同；明确记录验证通过的控制时机、会话是否相同、字段路径和两个值。
- 保存脱敏证据、实际命令、退出码、有效测试数量、槽位及绝对 `CARGO_TARGET_DIR`。证据不得包含认证头、凭据、提示词、完整请求体或生产用户数据。

## 完成

仅在达到所需证据时标记 `已完成`。外部无关故障导致无法回归时记录为 `已完成，待回归` 并保留本文件。能力或上游契约尚未证明时标记 `已阻塞`，记录已运行的探针、失败位置与仍缺证据。
有实际代码、测试、配置或文档变更时使用 `$git-commit` 只提交本任务改动。仅更新或删除本任务文件时无需提交。
