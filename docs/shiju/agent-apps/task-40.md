# 任务40：应用列表桌面入口

状态：已完成。2026-11-06，macOS。

## 依赖与实现范围

读取 do-task-plans、git-commit、仓库 RustCodeGraph 技能、只读 plan.md、来源与接口.md、任务33/39证据、实际SDK、DTO、授权处理器、设置导航、图标状态、统一设置样式与自定义模型弹窗。RustCodeGraph status/explore/node核对已有入口，图未覆盖的新文件直接读取。批次39实际SDK已交付；未将母计划旧标签当作符号缺失。无并行批次，plan.md保持只读。

系统设置增加“应用列表”导航及搜索词，独立 AppsCatalogState/AppsCatalogMessage，不复用图标库状态，不移除ACP注册或原页能力。只通过 GatewayClient 获取目录与逐应用安装观察，生产目录提供14项；不在桌面维护第二套应用目录、发现路径、安装配方或执行命令。单项读取失败保留其卡片和其他应用；总读取失败显示安全错误与重试。主机、公司、网关、项目和请求代次隔离，切换上下文要求重新输入管理令牌，过期响应不覆盖当前数据。

明确展示未安装、可运行、损坏、文件缺失、权限不足、运行失败、未验证、不支持与远端离线/服务不可用。503本身不能区分离线与服务故障，故不伪造更细结论。NativeCli探测不证明ACP、审查运行时或编辑器可用；ACP引导原页管理。Desktop presence仅表示发现，运行保持未验证。实例逐条显示来源、环境、版本和目标主机路径，中文及混合路径全部使用默认字体。

必要相邻底座：任务33服务端要求独立 x-agent-apps-token，但任务37 SDK没有该读取凭证入口。新增两个明确的授权只读SDK方法，原有无凭证API签名不变；令牌HeaderValue标为敏感，采用禁用重定向/重试的原生传输，不向其他业务端点附加该令牌。Wasm明确拒绝缺少安全传输能力的授权读取。主机ID和令牌通过统一设置样式弹窗输入，仅内存保留，Debug不包含令牌，服务器继续验证主体、租户、主机与权限；没有本地自动认领、默认主机或客户端授权推断。

UI复用settings_section_card/content、settings_modal_card/overlay/close_button、settings_text_input_style、统一padding/text size及主次按钮，错误反馈使用toast。列表复用设置页4px滚动条，内容两侧额外7px；连接弹窗是短表单，不引入长表单滚动布局。未增加安装、卸载、设置或终端交互。

## Cargo与验证

动态原子mkdir依次尝试1–10，实际领取槽位2：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

检查`.cargo/config.toml`中build.build-dir=target，通过后者覆盖，所有Cargo、测试与UI进程继承。最终仅释放本人owner/锁，保留缓存。

| 确切命令 | 退出码 | 有效数量/结果 |
| --- | --- | --- |
| `cargo fmt --all`（红灯前与各实现阶段后） | 0 | 无无关格式化差异 |
| `cargo test -p vw-desktop --bin shi-ju cc_apps_desktop_catalog_state -- --nocapture` | 0 | 0测试，31过滤；当前app测试位于lib，不能作为行为证据 |
| `cargo test -p vw-desktop --lib cc_apps_desktop_catalog_state -- --nocapture`（红灯） | 101 | 2项均失败：未安装与离线被骨架错误标为未验证 |
| 同上（首次实现编译） | 101 | 新增SystemTab::Apps未覆盖错误分支；已补全，无无关错误 |
| 同上（绿灯） | 0 | 5通过，8558过滤 |
| `cargo test -p vw-gateway-client --lib cc_apps_desktop_catalog_state -- --nocapture` | 0 | 1通过，290过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings`（实现后） | 0 | 全仓通过，27.18秒 |

桌面行为测试覆盖真实回环HTTP中14项目录、逐应用查询与授权头、Codex单项503而其他项正常；空列表、桌面presence、501/503、损坏/可运行/非零退出/版本未验证、ACP独立状态、加载失败重试及请求代次隔离。SDK测试覆盖专用授权头、307明确错误以及旧无凭证入口不带专用令牌。没有以工具缺失、编译失败或零测试充当预期红灯。

## 实际渲染与最终证据

- `APPS_CATALOG_WINDOW_OUTPUT="$PWD/docs/shiju/agent-apps/task-40-ui" cargo test -p vw-desktop --test apps-catalog-window`：首次101，夹具缺少根desktop_update_helper声明；按已有plugin-compat-window入口补齐后退出0，8项原生窗口场景通过。
- 夹具直接使用生产catalog view与连接overlay，通过真实Iced事件循环、窗口帧订阅、字形整形与窗口截图；不是只构建Element。600×650逻辑尺寸窄窗，Light/Dark各覆盖中文应用及混合路径、损坏/ACP未验证、离线错误/重试、加载禁用刷新、连接弹窗及空令牌禁用提交。8张截图见[task-40-ui](task-40-ui/)。人工查看明暗目录、连接及错误截图，文本可读、内容无覆盖，未出现整形panic。
- `cargo run -p vw-desktop --bin shi-ju`：正常构建0.28秒并进入主程序，未记录panic或event loop failed；验收结束向本次进程发送SIGTERM，shell退出143（主动结束，不当成程序自行退出0）。CUA不能定位未打包可执行文件，未操作用户已安装的ShiJu窗口；任务页面的实际渲染证据由上述原生夹具提供。没有宣称通过标准应用的完整手动点击导航。
- 最终 `cargo test -p vw-desktop --lib cc_apps_desktop_catalog_state -- --nocapture`：退出0，6通过、8558过滤。
- 最终 `cargo test -p vw-gateway-client --lib cc_apps_desktop_catalog_state -- --nocapture`：退出0，1通过、290过滤。
- 最终 `cargo clippy --all-targets --all-features -- -D warnings`：退出0，10.82秒，无新增诊断。
- `git diff --check`：退出0。差异仅限本任务的目录、状态、消息、导航、最小授权读取SDK、独立测试、显式窗口目标与本文/截图，无无关格式化改动。

测试链接器保留既有macOS `__eh_frame section too large` 性能提示；全仓Clippy通过。原生窗口日志出现系统IMKCFRunLoopWakeUpReliable mach-port提示，无panic；没有屏蔽字体错误。未运行无过滤全量cargo test、fmt检查、wasm检查或Makefile。没有实际安装、卸载或启动外部应用，没有读取个人官方账号或扩大网关权限。

所有Cargo及本次UI进程结束后，槽位2本人锁已释放，缓存保留。下一步批次41安装操作桌面交互；生产部署仍需管理员真实授权主机和独立应用令牌。

提交前审查补充：修复加载中切换网关后旧loading阻止重试的问题，先校验上下文再处理重复加载；上下文变化清空令牌/卡片、失效请求代次并打开连接弹窗。新增第6项回归测试验证旧响应拒绝及新上下文可重新加载；网关配置读取失败也清除pending状态。最终聚焦6项通过。
