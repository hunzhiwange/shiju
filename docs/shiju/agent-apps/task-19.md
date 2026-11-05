# 任务19 升级锚定与预检

2026-11-06，macOS。已完成。

已读取 do-task-plans、仓库 AGENTS、RustCodeGraph skill、只读 plan.md、来源与接口、任务18证据及实际领域代码。依赖批次18提交 acca12fa8；前序 AppInstallation、来源/环境/实例身份、发现报告、多安装确认规则、npm/native recipes 已交付。母任务1快照在 upstream，仅通过 RustCodeGraph 读取，未修改。母任务2正式 crate 与 DTO 已存在；本批次无新增 RPC、权限或持锁执行器。

新增 apps/lifecycle/update.rs、独立 update_tests.rs，沿用 lib.rs -> apps -> lifecycle 的现有公开模块链，仅补本领域模块声明。

服务端领域计划按明确安装 ID 选择；固定 argv、绝对程序路径、解释器 PATH 前缀和进程级环境设置。npm/Volta/bun/pnpm 使用授权主机重新发现的同级工具；缺失拒绝，禁止 PATH 兜底。brew 根据真身 Cellar/Caskroom token 选择 formula/cask，Homebrew Node 全局 npm 包仍走同级 npm。Hermes/Antigravity 保留自身更新，Claude/Grok/OpenCode 原生渠道锚定入口；Grok 保留 npm12 postinstall 的进程级设置。Codex standalone 保留 CODEX_INSTALL_DIR 和 CODEX_HOME，重跑官方 installer，不调用会假成功的 codex update；MiniMax 默认脚本布局按真身安装根目录原地重装，Kimi旧版按原生根目录重装，新版 upgrade --yes；Nix Omp 与未知渠道拒绝 unmanaged。未提供可靠回执的自定义 MiniMax 布局保守拒绝，不猜安装位置。

UpdatePlan 不可由客户端反序列化。revalidate 用于22/23在写锁内传入新发现报告和同级工具观察后复核：保留所选ID、主机、环境、来源、入口、真身、版本、运行态及相同命令；缺失/重复ID、跨scope、变化的安装或缺失包管理器拒绝。仅 PATH 默认标志/报告顺序变化可接受，不切换另一实例。预览或重新发现任一阶段出现多安装都要求 confirmed=true。发现与授权必须由执行主机完成，companions不是客户端输入；Windows .cmd/.exe 由主机实际发现，执行器需正确适配batch而非拼接shell。实际写锁、授权执行、持久预览ID和取消事件接线仍归22/23/34，未宣称已完成这些能力。

Cargo阶段均用原子mkdir动态领取共享槽位，记录计划/任务/会话/PID，退出trap释放自己的锁并保留缓存。检查.cargo/config.toml后同时设置 CARGO_BUILD_BUILD_DIR=CARGO_TARGET_DIR，防止build-dir逃出槽位。

实际使用：
- slot-2：CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2（红灯、格式化、最终验证）。
- slot-3：CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3（首次8项绿灯；slot-2当时仍有格式化阶段锁，未绕过）。

验证证据：
- 红：cargo test -p vw-agent-apps --lib cc_apps_update_anchors_installation -- --nocapture，退出101；2项执行，1通过1失败，80过滤；可编译占位实现返回unmanaged，选定npm实例的真实结果断言失败，非工具/底座/无关编译失败。
- 初轮绿：相同聚焦命令退出0，8通过、0失败、80过滤。
- cargo fmt --all：退出0，只保留本任务Rust差异。
- 扩展绿：相同聚焦命令退出0，9通过、0失败、80过滤。
- cargo clippy --all-targets --all-features -- -D warnings：退出0，52.45秒，无警告。
- 最终补齐Grok进程级postinstall环境后重新运行 cargo fmt --all：退出0；cargo test -p vw-agent-apps --lib cc_apps_update_anchors_installation -- --nocapture：退出0，9通过、0失败、80过滤；cargo clippy --all-targets --all-features -- -D warnings：退出0，17.81秒，无警告。
- git diff --check、git diff --cached --check：退出0；所有Cargo与子进程结束后，自己的slot-2/slot-3锁均已释放，缓存保留。

测试覆盖选定npm、Homebrew formula/cask与npm区别、Volta/bun/pnpm、Windows/WSL显式环境夹具、原生自更新、Codex自定义安装/home、Kimi版本门槛、MiniMax原地升级、多安装确认、PATH变化、实例消失、来源/主机/真身变化、缺失同级工具、重复身份与相对路径拒绝。macOS真实临时目录中创建两个记录argv的npm子进程夹具，PATH指向另一实例，执行最终计划仅所选目录生成npm.argv，另一目录无副作用。无外网下载/全局安装/个人凭据副作用；Windows/WSL为规则证据，非平台实机验收。

差异审查：只纳入本领域模块、实现、测试及本证据，排除前序编号任务删除；plan.md保持只读。无UI改动，不运行全量cargo test、fmt检查、wasm检查或Makefile。后续推荐任务20；22/23接入锁内发现和复核。
