# Oh My Pi macOS 实机验收

2026-11-06，Asia/Shanghai。读取只读plan.md、任务59、来源与接口、task-17/31/32与批次58实际证据。批次58为已完成，待回归，正式API/SDK/授权主机已存在；不把母任务标签当作符号缺失。RustCodeGraph status与node核对当前生命周期、发现和升级实现。其他平台未实机验收。

## 环境与官方来源

macOS 27.0 (26A428)、arm64；Node v26.13.0、npm 11.20.0；开始时command -v omp/bun均空。0700随机测试根包含独立HOME/config/prefix/Applications，0600随机令牌和管理员策略仅授权p/a/host及该前缀；不读取个人供应商凭据。Gateway仅监听127.0.0.1:19459。

[官方安装器](https://omp.sh/install)本次明示PI_INSTALL_DIR、--binary及--ref；支持例子v18.8.9，默认build服务https://build.stencil.so。源码说明预编译包以SHA256校验后原子移动并--version自检。安装器本次SHA256和原始脱敏日志见omp-runtime.txt，未编造二进制下载URL或使用npm/Pi包代替omp。

独立执行官方--binary到隔离prefix/bin成功，实际omp/18.17.0、help退出0；官方--binary --ref v18.8.9到隔离HOME/.local/bin成功、实际omp/18.8.9。尝试18.16.0的官方版本API返回404，未把它当可用旧版。旧版直接update首次因PATH没有其入口退出1；将唯一所选入口加入PATH后明确GitHub API rate limit exceeded，仍18.8.9，未搬个人GH_TOKEN或把失败当升级变化。

## 修复前真实失败与最小修复

旧生产Gateway目录200/Oh My Pi/native_cli，安装发现将真实18.17.0报告为probe_failed/invalid_version_output；SDK同样读取，launch-identities=[]；安装预检422。原因分别为通用版本解析未识别omp/前缀，和local生命周期仅放行Agy原生配方而Omp落入npm缺配方门控。独立测试先得到1项行为失败，增加实际版本格式用例后得到2项行为失败（非编译错误）。脱敏HTTP见omp-api.json。

最小补齐Omp专用本地适配：固定官方--binary和PI_INSTALL_DIR，安装/更新HOME、TMPDIR为prefix/.shiju-omp私有目录；沿现有env_clear、授权写锁、重建计划、Operation进度/取消。保留管理员npm/node路径验证契约，但不调用npm安装或Node元数据门控原生omp。只有本执行器官方命令成功、实际版本可运行及原生文件检查通过后才写0600 SHA256收据。发现、更新、指定卸载均核验精确prefix/bin/omp、摘要及路径；手工安装保持unknown，外部替换/自更新摘要变化同样降为unknown。拒绝bin/父路径重定向、二进制或收据符号链接。卸载只删所选入口，保留配置/缓存。未新设RPC、用户配置或新依赖。

版本解析仅额外识别omp/前缀，独立实际子进程测试同时拒绝URL、其他前缀与无效版本。供应商投影providers/mod.rs当前对Omp明确not implemented，为产品缺口而非上游不支持。

## Cargo与测试

从槽位1开始原子mkdir动态领取slot-6，owner记录task59/acceptance/PID54296；CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-6`，后者覆盖.cargo/config.toml的build.build-dir=target。未使用--target-dir或其他目录绕过。

- cargo build -p vw-cli --bin shiju：首次退出0，17m07s；既有__eh_frame链接warning保留。
- cargo build -p vw-desktop --bin shi-ju：首次为优先补做版本红灯测试主动SIGINT，退出-2，后续恢复构建；非构建成功证据。
- cargo test -p vw-agent-apps --lib cc_apps_omp_managed -- --nocapture：修复前退出101，先1失败、再2失败；修复后退出0，7通过。
- cargo fmt --all：退出0。
- cargo test -p vw-agent-apps --lib cc_apps_version_probe_broken_install -- --nocapture：退出0，5通过。
- cargo test -p vw-agent-apps --lib cc_apps_native_uninstall_scope -- --nocapture：退出0，5通过。
- cargo test -p vw-agent-apps --lib cc_apps_kimi_omp_install -- --nocapture：退出0，3通过。

共20项不同有效测试通过，含真实C原生夹具及shell子进程、安装/更新/指定卸载、取消回收、篡改/符号链接边界。夹具只替换下载边界，不将其当官方应用实机证据。未运行无过滤cargo test、fmt检查、wasm或Makefile。其余实机结果和最终检查在完成后补记。

## 正式API、SDK与Iced实机场景

新建网关产物后，catalog 200，安装前空列表；正式安装预检200/system，提交202，3d39e86b经过Executing/Postflight后completed；真实18.17.0/system/runnable，SDK退出0且一致。正式update a7b279eb completed，版本仍18.17.0，只证明当前无需更新，不证明旧版升级变化。

把管理员读取路径临时扩为两份隔离目录，API/SDK同时发现受管18.17.0/system和手工官方18.8.9/unknown。恢复单一写前缀后，指定首实例卸载1bcc0bf3 completed；再读取两路径只剩第二份18.8.9/unknown/runnable，实际--version退出0，SDK一致。未伪造第二份收据，未卸载个人实例。随后仅手动清理本任务第二份文件。

默认features `cargo run -p vw-desktop --bin shi-ju`进入Running，主动结束-15；专用临时Task59Omp.app复制同槽位产物并使用隔离HOME/config。CUA Raise后通过现有顶栏网关入口添加19459/p/a，链条入口选择授权host与随机测试令牌。首次desktop-local不匹配时真实未授权提示/toast见omp-iced-unauthorized.jpg；选择正确授权身份后目录14项，过滤omp显示“系统安装 · 18.17.0 · 可运行”，见omp-iced-installed.jpg，与同阶段SDK一致。一次paste工具提示冲突，立即核对状态并提交后成功加载，没有假定输入成功。未改GUI。

正式卸载后Iced刷新0安装/omp待安装，见omp-iced-empty.jpg。Iced展开macOS安装预检并确认，操作f4ca9875产生Queued显示；独立API观察到running/executing，点击取消后Iced显示Terminal/Cancelled，status/events独立重读均200/cancelled。见omp-iced-running.jpg（实际画面仍Queued，不能称其已显示Executing）、omp-iced-cancelled.jpg。未取得这次真实安装器PID采样，不能把测试夹具进程回收外推为该次已观测全部孙进程回收。最终无测试根相关进程。取消后无omp入口，发现为空，但官方安装器留下一个部分下载文件；最后随隔离根显式清理，不声称取消自动清掉了所有下载暂存。

管理员host online=false后API503，Iced刷新显示远端离线/服务不可用和错误toast（omp-iced-offline.jpg）；恢复true后API200，Iced重读保留Cancelled终态且0安装（omp-iced-recovered.jpg）。这是授权主机离线模拟，不是实际拔网线。截图直接保存工具JPEG原始字节，没有拼图或合成。

隔离prefix创建指向不存在目标的omp断链，读取/正确diagnostics查询200、damaged_installation/unknown，卸载预检422。首次diagnostics误带installationId返回400，按正式契约仅appId重试成功，保留原失败。移除断链后最终API/SDK均空；20项有效自动测试与这些真实场景分开计数。

## 首轮未收口项（终端已在后续修复）

默认launch返回200，确实观察PID20908执行所选隔离prefix/bin/omp，lsof确认cwd为授权隔离HOME。然而进一步ps环境检查发现该进程HOME为个人OS HOME，并非测试HOME，故隔离环境验收失败，立即结束该进程；没有抓取或保存其他环境值/凭据。`launch/posix.rs`在默认路径只构造shell cd/exec，再由独立OS Terminal启动，没有转送策略home；身份启动另走Gateway PTY材料生命周期。首轮未扩写此跨应用默认终端契约，后续获授权后已在底层修复，见下一节；未给omp增加上层绕过特例。需要专用macOS测试用户/主机，或由终端底座正式补齐受控环境后，再验证进程环境和相关临时材料。不声称个人配置未被该短暂进程访问，也不在无基线情况下删除个人资料。

launch-identities为空；`providers/mod.rs`对Omp明确not implemented，无专用供应商/适用官方账号与清理证据。旧版自更新实际受GitHub API限流，正式受管update只验证最新版无需升级；真实旧版到新版变化仍缺。上述是缺项/失败，不标作上游不支持。首轮据此记为**已阻塞**并保留编号文件；后续修复和最终状态见下一节，不宣称缺项已通过。

最终全仓Clippy退出0（5m19s），新CLI和默认desktop构建均0，desktop运行后主动-15。确切命令、退出码、红绿与20项有效测试数量见omp-validation.txt；仅本任务源码/测试/证据提交。所有自有Gateway、Iced、终端omp及使用槽位的进程结束后再释放slot-6，缓存保留；原有个人标准omp路径仍为空，未改个人安装。两份测试二进制、下载暂存、随机令牌/策略/配置/缓存与临时.app清理。plan.md保持只读。

## 继续执行：默认终端环境已修复

2026-11-06，用户明确授权必要底层范围扩展。Iced启动表单的“主机现有配置”和task-29说明表明，无身份启动应使用执行主机的配置；不能把外部OS Terminal继承的个人HOME当作管理员host.home。未增加客户端HOME参数、RPC或权限开关，也未改代理自身HOME/个人配置。

默认launch保留既有实例/cwd/权限校验，改走已有Gateway-owned TerminalSession及principal/tenant/host作用域Sessions。外部默认终端只运行不含凭据的私有socket客户端；真正CLI由Gateway以独立argv在PTY直接启动，绕过登录Shell的环境覆盖。HOME取管理员执行主机的规范目录；XDG配置/缓存/数据/状态目录分别位于该HOME，PATH取管理员搜索路径和固定系统目录；env_clear不继承宿主API key、应用profile选择器、Node钩子。TMPDIR位于0700会话目录，CLI及其后代回收后再关闭socket并清理目录。使用主机现有配置文件的语义保留；管理员若配置真实OS HOME则仍会明确使用该目录，不把此功能宣称为OS文件系统/钥匙串沙箱。

生产API测试先失败于缺少真正CLI写出的HOME证据（退出101，1失败），随后通过。新增第二调用方夹具初因token不足32字而返回401，按真实规则修正夹具后保持404归属断言，未改鉴权。最终API测试以不同的HOME和cwd验证真实TTY、程序、HOME/cwd；有有效主机grant的其他租户/其他principal仍不能读取会话（404）。环境污染子进程测试确保父HOME/API key不变且子进程不继承敏感变量/钩子；PTY回归覆盖私有TMPDIR、正常退出、断线终止进程组及目录清理；原有身份启动/账号轮换、请求边界与POSIX终端适配均回归通过。

本轮20项不同有效测试通过：host环境1、PTY3、生产API1、身份HTTP/SDK/轮换1、请求边界7、POSIX终端7；环境测试的辅助子进程不重复计数，API重复回归不累计。新证据见omp-terminal-validation.txt。CLI构建退出0；首轮本次全仓Clippy退出0。最终加强cwd断言后API仍0，最终全仓Clippy则被其他会话的`crates/vw-acp/src/client/process_tests.rs:445` E0282阻断，未修改无关ACP代码；本任务两个crate定向Clippy退出0。观察到ACP夹具类型标注已由其他会话补齐后重跑全仓，ACP错误消失，但遇无关Git面板`diff_view/history.rs:31` E0599和`repository.rs:271` E0061，退出101；未修改Git模块。未跑无过滤test、fmt检查、wasm或Makefile。

新建0700隔离根及0600随机双principal策略，复用首轮官方安装器--binary，SHA256和退出0日志见omp-terminal-runtime.json。正式生产Gateway在19459读取官方18.17.0/unknown/runnable；此份手工官方安装不伪造受管收据。真实无身份launch返回sessionId，并观察官方omp PID56006由Gateway直接启动。ps仅提取允许记录的环境字段/敏感变量存在性，不保存完整环境或令牌；HOME和全部XDG均指向隔离主机目录，lsof证明cwd为HOME/workspace，所选程序为隔离prefix/bin/omp。TMPDIR在/tmp/sj-tty-*且运行时存在；进程主动SIGTERM后status为Failed（不假报成功），进程已消失且该目录被删除。跨租户/跨principal正式HTTP各404。再以允许的位置参数help经同一正式通路运行，实际会话Completed。最终删除仅本任务官方入口后API空态。请求/响应见omp-terminal-api.json，程序/cwd/环境与清理见runtime JSON。

因此默认终端HOME问题已解决。最终任务59为**已完成，待回归**，保留编号文件：仍待专用供应商/适用官方账号、真实旧版到新版升级变化，以及其他会话Git面板编译修复后的最终全仓Clippy复核，不把缺专用身份/版本条件记为不支持或已通过。首轮Iced/API/SDK生命周期证据继续有效，本轮未改UI、未用Element测试代替原生渲染。

本轮动态领取slot-1，owner为task59/terminal-resume/PID46034；CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，覆盖build.build-dir。真实Gateway主动结束-15，所有自有CLI/socket客户端/产物进程结束后清理随机根、凭据、配置、官方二进制；确认验证命令退出后释放本人slot-1并保留缓存。plan.md未修改，其他工作区改动不纳入提交。
