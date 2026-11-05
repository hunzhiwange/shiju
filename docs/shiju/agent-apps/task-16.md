# 任务16 Hermes与Antigravity安装

2026-11-06，macOS；状态：已完成。

已核对只读plan.md、来源与接口、前序任务15证据、正式crate及AppId、原生InstallCommand/InstallEnvironment。来源为只读upstream misc.rs的Hermes/agy官方安装与更新分支；母任务底座与批次1–15实际产物已存在。沿用lib.rs已有pub mod apps及lifecycle声明，不新增RPC或修改upstream、plan.md。

新增native_hermes_agy公开安装/更新配方与独立测试。Hermes使用官方NousResearch安装器，不走npm/PyPI或系统Python门控；更新先hermes update，失败回退官方安装器。agy仅新装使用官方Antigravity安装器，更新直接agy update，失败不重装、不伪成功。POSIX和WSL下载成功才执行脚本，保留失败码、清理临时文件并支持含空格TMPDIR。Windows显式返回PowerShell结构化argv，子PowerShell隔离安装器exit并传播退出码；Hermes失败走专属安装器。配方不执行、不扩大权限，授权主机程序解析和特定实例来源锚定由后续执行器负责。

动态领取slot-2；CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2。仓库.cargo/config.toml有build.build-dir=target，故同时设置CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；以下所有Cargo命令均继承二者，无缓存越槽。

验证证据：
- 红：cargo test -p vw-agent-apps --lib cc_apps_hermes_agy_install -- --nocapture；退出101，2有效测试失败、75过滤，因可编译最小签名返回recipe missing。
- cargo fmt --all；退出0；审查无无关Rust格式化差异。
- 实现后首次运行1通过1失败，系夹具把npm程序文件误作调用记录；修正独立npm-called记录后重新格式化与验证。
- 绿：cargo test -p vw-agent-apps --lib cc_apps_hermes_agy_install -- --nocapture；退出0，2通过、0失败、75过滤。子进程测试实际执行32组（两应用×POSIX/WSL×安装/更新×四种下载/安装/更新结果），验证真实退出码、安装/更新记录、准确URL、无npm调用、临时文件清理；Windows结构化规则及非目标应用拒绝在macOS运行。
- cargo clippy --all-targets --all-features -- -D warnings；退出0，17.03秒，无警告；不计行为测试数量。
- git diff --check与暂存差异检查；退出0，提交仅本任务源码、测试、模块声明与此证据。

真实bash及临时文件/进程，只替代curl外网与全局CLI更新边界。没有修改HOME、个人安装或凭据。Windows/WSL为环境规则夹具，不代表实机安装证据；macOS真实安装验收归任务53/58。本任务无UI改动。未运行全量测试、fmt检查、wasm检查或Makefile。验证结束释放自己的slot-2锁，保留缓存。下一步执行任务17。
