# 任务28：终端启动输入边界

2026-11-06，macOS。状态：已完成。

依赖：读取只读plan.md、来源与接口.md及任务27证据；核对正式vw-agent-apps、AppId、AppInstallation及前序主机偏好产物。使用仓库RustCodeGraph技能检查索引并读取resolve_launch_cwd、split_launch_args、ProviderLaunch；新增未索引领域使用文件读取。未修改upstream、来源快照或plan.md。

## 实现与责任边界

新增共享LaunchRequest/LaunchResult和执行主机业务入口validate_launch_request。请求只包含应用ID、实例ID、cwd、结构化argv及默认false的skipPermissions；拒绝未知字段，包括program、command、credentials和targetHostId。LaunchResult只包含实例ID，终端适配器实际接受启动后才能返回，不将校验成功冒充启动成功。

入口按授权调用者的目标主机、应用及实例ID唯一匹配新鲜发现结果；拒绝重复、缺失、非Runnable实例和桌面应用。程序路径只取服务端观察值，要求绝对路径、现存文件、Unix可执行位，并与已有real_path锚定比较。cwd必须是绝对、现存目录，canonicalize后落在调用方授权根目录中；拒绝控制字符、文件、缺失目录、相对目录和符号链接越界。错误不回显原始路径/参数。

args保持独立argv，不解析命令串、不调用shell；允许空格和中文位置参数，拒绝控制字符、shell展开/连接元字符。原始以短横线开头的CLI选项明确拒绝，防止通过应用各异的配置/沙箱/权限选项及别名绕过策略；当前入口不承诺任意CLI选项透传，后续材料化只能按审查过的应用规则生成选项。最多32项、单项8192字节，拒绝超界输入。

skipPermissions显式true仍必须同时得到授权调用者的许可；即使许可为true，客户端也不能自行通过args塞入--yolo等选项。不在本任务新建授权策略、主机RPC或凭据接口。现有业务入口同样由调用方提供授权上下文；任务36负责从真实认证/权限策略接线，禁止直接信任客户端提交的发现DTO、根目录或许可布尔值。

文件系统检查仅在安装实例自己的执行环境进行。本机macOS入口拒绝Windows/WSL观察，避免把异机路径在本机解析；Windows/WSL显式环境夹具在macOS运行，证明错误环境拒绝，不冒充Windows/WSL实机证据。后续WSL适配器须在对应发行版内实施同等检查，不将本机拒绝当作全平台不支持。实际执行前需重验材料，当前校验不消除校验/启动之间的文件替换竞态。

不执行终端、不修改HOME、个人安装或凭据；本任务没有UI修改。后续终端适配器与API接线属于后续任务。

## Cargo槽位与验证

原子mkdir依次尝试1–10，实际领取slot-2。

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖.cargo/config.toml的build.build-dir=target，避免中间产物逃离槽位。锁记录计划、任务、会话及持有进程；验证和产物进程结束后释放自己的锁，保留缓存。

- 红灯：`cargo test -p vw-agent-apps --lib cc_apps_launch_request_validation -- --nocapture`，退出101；4个有效测试，2通过、2失败。失败是最小可编译签名明确返回未实现，合法启动与获准skipPermissions的真实结果断言失败；不是工具或依赖错误。

- `cargo fmt --all`，退出0；审查只有本任务模块注册及新增Rust文件，无无关格式化差异。
- 绿灯：`cargo test -p vw-agent-apps --lib cc_apps_launch_request_validation -- --nocapture`，退出0；7通过、0失败、121过滤。覆盖合法实例/中文空格argv、主机/应用/实例不匹配、未知程序路径冒充ID、授权根为空、cwd不存在/非目录/相对/换行、权限默认与拒绝/获准、参数注入、数量/长度边界、符号链接逃逸、失效/重复/非可执行/非Runnable实例及Windows/WSL环境拒绝。
- DTO：`cargo test -p vw-api-types --lib cc_apps_launch_request_validation -- --nocapture`，退出0；1通过、0失败、75过滤。覆盖默认false/空argv、往返及未知命令/凭据/主机字段拒绝。
- `cargo clippy --all-targets --all-features -- -D warnings`，退出0，无警告；最终修改后重新运行，15.82秒完成。
- `git diff --check`，退出0；只暂存本任务7个Rust文件及本证据，前序编号1–27的已有删除排除。槽位已在全部验证进程结束后释放，保留缓存。

下一步：任务29终端启动材料化；按上述授权和执行环境边界消费材料，不把校验结果当作已启动。
