# Codex macOS 实机验收（已完成，待回归）

2026-11-06，Asia/Shanghai。已阅读只读总计划、来源与接口及任务05–45相关持久证据；依赖47已有SDK/Iced实机结果，仍有待回归项，不把状态标签等同于实现缺失。本任务没有修改Rust或plan.md。

## 环境、来源与隔离

macOS 27.0 (26A428)，arm64；Node v26.13.0、npm 11.20.0；brew入口/opt/homebrew/bin/brew。个人PATH入口为/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex，版本0.166.0-alpha.6，前后SHA256均508388df35319042107560d095b271a98a512f59a542848c908b23ed1bad4b44。既定/opt/homebrew/bin/codex及个人.local/bin/codex不存在；不代表磁盘全范围无实例。未卸载、修改或复制个人Codex配置与凭据。

测试使用权限0700的随机临时根，独立home、XDG配置、npm prefix、Applications及0600管理员策略；随机应用令牌仅临时保存，策略仅保存SHA256。网关只监听127.0.0.1:19448，租户p/a、授权host；没有接入个人网关。node/npm入口链接到既有可信Homebrew程序，所有应用写入仅本测试前缀。

官方来源核对：[Codex CLI官方文档](https://learn.chatgpt.com/docs/codex/cli)提供npm安装渠道；实际读取registry.npmjs.org的@openai/codex元数据，latest=0.167.0，稳定旧版0.166.0来自版本列表，并非猜测URL。旧版tarball=https://registry.npmjs.org/@openai/codex/-/codex-0.166.0.tgz，integrity=sha512-2Szlp1l1cz5TX1ccpX4Zok3DZrot9xeZe0km+6/1v8JmqxADwyJxKBJah4dBymkwtS25j/u6dipJusTOI97Z8Q==。

## 实际场景与结果

脱敏请求/响应保存在codex-api.json，不含授权头；SDK输出在codex-sdk.txt，截图均来自唯一标识com.rsung.shiju.task48-acceptance的本次Iced窗口。

- 初始catalog为Codex/native_cli，installations为空，launch-identities为空，三项HTTP200。
- `HOME=<isolated-home> npm view @openai/codex dist-tags --json --registry=https://registry.npmjs.org`及versions、旧版dist查询均退出0。
- `HOME=<isolated-home> npm install --global --prefix <isolated-prefix> @openai/codex@0.166.0 --registry=https://registry.npmjs.org`退出0，约26秒新增2包；隔离codex --version退出0、0.166.0。
- 正式update预览200，提交202，583eb14b最终completed；真实版本变为0.167.0，随后CLI --version退出0。不是“已最新”替代升级。生命周期预览按管理员prefix和精确package.json复核来源为system；通用发现及UI仍显示unknown，保留这一真实差异。
- 正式卸载ae60cf77最终completed，发现为空；正式install af92d10f最终completed，发现0.167.0/runnable。每次状态与events均200，未把queued当安装成功。
- 第二份官方0.166.0安装至隔离home/.local，npm退出0。双搜索路径发现两个真实实例。生命周期预览422；源码apps/lifecycle/local.rs::validate明确要求search_paths恰为[write.prefix/bin]，因此并非多实例统一写入已支持。管理员策略临时缩回第一prefix后，正式指定卸载35d99004 completed；再恢复双路径只读发现，仅第二份0.166.0保留。SDK与Iced截图codex-iced-remaining.jpg对应同一路径。该证据证明实际剩余实例保持，不冒充双路径写入通过。
- 第一prefix为空后提交install 51969614，立即DELETE返回cancelled/terminal，events独立重读也cancelled，第一prefix发现为空。此为排队后立即取消，未证明运行中下载进程取消。
- 在本任务第一prefix创建断链codex入口，diagnostics发现unknown、probe_failed/damaged_installation，并保留另一个可运行实例。破损ID卸载预览及launch均422，未破坏个人实例；随后只清理本任务断链。
- Iced通过现有客户端网关设置修改端口19448及租户p/a，经选择主机表单加载host和随机令牌。升级后显示可运行、0.167.0及精确实例/真实路径，见codex-iced-installed.jpg；来源unknown时明确显示人工核实来源提示，未显示自动卸载成功。
- 正式SDK GatewayClient::agent_apps_catalog_authorized及agent_apps_installations_authorized真实调用，目录Codex/native_cli与实例版本/路径对应Iced；指定卸载后再次读取仅剩第二实例；最终读取为空。三次成功执行均退出0。断线SDK读取Transport，临时驱动unwrap退出101，单列为人工故障观察，不算产品panic或自动测试。
- 仅终止本任务19448网关，Iced刷新显示“加载失败，请重试”和错误toast，见codex-iced-offline.jpg。相同隔离配置重启后点击重试，已清理安装显示未安装，见codex-iced-cleaned.jpg。

## 未完成场景与阻塞

没有获授权的专用供应商/官方测试身份。正式launch-identities返回空，Iced启动弹窗明确显示两类身份暂无可用记录，见codex-iced-launch.jpg。不能使用个人登录替代；也不能伪造供应商/账号证明身份材料化及清理。

默认终端的真正执行程序/cwd/环境、供应商与官方账号启动/临时材料清理仍未验证。曾对可运行实例尝试args=["--version"]的正式launch，返回422；源码apps/launch/request.rs:69–82拒绝以-开头的通用参数，此响应不能证明终端已启动。未削弱参数校验。apps/launch/posix.rs:70–105通过Terminal AppleScript启动shell命令，未给默认主机配置启动注入独立HOME；现有个人Terminal会话不是专用测试用户，故没有执行可能读取个人登录的默认TUI来制造“隔离通过”。需要专用macOS测试用户/可证明隔离的终端与专用身份，才能完成这一组验收。

运行中下载取消尚未验证；现有立即取消证据不外推到进程树时序。多搜索路径生命周期写入由源码明确拒绝且API422，本任务不扩大底座支持范围。未把缺账号称为不支持。其他平台未做实机验收。任务保持已阻塞，保留编号文件；不能以本证据文档完成认领全部验收通过。

## Cargo、SDK驱动与清理

原子mkdir从1–10动态领取slot-2；owner记录计划、任务48、会话和控制进程PID。实际CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，CARGO_BUILD_BUILD_DIR同一绝对路径，覆盖.cargo/config.toml的build.build-dir=target。无--target-dir绕过。所有槽位产物使用结束才释放本人锁，缓存保留，slot-1/9他人锁不动。

- `cargo build -p vw-cli --bin shiju`：退出0，1.00秒。
- `cargo build -p vw-desktop --bin shi-ju`：退出0，0.48秒。
- `cargo run -p vw-desktop --bin shi-ju`：默认features，构建0.27秒并进入Running；最终主动SIGTERM返回-15。仅有既有ld __eh_frame提示，不改链接设置。
- 网关命令：`<slot>/debug/shiju gateway --host 127.0.0.1 --port 19448`，两轮主动SIGTERM结束。CUA不能直接定位未打包binary，复制同次产物为唯一测试bundle，独立HOME启动并获得实际截图；无UI源码变更。
- SDK临时驱动确切链接命令：`rustc --edition=2024 <isolated-root>/sdk.rs -L dependency=<slot>/debug/deps --extern vw_gateway_client=<slot>/debug/deps/libvw_gateway_client-1a94bf756d53cb1d.rlib --extern tokio=<slot>/debug/deps/libtokio-f993a923b732abac.rlib -o <slot>/task48-sdk`，退出0。使用SDK重导出的vw_api_types，未伪造接口或新建Cargo构建目录。
- 有效自动测试0；本项为人工场景验收，没有Rust改动，按任务规则无需Clippy；编译或0测试均不替代行为证据。
- 第二prefix通过`npm uninstall --global --prefix <isolated-home>/.local @openai/codex`退出0清理；第一prefix通过正式API卸载及取消后为空。最终API200/installations=[]，SDK成功空列表，Iced重连未安装。
- 本任务Iced bundle、标准桌面、两轮网关均终止，进程检查无本测试根残留；删除临时令牌/策略/应用包/隔离数据，个人Codex前后哈希一致。保留脱敏证据及共享编译缓存。

## 恢复复核与最新结论（2026-11-06）

按追加调度要求复核task-44及提交7f76dc816后当前生产代码：前节“已阻塞”为此前判断，本节结论取代该状态。身份列表与托管PTY通路已交付，不存在“身份契约未实现”或一概501的结论。正式处理器launch.rs在providerId/accountId分支调用唯一AppProviderStore/AuthAccountStore进行材料化，再经TerminalSession::connect_with、prepared.run_with、transport.run运行真实子进程；租约保持到退出、轮换回写与清理完成，sessionId只表示已spawn，最终状态另读launch-sessions。主机现有配置是单独的无身份分支，不能将其个人Terminal环境问题泛化为托管PTY缺陷。

本次没有读取个人全局凭据、扫描钥匙串或自动认领个人登录。独立网关身份列表实机已为空；创建虚构密钥/令牌只能证明材料契约，不能形成真实厂商登录证据；因此没有向生产账号库写入假身份或发起外部模型调用。安全可执行的补充路径是运行已有隔离夹具回归，验证当前正式SDK/HTTP/PTY通路与权限边界，再明确保留真实账号场景待回归。

新验证阶段动态原子领取slot-2，CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖仓库build.build-dir。owner记录任务48/resume及持有PID；全部Cargo和测试子进程结束后已释放自有锁、保留缓存，其他锁不动。

| 本次确切命令 | 退出码 | 有效结果 |
| --- | --- | --- |
| cargo test -p vw-agent --lib cc_apps_launch_identity_list -- --nocapture | 0 | 3通过，11467过滤；0.49秒 |
| cargo test -p vw-agent --lib cc_apps_launch_native_terminal -- --nocapture | 0 | 1通过，11469过滤；1.36秒 |

共新增4项顶层有效回归通过。后者使用真实macOS Terminal、正式SDK/HTTP/PTY与独立临时SQLite/凭据目录；CLI和身份内容为受控夹具，覆盖供应商/账号材料存活、TTY、账号轮换及清理。它不是本任务官方Codex包登录真实服务的证据，不将假密钥当成真实登录成功。无Rust变更，按本任务规则无需Clippy；没有执行全量测试、fmt检查、wasm或Makefile。

最新状态为**已完成，待回归**，保留任务48供回归；不会因缺少真实测试身份阻止下一应用批次。已完成的官方包生命周期、SDK/Iced一致性及故障场景证据继续有效。仍待真实身份/专用终端环境下确认官方Codex的默认终端程序/cwd/环境、供应商和官方账号登录启动及临时材料清理，以及运行中下载取消时序。缺账号属于未验证，不标不支持，不宣称整项完整通过。plan.md保持只读。
