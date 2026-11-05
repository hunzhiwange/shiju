# 任务21：原生卸载安全边界

2026-11-06，macOS。读取 do-task-plans、仓库 RustCodeGraph、git-commit 技能、只读计划和来源接口，核对任务20提交 017e67ed9、正式 AppInstallation 与前序领域模块。未修改 upstream、plan.md、来源快照或其他任务内容。

新增 apps/uninstall/native.rs 和独立 native_tests.rs，沿用来源 known_native_install 的 Claude、Codex standalone、Grok、OpenCode、Antigravity、OMP 官方目录与原生文件头规则。仅接受执行主机 System 来源和选定入口的 canonical target；未知、npm、脚本、自定义路径、目录、父目录符号链接明确拒绝。计划字段私有且不可反序列化，调用者只能读取文件集合，不能提交任意删除路径。执行前重建计划并比较集合及文件长度/修改时间，拒绝目标变更；不做递归删除。选定入口是链接时仅 unlink 链接，并独立删除已验证真身。

Grok 仅收集 .grok/bin 与 downloads 中官方命名且具有原生文件头的版本，agent/grok 链接只在指向同一安装根时加入；外部链接及目标、配置、相邻用户文件保留。提示函数接受主机已知更新器模式，保留 Grok npm 副本重装警告与 Hermes 数据残留提示；Hermes 实际 CLI 卸载仍由任务20负责。无需读取个人 HOME 或凭据。

Unix 删除从根目录逐组件 openat(O_DIRECTORY|O_NOFOLLOW)，持有父目录描述符后 unlinkat，拒绝通过被替换的父目录链接删除外部文件。外部进程仍可能在最终检查后替换同目录叶节点，unlinkat 不跟随叶节点链接；上层任务22/23/34必须在安装写锁内重新发现、授权和执行。本任务没有新增网关/RPC，没有承诺原子多文件事务。

Windows/WSL 官方命名与安装环境夹具在 macOS 实际运行；不是 Windows/WSL 实机执行证据。非 Unix 删除显式返回需要 host no-follow 实现的错误，避免使用有竞态的路径删除假冒安全支持。其他搜索目录的 launcher 需要后续授权主机发现输入，本任务删除选定入口、真身和 Grok 官方副本，不扫描全局 PATH。

共享 Cargo 槽位：原子 mkdir 领取 slot-2，CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；检查 .cargo/config.toml 的 build.build-dir 后同步设置 CARGO_BUILD_BUILD_DIR 为同一绝对路径。锁记录计划、任务、会话及构建阶段进程 PID，结束后只删除自有 owner 和锁目录，保留缓存。

验证：
- 红灯：cargo test -p vw-agent-apps --lib cc_apps_native_uninstall_scope -- --nocapture，退出101，1有效测试，0通过1失败，95过滤。可编译占位返回 native removal unavailable，真实删除断言因未实现而失败。
- 中间迭代发现 Wsl 是带 distribution 的枚举变体，修正测试构造；不将该编译错误计为行为红灯。
- cargo fmt --all，退出0；审查 tracked Rust 差异仅卸载模块声明，新文件为当前领域实现和测试。
- cargo test -p vw-agent-apps --lib cc_apps_native_uninstall_scope -- --nocapture，退出0，5通过0失败，95过滤。真实临时文件验证选定删除、数据保留、脚本/未知来源拒绝、链接替换后零副作用、父目录链接拒绝、Grok缓存/agent清理且外部文件保留、目录替换拒绝及 Windows/WSL规则。
- cargo clippy --all-targets --all-features -- -D warnings，退出0，44.76秒，无警告。
- git diff --check，退出0；未运行全量测试、fmt检查、wasm检查或Makefile。无UI变更。

状态：已完成。阶段进程全部结束，自有 slot-2 锁释放，缓存保留。提交仅含卸载模块声明、native.rs、native_tests.rs 与本证据，排除前序编号任务删除。下一步由任务22/23/34接入授权主机发现、安装写锁与执行；Windows原生安全删除实现需在承诺该平台执行前补齐。
