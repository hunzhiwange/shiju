# 任务25：WSL生命周期执行

2026-11-06，macOS。状态：已完成。

依赖与范围：核实任务7的WslTarget/Shell解析、13–17的固定配方、19的升级计划、22的宿主写锁、23的进程执行及24的卸载复核接口均已交付。来源快照与正式crate存在；母计划旧状态不作为阻塞。使用仓库RustCodeGraph技能status/explore/node核对来源build_wsl_tool_action_line、wsl_plan_and_process、wsl_uninstall_command。plan.md、来源快照和upstream未改；已有编号1–24任务文件删除不属于本次提交。

实现：新增后端WslPlan及install/upgrade/uninstall/execute，不接收客户端脚本。复用既有安装配方（process仅提取共同配方选择）与升级计划。WSL命令使用独立argv传递-d发行版、--、绑定Shell和flag，外层Shell内显式sh隔离POSIX脚本；bash安装保留pipefail。Auto必须先由授权宿主发现并解析为Explicit，未解析明确拒绝，不猜默认Shell。生产执行仅允许Windows宿主，不在其他系统或Windows本地Shell回退。

执行在全局ToolLifecycleCoordinator写锁内重新获取并比较完整计划，包括发行版、Shell/flag、实例ID、主机、来源、路径、真实路径和固定命令；未确认、变化及Node门控失败均拒绝。授权宿主回调负责认证作用域、目标发行版内的新鲜发现、Node检查及安装/升级后验；后续34接入真实授权API，本任务不新增RPC或自行授予主机权限。

卸载沿用来源明确支持的npm前缀与Hermes自身卸载；Homebrew或原生等非支持来源返回明确原安装器提示。npm严格匹配包路径和CLI入口，固定选中前缀的npm，不借宿主PATH删除别处安装；OpenClaw先卸载服务。跨系统路径用Linux规则验证，拒绝Windows路径/UNC/反斜杠/控制字符/点段。真实目标readlink复核、卸载及入口/悬空链接后验均在发行版内执行，不调用宿主canonicalize检查Linux安装。

## Cargo槽位

原子mkdir依次领取，实际slot-2。CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖.cargo/config.toml中build.build-dir=target。锁记录计划、任务、会话及持有进程；所有Cargo和夹具进程结束后清理本任务owner和slot-2.lock，保留缓存。没有删除其他槽位锁。

## 验证

以下Cargo命令均继承上述两个绝对路径环境变量。

- 红灯：`cargo test -p vw-agent-apps --lib cc_apps_wsl_lifecycle_target -- --nocapture`，退出101，1个有效测试，0通过1失败；最小uninstall签名返回WSL lifecycle unavailable，成功用例明确失败。初次因python别名不存在未写入测试而出现的0测试运行不计验收证据，改用python3后取得上述红灯。
- `cargo fmt --all`，退出0；审查仅保留本任务相关差异。
- 绿灯：`cargo test -p vw-agent-apps --lib cc_apps_wsl_lifecycle_target -- --nocapture`，退出0，5通过0失败，109过滤，0.05秒。覆盖13个CLI的WSL安装配方与fish参数、升级固定管理器、跨发行版/Windows路径/未解析Shell拒绝，真实临时文件和隔离子进程记录npm安装/升级argv并删除选中入口，保留其他文件；覆盖中文与引号路径、预览发行版与Shell变化、未确认、Node缺口、后验失败、exit17、零退出但入口残留和悬空链接、非Windows执行拒绝且不调用宿主回调。
- 新增两个异步用例首次并行时正确触发生产TOOL_ACTION_IN_PROGRESS，3通过1失败；修复测试夹具串行化，未削弱生产互斥。期间目录字段拼写编译错误已修复，不当作预期红灯。
- 共享配方/执行器回归：`cargo test -p vw-agent-apps --lib cc_apps_lifecycle_process_result -- --nocapture`，退出0，5通过0失败，109过滤。
- `cargo clippy --all-targets --all-features -- -D warnings`，最终退出0，无警告。
- `git diff --check`及暂存差异检查，退出0。

无UI变更，不操作个人安装或凭据，不修改HOME。WSL边界由可执行进程夹具替代，在macOS实际运行，不代表Windows/WSL实机验收。下一步26继续取消收尾，34将本领域接入授权宿主调用。
