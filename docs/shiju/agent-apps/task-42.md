# 任务42：卸载选择与确认弹窗

状态：已完成。2026-11-06，macOS。

## 范围及依赖

已读 do-task-plans、仓库协议、RustCodeGraph技能及status/explore、只读plan.md、来源与接口、任务41证据、现有SDK/消息/状态/生命周期调用者与测试、统一设置组件和custom_model_modal。任务41真实产物已交付；本任务没有与其他批次并行，未修改plan.md或来源快照。

复用生命周期SDK预览/确认/操作状态链路，按具体installationId提供卸载入口。列表与确认弹窗展示实例路径、真实路径、来源、环境及执行主机；真实路径未被发现时明确显示未确认，不猜测路径。未知来源仅展示人工处理指引，不提供卸载动作，消息入口也拒绝未知、缺失及环境级卸载目标。Grok显示npm更新器可能重建副本警告，Hermes显示保留配置、会话及用户数据提示。

桌面不生成或执行卸载命令，确认只发送previewId及confirmed。服务端预览返回其他实例/来源未知时拒绝确认；后端409/404失效时要求重新预检和确认，不自动重放删除。确认会消费预览，明确失败后再次确认无效；未知提交结果继续使用已有锁定机制。复用既有轮询/取消、代次及上下文隔离、toast反馈。未扩大执行权限或修改后端卸载策略。

## Cargo共享槽位

原子mkdir依次尝试，实际领取槽位2。所有Cargo及桌面/窗口子进程继承：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

已检查`.cargo/config.toml`的`build.build-dir=target`，用后者覆盖，防止中间产物逃离槽位。锁记录计划、任务、会话及当前验证控制进程PID。缓存保留；所有本次子进程结束后才释放本人锁。

## 红灯与通过证据

任务模板的bin目标实际没有本领域测试，测试位于lib。沿用任务41已核实的入口调整，零测试不视为行为验收。

| 确切命令 | 退出码 | 有效数量与结果 |
| --- | --- | --- |
| `cargo fmt --all`（每次Rust修改后） | 0 | 已审查差异，无无关格式化 |
| `cargo test -p vw-desktop --lib cc_apps_desktop_uninstall_state -- --nocapture`（红灯） | 101 | 1项失败：selected uninstall must request backend preview；现有消息明确拒绝卸载，属于目标行为失败 |
| 同上（最终通过） | 0 | 4通过、8569过滤 |
| `cargo test -p vw-desktop --bin shi-ju cc_apps_desktop_uninstall_state -- --nocapture` | 0 | 0测试、31过滤；不作为行为证据 |
| `cargo test -p vw-desktop --lib cc_apps_desktop_lifecycle_state -- --nocapture` | 0 | 5通过、8568过滤，复用链路回归 |
| `cargo test -p vw-agent-apps --lib cc_apps_uninstall_revalidates_plan -- --nocapture` | 0 | 3通过、158过滤，实际临时文件与受控包管理器进程副作用 |
| `APPS_UNINSTALL_WINDOW_OUTPUT="$PWD/docs/shiju/agent-apps/task-42-ui" cargo test -p vw-desktop --test apps-uninstall-window` | 0 | 12原生中文窗口场景，通过真实字体整形与截图 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓通过，无新增警告 |
| `cargo run -p vw-desktop --bin shi-ju` | 143 | 构建0.30秒，正常进入标准桌面后主动SIGTERM结束；无panic，不声称自然退出0 |
| `git diff --check` | 0 | 无空白错误 |

一次扩充HTTP测试遗漏LifecycleState导入导致编译101，已补导入并重跑4项通过，不计作预期红灯。

4项桌面用例覆盖：第二实例精确选择与真实路径、无预览不可确认、确认消费预览、失效后重复确认不提交、必须重新预检；未知来源/缺失实例/环境目标拒绝；返回预览目标或来源不匹配拒绝；真实回环HTTP授权头/主机与精确请求体、409过期提交拒绝、503未知结果不可重放、同操作ID断线恢复与取消。HTTP夹具只替代远端网关边界，不使用个人安装/凭据，不修改HOME。

3项后端回归在真实临时目录及受控npm进程中检查：选定目标成功删除、其他文件保留、退出非零/目标仍存在/悬空入口不能冒充成功；来源、真实路径、环境、主机、实例ID、入口及确认变化均在执行前拒绝，标记文件不存在且目标保留；二进制指纹变化拒绝，重新预览确认后才能删除。不是仅断言函数调用次数。

## 实际渲染

独立窗口目标显式运行，不纳入默认测试。直接渲染生产应用列表及确认弹窗；600×650逻辑尺寸，两种主题各检查多实例/未知来源指引、Hermes确认、操作进度、失效错误、Grok长内容与禁用确认、取消终态，共12场景。中文及混合路径使用默认字体，无MONOSPACE。模态复用settings组件、主次按钮；内容独立滚动，标题和底部按钮固定，4px滚动条，两侧7px留白。

截图保存在[task-42-ui](task-42-ui/)。人工检查明亮/暗黑确认、暗黑实例与长内容禁用截图。首次短等待截图有按钮文字尚未绘出，延长原生渲染等待后复验；不以初始截图宣称完成。标准应用启动不替代专用窗口的本领域页面验证；未声称管理员真实网关的完整个人安装删除链路，逐应用实机验收归后续46–59。

macOS链接器继续出现既有__eh_frame性能提示，窗口日志有系统IMKCFRunLoopWakeUpReliable消息，无字形整形panic。未运行无过滤全量cargo test、fmt检查、wasm检查或Makefile。未处理其他任务编号文件既有删除，未将其纳入提交。

最终复验：4项卸载行为测试、12个原生窗口场景和全仓Clippy均退出0。明亮确认按钮“确认卸载”与暗黑禁用按钮“处理中…”文字清晰可见，无字形整形panic。截图等待60帧后保存，标题和底部操作固定；未改底层渲染器。所有本次Cargo与桌面子进程已结束，释放本人slot-2锁并保留缓存。建议下一步执行批次43。
