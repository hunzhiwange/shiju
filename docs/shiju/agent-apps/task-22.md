# 任务22：生命周期写入互斥

2026-11-06，macOS。状态：已完成。

已读取 do-task-plans、RustCodeGraph 与 git-commit 技能、仓库协议、只读 plan.md、来源与接口、任务21证据及底座任务事件与错误契约。批次21提交 e745824ef，正式 crate、AppId、安装来源与生命周期配方已交付。RustCodeGraph status/explore 成功；新增领域文件未被索引，使用直接读取。未修改 upstream、来源快照或 plan.md。

新增 apps/lifecycle/coordinator.rs 和独立 coordinator_tests.rs，通过现有 apps/lifecycle/mod.rs 公开，无需重复修改 lib.rs。迁入来源 ToolLifecycleCoordinator::run 的工具预占、全局写锁与 spawn_blocking 所有权转移。所有协调器句柄共用同一执行主机进程的 OnceLock 状态；锁不按租户划分，调用者无法用另建实例绕过互斥。同工具排队或执行时重复提交返回 TOOL_ACTION_IN_PROGRESS，不执行闭包。不同工具串行写包目录；同批空目标、重复目标、桌面应用拒绝，失败时释放部分已获取的预占。

写锁和工具锁一起转移到不可中止的 blocking worker，HTTP 等待者断开不释放正在执行的写锁；排队等待者断开不启动操作，释放工具预占。闭包返回错误或 panic 后释放锁，panic 映射为不含载荷的固定 join 错误。闭包必须在锁内重新发现、授权复核并等待所有实际子进程退出，不能 spawn 后提前返回；实际受管进程清理与取消由任务23/26接入。

底座现有 TaskId/TaskStatus 和操作所有权合同没有已交付的领域操作存储/执行接口，文档明确由母任务3提供。因此本任务不另造操作身份、权限、RPC、事件、存储或取消状态机；闭包以 Send + 'static 所有权移入后台，调用方保留底座要求的认证/租户/主机/任务所有权。任务34须将其放入真实授权执行主机调用链。当前保证是单执行主机服务进程内跨租户共享，与来源一致；不是跨 OS 进程文件锁，不声明多个独立网关进程已共用安装锁。多进程部署必须路由到同一执行主机协调所有者后才可开放共同目录写入。

Cargo 阶段原子 mkdir 领取 slot-2，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；检查 .cargo/config.toml 后同步设置 CARGO_BUILD_BUILD_DIR 为同一绝对路径，覆盖 build.build-dir=target。owner 记录计划、任务、会话和阶段 shell PID。仅新增既有 tokio 依赖的正常 rt/sync 特性，不引入新库或修改 Cargo.lock。

验证证据：
- 首次编写夹具有 Rust 字符串转义编译错误，修正后再作红灯；该编译错误不计行为证据。
- 红灯：cargo test -p vw-agent-apps --lib cc_apps_lifecycle_lock_survives_disconnect -- --nocapture，退出101，1有效测试，0通过1失败，100过滤。最小可编译签名返回 lifecycle coordination unavailable，真实子进程未启动，启动标记等待超时；不是缺工具、底座或无关编译失败。
- cargo fmt --all，退出0。审查只保留本领域 Cargo 依赖、模块声明及新增文件，无其他 Rust 格式化差异。
- 绿灯：cargo test -p vw-agent-apps --lib cc_apps_lifecycle_lock_survives_disconnect -- --nocapture，退出0，1通过0失败，100过滤。真实临时目录和 /bin/sh 子进程通过文件门闩控制退出：断开等待者后重复提交被拒绝且无副作用，另一工具只在前一进程完成标记后写入；排队断开后未写入且可重试。另验证实际子进程退出17传播失败、panic 后锁可复用、无效批次拒绝及部分预占回滚。日志中的 fixture panic 是主动验收场景，不是用例失败。
- cargo clippy --all-targets --all-features -- -D warnings，退出0，无警告，首次72秒。审查时补充子进程门闩的有限轮询保护，避免断言失败时挂住测试；再次 cargo fmt --all 退出0、相同聚焦命令退出0（1通过0失败，100过滤，0.07秒），再运行相同全仓 Clippy 退出0（1.33秒）。
- git diff --check，退出0。未运行无过滤全量测试、fmt检查、wasm检查或Makefile。无UI变更。

本任务没有执行真实全局安装，没有访问个人凭据，也不声明 Windows/WSL/Linux 实机通过。构建和槽位产物进程均已结束，删除自有 slot-2 的 owner 后 rmdir 释放，保留缓存。提交排除任务1–21已有删除。下一步任务23在本协调器闭包内接入受管进程、计划复核和实际写入；任务26/34补齐操作所有权、显式取消及授权 API 的实际链路。
