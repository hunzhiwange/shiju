# 任务 4: plan-run 十秒增量刷新

批次：【批次 4】 依赖批次 3

状态：已完成，待回归

目的：把 plan-run 普通文本增量的桌面转发窗口从 220ms/360 字符改为 10 秒，降低 Iced 唤醒和日志噪声。

来源任务：用户提出“轮询频次太高了，降低到 10 秒每次”，并给出密集 `session_delta` info 日志。

预计会话范围：只收敛已有 `PlanRunDeltaBuffer` 的时间/大小冲刷策略及相关测试；不修改 SSE 服务端、协议或普通聊天流频率。

## 文件

- 修改：`crates/vw-desktop/src/app/impls/agent_stream.rs`
- 修改：`crates/vw-desktop/src/app/impls/agent_stream_tests.rs`

## 上下文

- 工作树已有未提交的 220ms、360 字符、16KiB 合并器和 session_delta 日志降为 debug 的改动，来自已完成计划 `.plans/2026-08-21-工作流流式卡顿/plan.md`。
- SSE 不是轮询；本任务降低的是 SSE delta 进入 Iced 消息循环的普通刷新频率。
- 只有成功转发完整事件后才能推进恢复游标；生命周期事件之前和流结束时必须冲刷缓存。

## 测试计划

- 行为：同一 session 的普通小增量在 10 秒前不转发，到 10 秒窗口、生命周期、session 切换或 16KiB 硬上限时冲刷；文本顺序、UTF-8 和恢复游标不丢失。
- 失败验证测试：增加编译期 10 秒常量断言，并把 `plan_run_delta_buffer_coalesces_before_ui_dispatch` 扩展为 9.999 秒不冲刷、10 秒冲刷且 360 字符不再提前冲刷。
- 失败验证命令：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 预期失败原因：编译期断言在现有 220ms 常量上失败；行为测试在旧 360 字符软阈值下会产生额外批次（因仓库禁令不实际执行，交接为待回归证据）。
- 通过验证命令：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 模拟策略：用确定性的 `Duration` 输入测试纯缓冲器；不启动网络或模拟 SSE 服务端。

## 步骤

1. 先加入 10 秒编译期断言和同 session 边界测试，运行 clippy 记录预期失败。
2. 将普通时间窗口改为 10 秒，并移除会让 360 字符提前冲刷的软阈值；保留 16KiB 硬上限。
3. 保留 session 切换、生命周期、终态、重连前的立即冲刷，以及背压失败不推进游标的行为。
4. 确认原始 `SessionDelta` 只写 debug；聚合转发可写 debug，其他结构化事件继续 info。
5. 更新合并测试的期望批次、事件范围和 UTF-8 顺序断言，避免只测试常量。
6. 运行 clippy，并审查未影响普通聊天 `ACTIVE/BACKGROUND_STREAM_DELTA_FLUSH_INTERVAL`。

## 验证

- 运行：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 预期：退出码 0；10 秒编译期断言通过，全部缓冲器测试可编译，生命周期与恢复游标分支仍覆盖。
- 所需证据：现有 220ms 下的编译期失败、修改后 clippy 退出码、9.999/10 秒边界差异、360 字符不提前冲刷的测试代码、日志级别差异审查。

## 完成

- `PlanRunDeltaBuffer` 的正常 UI 转发窗口为 10 秒，例外冲刷仅用于正确性和内存边界，密集 delta 不再产生 info 日志。
- 行为测试未获准执行时状态使用 `已完成，待回归`。

## 验证结果

- 依赖核对：批次 3 已实现并通过聚焦与全量 clippy，未完成项仅为 GUI 手工回归，不阻塞本任务。
- 失败验证：聚焦 clippy 退出码 101；编译期断言在原 220ms 常量上触发 `E0080`。
- 通过验证：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings` 退出码 0。
- 全量门禁：`cargo clippy --all-targets --all-features -- -D warnings` 退出码 0；`git diff --check` 退出码 0。
- 测试代码：覆盖 360 字符与 9.999 秒不冲刷、10 秒冲刷、session 切换、16KiB、UTF-8 顺序、事件范围及恢复游标；生命周期与背压测试保留。
- 日志与范围审查：原始 `SessionDelta` 和聚合转发仍为 debug，结构化事件仍为 info；普通聊天 50ms/100ms 刷新常量未改。
- 待回归：仓库禁令要求不执行 `cargo test`，新增行为测试仅由 all-targets clippy 证明可编译。
