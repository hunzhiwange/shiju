# Claude macOS 实机验收

2026-11-06，macOS 27.0 (26A428)，Darwin arm64，Node v26.13.0、npm 11.20.0。仅涉及 Claude Code CLI；不代表 Claude Desktop 或其他平台。

## 最终补验结论（2026-11-06）

状态：已完成（按用户调整后的账号验收范围）。用户授权其余应用沿用 OMP 口径：无专用账号时，以官方程序访问隔离测试接口及材料清理为准；真实账号登录以后配置。本次未执行 Anthropic 真实账号登录，loopback 测试密钥不代表厂商认证。下文“待回归”等状态为首轮历史记录，由本节补验结果取代。其他平台和 Claude Desktop 不在本项结论内。

### 实际界面、SDK与进程

使用隔离 HOME、配置、管理员 npm prefix，未复制个人凭据。默认 `cargo run -p vw-desktop --bin shi-ju` 确认进入 Running；为 CUA 定位，将同一槽位桌面产物复制到唯一标识的临时 Task46Claude.app 后实际运行。使用现有网关连接表单连接测试 host；初始错误凭据得到 Unauthorized toast，纠正为本测试授权后读取成功。

- 实际 `vw-gateway-client` SDK 取得 Claude / NativeCli、2.1.318 / runnable；修复后来源为 System，与 Iced 一致。卸载和取消后两者均返回零安装。记录：[SDK](claude-final-sdk.txt)、[API](claude-final-api.json)。
- 默认终端 API 与 Iced“终端启动”均执行官方 npm 包的真实 claude.exe。实际 PID 33068、35373，cwd 为授权 `<isolated-home>/workspace`；HOME、XDG、PATH 隔离，未继承供应商密钥或 Node hooks。主动 SIGTERM 后 session 为 Failed，进程及 sj-tty 临时目录消失；这是受控停止，不是成功完成聊天。[进程证据](claude-final-runtime.json)、[Iced 运行](claude-final-iced-launch-running.jpg)、[停止结果](claude-final-iced-launch-stopped.jpg)。
- 供应商配置经真实 API 保存、授权身份可列出，未授权 ID 返回403。API 启动观察到测试 baseURL、测试 key 和私有配置正确投影，停止后配置及终端临时目录清理；该次在修复参数拼写前，不作为协议成功证据。[脱敏身份 API](claude-final-provider-api.json)。
- 之后用修正后的生产 `prepare_provider_launch`，在授权隔离 cwd 执行官方 Claude 2.1.318；仅测试传输追加 print / 禁工具 / 禁会话持久化参数，没有开放客户端任意 flags。真实收到 loopback Anthropic `/v1/messages?beta=true`，model=claude-sonnet-4-5、stream=true、测试 key/prompt 匹配；官方 CLI 输出 CLAUDE_PROVIDER_OK，退出0，私有材料移除。此为官方程序实际协议访问；不声称真实厂商登录。详见同一进程证据中的 provider-helper-result、provider-requests。

### Iced 生命周期与连接失败

- 管理员 npm manifest 锚定后 Iced 显示 System，指定实例卸载操作 `88466af3-ffef-460e-8d7e-c1c5e9fa73a1` Completed。API、SDK和刷新后的 Iced 均确认未安装，个人原有实例保留。[来源](claude-final-iced-anchored.jpg)、[卸载确认](claude-final-iced-uninstall-confirm.jpg)、[卸载后](claude-final-iced-uninstalled.jpg)。
- 实际 Iced 安装后，观察官方 npm 子进程 PID/PGID 60772、父进程测试网关53010、cwd=隔离 prefix；Iced 为 Executing / Running 时点击取消，操作 `3f346ecb-fe21-4713-b35e-65bc48c8b098` 变为 Cancelled。独立状态/events重读一致，PID不存在、进程组为空、发现为空。此前两次下载过快已 Completed，未将它们计为取消通过。[运行中](claude-final-iced-install-running.jpg)、[取消后](claude-final-iced-cancelled.jpg)。
- 使用只转发原请求的本地 TCP relay 切断客户端至测试网关连接，网关进程及操作状态保留。Iced刷新实际出现加载失败及错误 toast，恢复传输并重试后重新显示零安装及同一 Cancelled 操作；独立重读状态/events仍 Cancelled。relay只记录方法、路径、时间，不记录认证头/正文；断开前后 actions POST 数均为6，没有重复提交。[断网](claude-final-iced-disconnected.jpg)、[恢复](claude-final-iced-recovered.jpg)、计数见进程证据 network-proof。
- 单独的管理员 host.online=false 场景返回503并显示错误 toast；此与实际 TCP 断线分开记录。[主机离线](claude-final-iced-host-offline.jpg)。

### 补验发现的最小修复

正式读取 API 未复用生命周期已验证的 npm ownership anchor，导致 Iced 显示 Unknown、禁止卸载。独立测试先失败 Unknown != System，再令 installations/diagnostics/versions 复用底层 scoped_discover；错误包名仍为 Unknown，只读授权不获得写权限，Claude Desktop 仍走原发现路径。

官方 CLI 实际报 unknown option --settings-sources，并提示 --setting-sources。独立测试先红灯，再将固定隔离参数改为单数，实际官方程序协议访问通过；未放宽客户端参数或凭据边界。

### 补验验证与清理

本阶段动态领取 slot-1，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。先 `cargo fmt --all` 退出0；最终聚焦测试为 provider isolation 9项、read authorization 8项、lifecycle scope 5项，共22项有效通过，不重复计子进程探针摘要。`cargo clippy --all-targets --all-features -- -D warnings` 退出0（1m21s），最终CLI构建退出0（1m33s）；链接器既有 __eh_frame 提示单列。一次中间构建因无关 ACP 字段失败，由该任务维护者修复，最终构建通过；本任务未修改ACP。确切命令、红绿灯、退出码与辅助 Rust 探针编译命令见 [最终验证](claude-final-validation.txt)。未执行无过滤全量测试、wasm或Makefile。

取消后SDK/API发现为空；最后个人 Claude 仍2.1.316，SHA256=`64c327e92b2c5e0f78c0a980c5a518121e1f0d140ca771ca5dd4488d1a2dcb3f`，与初始相同。所有本任务桌面、网关、relay、协议fixture、npm/CLI子进程退出后清理隔离安装、策略、令牌、临时app与辅助可执行文件，随后释放本人slot-1锁，保留编译缓存。受控SIGTERM退出-15，不冒充自然退出0；未终止个人进程。终端应用可能保留空闲窗口，不认领为已关闭，测试CLI及其临时资源已确认消失。

## 首轮历史记录

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
