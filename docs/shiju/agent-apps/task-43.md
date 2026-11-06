# 任务43：应用设置桌面表单

状态：已完成。2026-11-06，macOS。

## 范围与依赖

已读 do-task-plans、仓库协议、RustCodeGraph status/explore、只读 plan.md、来源与接口、任务42证据、当前 SDK/API/状态/消息/列表调用者、统一设置组件与 custom_model_modal。前序真实产物已交付，未因旧计划状态标签判定缺失；未与其他批次并行，未修改 plan.md 或来源快照。

复用已授权主机、独立应用管理令牌、设置 API 与 SDK。用户可见性/顺序和主机终端/WSL shell、flag 分域 PATCH；列表只投影 API 返回的已保存值，草稿不改变列表。成功以完整服务端响应替换保存值和草稿，明确失败恢复原值；至少保留一项可见。上下文、主机、令牌或目录请求代次改变后丢弃旧设置响应；保存中拒绝重复提交和编辑。

必要的最小底座补齐：已有设置 SDK 只有普通网关认证，而真实设置 API 要求 `x-agent-apps-token`，新增两个显式授权读/写方法，不替换网关认证、不重试写入、不扩权。已有设置响应没有执行主机能力；新增可选 `capabilities`，由领域既有终端校验列表和 HostPlatform 生成，客户端不复制终端白名单、不按桌面 OS 猜测远端能力。旧响应仍可解码，缺少能力时明确禁用主机设置。

WSL 发行版显示来自 API 观察到的 WSL 安装实例，未发现时明确显示“尚未发现”；现有保存契约只包含按应用的 shell/flag，不伪造发行版保存字段或新增任意执行接口。发行版由具体安装实例决定，选择启动实例属于后续启动任务。Windows 能力夹具仅验证纯规则和 macOS 上的表单渲染，不声称 Windows/WSL 实机执行通过。

主机设置响应 `restartRequired` 仅提示手动重启所选执行主机上使用新设置的应用终端会话，不自动重启应用、网关或桌面。反馈通过 toast；错误文案按类型归类，不展示原始响应载荷或令牌。

## Cargo 共享槽位

原子 mkdir 依次尝试后领取槽位2，锁记录计划/任务/会话/本阶段控制进程 PID。所有 Cargo 与原生窗口子进程继承：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

已检查 `.cargo/config.toml` 的 `build.build-dir=target`，用上述环境覆盖，防止中间产物逃离槽位。所有本次子进程结束后释放本人锁，保留缓存。

## 行为证据

任务模板 bin 入口没有本领域测试，沿用任务42已核实的 lib 入口；bin 的零测试不计验收。

| 确切命令 | 退出码 | 有效数量/结果 |
| --- | --- | --- |
| `cargo fmt --all`（Rust 修改后） | 0 | 审查差异，无无关格式化 |
| `cargo test -p vw-desktop --lib cc_apps_desktop_preferences_state -- --nocapture`（红灯） | 101 | 1失败：`state.finish(2, Err("明确拒绝".into()))`，保存失败未恢复原值，属于目标行为失败 |
| 同上（最终通过） | 0 | 5通过、8573过滤 |
| `cargo test -p vw-desktop --bin shi-ju cc_apps_desktop_preferences_state -- --nocapture` | 0 | 0测试、31过滤，不计行为证据 |
| `cargo test -p vw-agent-apps --lib cc_apps_preferences_normalization -- --nocapture` | 0 | 5通过、157过滤；含Mac/Windows/Linux显式夹具 |
| `cargo test -p vw-gateway-client --lib cc_apps_settings_launch_sdk_roundtrip -- --nocapture` | 0 | 5通过、287过滤；真实回环HTTP |
| `cargo test -p vw-agent --lib cc_apps_preferences_api_scope -- --nocapture` | 0 | 2通过、11461过滤；真实临时数据库及权限策略 |
| `cargo test -p vw-desktop --lib cc_apps_desktop_catalog_state -- --nocapture` | 0 | 6通过、8572过滤；目录接线回归 |
| `APPS_PREFERENCES_WINDOW_OUTPUT="$PWD/docs/shiju/agent-apps/task-43-ui" cargo test -p vw-desktop --test apps-preferences-window` | 0 | 最终10个原生中文窗口场景 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 首轮及最终复验均通过 |
| `cargo run -p vw-desktop --bin shi-ju` | 143 | 标准桌面进程已启动，运行5秒后主动SIGTERM结束，无panic，不声称自然退出0 |
| `git diff --check` | 0 | 无空白错误 |

5项桌面测试覆盖：API成功值覆盖草稿、失败恢复、过期/重复响应拒绝；最后一项可见保护、保存中不编辑；无能力与非Windows能力拒绝WSL编辑、Windows显式夹具shell/flag可编辑、桌面应用排除；真实消息路由按主机/上下文/代次隔离，变更主机使旧响应失效；列表只投影已保存的顺序和可见性，不采用未保存草稿。

## 实际渲染与审查

原生窗口直接渲染生产 preferences 弹窗，600×650逻辑尺寸；明暗主题分别检查保存/重启提示、错误、保存中禁用和 Windows/WSL 中文动态数据。复用设置模态、输入/选择/复选控件、主次按钮样式；长表单只滚动内容，标题与操作固定，4px滚动条、两侧7px留白。中文与混合内容使用默认字体，无 MONOSPACE。

截图保存在 [task-43-ui](task-43-ui/)。最终10场景覆盖两主题的保存提示、错误、保存中禁用、WSL表单顶部与滚动底部；人工检查明亮保存/错误、暗黑禁用/WSL及shell/flag底部截图。60帧后捕获，实际经历中文字体整形，无panic。标准桌面启动检查不替代专用生产弹窗窗口验收，也不宣称已在个人安装或真实Windows机器上执行设置。

SDK回环HTTP检查独立应用令牌与租户头、含中文主机的精确URL编码、完整单域PATCH请求、服务端403拒绝、原有不重放/不跟随重定向规则；令牌未进入请求体。设置API回归验证数据库持久化、租户/用户/主机隔离、未授权和无效配置不写入。领域能力测试复用生产校验列表，覆盖Mac/Windows/Linux，非Windows不开放WSL字段。

编译仍有既有macOS链接器 `__eh_frame` 性能提示，首轮窗口日志有系统IMKCFRunLoopWakeUpReliable消息；均无字形整形panic。最终所有行为测试、10场景窗口及全仓Clippy退出0，23项聚焦行为/回归测试通过。所有本次Cargo、原生窗口和标准桌面进程已结束，已释放本人slot-2锁并保留缓存。未运行无过滤全量 cargo test、fmt检查、wasm检查或 Makefile。既有其他编号任务删除不纳入提交。
