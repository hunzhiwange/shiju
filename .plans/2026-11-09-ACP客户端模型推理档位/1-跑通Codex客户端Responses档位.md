# 任务 1：跑通Codex客户端Responses档位

批次：【批次 1】 依赖批次 无

状态：未开始

目的：用真实托管 Codex 在同一客户端模型会话内切档，验证上游 Responses 请求的 `reasoning.effort`。

来源任务：无

预计会话范围：一个生产链路基准及最小验收夹具扩展。为后续适配复用证据格式。只覆盖原生 Responses，当前工作树的 Codex Chat 转换单独保留。

## 文件

- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/config.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/request.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/controls.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/controls_tests.rs`
- 检查或按行为最小修改：`crates/vw-acp/src/session_mode_preference.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/codex.rs`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/codex_tests.rs`
- 检查或按行为最小修改：`scripts/verify_acp_model_source.py`
- 检查或按行为最小修改：`crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs`
- 检查或按行为最小修改：`docs/shiju/acp-model-source/evidence/codex/verify_production.py`
- 检查或按行为最小修改：`docs/shiju/acp-model-source/evidence/codex/responses_fixture.py`

## 上下文

- 固定 ACP 2.5.0 / Codex 0.165.0 为基线。先核对托管模型实际公布的选项、当前模型 ID 与协议；同一个设置可影响后续轮次，且旧会话的选项回放须继续有效。
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

- 行为：同一 session-model、同一上游模型、同一路径分别选择两个明确支持的档位。每轮使用可识别的合成标记，观察实际发送到回环上游的 `reasoning.effort`。
- 失败验证测试：在 `crates/vw-agent/src/session/llm/acp/model_source/drivers/codex_tests.rs` 添加 `acp_model_source_codex_effort_real_cli`，沿用真实生产 ACP 请求和会话设置入口。扩展现有 runner 的验收专用 `--effort-check` 参数，选择具名 ignored 实测入口。此参数只改变验收选择和证据检查；默认模型来源验收路径照旧。
- 失败和通过验证命令：`python3 docs/shiju/acp-model-source/evidence/codex/verify_production.py --agent codex --mode session-model --manifest docs/shiju/acp-model-source/evidence/codex/manifest.json --output target/acp-effort-evidence/codex.json --offline-fixture --effort-check`。测试新增后先在未改生产行为的基线运行；预期选项未公布、设档未应用或出站值缺失/相同。若基线已真实通过，保留通过证据，只修复另有明确失败证明的行为。
- 模拟策略：真实 Codex ACP、网关模型配置与请求中继。仅上游 Responses 使用现有认证回环夹具。记录两轮标签、来源、路由、绑定模型和白名单档位值。记录仅来自 fixture 收到的字段。

## 步骤

1. 先扩展现有 fixture 和 runner。定义后续复用的证据结构，至少含适配器及引擎版本、模型来源、脱敏模型标识、会话是否相同、两轮标签、请求路径、字段、值、有效测试数量。根据已知模型元数据选择两个档位；由真实主请求的合成标记区分轮次，排除标题、辅助请求和旧请求。
2. 通过现有会话选项读写路径先设 A，再在同一会话设 B；首轮可先发默认请求使会话连接。补测一个未公布档位被拒绝且此前确认值仍可用。对原生/托管及模型身份明确断言。
3. 逐层定位缺档或值不一致，优先复用 `vw_acp::resolve_config_value` 和已验证的模型档位元数据。仅在托管探针证明需要时补齐 Codex 模型能力配置。保留原生 Responses 转发、模型锁、权限和会话保存逻辑；禁止凭观察值改写请求来让测试通过。
4. 先收集成功后再验证恢复后的档位回放、普通无强度请求与已有选项持久化。对同一选项的显式会话选择、通用请求值和默认值确定一致优先级，不悄然覆盖已确认选择。
5. 将通用 `--effort-check` 对非 Codex 的扩展点限定在同一个验收 runner。后续任务可增加具体协议与字段断言，原有验收不得降级。

## 验证

- Rust 源码改动后先运行 `cargo fmt --all`，核对差异并排除无关格式改动。
- 运行：`python3 docs/shiju/acp-model-source/evidence/codex/verify_production.py --agent codex --mode session-model --manifest docs/shiju/acp-model-source/evidence/codex/manifest.json --output target/acp-effort-evidence/codex.json --offline-fixture --effort-check`
- 运行：`cargo test -p vw-agent --lib --all-features acp_session_options_follow_capabilities -- --nocapture`
- 运行：`cargo test -p vw-acp --lib --all-features config_resolution -- --nocapture`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`。
- 预期与所需证据：真实测试命中 1 项，认证上游 `/v1/responses` 收到同一模型的 A 与 B，两值准确且不同；存在无效档位拒绝与恢复后回放证据。
- 保存脱敏证据、实际命令、退出码、有效测试数量、槽位及绝对 `CARGO_TARGET_DIR`。证据不得包含认证头、凭据、提示词、完整请求体或生产用户数据。

## 完成

仅在达到所需证据时标记 `已完成`。外部无关故障导致无法回归时记录为 `已完成，待回归` 并保留本文件。能力或上游契约尚未证明时标记 `已阻塞`，记录已运行的探针、失败位置与仍缺证据。
有实际代码、测试、配置或文档变更时使用 `$git-commit` 只提交本任务改动。仅更新或删除本任务文件时无需提交。
