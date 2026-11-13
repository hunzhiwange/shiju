# Relax 桌面 SIGILL 调查与临时缓解

用户补充触发条件：操作终端拆分、连续点击时崩溃，不能只用悬停会话详情覆盖。

用户日志：signal 4，PC 落在 `{closure#1392}@154741 +0xfd8`，标记 `O3 fresh`；调用者为 `Option::map_or_else`、`session_row::local_time`、`details_card`。这确定了故障位置，尚不足以确定是哪一个优化 pass 生成了错误代码，或排除上游内存破坏。

本轮只读检查当前运行的 Relax 桌面，并通过界面打开 shiju 项目和现有会话。详情卡能显示创建/更新时间，检查时闭包运行于 O2。该进程另有其它开发任务引入的布局变更等待重启，未代替用户重启进程。

独立样例使用当前安装的运行器 SHA-256 `934974d77427927d96cb51c3c84f805c87bb152baf40f987e2d3e2a8e9cb579b` 和 vendored chrono 0.4.45。`repro.rs` 循环覆盖 0 和有效毫秒时间戳，200 轮、每轮各调用 1000 次。`MAKEPAD_RUN_O3_CALLS=1` 下通过，日志确认 local_time 及其闭包实际升级到 O3。同一缓存目录以 MAKEPAD_RUN_O3=0 再次运行也通过，未出现 O3 安装统计。

因此样例不是有效的崩溃回归，不能声称已修复编译器。

临时缓解位于 scripts/relax_dev.py：仅 desktop 入口在未显式设置时使用 MAKEPAD_RUN_O3=0，并在终端打印当前设置与原因。Relax 缓存配置哈希包含 tier1 开关（compiler/src/cache.rs 的 OpenFrom 与配置哈希），无需清空所有缓存。显式设置 O3=1 仍可用于复现。Gateway 和常驻测试策略不变。代价是桌面热代码失去 O3 优化；其他层的 JIT 缺陷仍可能出现。

验证：启动脚本 12 项测试通过，覆盖桌面默认 O3=0、显式 O3=1、保留热更新、其它入口既有行为。下一步应在完整应用中捕获故障闭包的 O3 汇编与对应 RIR，再缩减真实回归；在此之前维持临时降级，不改 UI 时间显示逻辑。

最终整仓验证：`python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings` 退出 0，耗时 4 分 48 秒，见 clippy.log。入口脚本测试最终 12 项通过，见 launcher-tests.log。尚未在关闭 O3 的完整桌面进程中重跑用户的终端拆分操作，因此缓解效果仍需实机复测。
