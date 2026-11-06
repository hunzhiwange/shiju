# 本机应用默认初始化

桌面端连接本机网关时，打开「应用列表」会自动初始化、发现和加载当前用户的应用，无需输入主机 ID、令牌、JSON 或环境变量。再次打开会复用已有凭证。

应用列表显示「应用位置：本机」。使用程序生成的私有本机授权时，启动弹窗默认选择「主机现有配置」，并填写当前项目目录（未打开项目时使用用户目录），沿用用户已有登录；初始化不会导入官方账号或授权其他供应商身份。未安装应用沿用已有安装预检与确认流程，需要本机已有可用的 Node.js/npm。新安装统一写入当前构建配置目录下的 `apps`，不会因发现系统安装就获得系统目录的写权限。

程序在 `~/.shiju-harness/local-apps/<租户摘要>/` 保存授权策略和独立凭证；开发构建使用 `~/.shiju-harness-dev/`。目录权限 0700，文件 0600。策略只保存凭证摘要，并精确限制当前平台、公司和 `desktop-local` 主机，允许应用读取、设置保存、普通终端启动及受现有预检约束的生命周期操作。常用用户目录、Node 版本目录、系统命令目录和当前进程 PATH 是首次授权的发现范围，不执行登录 shell。

网关未设置 `SHIJU_AGENT_APPS_ACCESS_FILE` 时，逐请求读取当前租户的默认本机策略，因此初始化无需重启网关。显式管理员策略始终优先，桌面不会覆盖它。缺失、损坏或权限不安全的策略继续拒绝请求。没有 HTTP 自动登记或认领入口，远程网关不会在桌面本机创建授权。

高级部署仍可使用「手动连接（高级）」输入管理员提供的主机和令牌。当前本机适配器支持 macOS/Linux；Windows/WSL 尚不支持此自动授权路径。

## 验证记录

2026-11-06，macOS，动态领取共享槽位 3；`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。

- `cargo test -p vw-shared --lib local_apps -- --nocapture`：3 项通过，验证初始化复用、租户隔离、文件私有权限及不安全存储拒绝。
- `cargo test -p vw-agent --lib local_apps -- --nocapture`：2 项通过（另有子进程 1 项，不重复计数），包括真实注册路由在无环境变量情况下、网关启动后读取新建授权，以及越权拒绝。
- 已构建的网关测试二进制分别执行 `cc_apps_read_api_authorization`、`cc_apps_lifecycle_api`：7 项、5 项通过，覆盖读取与安装/升级/卸载、撤权和租户隔离。
- `cargo test -p vw-desktop --lib local_apps -- --nocapture`：1 项通过，验证旧连接或旧代次的凭证不会应用到当前连接。
- `cargo test -p vw-desktop --lib cc_apps_desktop_catalog_state -- --nocapture`：6 项通过。
- `APPS_CATALOG_WINDOW_OUTPUT="$PWD/target/local-apps-ui" cargo test -p vw-desktop --test apps-catalog-window`：8 个明暗主题原生中文渲染场景通过；检查 `light-catalog.png`、`dark-catalog.png`，无字形整形 panic、控件裁切或对比不足。

以上命令均退出 0。测试链接器仍有既有 `__eh_frame section too large` 提示，未改动链接选项。曾误用 `--bin shi-ju local_apps` 得到 0 项测试，未作为验收证据，随后改用上述 `--lib` 验证。

已为当前电脑的开发版及正式版本机网关各生成私有默认授权，核对凭证摘要、文件/目录权限，以及 Claude、Codex、npm、Node 的可发现性；记录不含凭证。当前已启动的旧网关仍需更新并重启后才会消费新的默认授权路径。

最终 `cargo clippy --all-targets --all-features -- -D warnings` 退出 0，39.92 秒，全仓通过；`git diff --check` 通过。验证结束后释放本任务的槽位锁，保留共享编译缓存。

## 应用列表与拖动排序（2026-11-06）

Rust 应用页采用紧凑卡片、品牌图标、状态标签与搜索；按已安装、待安装、待确认分组。网络读取失败或缺少桌面应用观察不会被当作未安装。安装路径、运行诊断及升级/卸载放在可展开的详情中，保留网关预检与确认边界；本机安装快捷入口使用已确认的本机平台，远程目标仍需在详情中明确选择环境。

左侧六点手柄默认隐藏，悬停到已安装行时显示，拖动中保持可见，预留位置不发生位移。已安装项可按住手柄拖到另一已安装项上，松开后通过现有用户偏好 API 保存顺序。保存成功前继续显示最后一次服务端确认的顺序；失败保留原顺序并提示。隐藏项和未安装项保持其原有排序位置。搜索、设置同步或管理弹窗期间不启动拖动；Esc、移出列表、切换设置页或窗口失焦会取消本次拖动。

拖动保存测试使用隔离子进程与临时本机 HTTP 服务，检验实际 SDK PATCH 的用户作用域、主机和凭证头，以及成功提交、拒绝回滚和取消不提交；仅替换外部网关边界，不访问个人应用或修改用户偏好。`apps_drag` 2 项、`apps_presentation` 2 项通过（子进程测试不重复计数）。原生窗口验收覆盖明暗主题各 8 个场景：列表、展开详情、拖动高亮、搜索、错误、加载、高级连接和空态，截图位于 `target/apps-design-ui/`。图标复用已有资源，Gemini/Hermes 图稿来自仓库保留的原 TS 资产。

本轮最终验证使用共享槽位 3（两项 Cargo 路径同上），全部退出 0：`cargo test -p vw-desktop --lib cc_apps_desktop -- --nocapture` 27 项通过，加上拖动/展示的 4 项共 31 项；原生 `apps-catalog-window` 16 场景通过；最终 `cargo clippy --all-targets --all-features -- -D warnings` 10.25 秒通过。视觉检查另修正主操作的固定宽度和禁用态：禁用时复用通用次级按钮样式，确认展开和拖动切换后文字仍完整绘制。初次全仓检查曾因另一项 ACP 并行改动失败，后续工作区更新后已恢复通过，本任务未修改相关 ACP 文件。


本机启动默认值只对当前租户、与私有本机凭证匹配的连接生效；远程主机、其他租户或不匹配的凭证不会借用桌面的用户目录或自动选择身份。身份列表仍须由网关成功读取，目录、实例与权限仍由网关验证。启动按钮统一依据状态显示禁用原因，并使用明确的禁用样式；结果不明时保留重新提交保护。启动聚焦测试 `cc_apps_desktop_launch_state` 9 项通过，包含本机默认值、跨租户/凭证拒绝、禁用原因和不重复启动。

本轮悬停与启动修复使用共享槽位 5：`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-5`。启动聚焦测试 9 项、应用页原生渲染 18 场景、启动弹窗原生渲染 12 场景均退出 0，含本机默认配置就绪且按钮启用、远程未选择身份时明确禁用的对照。截图分别在 `target/apps-hover-ui/`、`target/apps-launch-ready-ui/`。最终全仓 Clippy 退出 0（26.87 秒）；初期其他 Git/ACP 并行改动的编译失败在工作区更新后已消除，本任务未修复无关文件。验证完成后释放本任务槽位锁。
