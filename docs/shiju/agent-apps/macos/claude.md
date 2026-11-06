# Claude macOS 实机验收

2026-11-06，macOS 27.0 (26A428)，Darwin arm64，Node v26.13.0、npm 11.20.0。仅涉及 Claude Code CLI；不代表 Claude Desktop 或其他平台。

本次隔离 HOME、配置和 npm prefix 位于专用临时根，没有复制个人凭据。原有 `/opt/homebrew/bin/claude` 为 npm 包入口，版本 2.1.316；原桌面/网关进程不属于本次资源，不终止或卸载。

## 官方包渠道实际变化

- `HOME=<isolated-home> npm view @anthropic-ai/claude-code version dist.tarball --json`：退出0，2.1.318，tarball 为官方 registry.npmjs.org 的该包版本路径。
- `HOME=<isolated-home> npm install --global --prefix <isolated-prefix> @anthropic-ai/claude-code@2.1.316 --registry=https://registry.npmjs.org`：退出0，新增2包；隔离入口 `claude --version` 退出0，2.1.316。
- 同命令改为 `@2.1.318`：退出0，变更2包；隔离入口版本退出0，2.1.318。新版执行文件SHA256为 `677a2ea7bf576cedd20e191d973914ed8a41796f53a63b1923381ddc4d5b5e7d`。
- npm 11.20.0 提示上述手动版本准备未显式允许 postinstall；入口仍真实可运行。不能据此证明生产固定安装动作成功；生产配方包含 `--allow-scripts=@anthropic-ai/claude-code`。

这些是官方渠道的真实安装/版本变化证据，尚不能单独证明 SDK/Iced 的生命周期闭环。

## 发现并修复的正式网关装配缺陷

使用当前源码构建的 `shiju gateway --host 127.0.0.1 --port 19446`，对 catalog、installations、launch-identities 携带合法租户头仍全部404。旧 `gateway::router::handler_router` 已合并应用管理；真正命令使用的 `bootstrap::stateful_handler_router` 缺少该合并。修复限于正式装配，独立测试检查端点注册与网关认证、租户、应用授权拒绝行为。具体红绿灯及最终命令见下方执行结果。

## 未验证范围

尚无专用真实供应商/官方测试身份，不认领个人配置或凭据。没有把缺账号称为不支持。CUA 无法定位 Cargo 未打包应用；唯一标识的临时启动包装器也返回 timeoutReached，未获得可归属于本次窗口的截图。原有同名个人窗口保持不动。SDK与Iced一致性、真实身份启动与清理、取消/断线/错误toast仍须回归，不能据编译成功或人工版本命令判定整项通过。

## Cargo 与验证记录

动态领取 slot-2，整个阶段持有者为本任务控制进程。

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖仓库 build.build-dir=target，未通过 --target-dir 绕过槽位。所有使用产物的进程结束后再释放锁，保留缓存。

- `cargo run -p vw-desktop --bin shi-ju` 默认features：构建22.24秒，进入Running；隔离HOME/配置，仅计启动，不计业务通过。
- `cargo build -p vw-cli --bin shiju`：退出0，1m25s。最初既有槽位二进制和本次重建二进制都实际返回上述404。
- `cargo fmt --all`：退出0，审查保留本任务相关变更。
- `cargo test -p vw-agent --lib cc_apps_production_gateway_routes -- --nocapture` 首轮：退出101，1有效失败，11469过滤；正式catalog返回404。
- 最小装配后首轮：退出101，1有效失败，11469过滤；catalog已返回401。纠正新增测试对缺少应用令牌的预期（access.rs明确UNAUTHORIZED，而非最初写的FORBIDDEN），不改变任何生产授权逻辑。

既有链接器 __eh_frame 大小提示单列，不修改链接设置；未运行无过滤全量cargo test、fmt检查、wasm或Makefile。

最终聚焦命令 `cargo test -p vw-agent --lib cc_apps_production_gateway_routes -- --nocapture`：退出0，1通过、0失败、11469过滤，0.03秒。`cargo clippy --all-targets --all-features -- -D warnings`：退出0，20.15秒。测试最终覆盖catalog、installations、launch-identities均须应用令牌；缺租户400、网关未认证401。

原生窗口追加尝试：在临时目录建立唯一bundle标识并复制本次构建产物，独立HOME启动后，CUA仍报告procNotFound，运行日志包含sandbox_extension_issue_file_to_process Operation not permitted。没有使用个人窗口截图替代。

## 最终正式 API 实机场景

执行时间为2026-11-06约12:45–13:03（Asia/Shanghai）。最终网关构建命令 `cargo build -p vw-cli --bin shiju` 退出0，38.35秒。实际启动命令 `shiju gateway --host 127.0.0.1 --port 19446`，HOME、配置目录及SHIJU_AGENT_APPS_ACCESS_FILE均指向隔离根，仅loopback监听；未启用外部网关账号认证，应用专用授权仍强制执行。脱敏请求/响应见claude-api.json，移除重复轮询与其他应用目录项，不含认证头或令牌。此记录由HTTP调用，尚未完成SDK与Iced逐项一致性验收。

- catalog/installations/launch-identities均200；身份列表为空，没有默认选用个人账号。
- 旧版2.1.316准备后，update预览200、source=system、提交202；操作8721c18d最终completed，实际入口版本2.1.318。管理员prefix和固定包manifest复核锚定来源，未编造旧版URL。首次隔离prefix缓存下载约40秒，等待不计失败。
- 卸载58e1a1e2最终completed，发现为空；重新安装2bf39362最终completed，发现2.1.318/runnable；再卸载ddf74444完成且列表为空。
- 安装8311f427提交后立即DELETE，返回200/cancelled；独立重读及events均200/cancelled，发现为空。此项证明提交后立即取消，不声称覆盖运行中下载进程取消的实机时序。
- 在隔离prefix创建指向不存在目标的claude入口，diagnostics发现source=unknown、runtimeStatus=probe_failed/damaged_installation；对实际安装ID预览卸载422，启动422。移除本次损坏入口后发现为空，没有破坏个人实例。
- 个人实例最终仍2.1.316，执行文件SHA256为64c327e92b2c5e0f78c0a980c5a518121e1f0d140ca771ca5dd4488d1a2dcb3f，与操作前一致。
- 通用只读发现将自定义npm prefix报告unknown，而生命周期预览在管理员prefix/manifest复核后报告system；记录真实差异，不将其改写为来源一致。

最终 `cargo test -p vw-agent --lib cc_apps_lifecycle_api_scope -- --nocapture` 退出0，5通过/11465过滤；最终全仓Clippy命令同上，退出0，20.48秒。连同正式路由1项、空配置文件1项，共7项有效自动测试通过。人工HTTP与厂商CLI场景另计。

状态：已完成，待回归。两处底层缺陷已修复并验证；完整验收仍缺SDK/Iced对应截图、默认终端及授权cwd启动、专用供应商/官方账号材料与清理、运行中安装取消、UI断线/错误toast。缺项依赖可控制的原生窗口及专用身份，不能以替代边界测试冒充实机通过。其他平台未做实机验收。

## npm 配置隔离的实机缺陷

正式API返回200目录/实例和空身份列表；卸载预览在npm位于其他prefix时422，按既有所有权规则将已安装可信node/npm的入口链接到本测试prefix/bin，并更新仅本任务管理员策略后预览200、提交202，操作最终failed。精确复制子进程环境可见 npm 11.20.0 报 `double-loading config "/dev/null" as "global", previously loaded as "user"`。没有修改个人npm配置。

最小修复将用户配置改为NamedTempFile创建的私有空文件，global仍使用/dev/null，调用者持有守卫到子进程结束。原有env_clear、固定PATH、prefix、engine_strict、cwd不变，无新依赖。测试检查不同路径、空内容及释放后文件消失；红灯退出101，1失败/166过滤，绿灯退出0，1通过/166过滤。失败命令仅返回固定状态，不将npm载荷透传API。

空配置红绿灯的确切命令为 `cargo test -p vw-agent-apps --lib cc_apps_npm_configuration_files -- --nocapture`。最终Rust修改后执行 `cargo fmt --all` 退出0，再执行上述聚焦测试、生命周期回归、全仓Clippy及网关构建。`git diff --check` 退出0。

清理：本任务网关、两个独立桌面尝试进程均已退出；标准Cargo桌面启动由控制进程发送SIGTERM，实际返回-15（shell等价143），没有冒充自然退出0。真实npm操作已全部终止，测试入口和临时令牌/策略/应用包已清理。最后终止标准桌面后，控制进程删除本人slot-2 owner及锁；slot-1/9他人锁保持原状，slot-2编译缓存保留。个人既有桌面/网关和Claude安装保持原状。
