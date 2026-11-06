# 任务44：终端启动桌面表单

状态：已阻塞（供应商/账号列表及托管终端会话契约）；已完成部分保留验证证据。

2026-11-06，macOS。当前现有CLI配置启动与明确拒绝边界已实现；不宣称供应商或官方账号启动完整验收，也不宣称Windows/WSL实机通过。

## 依赖与范围

已读do-task-plans、git-commit、仓库RustCodeGraph技能、status/explore、只读plan.md及来源与接口、任务31/32/39/43证据、实际SDK/启动API/消息状态与相邻测试、统一设置组件和custom_model_modal。前序真实目录、实例、主机授权、设置与SDK均存在；未因旧计划状态标签判定缺失。未修改plan.md、来源快照或其他任务实现。

列表每个已观察实例新增终端启动入口，表单明确显示执行主机、AppId、安装实例及身份语义；当前只能显式确认使用所选主机现有CLI配置，可能使用该主机既有登录身份。默认空cwd，要求输入所选主机目录，由执行端验证，客户端不canonicalize或探测远端路径。权限检查始终保留，不提供skipPermissions。参数原样交给SDK，不解释shell、不创建桌面临时文件、不打开本机终端。后端负责输入上限、目录/实例/参数校验和执行材料生命周期。

最小相邻修复：此前启动SDK未发送独立x-agent-apps-token，而真实API要求该授权；新增agent_apps_launch_authorized，不替换网关认证、不重试、不跟随重定向。主机、应用管理令牌、网关/租户/项目上下文和目录代次变化使旧启动状态失效。重复提交和处理中编辑被拒绝，成功须返回同一安装实例；错误保留选择，禁止自动重试，未确认响应明确提示先检查执行主机。UI只显示固定类别错误，不展示响应正文、凭据或参数；状态及消息的Debug不输出动态材料。反馈使用toast。

## 身份能力缺口（未宣告完整验收）

真实供应商SDK只有app_provider_save/get（按已知ID），没有可用列表；官方账号所有者是主机内部AuthAccountStore，没有HTTP列表。生产POST /v1/agent-apps/launch在任何providerId/accountId请求上均返回501；任务31/32运行对象的私有资源/凭据租约生命周期没有交接到OS终端。这不是个人账号测试缺失，而是当前API能力缺口。没有复制账号库、猜测列表端点、读取个人凭据、伪造可用账号或在桌面绕过权限/执行边界。

表单分别禁用供应商和官方账号入口，明确说明列表与安全终端交接尚未就绪。当前通过证据只覆盖已交付的现有CLI配置启动和这些禁用/拒绝边界；供应商/账号选择、材料化后终端会话清理与凭据回写必须待正式所有者补齐契约后回归，不标记母任务3/4/6完整完成。

## Cargo槽位与验证

动态原子mkdir依次尝试，实际领取slot-2，记录任务/会话/PID；所有Cargo和窗口子进程继承：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖仓库build.build-dir=target，防止中间产物逃离槽位。阶段退出/中断使用trap清理本人owner和锁，保留缓存，不操作其他持有者。

| 确切命令 | 退出码 | 有效数量/证据 |
| --- | --- | --- |
| cargo fmt --all（每次Rust修改后） | 0 | 无无关格式化差异 |
| cargo test -p vw-desktop --lib cc_apps_desktop_launch_state -- --nocapture（红灯） | 101 | 3失败；最小可编译finish返回false，目标成功/失败/错误实例断言失败 |
| 同上（首轮绿灯） | 0 | 3通过 |
| 同上（最终） | 0 | 4通过、8578过滤 |
| cargo test -p vw-desktop --bin shi-ju cc_apps_desktop_launch_state -- --nocapture | 0 | 0测试、31过滤，不计行为证据；沿用前序已确认lib入口 |
| cargo test -p vw-gateway-client --lib cc_apps_settings_launch_sdk_roundtrip -- --nocapture | 0 | 6通过、287过滤 |

四项桌面行为验证成功确认、重复提交拒绝、Windows远端中文cwd不作本地检查、默认不跳过权限、错误保留选择、过期代次/错误主机/其他实例响应拒绝、上下文失效和动态参数Debug脱敏。六项SDK测试使用真实回环HTTP/TCP，验证独立应用令牌、租户、特殊字符主机编码、精确请求体、授权失败及响应丢失/重定向均不重放；其余身份字段透传及501拒绝沿用既有SDK覆盖，不将其当作身份启动成功。

| 确切命令 | 退出码 | 有效数量/证据 |
| --- | --- | --- |
| cargo test -p vw-agent --lib cc_apps_launch_api_target_scope -- --nocapture | 0 | 1通过、11462过滤 |
| APPS_LAUNCH_WINDOW_OUTPUT="$PWD/docs/shiju/agent-apps/task-44-ui" cargo test -p vw-desktop --test apps-launch-window | 0（最终逐项参数版本） | 10原生中文场景通过，无字形整形panic |
| cargo clippy --all-targets --all-features -- -D warnings | 0（首轮及最终复验） | 全仓通过，无Clippy诊断 |
| cargo run -p vw-desktop --bin shi-ju | 143 | 已启动标准桌面，25秒计时后主动SIGTERM；日志确认已进入Running，无panic |
| git diff --check | 0 | 无空白错误 |

后端真实临时目录、数据库与授权策略测试，以执行端terminal边界哨兵验证未授权、错误cwd/实例、权限跳过参数、providerId/accountId均在副作用前拒绝；终端失败也不会产生成功哨兵，成功返回精确安装实例并写入预期argv。

实际渲染：生产launch::overlay在560×650逻辑窗口，明暗两主题分别覆盖成功、错误、处理中禁用、就绪、滚动底部，共10截图。标题/底部操作固定，内容单独滚动、4px滚动条与两侧7px留白；中文使用默认字体，未用MONOSPACE。截图审查发现错误在底部不可见，已移到顶部。尝试统一text_editor时预填文本未在截图显示；强制布局仍不能证明显示，未把该轮当成参数可见性验收。最终采用逐项文本控件（添加/编辑/移除，每项严格对应一个argv），沿用统一settings_text_input_style及主次按钮样式。最终截图须显示两项中文参数才计验收，见task-44-ui。

既有macOS链接器提示__eh_frame过大、系统IMKCFRunLoopWakeUpReliable消息单列；不将它们冒充Rust/Clippy诊断，也不修改全仓链接配置。未运行无过滤cargo test、fmt检查、wasm检查或Makefile。

补充调查：直接调用任务31/32的run只能证明受控子进程运行，不能交接到用户OS终端；现有launch_posix_terminal只消费LaunchMaterial，不能持有ProviderLaunch/AuthAccountLaunch的私有配置、CredentialLease及Codex轮换回写生命周期。把prepare结果转为普通终端命令会过早删除材料/释放租约或泄露秘密，不能作为等效实现。安全补齐需要由正式执行/账号所有者提供持续到终端CLI退出的托管会话契约，超出本桌面表单的最小接线范围。当前任务整体保持已阻塞，保留编号文件供该契约交付后继续，不删除或宣称完整完成；已可独立验证的表单与授权启动代码保留为可审查成果。


最终补充：Rust修改后cargo fmt --all退出0；最终cargo test -p vw-desktop --lib cc_apps_desktop_launch_state -- --nocapture退出0，4通过、8578过滤，含参数添加/编辑/移除和精确argv断言。SDK6项与后端1项未受该局部UI修改影响，合计11项行为/回归通过。最终10场景窗口退出0，人工检查light-error、light-fields、dark-disabled，确认顶部错误、两项中文参数、处理中禁用和固定底部操作正常显示，无字体panic；截图已覆盖保存为最终逐项参数版本。最终cargo clippy --all-targets --all-features -- -D warnings退出0。所有本次Cargo、窗口与标准桌面进程均已结束，本人slot-2锁已释放，缓存保留。git diff --check退出0，编号任务状态和其他既有编号文件删除排除提交。任务44整体保留已阻塞，下一步正式身份列表与托管终端交接契约交付后继续。
