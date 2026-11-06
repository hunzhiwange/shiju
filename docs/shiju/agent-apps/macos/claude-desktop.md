# Claude Desktop macOS 实机验收（已阻塞）

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
