# 任务17 Kimi与Oh My Pi安装

2026-11-06，macOS；已完成。

已读只读plan.md、来源与接口、任务16证据及正式crate。前序依赖实际存在；lib.rs已有pub mod apps，沿用lifecycle最小模块注册。RustCodeGraph核对upstream misc.rs的KIMI_INSTALL_UNIX/WINDOWS、OMP_INSTALL_UNIX/WINDOWS、kimi_installer_update_command、official_update_args及Nix门控；未修改upstream、plan.md或RPC。

新增native_kimi_omp与独立测试：Kimi使用官方安装器，现代更新upgrade --yes，旧版原生安装可在检测根目录以KIMI_INSTALL_DIR重装；单引号安全转义。omp通过官方omp.sh安装器保留bun运行时及自身包语义，更新omp update，不走npm或Pi包。Nix真实目标明确拒绝，空更新目标/空根目录/其他应用明确拒绝。配方不执行；实际授权、真实路径解析和特定实例锚定仍由后续执行器负责。Windows使用隔离子PowerShell及退出码传播；POSIX/WSL仅下载成功才运行脚本并清理临时文件。

动态领取slot-2；CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2。由于.cargo/config.toml的build.build-dir=target，同时设置CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；所有Cargo均继承二者。

验证证据：
- 红：cargo test -p vw-agent-apps --lib cc_apps_kimi_omp_install -- --nocapture；退出101，2失败、77过滤；可编译签名返回recipe missing，属预期行为缺失。
- cargo fmt --all；退出0。审查无无关Rust格式化差异。
- 绿：cargo test -p vw-agent-apps --lib cc_apps_kimi_omp_install -- --nocapture；退出0，3通过、0失败、77过滤。真实bash/sh与临时文件验证18组安装过程（含旧版Kimi根目录）及8组更新过程；覆盖下载22、安装8、更新9退出码，成功副作用、实际argv和带空格TMPDIR清理。安装脚本边界与CLI更新以本地可控进程替代，未执行全局安装或修改HOME。
- cargo clippy --all-targets --all-features -- -D warnings；退出0，17.47秒，无警告。
- git diff --check及暂存差异检查退出0；仅提交本任务模块、测试、声明和本记录，排除既有编号任务删除。

Windows/WSL为在macOS运行的规则夹具，不代表实机安装。macOS真实安装归任务56/59，无UI变更。未运行无过滤全量测试、fmt检查、wasm检查或Makefile。验证结束释放自己的slot-2锁并保留缓存。下一步任务18。
