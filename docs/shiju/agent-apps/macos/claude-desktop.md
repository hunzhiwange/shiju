# Claude Desktop macOS 实机验收（已完成，待回归）

## 执行登记（2026-11-06，Asia/Shanghai）

依赖46为已完成，待回归；已读取其持久证据及任务08、来源与接口，核实正式API、发现、生命周期与启动实现存在，不以母任务状态标签推断符号缺失。沿用正式网关及管理员文件授权，没有复用个人网关或凭据。未修改plan.md或他人变更。

实机：macOS 27.0 (26A428)、arm64；Node v26.13.0，npm 11.20.0，brew入口/opt/homebrew/bin/brew。基线ls /Applications/Claude.app及用户Applications/Claude.app均不存在（退出1）；这仅代表既定搜索根内没有应用，不代表磁盘全范围无安装。未安装、卸载或修改任何个人应用或配置。

### 独立完成的检查

使用私有临时home、Applications、prefix和权限0600的临时管理员策略（随机令牌，仅保存SHA256），本地网关仅监听127.0.0.1:19447。请求均含x-platform-id=p、x-company-id=a及临时x-agent-apps-token；不保存原始令牌。以下均为正式HTTP实机调用，不冒充SDK或UI证据：

- GET /v1/agent-apps/catalog?targetHostId=host → 200；对应项为{"id":"claude-desktop","name":"Claude Desktop","cliCommand":null,"runtimeKinds":["desktop_app"]}。
- GET /v1/agent-apps/installations?targetHostId=host&appId=claude-desktop → 200；{"installations":[],"desktop":{"supported":true,"appBundles":[],"configDirectories":[]}}。
- GET /v1/agent-apps/diagnostics?targetHostId=host&appId=claude-desktop → 200，同上；未将空CLI实例列表代替桌面观察。
- POST /v1/agent-apps/actions/preview?targetHostId=host，body={"appId":"claude-desktop","action":"install","target":{"kind":"environment","environment":{"kind":"macos"}}} → 422空正文；action改update或uninstall各为422空正文。三项都有管理员动作授权，不是未授权拒绝。
- POST /v1/agent-apps/launch?targetHostId=host，body={"appId":"claude-desktop","installationId":"absent","cwd":"<isolated-home>","args":[]} → 422空正文。该证据只证明无实例时拒绝；源码apps/launch/request.rs::validate_launch_request另明确拒绝Desktop终端启动，不能把本次响应单独说成已执行该分支。

来源边界：apps/catalog.rs::capability明确Desktop不支持CLI生命周期/终端；apps/discovery/read.rs独立返回desktop观察，当前不提供桌面版本或runnable断言。上游与任务08只迁移桌面探测，不以CLI白名单证明桌面安装。Desktop无自动生命周期操作，故未制造取消任务；UI限制展示/错误toast仍未验证。

### 外部阻塞及已尝试路径

CUA getState明确返回：Native apps: The Mac is locked and automatic unlock could not unlock it. Ask the user to unlock the Mac manually before continuing. 原生app列表为空，无法检查本次Iced窗口、截图、连接入口、断线重读或客户端安装指引。未使用个人窗口或伪造截图替代。

官方https://claude.com/download可读取，当前macOS链接为https://claude.ai/api/desktop/darwin/universal/dmg/latest/redirect。web打开下载端点超时；curl -IL --max-time 30同地址退出0但HTTP403、cf-mitigated: challenge；退出0仅证明传输完成，不证明下载成功。搜索缓存中另有旧storage链接，当前官网已不同，未当成当前版本来源，也未编造旧版URL或绕过挑战。

仍缺：官方允许渠道的隔离真实安装及安装后发现、包版本/实际运行、SDK与Iced一致性及截图、适用失败/断线/错误toast。没有专用测试账号，未复制个人身份；Desktop终端身份启动按源码明确不支持，不把缺账号当不支持理由。其他平台未做实机验收。

最少恢复条件：用户手动解锁Mac，恢复原生UI可控状态；若官方下载仍挑战，需在官方浏览器流程由用户完成挑战后继续隔离安装。此为外部实机条件阻塞，不是既有无关测试失败，不能改为已完成，待回归。

### Cargo与清理

两个顺序阶段均按原子mkdir动态领取slot-2，owner记录任务/会话及控制进程PID；CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，CARGO_BUILD_BUILD_DIR同一绝对路径，覆盖.cargo/config.toml的build.build-dir=target。没有--target-dir绕过。

- cargo build -p vw-cli --bin shiju：两轮均退出0（0.78秒、0.74秒）；仅既有ld __eh_frame提示。
- 第一轮cargo run -p vw-desktop --bin shi-ju因隔离HOME触发新的rustup/Cargo缓存与索引下载，主动终止返回-15，未算启动成功。第二轮显式保留原RUSTUP_HOME及CARGO_HOME，应用HOME/配置仍隔离；同命令默认features构建0.41秒，日志出现Running target/rust-slot-2/debug/shi-ju。随后主动SIGTERM退出-15（shell等价143），不算业务验收通过。
- 网关实际命令：槽位产物debug/shiju gateway --host 127.0.0.1 --port 19447；两轮均主动SIGTERM退出-15。
- 有效自动测试数量0；本任务人工场景验收，未改Rust，无需Clippy；编译及0测试均不代替行为验收。

本次桌面、网关及构建进程均已结束，进程检查无本任务残留；临时授权策略/令牌与隔离目录清理，未创建Claude安装。两轮本人锁均释放，slot-2缓存保留，他人锁不动。恢复检查后按追加调度要求将本记录持久化；任务仍阻塞，保留编号任务文件。

## 恢复路径复核

2026-11-06再次执行CUA getState，仍返回apps=[]及确切错误：

> Native apps: Error: The Mac is locked and automatic unlock could not unlock it. Ask the user to unlock the Mac manually before continuing.

未请求或输入解锁凭据。浏览器扩展仍可列出标签，但不存在本任务Iced原生窗口控制目标；个人网页标签不能代替本任务窗口或授予个人凭据使用权。没有获得可归属本任务的截图，故无截图附件，不能将锁屏截图或测试夹具写成Desktop安装证据。

检查了`crates/vw-desktop/src/app/components/system_settings_apps/catalog_tests.rs`：当前包含目录状态、Desktop观察及SDK响应测试，Desktop应用包示例为测试数据；没有本任务官方应用包安装结果。仓库其他组件有tiny-skia离屏截图测试，但它们不能替代本应用安装后发现、网关连接操作、客户端安装指引和原生窗口的实机场景。不新增镜像测试或把其他组件离屏产物当成本次验收截图。

### 当前官方渠道

- [官方下载页](https://claude.com/download)：macOS按钮指向DMG redirect端点；此前DMG检查为403/challenge。
- [官方macOS部署说明](https://support.claude.com/en/articles/12611117-deploy-claude-desktop-for-macos)：明确提供Universal PKG，适配Intel及Apple Silicon；同时说明DMG为拖放安装格式。不是把MDM系统安装当作隔离安装授权。
- 该文档实际PKG按钮指向`https://claude.ai/api/desktop/darwin/universal/pkg/latest/redirect`。web工具返回restricted URL；实际`curl -sSIL --max-time 25`该精确地址得到HTTP/2 403和`cf-mitigated: challenge`。命令经sed截取前12行的shell退出0，只表明输出检查完成，不证明下载成功。
- [官方安装说明](https://support.claude.com/en/articles/10065433-install-claude-desktop)：macOS通过官网下载并打开文件，再从Applications启动及登录。未发现可以替代原生安装验收的官方离屏入口；不套用Linux apt或Claude Code CLI安装命令，不采用第三方镜像、猜测URL、绕过挑战或修改安全策略。

恢复所需外部操作：用户手动解锁Mac使原生UI会话可用；若官方DMG/PKG下载仍挑战，由用户在官方下载流程处理挑战。随后再在隔离安装范围完成实际安装/发现和Iced证据。当前任务是验收任务，没有“实现已完成但只缺无关回归”的事实，仍标已阻塞，不能以文档完成冒充整项完成。

本恢复阶段仅文档与只读检查，无Cargo命令、无新槽位；前阶段slot-2和绝对CARGO_TARGET_DIR记录如上。无Rust改动，不执行Clippy或新增测试；累计有效自动测试仍0。持久化本文及脱敏API记录，提交排除其他改动。

## 解锁后的实机恢复结果（2026-11-06，约13:17–13:31）

本节为最新结论，前文锁屏/403为历史证据，不再作为当前阻塞。用户确认解锁后CUA再次列出原生应用；没有登录、解锁或读取个人凭据。

### 官方下载、版本与真实启动

从Chrome新建本任务标签访问https://claude.com/download，点击实际macOS按钮；官方重定向解析到：

`https://downloads.claude.ai/releases/darwin/universal/4.23118.0/Claude-8175c354eda704cea00fdeddfa69cea329dd321a.dmg`

Chrome显示ERR_BLOCKED_BY_CLIENT，没有禁用扩展或更改安全设置。独立curl对已观察到的精确官方产物URL返回HTTP200；`curl -fL --max-time 180 -o <isolated-root>/Claude.dmg <上述URL>`退出0，下载390297994字节，约76秒。未猜测版本或使用第三方镜像。

- DMG SHA256：`0e3d86a8b7ac6d7e6a3de011134db1717716fa6ecbe323f5ea4a5356fba62810`。
- `hdiutil attach -readonly -nobrowse -mountpoint <isolated-root>/mount <isolated-root>/Claude.dmg`退出0，全部CRC验证通过。工具提示旧参数已弃用，未隐藏或当作失败。
- `codesign --verify --deep --strict <mount>/Claude.app`退出0；`codesign -dv --verbose=2`退出0，Developer ID Application为Anthropic PBC，TeamIdentifier Q6L2SF6YDW，Apple Root CA，stapled notarization ticket。
- PlistBuddy读取CFBundleShortVersionString退出0：4.23118.0；Mach-O universal含arm64/x86_64。本次实机仅arm64。
- `ditto <mount>/Claude.app <isolated-home>/Applications/Claude.app`退出0，准备真实官方应用包；非生成目录夹具。再直接启动该包Contents/MacOS/Claude，HOME/XDG配置指向隔离目录，显式`--user-data-dir=<isolated-root>/claude-user-data`。CUA选中精确测试包路径，真实欢迎窗口显示“Claude for Mac / 开始使用”。未继续登录或同意额外条款；不能据此证明账号服务功能或终端身份启动。
- 日志提示当前客户端为最新版，但这不算升级证据。本领域源码明确不支持Desktop自动安装、升级、卸载及终端启动，不将CLI配方外推；供应商/官方账号终端启动、可取消生命周期操作不适用于该声明边界。外部手动复制/清理不冒充网关自动生命周期成功。

官方欢迎界面见`claude-desktop-official-start.jpg`。

### 正式SDK与Iced真实观察

专用网关仍为127.0.0.1:19447，临时home/Applications/prefix及独立租户p/a、管理员令牌不变。通过UI客户端网关入口设置地址/端口/平台/公司，再从应用列表选择host并输入本任务随机应用令牌；令牌仅进入本任务内存，不截取令牌输入画面。

按规定先执行`cargo run -p vw-desktop --bin shi-ju`默认features，构建0.28秒并出现Running。未打包可执行文件不能被CUA直接当app定位；将同次槽位二进制原样复制为唯一标识`com.rsung.shiju.task47-acceptance`的临时Task47Iced.app，隔离HOME启动，CUA成功获取专用窗口。未修改源码或借用个人ShiJu窗口。窗口日志虽有sandbox_extension_issue_file_to_process提示，本次窗口确实可操作，不再将该日志单独当阻塞。

使用正式`vw_gateway_client::GatewayClient`的`agent_apps_catalog_authorized`和`agent_apps_installations_authorized`读取同一网关。临时Rust驱动通过当前槽位已编译rlib链接，没有创建Cargo项目或旁路API；SDK保留授权头和租户上下文。最终链接命令：

`rustc --edition=2024 <isolated-root>/sdk.rs -L dependency=target/rust-slot-2/debug/deps --extern vw_gateway_client=target/rust-slot-2/debug/deps/libvw_gateway_client-1a94bf756d53cb1d.rlib --extern tokio=target/rust-slot-2/debug/deps/libtokio-f993a923b732abac.rlib -o target/rust-slot-2/task47-sdk`

退出0；执行`target/rust-slot-2/task47-sdk`，四次成功读取均退出0，目录对应DesktopApp且cli_command=None。输出保存在`claude-desktop-sdk.txt`，含以下真实路径变化：

1. 首次真实官方包：发现隔离home/Applications/Claude.app。Iced显示“桌面应用：已发现，运行未验证”及完全相同路径；正确保留发现/运行边界。
2. ditto增加第二个真实包至隔离Applications/Claude.app：SDK同时发现两条路径。
3. 终止本任务Claude进程，将home实例移出搜索根：SDK仅发现隔离Applications/Claude.app；Iced刷新显示剩余路径，没有清空其他实例。
4. 清理两份本任务包，SDK返回desktop.app_bundles=[]；重启测试网关并在Iced点重试，显示“Claude Desktop / 桌面应用：未安装”，不保留旧路径。

初次驱动直接链接多个缓存中最新vw_api_types，rustc退出1提示crate身份不同；改用SDK公开重导出的类型。随后最新tokio缓存与SDK的tokio身份不同，驱动退出101/no reactor；按SDK Cargo fingerprint精确匹配tokio-f993a923b732abac后链接与真实调用通过。未把这些驱动准备失败当作产品失败或测试红灯。SDK断线读取返回Transport，临时驱动unwrap产生退出101；属于人工断线观察，不计自动测试。

### 断线、错误toast与恢复

仅终止本任务19447监听网关，Iced刷新显示红色“加载失败，请重试”、按钮变为“重试”，即时截图右下角同时包含“操作失败 / 加载失败，请重试”的错误toast。随后按相同隔离配置重启网关，UI点击重试成功，显示清理后的未安装状态。没有用空列表替代网络错误。

截图（均为真实本任务原生窗口；不含授权令牌）：

- `claude-desktop-iced-start.jpg`：本任务隔离初始项目窗口。
- `claude-desktop-iced-discovered.jpg`：SDK对应的真实官方包发现。
- `claude-desktop-iced-remaining.jpg`：移除指定测试实例后保留另一实例。
- `claude-desktop-iced-offline.jpg`：断线错误与重试入口。
- `claude-desktop-iced-offline-retry.jpg`：即时错误toast。
- `claude-desktop-iced-cleaned.jpg`：重连后的未安装状态。

### 验证边界与待回归

状态改为**已完成，待回归**，保留任务47。已恢复并验证官方包来源/签名/版本/欢迎界面、正式SDK及Iced对应、多实例发现/指定清理/重读、断线toast及重试恢复；不再需要用户解锁。仍不能宣称全部原任务场景通过：

- Iced Desktop行当前只有发现状态/路径，没有观察到客户端外部手动安装指引/安全URL入口；官网安全URL及手动安装流程已独立核实，不能冒充该客户端入口已验收。后续应复核该交互要求与既有只读Desktop能力的落点，不在本任务未经行为失败证据新增功能。
- 尚未对真实Desktop包进行独立破损入口/不可运行故障操作。本实现只做包目录观察，不提供Desktop版本/运行/来源诊断；本次欢迎界面是额外人工运行证据，不能改写API能力。
- 没有执行账号登录、终端/cwd/身份材料验证；源码声明Desktop不属于终端目标，此项不外推为CLI或其他平台结果。

### 本阶段命令与资源清理

再次动态原子领取slot-2；CARGO_TARGET_DIR和CARGO_BUILD_BUILD_DIR均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，所有Cargo及临时SDK产物均在此槽位，锁覆盖整个产物使用阶段。

- `cargo build -p vw-cli --bin shiju`：退出0，0.93秒。
- `cargo build -p vw-desktop --bin shi-ju`：退出0，0.46秒。
- `cargo run -p vw-desktop --bin shi-ju`：构建成功并启动，主动终止返回-15；非自然退出0。网关两次及专用bundle均主动SIGTERM结束。
- 只存在既有ld __eh_frame提示；无仓库Rust修改、不运行Clippy。有效自动测试数量仍0；SDK实机读取与UI人工场景单列，不包装为单元测试数量。
- 本任务Claude、Iced、两轮网关及相关子进程终止；两份测试应用包已清理，卸载DMG挂载并删除隔离数据/策略/令牌。个人已有ShiJu/网关/CLI保持不动；既定/Applications/Claude.app与个人home/Applications/Claude.app前后均不存在。
- 本人slot-2锁释放，保留编译缓存；不碰slot-1/9他人锁。plan.md保持只读，提交仅本应用持久证据。
