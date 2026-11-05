# 任务05：POSIX安装路径发现

状态：已完成。2026-11-06，macOS。

## 依赖与范围

已读执行技能、只读 plan.md、来源与接口.md，以及任务01–04证据。正式 vw-agent-apps、AppId/应用目录、ACP投影、安装实例 DTO 已交付。以 RustCodeGraph status/explore/node 核对 upstream/misc.rs 的 build_tool_search_paths、login_shell_path、merge_path_segments 与应用额外路径；图未覆盖的新模块直接读取。未修改 upstream、认证、主机 RPC、安装执行或 UI。

新增 apps/discovery/posix 模块，由既有 apps 模块导出。PosixEnvironment 是授权执行主机的本地环境快照，不能接受未经授权客户端传入的路径作为执行权限。候选目录涵盖 brew、npm/pnpm、volta/n、mise/fnm/nvm、登录与继承 PATH、Kimi/Grok/MCode/OpenCode/OMP 和 Hermes Python 额外目录；仅保留现存绝对目录，稳定去重，目录枚举排序，忽略空 PATH 与相对路径，不执行安装。路径不 canonicalize，保留后续实例来源判定所需的符号链接入口。读取失败或不存在的候选目录跳过。

Shell 查询使用调用者提供的可信绝对 Shell 路径，按 sh/dash、fish、其他 Shell 选择与来源一致的标志；固定执行 /usr/bin/printenv PATH，避免捕获完整环境与凭据。stdin/stderr 关闭，临时匿名文件承接 stdout，有限等待并清理进程组；非零退出、缺少绝对 PATH、超时、无效参数明确返回错误。沿用来源对 Shell 欢迎行的容忍。libc/tempfile 均为仓库已锁定的轻量依赖，未下载新包。

无用户 HOME/凭据修改，无安装或全量测试。Linux目录规则在macOS执行，不宣称Linux实机通过；Windows/WSL属于后续任务。

## TDD与验证

红、绿与最终阶段均从仓库根目录原子 mkdir 动态领取槽位2，owner记录计划/任务/会话/PID；EXIT trap删除自己的owner并rmdir释放锁，缓存保留。

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖 .cargo/config.toml 的 build.build-dir=target，所有 Cargo 子进程继承上述设置。

红：`cargo test -p vw-agent-apps --lib cc_apps_posix_discovery_paths -- --nocapture`，退出101，实际运行3项、3项预期失败。仅先建立可编译签名：空候选、空合并结果、Shell未实现错误，分别被真实结果断言捕获；不是无关编译/工具错误。

绿：`cargo fmt --all`退出0；同一聚焦命令退出0，3/3通过，31项过滤；`cargo clippy --all-targets --all-features -- -D warnings`退出0，耗时1m14s。

最终追加 Bun/Go 与真实 /bin/sh 验收后重新运行：`cargo fmt --all`退出0；`cargo test -p vw-agent-apps --lib cc_apps_posix_discovery_paths -- --nocapture`退出0，4/4通过，31项过滤；`cargo clippy --all-targets --all-features -- -D warnings`退出0。

覆盖真实临时目录下多版本管理器与登录PATH优先级、应用覆盖路径隔离、Bun/Go目录、非目录/缺失/相对路径拒绝、PATH空段与重复项、真实Shell成功、进程夹具参数/欢迎行/非零退出/超时及相对Shell路径拒绝。纯规则跨平台；进程测试在macOS执行。

`git diff --check`退出0。审查全部任务改动，未产生无关格式化；仅提交模块、独立测试、依赖及本证据，排除前序编号文件的既有删除。所有验证子进程结束后释放槽位。推荐继续任务06。
