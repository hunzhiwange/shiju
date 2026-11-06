# 任务24：卸载确认执行与后验

2026-11-06，macOS。状态：已完成。

依赖：20/21的packages/native预览、22的ToolLifecycleCoordinator、23的进程等待/脱敏输出均已交付，前序证据与真实接口已核实。plan.md、来源与接口及upstream未修改。RustCodeGraph status确认索引，explore导航后读取新增领域文件核实。

新增后端UninstallPlan，字段私有，无客户端命令反序列化入口。授权主机回调在共享写锁内重新发现选中实例和包管理器/所有权，重建预览并比较全部安装元数据及动作（来源、路径、真实路径、环境、主机、ID、原生文件指纹等），未确认、过期或变更均在启动/删除前拒绝。只执行后端重建动作；原生复用no-follow删除，包管理器复用23的等待、退出码及脱敏输出，抽出run_command但保留安装后验不变。symlink_metadata只接受NotFound，exit0但入口仍在、悬空链接和无法验证都失败。环境与本机不符明确拒绝，WSL必须交给后续真实环境执行边界，未伪造主机RPC。授权API接线仍归34。

Cargo原子mkdir从1开始领取slot-2；CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖.cargo/config.toml的build.build-dir=target，避免中间产物逃离槽位。锁记录计划、任务、会话和持有进程；验证进程结束后清理自身锁，保留缓存。

验证命令和证据：
- 红灯：cargo test -p vw-agent-apps --lib cc_apps_uninstall_revalidates_plan -- --nocapture，退出101，2个有效测试，1通过1失败；最小签名返回uninstall execution unavailable导致真实成功场景失败。不是依赖或工具错误。
- cargo fmt --all，退出0；审查仅保留本任务代码格式化，无无关源文件差异。
- 绿灯：cargo test -p vw-agent-apps --lib cc_apps_uninstall_revalidates_plan -- --nocapture，退出0，3通过0失败，106过滤。真实临时文件和npm子进程验证精确argv及选定前缀、成功删除入口、保留另一文件、exit17、exit0入口残留、删除目标而保留悬空链接；来源/真实路径/环境/主机/实例ID/入口变更和未确认均在删除前拒绝；原生指纹变更拒绝及重新预览后的成功删除。
- 共享执行器回归：cargo test -p vw-agent-apps --lib cc_apps_lifecycle_process_result -- --nocapture，退出0，5通过0失败，104过滤。
- 全仓Clippy：cargo clippy --all-targets --all-features -- -D warnings，退出0，无警告，40.07秒。
- git diff --check：退出0。

无UI变更；未操作个人安装或凭据，未修改HOME。测试中的npm为隔离临时进程边界；不能替代46–59真实应用验收或Windows/WSL/Linux实机证据。后续25继续卸载进度，34接线授权主机调用。
