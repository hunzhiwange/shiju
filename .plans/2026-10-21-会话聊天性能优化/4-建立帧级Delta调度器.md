# 任务 4: 建立帧级 Delta 调度器

批次：【批次 2】 依赖批次 1

状态：未开始

目的：建立与 UI 解耦的积压感知文本 delta 调度器，使活动会话按帧平滑推进、后台降频，并保证终态事件只在内容排空后提交。

来源任务：Codex Desktop 的帧级 delta 队列对照与任务 1 流式基线

预计会话范围：新增纯 Rust 状态机和可控时钟测试，仅在 stream 模块注册类型；不修改当前事件消费、渲染或持久化行为，便于独立验证调度语义。

## 文件

- 新建：`crates/vw-desktop/src/app/message/chat/stream/delta_scheduler.rs`
- 新建：`crates/vw-desktop/src/app/message/chat/stream/delta_scheduler_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/stream/mod.rs`

## 上下文

- 调度器按 session/item key 保存 UTF-8 安全缓冲区、上次释放时刻、首包状态和待处理 terminal。
- 活动会话目标间隔 16 ms，后台会话 100 ms；首个文本 delta 立即释放，正常目标约 24 个 Unicode 标量/帧。
- 根据当前积压和最长 500 ms 落后量放大单次配额，但每次最多释放 8 帧额度，避免一次追赶阻塞 UI。
- terminal、取消和错误必须等待同一 item 文本排空后按原顺序释放；工具/命令替换事件不与文本追加混成字符串。
- 不新增配置开关；常量仅在模块内显式定义并由行为测试约束。

## Cargo 共享槽位规则（仅 Rust/Cargo 任务）

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

- 行为：给定可控时间和事件序列，调度器产生确定的首包、帧配额、后台降频、积压追赶和 terminal 排空顺序。
- 失败验证测试：`stream_delta_scheduler_emits_first_delta_immediately`、`stream_delta_scheduler_targets_24_chars_per_frame`、`stream_delta_scheduler_caps_backlog_catchup`、`stream_delta_scheduler_drains_before_terminal`、`stream_delta_scheduler_keeps_replacements_separate`。
- 失败验证命令：`cargo test -p vw-desktop stream_delta_scheduler_emits_first_delta_immediately -- --nocapture`
- 预期失败原因：当前只有基于固定时长的流式合并，没有独立帧队列、积压配额或 terminal 排空状态机。
- 通过验证命令：`cargo test -p vw-desktop stream_delta_scheduler_ -- --nocapture`
- 模拟策略：时间作为函数参数传入，测试不 sleep；事件使用真实字符串与事件枚举，覆盖 ASCII、中文和 emoji 边界。

## 步骤

1. 定义调度输入/输出与不变量，用失败测试锁定首包和 UTF-8 边界。
2. 添加前台/后台间隔、24 字符目标、500 ms 落后窗口和 8 帧上限测试。
3. 添加 terminal、cancel、error 与工具替换排序测试。
4. 实现最小纯状态机，不依赖 Iced subscription 或真实时钟。
5. 运行大积压场景，证明单 tick 输出有上限且最终无丢失/重复。
6. 运行格式化、聚焦测试和完整 clippy。

## 验证

- 运行：`cargo fmt --all`
- 运行：`cargo test -p vw-desktop stream_delta_scheduler_ -- --nocapture`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：所有时序测试无需 sleep 且确定通过；Unicode 不被切坏；terminal 永远位于最后一个文本片段之后；单 tick 不超过 8 帧额度。
- 所需证据：首次缺失实现失败、各时序输出序列、有效测试数、实际槽位/路径/命令/退出码和 clippy 结果。

## 完成

完成时记录调度器状态、输入输出事件、关键常量和不变量。本任务不允许接线现有流消费路径；集成由任务 11 完成。
