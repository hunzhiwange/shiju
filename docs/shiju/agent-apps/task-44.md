# 任务44：终端启动桌面表单

状态：已完成。2026-11-06，macOS。前次提交48d7a1195的身份契约阻塞已按新增授权补齐；不宣称14个厂商CLI、个人真实登录或其他平台实机验收完成。

## 产物与边界

已读do-task-plans、git-commit、仓库RustCodeGraph技能及status/explore、只读plan.md/来源与接口、任务31/32/39/43证据、真实身份所有者和终端适配器、统一设置组件与custom_model_modal。前序真实依赖存在；用户随后授权补齐身份列表与托管终端生命周期。本任务未修改plan.md或来源快照，未与其他批次并行。

- 每个已观察实例新增启动表单。明确显示所选网关执行主机、应用、实例及身份；SDK加载真实脱敏列表，必须明确选择供应商、官方账号或主机现有CLI配置。没有记录、材料缺失或未授权时该类列表为空并禁用；读取失败不回退其他账号。
- 供应商列表来自唯一AppProviderStore；官方列表来自唯一AuthAccountStore及CredentialFileStore，不复制账号库、不扫描个人HOME/钥匙串、不创建导入API。供应商沿既有Claude/Codex/Gemini/AGY投影；官方沿既有Claude/Codex材料。其余未交付/不安全投影维持空列表或明确拒绝，不将其宣称为完整厂商能力。
- cwd在表单默认空，由执行端按授权主机根目录和重新发现的实例验证；客户端不canonicalize、不探测远端路径。参数逐项添加、编辑、移除并保持精确argv边界；不解释shell。skipPermissions始终false，服务端也拒绝跳过权限。
- 主机、令牌、网关/租户/项目上下文与目录代次变化使旧响应失效；身份读取不自动选择账号。重复提交、处理中编辑被拒绝，成功须匹配同一实例。网关配置失效会清除草稿，避免残留不可关闭的启动状态。动态材料和令牌不进入App/Message的Debug，UI使用固定类别错误和toast，不展示原始响应正文。

## 授权与接口

全部挂在既有认证/租户中间件和独立x-agent-apps-token之下，使用管理员授权的执行主机；没有桌面本地执行兜底，也没有猜测执行主机RPC。既有Remote binding继续明确拒绝，本任务不创建任意远程命令入口。

| 接口 | SDK/行为 |
| --- | --- |
| GET /v1/agent-apps/launch-identities?targetHostId=…&appId=… | agent_apps_launch_identities，仅kind/id/label |
| POST /v1/agent-apps/launch?targetHostId=… | agent_apps_launch_identity_authorized，只有既有LaunchRequest和providerId或accountId；互斥，密钥不在请求中 |
| 同上，显式选择主机现有配置 | agent_apps_launch_authorized，不隐式采用该选项 |
| GET /v1/agent-apps/launch-sessions/{id}?targetHostId=… | agent_apps_launch_session，只读phase，不重放POST |

旧launch授权不自动扩为凭据使用权限。Grant新增默认空的identities白名单，按租户、主机、AppId、kind和ID匹配；未授权身份不出现在列表，伪造ID或类别在材料化/终端副作用之前403。管理员策略片段示例：

```json
{"launch":true,"identities":[{"appId":"claude","kind":"provider","id":"provider-id"},{"appId":"codex","kind":"official_account","id":"account-id"}]}
```

官方账号还要求可信Host配置credential_root，指向已由正式所有者维护的私有目录；未配置时官方列表为空。路径只来自管理员策略，不接收客户端路径，不认领既有个人登录。原有策略文件所有者、0700/0600边界、O_NOFOLLOW及默认拒绝检查保持有效。会话状态额外绑定principal，其他主体即便获准同一主机也不能读取他人的会话。

## 执行端生命周期

既有POSIX终端适配器只启动无凭据的attach脚本。执行端创建随机0700目录与私有Unix socket，连接一次后移除监听入口；终端脚本只包含连接路径，恢复TTY状态，不包含profile、密钥或账号材料。网关通过现有libc创建0600 PTY，真实CLI获得控制终端和独立进程组，固定初始80×24及TERM=xterm-256color。当前传输不投影动态终端窗口尺寸，不宣称其他系统终端实机通过。

ProviderLaunch/AuthAccountLaunch及CredentialLease保留到真实CLI回收、IO线程结束、Codex轮换回写和临时文件清理完成。关闭终端使连接断开，执行端终止并回收整个CLI进程组；正常退出与PTY EOF分开处理，避免将正常退出误判为连接失败。遗留子进程在PID仍由未回收子进程保留时终止，避免PID复用；异常解栈也有进程组回收守卫。所有材料均在执行端私有资源或子进程环境，不经过终端启动argv、HTTP响应或应用日志。

POST只在真实CLI已spawn到PTY后确认sessionId，UI显示运行中，不能据此宣称CLI完成。运行记录有固定容量，读取只返回Starting/Running/Completed/Failed；只有真实退出、回写和清理成功才标Completed，非零退出/断开/回写失败均为Failed。状态是网关进程内的有界记录，重启后不伪造持久会话状态；状态不可读不重新启动CLI。

必要底层修复：账号回归暴露CLOEXEC锁描述符在并发fork至exec之间延长flock租约的竞态。新增真实fork/exec握手红灯，CredentialLease在所有者Drop时显式LOCK_UN并重试EINTR；文件仍保持原inode、不unlink。CLI运行时的租约仍由正式所有者持有，不因增加终端传输提前释放。

## Cargo槽位

所有阶段均从仓库根原子mkdir依次动态领取，实际均slot-2，记录任务/会话/控制进程PID。所有Cargo、脚本和使用产物的子进程继承：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖仓库build.build-dir=target，避免中间产物逃离槽位。阶段EXIT/INT/TERM清理本人owner与锁，保留缓存，不删除其他持有者锁、进程或缓存。

## 验证证据

| 确切命令 | 退出码 | 有效数量/结果 |
| --- | --- | --- |
| cargo fmt --all（Rust修改后） | 0 | 审查无无关格式化 |
| cargo test -p vw-desktop --lib cc_apps_desktop_launch_state -- --nocapture（最初红灯） | 101 | 3有效失败，finish骨架不满足成功/失败/实例匹配 |
| cargo test -p vw-agent-apps --lib cc_apps_terminal_session -- --nocapture（红灯） | 101 | 2有效失败，缺少真实终端传输 |
| cargo test -p vw-agent-apps --lib cc_apps_credential_lease -- --nocapture（红灯） | 101 | 1有效失败，父所有者释放后仍因fork描述符复制而被占用 |
| cargo test -p vw-agent-apps --lib cc_apps_credential_lease -- --nocapture（绿灯） | 0 | 1通过，165过滤 |
| cargo test -p vw-agent-apps --lib cc_apps_terminal_session -- --nocapture（最终） | 0 | 3通过，163过滤；其中正常退出内部重复10次，不重复计顶层数量 |
| cargo test -p vw-agent --lib cc_apps_launch_identity_list -- --nocapture | 0 | 3通过，11464过滤，含真实SDK/HTTP/SQLite/PTY |
| cargo test -p vw-agent --lib cc_apps_launch_native_terminal -- --nocapture | 0 | 1通过，11466过滤，真实macOS Terminal，含供应商及官方账号两个子场景 |
| cargo test -p vw-desktop --lib cc_apps_desktop_launch_state -- --nocapture | 0 | 7通过，8578过滤 |
| cargo test -p vw-gateway-client --lib cc_apps_settings_launch_sdk_roundtrip -- --nocapture | 0 | 7通过，287过滤 |
| cargo test -p vw-agent --lib cc_apps_launch_api_target_scope -- --nocapture | 0 | 1通过，11466过滤 |
| cargo test -p vw-agent --lib cc_apps_preferences_api_scope -- --nocapture | 0 | 2通过，11465过滤 |
| cargo test -p vw-agent --lib cc_apps_account_launch_isolation -- --nocapture | 0 | 4通过，11463过滤 |
| cargo test -p vw-agent-apps --lib cc_apps_provider_launch_isolation -- --nocapture | 0 | 8通过，157过滤 |
| cargo test -p vw-agent-apps --lib cc_apps_account_launch_isolation -- --nocapture | 0 | 12通过，154过滤 |
| cargo test -p vw-agent-apps --lib cc_apps_launch_request_validation -- --nocapture | 0 | 7通过，158过滤，含Windows/WSL显式夹具 |
| cargo test -p vw-desktop --bin shi-ju cc_apps_desktop_launch_state -- --nocapture | 0 | 0测试、31过滤；沿用已确认lib入口，零测试不计验收 |
| APPS_LAUNCH_WINDOW_OUTPUT="$PWD/docs/shiju/agent-apps/task-44-ui" cargo test -p vw-desktop --test apps-launch-window | 0 | 10个原生中文窗口场景，无字形整形panic |
| cargo clippy --all-targets --all-features -- -D warnings | 0 | 最终全仓通过 |
| cargo run -p vw-desktop --bin shi-ju | 143 | 日志确认进入Running，12秒计时后主动SIGTERM，不冒充自然退出0 |
| git diff --check / git diff --cached --check | 0 | 仅本任务实际变更 |

最终共56项顶层行为/回归用例通过，另10个原生窗口场景。最初PTY实现编译曾修正macOS openpty可变指针签名，不将编译错误当作业务红灯；Clippy首次发现测试不必要clone，已修正并复验。账号回归最初11通过/1失败，随后由新增锁竞态红灯定位、底层修复，默认并发的12项最终全过，没有改为串行或弱化断言。

HTTP/SQL/真实子进程证据覆盖：缺记录空列表、跨公司/平台/AppId/主机隔离、旧launch授权不扩权、错误身份类别及缺记录在终端前拒绝、错误cwd/实例/权限参数与未授权无终端副作用；供应商实际环境、控制TTY、0700私有材料在运行中存在、账号并发租约拒绝、非零退出后轮换刷新、清理完成才发布最终状态；其他主体和租户不能查询会话。原有SDK丢失响应与重定向不重放用例保留通过。没有外网模型请求或个人凭据操作。

实际渲染使用生产launch::overlay，560×650逻辑窗口。明暗主题分别检查完成、错误、处理中禁用、无记录、已选中文身份与参数底部；标题/操作固定，内容单独滚动，4px滚动条与两侧7px留白。所有控件复用settings_*样式及统一主次按钮，中文/混合路径用默认字体。截图保存在task-44-ui/，人工检查明亮空列表、暗黑完成、错误、禁用和中文参数，无panic。早期参数编辑器预填内容不可见的试验已移除，最终为逐项统一文本控件，不将旧试验截图当作最终证据。

既有macOS链接器__eh_frame大小提示、系统IMKCFRunLoopWakeUpReliable消息单列，不改全仓链接配置掩盖。未运行无过滤cargo test、fmt检查、wasm检查或Makefile。下一步任务45及各厂商CLI实机批次；母任务3/4/6整体状态不在本任务中宣告完成。


最终收口：全部本次Cargo、CLI/attach客户端、原生验收窗口及标准桌面进程均已结束；本人slot-2锁已释放，缓存保留。最终桌面7项与10场景窗口复验、全仓Clippy均退出0；真实Terminal在新增身份白名单及租约修复后再次通过。编号任务状态/删除与其他既有编号任务删除均不纳入代码提交，plan.md保持只读。
