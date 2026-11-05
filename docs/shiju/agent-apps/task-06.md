# 任务06 Windows安装路径发现

状态：已完成。2026-11-06，macOS规则验证；不是Windows实机执行证据。

已读执行技能、plan.md、来源与接口、任务05证据；正式crate、共享实例及发现模块存在。通过RustCodeGraph核对effective_path_string、windows_preferred_tool_executable及调用者，来源快照保持只读。

新增独立Windows规则与测试，既有apps/discovery导出：process/user/machine/package目录按传入顺序合并，Windows绝对路径与Unicode保留、大小写去重、空段和相对路径拒绝；包管理器npm/pnpm/Volta/fnm/scoop/Bun/Cargo目录；PATH目录优先、PATHEXT扩展名优先且仅允许COM/EXE/BAT/CMD，不搜索cwd，跳过Microsoft/WindowsApps执行别名目录；OpenCode选择CLI sidecar并排除无sidecar的Electron桌面入口。读取真实文件，不执行候选程序。主机环境/注册表读取由调用者提供快照，本模块未新增系统读取或权限边界。

验证阶段原子领取槽位2，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；同时设置CARGO_BUILD_BUILD_DIR为相同绝对路径，覆盖仓库build.build-dir配置。

- 红：`cargo test -p vw-agent-apps --lib cc_apps_windows_discovery_paths -- --nocapture`，退出101，2项实际失败，空实现违反合并与入口结果断言。
- 绿：`cargo fmt --all`退出0；相同聚焦测试退出0，2/2通过。
- 追加包管理器场景后：`cargo fmt --all`退出0；相同聚焦命令退出0，3/3通过、35项过滤。
- `cargo clippy --all-targets --all-features -- -D warnings`两次退出0。
- `git diff --check`退出0。审查新模块与测试，无无关格式化；排除前序编号任务删除。

退出后释放自己的槽位锁、保留缓存。下一步任务07；Windows系统读取及实机执行不作为macOS规则测试证据。
