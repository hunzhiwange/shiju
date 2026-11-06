# 任务 59: macOS-omp实机验收

批次：【批次 59】 依赖：批次 58（传递依赖此前全部批次）。线性执行；不与其他任务并行占用测试安装、终端或桌面进程。

状态：已完成，待回归

目的：用macOS真实环境证明 `omp` 的已声明安装运行能力，不外推到其他平台。

来源任务：母任务3需求1–5；用户追加约束“只需要验收macos”。

预计会话范围：只验收一个应用，通常20–40分钟操作，下载/首次构建等待单列；不在此修复其他应用或重做模块。

## 文件

- 读取：`docs/shiju/agent-apps/task-01.md` 至 `task-45.md` 中本应用相关证据；`来源与接口.md`。
- 新建证据：`docs/shiju/agent-apps/macos/omp.md` 与同目录本应用截图。
- 本任务不预设代码变更；若确有本应用缺陷，先在本任务登记具体独立测试与失败命令，再作最小修复。

## 上下文

- 只用macOS专用测试用户/隔离安装前缀，记录OS/架构/应用版本与来源；先列出原有安装，禁止卸载个人现有实例，不搬原始凭据。
- CLI安装能力与ACP、审查运行时分开；Claude Desktop不得用CLI白名单验证。某功能只有来源代码明确不支持时才可标不支持；缺账号、缺版本、缺环境一律未验证。
- 前置母任务4、6不齐时，供应商/官方账号启动记录待回归，不能假报整项通过；此验收不会验证Windows/WSL/Linux系统调用。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-10`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–10。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 10 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 验证计划

测试驱动开发不适用：本任务是已有实现的真实场景验收，不添加镜像测试。缺陷修复另需失败/通过证据；只做文档记录无需Clippy。

- 启动命令：`cargo run -p vw-desktop --bin shi-ju`（默认features）；使用该程序现有网关连接入口连接测试Gateway，不臆造CLI启动参数。必须先完成槽位领取，桌面进程退出后释放。
- 有效自动测试数量如未执行记0并注明本项以人工场景验收，不把编译成功或0测试当通过。若改Rust，先 `cargo fmt --all`，运行对应实现任务的精确过滤测试，再 `cargo clippy --all-targets --all-features -- -D warnings`。
- 场景：从SDK调用对应API取得目录/实例与Iced一致；验证 `omp` 安装前状态、官方允许渠道安装、安装后版本/可运行、可用升级及来源锚定、指定实例卸载及剩余实例保持。只对来源声明支持的动作执行，明确限制须附源码与API错误证据。
- 升级必须有可观察版本/安装产物变化；已为最新版本只证明无需升级，不能代替升级证据。可以使用来源明确支持的旧版本测试安装；没有安全可用旧版则保留升级待回归，不编造下载URL。
- 启动：默认终端、授权cwd、指定供应商/适用官方账号；核对实际执行程序与环境及临时材料清理，不只看API返回成功。非CLI或没有此能力返回明确限制。
- 失败：破损入口、不可运行或来源未知至少覆盖适用项；不通过破坏个人安装制造故障。检查进度/取消（隔离测试安装）、断线重读与错误toast。
- 如果来源只允许外部手动安装，验证指引、安全URL和安装后发现，不发明自动支持；浏览器/剪贴板在客户端，安装及终端在目标主机。

## 步骤

1. 核对依赖与对应能力、准备本应用专用测试安装并记录基线。
2. 在真实macOS依次执行上述适用场景，每一步留API请求/脱敏响应、实际进程/实例变化、Iced截图和结果。
3. 复核其他实例与测试前基线；清理本任务创建的测试安装/临时凭据，不删除个人资料。
4. 把成功、来源明确限制、未验证分开记录到持久证据；关闭本任务桌面/子进程再释放锁。

## 验证

- 所需新证据：macOS版本、CPU架构、应用/包管理器/Node版本、来源、安装前后实例、升级变化、卸载后验、进程/cwd/账号隔离、API与UI对应记录、执行时间及截图。
- 完成条件：全部适用场景有实际证据；未支持项有来源依据及明确API/UI行为；其他平台注明未做实机验收。缺依赖/账号/可验证升级保留待回归，不能记通过。
- 若使用Cargo，记录确切命令、退出码、实际槽位、绝对CARGO_TARGET_DIR及有效测试数量；仅日志中包含路径/账号的部分脱敏，不保存密钥。

## 完成

有未验证场景按执行技能保留 `已完成，待回归` 或真实依赖阻塞时 `已阻塞`，只在本编号文件记录原因。完整通过方可 `已完成`；使用 `$git-commit` 提交本任务实际证据/修复并排除无关变更，只有任务状态变化不提交。最终报告后删除已完整通过的任务文件，证据保留；不修改plan.md。

## 2026-11-06 实施前缺陷登记

正式local.rs仅对Agy绕开npm安装门控，Omp官方原生配方未接线，原生收据/来源锚定亦缺失。拟以独立cc_apps_omp_managed测试覆盖空前缀预检、官方安装器环境/真实进程更新与指定卸载、手工来源拒绝、摘要/符号链接与取消回收；先运行失败，再最小补齐Omp专用本地适配。官方安装器支持PI_INSTALL_DIR和--binary、--ref v18.8.9，下载实际SHA256校验通过后才能取得来源。

真实API/SDK读取官方18.17.0原生文件为invalid_version_output；实际--version为omp/18.17.0。增加cc_apps_omp_managed_version_probe_accepts_official_binary_output精确失败/通过测试，仅识别官方omp/前缀且拒绝URL/未知前缀。

## 实际执行结果与回归条件

已修复Omp正式原生生命周期缺口及官方omp/版本格式，红灯2项、绿灯7项，加相邻回归13项，共20项有效测试通过；cargo fmt --all与全仓cargo clippy --all-targets --all-features -- -D warnings均0。正式安装3d39e86b、update a7b279eb、指定卸载1bcc0bf3均completed；两实例/剩余实例、SDK/Iced一致、UI提交后取消f4ca9875、离线503/恢复200/toast、破损断链/unknown拒绝与最终API/SDK/Iced空态有实证。持久证据docs/shiju/agent-apps/macos/omp.md及omp-api.json、omp-sdk.txt、omp-runtime.txt、omp-validation.txt与原生JPEG。

阻塞：默认终端实际程序和cwd正确，但PID20908的HOME为个人OS HOME，未使用隔离测试HOME，已停止，不能判隔离通过。posix默认shell路径没有传送策略HOME；需专用测试OS用户/执行主机，或终端底座补齐受控环境，禁止在本应用加绕过特例。无专用供应商/适用官方账号，且Omp供应商投影not implemented；旧版18.8.9自更新GitHub限流退出1，正式最新版update无版本变化，仍需真实升级变化。取消虽终态正确且最终无相关进程，留下1个下载暂存文件，已随隔离根显式清理，不假报自动暂存清理。保留本任务待条件恢复回归，不删除，不改plan.md。

动态slot-6；CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-6。CLI首次构建0（17m07s），desktop首次为优先版本红灯中断-2；最终CLI/desktop构建0，cargo run -p vw-desktop --bin shi-ju默认features进入Running后主动-15。所有自有产物进程结束后释放本人锁保留缓存，测试二进制/凭据/暂存/app清理。其他工作区改动不纳入提交。

## 终端隔离继续执行

用户授权底层范围扩展。源码task-29和Iced启动表单明确主机现有配置；保留其含义，默认launch转由既有Gateway PTY通路执行，以管理员host.home明确设置子进程HOME及XDG目录，隔离终端环境且不改父环境/个人配置。先扩展生产API测试cc_apps_launch_api_target_scope_accepts_and_rejects_before_terminal，要求实际CLI的TTY、HOME/cwd/program证据与sessionId，确认修复前失败。

## 终端隔离修复后的最终结果

默认“主机现有配置”现经正式Gateway PTY执行，HOME来自管理员host.home，XDG为该HOME的标准目录，PATH为管理员/系统目录，TMPDIR为私有会话目录；不改代理父环境，不让登录Shell/OS Terminal覆盖CLI环境。不新增客户端HOME输入或权限开关；保持当前配置语义和tenant/host/principal拒绝。

红灯生产API1项失败（未执行真正CLI、HOME证据缺失）；最终本轮20项有效测试通过：环境污染1、PTY3、生产API1、原有身份HTTP/SDK轮换1、请求7、POSIX终端7。一次跨principal夹具token不足32字的401已修正为合法长度，保持404断言。cargo fmt --all、CLI构建0；首轮本次全仓Clippy0；最终API加强HOME/cwd不同目录断言仍0，最后全仓Clippy受无关ACP process_tests.rs:445 E0282阻断，未越界修复，定向检查结果见日志。

官方18.17.0在新隔离主机通过真实HTTP启动，实际程序/cwd=HOME/workspace、HOME/XDG/TMPDIR均正确，无宿主API key/钩子；主动结束后真实Failed、进程消失、私有目录删除。正式help位置参数启动会话Completed；跨租户/跨principal均404，最终API空态。证据docs/shiju/agent-apps/macos/omp-terminal-api.json、omp-terminal-runtime.json、omp-terminal-validation.txt及更新后的omp.md。

本轮动态slot-1；CARGO_TARGET_DIR和CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1，owner PID46034。验证和自有产物进程结束后清理隔离根/凭据/配置/二进制并释放本人锁保留缓存。默认终端问题已解决；按追加授权将状态改为已完成，待回归并保留文件，剩余专用身份/官方账号、旧版真实升级与无关ACP全仓检查待回归。plan.md只读。

最终定向cargo clippy -p vw-agent-apps -p vw-agent --all-targets --all-features -- -D warnings退出0。其他会话补齐ACP类型标注后，全仓重跑转而被无关Git面板diff_view/history.rs:31 E0599、repository.rs:271 E0061阻断（101），未修改这些模块。默认终端环境修复与20项行为验证均通过；最终全仓复核随这些独立改动稳定后回归。
