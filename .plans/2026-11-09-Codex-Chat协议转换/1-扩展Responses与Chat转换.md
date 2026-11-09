# 任务 1 扩展Responses与Chat转换

批次：【批次 1】 依赖：无

状态：已完成

补齐 Codex 文本、函数、命名空间和自由文本工具双向转换，验证多轮及流式工具回放。

## 文件

- 修改 `crates/vw-agent/src/` 下的 responses.rs、responses_stream.rs、transport.rs 及独立测试文件。

## 上下文

按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

沿用现有中继的绑定、授权、取消和失败路径。独立会话不得复用工具映射。

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

- 先新增行为断言，运行 `cargo test -p vw-agent --lib --all-features acp_model_source_relay_responses_codex -- --nocapture` 验证失败。
- 预期失败原因。当前转换拒绝 Codex 请求字段、命名空间和自定义工具。
- 最小实现后重新运行同一命令。用本地 Chat HTTP 服务记录请求并返回逐帧 SSE，验证原始补丁、工具 ID、回放参数与最终用量。
- 完成证据包含失败与通过的退出码、有效测试数和真实工具回合观察。最终运行 `cargo clippy --all-targets --all-features -- -D warnings`。

## 实际验证

- 槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo test -p vw-agent --lib --all-features acp_model_source_relay_responses_codex -- --nocapture` 执行 1 项，旧转换返回 422，退出 101。
- `RUST_MIN_STACK=33554432 cargo test -p vw-agent --lib --all-features -- acp_model_source_relay_responses acp_model_source_codex --nocapture --test-threads=1` 14 项通过，3 项实机入口默认忽略，退出 0。覆盖工具名称双向映射、分片自定义工具参数、空推理字段与空 logprobs 回放。
- `cargo clippy -p vw-agent --lib --all-features -- -D warnings` 退出 0。共享工作区已有修改，格式化限制在本任务 Rust 文件。
