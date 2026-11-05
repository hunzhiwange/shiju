# 任务20 包管理器卸载预览

2026-11-06，macOS。

读取 do-task-plans、RustCodeGraph 与 git-commit 技能、仓库协议、只读 plan.md、来源与接口及 task-19 证据。通过 RustCodeGraph 核对上游 tool_uninstall 的调用链和所有权规则；批次19提交 5e62d898a 与正式 AppInstallation、来源、环境、npm 配方均已交付，不依赖母计划中的旧状态标签。未修改 upstream、来源快照或 plan.md。

新增 apps/uninstall/packages.rs、独立 packages_tests.rs 与公开模块声明。预览只接收授权执行主机的安装发现、管理器绝对路径与 brew info 所有权元数据；不反序列化客户端命令、不新增 RPC。npm 读取真实 package.json、匹配固定包名与 canonical target，要求全局布局，固定 --prefix。Bun 仅接受其 install/global 布局并设置进程级 BUN_INSTALL_GLOBAL_DIR/BUN_INSTALL_BIN；pnpm 验证 global/数字目录并固定 --global-dir/--global-bin-dir；无法验证的自定义布局人工处理。管理器来自主机已发现的同目录工具，不回退 PATH。Windows npm 保留全局 shim 内容与清单联合判定，拒绝 .bin 本地依赖。

Homebrew 在 npm 前判定，核对已审查应用的 formula/cask token、prefix、kind 与官方 tap，使用完整 homebrew/core 或 homebrew/cask 名称；其他 tap 同名包、缺元数据、错误 kind 均拒绝。元数据必须由后续执行主机从 brew info 获取，不能由客户端提交或只凭 Cellar 路径猜测。Hermes 调用所选 CLI uninstall --yes，不调用 pip、不加 --full；未知来源返回明确人工处理原因。实际授权、预览持久化、写锁内重新发现比较和执行接线属于22/23/34，本任务只交付领域预览，不宣称已完成卸载执行。

测试覆盖 npm 成功、未知/不相关真身、缺失同级工具、Bun/pnpm 固定根与错误包名、brew formula/cask 与其他 tap/类型拒绝、Hermes 保留数据参数、相对/遍历路径拒绝及显式 Windows/WSL 规则。macOS 临时目录创建两个记录 argv 的 npm 子进程夹具，PATH 指向另一目录，最终仅选定前缀生成 argv 文件，另一实例无副作用，包目录保留。未调用真实卸载器、外网或个人配置；Windows/WSL 是规则验证，并非平台实机验收。

Cargo 阶段使用原子 mkdir 逐一领取共享槽位，owner 记录计划/任务/会话/PID，退出 trap 删除自身记录并 rmdir，保留缓存。检查 .cargo/config.toml 的 build.build-dir 后同时设置 CARGO_BUILD_BUILD_DIR=CARGO_TARGET_DIR。
实际槽位：
- slot-2：CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2（红灯、格式化、聚焦验证、Clippy）。
- slot-3：CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3（上一轮 Clippy 等待其他子任务退出期间的格式化；未占用同一槽位）。

验证记录：
- 红灯：cargo test -p vw-agent-apps --lib cc_apps_package_uninstall_preview -- --nocapture，退出101；1项有效测试，0通过1失败，89过滤。可编译占位返回人工处理，验证 npm 预览结果失败；非缺工具或无关编译失败。
- 首次绿灯：同命令退出0；1通过，89过滤。
- 扩展绿灯：同命令退出0；6通过，89过滤。
- 中间迭代修复 Windows shim 字符转义编译错误；不将其计为行为红灯。
- 首轮 cargo clippy --all-targets --all-features -- -D warnings 退出101，发现本任务测试 cloned_ref_to_slice_refs；已改为 std::slice::from_ref，无关闭 lint。
- 最终 cargo fmt --all：退出0；审查 tracked Rust 差异仅 apps/mod.rs，新增文件仅本领域实现/独立测试/模块。
- 最终 cargo test -p vw-agent-apps --lib cc_apps_package_uninstall_preview -- --nocapture：退出0；6通过、0失败、89过滤。
- 最终 cargo clippy --all-targets --all-features -- -D warnings：退出0，17.55秒，无警告。
- git diff --check：退出0。未运行无过滤全量测试、fmt检查、wasm检查或Makefile。

状态：已完成。全部 Cargo 与产物子进程已退出，自有 slot-2/slot-3 锁已释放，缓存保留。提交只含 apps/mod.rs、apps/uninstall/ 与本证据，排除前序任务文件删除；本编号文件完成后删除，不修改只读 plan.md。无UI改动。下一步执行任务21，后续22/23/34接入授权发现、锁内复核与执行。
