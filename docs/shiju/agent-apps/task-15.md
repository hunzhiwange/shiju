# 任务15 Grok与MiniMax原生安装

2026-11-06，macOS。状态：已完成。

核对只读总计划、来源与接口、任务14完成证据、正式AppId、npm固定参数及来源misc.rs的Windows安装链、POSIX安装链、mcode_installer_update_command和非TTY更新约束。依赖1–14实现可用；沿用任务14的InstallEnvironment/InstallCommand，不新增RPC、权限或桌面逻辑，不修改upstream或plan.md。

交付native_grok_mcode安装配方：AppId::GrokBuild映射官方x.ai CLI，Mcode映射MiniMax官方安装器；POSIX/WSL下载成功才执行，失败才进入固定npm回退，保留最终失败码并清理临时文件。原生尝试不要求系统Node，保留MiniMax安装器自带Node行为。Windows返回结构化PowerShell argv，官方脚本在子PowerShell执行，避免其exit跳过父进程的npm.cmd回退；下载错误和非零退出均触发回退。执行器必须解析授权主机上的程序路径，不得拼接客户端文本。

MiniMax脚本安装升级通过MCODE_INSTALL_DIR锚定既有目录并重跑安装器，拒绝空、换行、NUL路径；单引号转义，不回退全局npm，不使用非TTY下伪成功的mcode update。测试验证带空格/单引号目录以及失败码。升级入口只生成配方，来源/绝对路径授权复核仍属于后续执行器。

Cargo动态领取slot-2，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；同时设置CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖仓库build.build-dir。所有Cargo命令均继承这两个变量。

验证命令与证据：
- 红：cargo test -p vw-agent-apps --lib cc_apps_grok_mcode_install -- --nocapture，退出101；2有效测试失败，71过滤；可编译空配方返回recipe missing，符合预期缺失行为。
- cargo fmt --all，退出0；审查仅本任务Rust变更，无无关格式化。
- 绿：cargo test -p vw-agent-apps --lib cc_apps_grok_mcode_install -- --nocapture，退出0；4通过0失败，71过滤。安装子进程用例实际执行16组（两应用×POSIX/WSL×四种下载/安装/npm结果），升级子进程4组。
- cargo clippy --all-targets --all-features -- -D warnings，退出0，无警告；检查不计行为测试数量。
- git diff --check及暂存差异审查，退出0。

真实bash、临时文件、cwd/env与子进程；仅替代curl/npm外网及全局安装边界。覆盖成功不回退、下载残缺脚本不执行、安装失败回退、npm失败传播、清理、指定目录与非TTY拒用update。Windows为显式规则夹具，未声称Windows/WSL实机安装；未改HOME/凭据或个人安装。真实macOS应用验收留给任务50/55。本次无UI变更。未运行全量测试、fmt检查、wasm或Makefile。

验证结束后释放自己的slot-2锁，保留缓存。下一步执行任务16。
