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

## 仍阻塞的收口条件

默认launch返回200，确实观察PID20908执行所选隔离prefix/bin/omp，lsof确认cwd为授权隔离HOME。然而进一步ps环境检查发现该进程HOME为个人OS HOME，并非测试HOME，故隔离环境验收失败，立即结束该进程；没有抓取或保存其他环境值/凭据。`launch/posix.rs`在默认路径只构造shell cd/exec，再由独立OS Terminal启动，没有转送策略home；身份启动另走Gateway PTY材料生命周期。此跨应用默认终端契约不在本任务改动中扩写，也不通过给omp增加上层特例绕过。需要专用macOS测试用户/主机，或由终端底座正式补齐受控环境后，再验证进程环境和相关临时材料。不声称个人配置未被该短暂进程访问，也不在无基线情况下删除个人资料。

launch-identities为空；`providers/mod.rs`对Omp明确not implemented，无专用供应商/适用官方账号与清理证据。旧版自更新实际受GitHub API限流，正式受管update只验证最新版无需升级；真实旧版到新版变化仍缺。上述是缺项/失败，不标作上游不支持。任务59保持**已阻塞**并保留编号文件；下一步在隔离终端、身份与可成功旧版更新条件具备后收口，不宣称整项通过。

最终全仓Clippy退出0（5m19s），新CLI和默认desktop构建均0，desktop运行后主动-15。确切命令、退出码、红绿与20项有效测试数量见omp-validation.txt；仅本任务源码/测试/证据提交。所有自有Gateway、Iced、终端omp及使用槽位的进程结束后再释放slot-6，缓存保留；原有个人标准omp路径仍为空，未改个人安装。两份测试二进制、下载暂存、随机令牌/策略/配置/缓存与临时.app清理。plan.md保持只读。
