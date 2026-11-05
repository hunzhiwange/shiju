# 任务08 Claude桌面安装探测

2026-11-06，macOS；状态：已完成。

依赖核对：任务01–07的正式crate、应用目录、安装实例、POSIX/Windows/WSL发现模块及持久证据已存在，任务07提交bd2827892。通过RustCodeGraph核对来源get_status、detect_installed、macos_app_bundle_candidates、各平台配置路径及Windows目录选择。现有来源将配置目录或已配置状态推断成安装；本领域将两类证据分离，不读取供应商、数据库、profile内容或凭据，不写配置、不启动CLI。

公开入口：`vw_agent_apps::apps::discovery::claude_desktop::probe`。参数为授权执行主机提供的平台与绝对根目录（与前序discovery快照接口一致），不是客户端路径授权；后续管理API任务负责接线。无需新增lib.rs声明，既有apps/discovery公开模块已可达。

- macOS观察系统Applications/Claude.app及home/Applications/Claude.app目录，`Some([])`才表示既定搜索范围内未发现；CLI或Claude/Claude-3p遗留配置目录不影响应用包结果。
- Windows以LocalAppData为根，保留精确配置目录优先、版本目录排序及普通/3p区分；Linux以XDG配置根为输入。两者延续来源的配置路径观察支持，但来源未提供可靠应用包定位规则，`app_bundles=None`明确表示未提供应用包探测，不得转换为未安装，也不据此宣称这些平台不支持Claude Desktop。
- Unsupported平台返回supported=false和未知应用包状态，不能误报未安装。相对根拒绝，非NotFound文件系统错误向上传递；普通文件不算应用包。
- 全部夹具使用真实临时目录，不改HOME；Windows/Linux规则在macOS运行，不是其他平台实机证据。个人安装与配置未修改；真实macOS安装验收归任务47；本任务无UI改动，无UI验证要求。

Cargo阶段原子领取共享槽位2：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，覆盖仓库`.cargo/config.toml`的build.build-dir，保证中间产物也留在槽位。

验证证据：

1. 红：`cargo test -p vw-agent-apps --lib cc_apps_claude_desktop_presence -- --nocapture`，退出101；3个有效测试，0通过/3失败，42过滤。可编译空实现返回不支持/未知，分别违反CLI独立性、受支持配置观察及错误拒绝断言；不是依赖或工具错误。
2. `cargo fmt --all`，退出0；无无关格式化差异。
3. 绿：`cargo test -p vw-agent-apps --lib cc_apps_claude_desktop_presence -- --nocapture`，退出0；4通过/0失败，42过滤。另增加Windows版本目录确定性/精确目录优先用例。覆盖只读副作用边界、配置与安装分离、平台边界、相对路径与文件系统明确错误。
4. `cargo clippy --all-targets --all-features -- -D warnings`，退出0，58.61秒，无警告（静态检查，无行为测试计数）。
5. `git diff --check`，退出0；审查新增实现/独立测试、模块声明与本文，排除前序编号任务删除；plan.md、upstream未修改。

验证所有进程结束后释放自己的slot-2锁，保留缓存。后续建议执行任务09；安装实例API接线与macOS真实安装验收按后续任务推进。
