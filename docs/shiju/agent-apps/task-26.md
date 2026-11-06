# 任务26：操作进度与取消

2026-11-06，macOS。状态：已完成。

依赖：核对来源与接口.md、任务25证据、正式vw-agent-apps、TaskStatus、AppActionExecutionResult，以及22的宿主写锁、23/24的进程执行和25的WSL执行。使用仓库RustCodeGraph skill status/explore检查索引及调用链；新增文件用rg与文件读取核对。母计划旧状态不代表符号缺失。plan.md、来源快照、upstream未修改。

实现：Operation复用底座TaskStatus，提供Queued/Preflight/Executing/Postflight/Terminal进度、watch订阅、快照、等待及cancel_and_wait。授权调用者负责将句柄绑定已有TaskId和权限作用域；这里不新增RPC、授权、第二个持久化任务调度器。start_action包装既有install/upgrade/uninstall未来体，coordinator将任务局部上下文带入不可中止阻塞工作线程，所有现有run_command调用共享取消逻辑。start提供直接写锁内操作入口。重复提交同一句柄被拒绝，订阅断开不重新提交，也不停止执行。

POSIX执行器为子进程创建独立进程组；取消时SIGKILL进程组，轮询等待真实子进程退出及stdout/stderr排空后返回。父进程先退出而后代仍持有输出管道时，仍可响应取消。取消期间继续持有宿主写锁，coordinator结束后才发布终态；取消不会误报安装成功。进程输出继续只保留有界字节计数，不保存原始凭据或命令输出。预检取消不会启动进程。闭包panic经coordinator转换为失败终态。进度订阅重连可以读取当前终态。

平台边界：本轮macOS实际执行POSIX进程组夹具；没有Windows/WSL/Linux实机证据。带Operation的非Unix进程执行在spawn前明确拒绝，不能以只杀根进程冒充进程树取消；原无Operation的Windows流程保持原行为。POSIX进程组覆盖继承组的安装器后代，不承诺跟踪自行脱离会话/进程组的守护进程。任务34负责将句柄接入已有授权任务API/事件存储；Operation为宿主存活期间的状态，未宣称跨宿主重启恢复。

## Cargo槽位

原子mkdir按1–10顺序领取实际slot-2；CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2。后者覆盖.cargo/config.toml中build.build-dir=target，确保中间产物也在所领槽位。锁owner记录计划、任务、会话及持有进程。全部验证和夹具结束后释放本锁，保留缓存，不清理其他锁。

## 验证

以下Cargo命令均继承上述两个绝对路径变量。

- 红灯：`cargo test -p vw-agent-apps --lib cc_apps_operation_cancel_reaps_process -- --nocapture`，退出101，1个有效测试，0通过1失败；真实进程正常完成，忽略取消，断言Completed与Cancelled不匹配。此前最小签名的Result::as_deref编译错误已修正，不作为红灯证据。
- `cargo fmt --all`，退出0；审查只有本任务Rust文件差异，无无关格式化。
- 绿灯：`cargo test -p vw-agent-apps --lib cc_apps_operation_cancel_reaps_process -- --nocapture`，退出0，3通过0失败、114过滤。真实临时目录与/bin/sh进程覆盖取消、根进程已回收、后代延迟写入未发生、写锁拒绝重复操作和终态后可重新获取；覆盖成功、exit17失败、输出脱敏、预检取消、重复提交拒绝、panic失败、重读终态，以及现有install入口的上下文传播。panic输出为受控失败夹具。
- 执行器回归：`cargo test -p vw-agent-apps --lib cc_apps_lifecycle_process_result -- --nocapture`，退出0，5通过0失败、112过滤。
- `cargo clippy --all-targets --all-features -- -D warnings`，最终退出0，无警告，58.27秒。首次发现本次新增collapsible_if警告，已修正并重跑。
- `git diff --check`，退出0；前序编号1–25任务删除为已有差异，排除本次提交。

无UI修改，不操作个人安装，不修改HOME。下一步任务27；任务34接入授权操作查询与取消。
