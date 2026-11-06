# 任务41：安装升级桌面进度

状态：已完成。2026-11-06，macOS。

## 范围与依赖

已读 do-task-plans、仓库 AGENTS、RustCodeGraph 技能与 status/explore、只读 plan.md、来源与接口、任务40证据、实际目录状态/消息、统一设置组件、自定义模型弹窗、生命周期SDK/DTO/授权处理器及执行器。批次40入口已交付，没有依据母计划旧状态猜测接口缺失；本批次未与其他批次并行，plan.md保持只读。

应用列表增加按明确实例ID升级、明确macOS/Linux环境安装的服务端预检入口。每个升级按钮紧邻对应路径、环境与来源，确认弹窗显示实际路径、观察来源、服务端确认来源及执行主机。桌面不推断远端OS，不把客户端环境当执行授权；服务端拒绝不支持的目标。没有新增Windows/WSL执行承诺或改变应用能力目录。

只通过SDK预检与确认提交，只提交previewId和confirmed，不执行displayCommand或保存安装配方。进度保持同一个operationId；长轮询断连保留状态，刷新读取同一ID再继续事件查询；取消通过SDK等待服务端实际停止。取消/刷新使旧轮询响应失效，跨网关、租户、项目、主机与代次响应不覆盖当前状态；令牌输入与上下文变更递增代次清理旧操作。终态不被运行态覆盖，其他操作ID拒绝。结果未知的提交锁定交互并提示管理员核对，不自动重放。错误与失败使用既有toast；列表刷新与操作刷新分开。

必要底座补线：既有生命周期SDK未带服务端要求的独立应用令牌，新增五个显式authorized入口；原有入口签名与无独立令牌行为保留。所有调用仍使用禁用重定向/重试的原生传输，专用HeaderValue标记sensitive，仅在对应应用请求中携带，桌面Debug不含令牌。

继续检查发现Node领域门控虽在任务12交付，local授权生命周期适配器没有调用。服务端local预检现在调用现有Node规则，读取所选实例旁Node及管理员npm旁的后备Node，固定npm registry latest元数据；run在已有写锁内重新preview，因此执行前再次检查。明确Gap拒绝，Unknown沿用原领域上游咨询门控，不把未知伪报满足。没有在UI复制Node规则，也没有新增远程RPC、用户可配registry或执行器。内部HTTP边界参数仅用于生产固定地址和隔离元数据夹具。

UI复用settings_modal_card/overlay/close_button和主次按钮样式。长预检内容单独滚动、标题与按钮固定，4px滚动条、内容两侧7px留白；中文和混合路径使用默认字体。

## Cargo共享槽位

原子mkdir从1开始尝试，实际获得槽位2：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

已检查.cargo/config.toml的build.build-dir=target并用后者覆盖，所有Cargo及窗口/应用子进程继承；没有另建构建目录。锁记录本计划、任务、会话与控制/验证进程PID；结束所有子进程后仅清理本人锁，保留缓存。

## 红灯证据

| 确切命令 | 退出码 | 有效数量与失败原因 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | Rust修改后运行；审查没有无关格式化差异 |
| `cargo test -p vw-desktop --bin shi-ju cc_apps_desktop_lifecycle_state -- --nocapture` | 0 | 0测试、31过滤；桌面测试实际在lib，不能作行为证据 |
| `cargo test -p vw-desktop --lib cc_apps_desktop_lifecycle_state -- --nocapture` | 101 | 1项失败、8564过滤；骨架接受另一operationId，真实状态断言失败 |
| `cargo test -p vw-agent-apps --lib cc_apps_desktop_lifecycle_state -- --nocapture` | 101 | 1项失败、160过滤；未接门控时Node18预检返回Ok计划，而应拒绝 |

Node红灯运行时只移除本次新增门控调用，保留可编译签名与真实发现/元数据夹具；随后恢复调用。没有将工具缺失、无关编译错误或零测试充当红灯。

## 通过验证

| 确切命令 | 退出码 | 有效数量/结果 |
| --- | --- | --- |
| `cargo fmt --all`（各Rust阶段后） | 0 | 最终差异仅本任务，无无关格式化 |
| `cargo test -p vw-agent-apps --lib cc_apps_desktop_lifecycle_state -- --nocapture` | 0 | 1通过、160过滤 |
| `cargo test -p vw-desktop --lib cc_apps_desktop_lifecycle_state -- --nocapture`（最终布局修改后） | 0 | 5通过、8564过滤 |
| `cargo test -p vw-gateway-client --lib cc_apps_lifecycle_sdk_roundtrip -- --nocapture` | 0 | 4通过、287过滤 |
| `cargo test -p vw-agent-apps --lib cc_apps_node_requirement_gap -- --nocapture` | 0 | 4通过、157过滤 |
| `cargo test -p vw-desktop --bin shi-ju cc_apps_desktop_lifecycle_state -- --nocapture`（最终） | 0 | 0测试、31过滤；由上述lib入口的5项提供行为证据 |
| `APPS_LIFECYCLE_WINDOW_OUTPUT="$PWD/docs/shiju/agent-apps/task-41-ui" cargo test -p vw-desktop --test apps-lifecycle-window`（最终布局修改后） | 0 | 12场景全部通过，无字形整形panic |
| `cargo clippy --all-targets --all-features -- -D warnings`（最终布局修改后） | 0 | 全仓通过，9.96秒，无新增警告 |
| `git diff --check` | 0 | 无空白错误 |

桌面真实回环HTTP用例覆盖独立应用授权头、所选第二实例ID原样提交、预检/确认提交分离、断连后同ID读取及取消、Node预检422拒绝和503提交结果未知；精确断言方法、路由和请求体。消息路由用例另覆盖未预检不能确认、重复确认不产生新代次、跨主机和旧轮询结果拒绝、取消实际终态、Node错误toast、未知提交不可重放。状态测试覆盖另一操作ID及终态回退拒绝、上下文失效时递增代次。

底层Node用例使用真实临时prefix、package.json、所选可执行软链接、Node及npm进程夹具，以及返回真实engines/node形状的回环HTTP服务。Node18因>=22要求预检失败，npm标记文件不存在；替换为Node24重新预检成功，仍无npm写入。仅替代外部元数据与全局安装边界，不修改HOME，不访问个人凭据。现有Node规则/HTTP未知处理和SDK断连/307/丢失回执不重放回归全部通过。

## 实际渲染

`APPS_LIFECYCLE_WINDOW_OUTPUT="$PWD/docs/shiju/agent-apps/task-41-ui" cargo test -p vw-desktop --test apps-lifecycle-window`：真实Iced事件循环、字体整形、窗口帧与截图，600×650逻辑尺寸，明暗两种主题分别展示多实例按钮、预检确认、同ID进度恢复/取消、结果未知错误、长内容与禁用确认、取消终态，共12场景。截图在[task-41-ui](task-41-ui/)。不是仅构建Element；夹具不模拟安装成功或写入个人安装。

初次长说明截图发现正文挤掉底部按钮，已将弹窗高度与内容区分配明确化，只让正文填充剩余空间。布局修正后重跑5项桌面测试、12场景渲染及全仓Clippy，并重新保存全部截图；人工查看最终明亮确认、暗黑多实例及暗黑长内容/禁用截图，标题和底部按钮固定可见、文字清晰、滚动条无覆盖。没有依赖Element构造测试冒充渲染验收。

`cargo run -p vw-desktop --bin shi-ju`：构建正常进入标准应用，0.48秒完成构建；日志无panic或event loop failed。验收结束仅对本次槽位内的桌面进程发送SIGTERM，shell退出143（主动结束，未声称自然退出0）。标准应用启动不替代上述生产页面/弹窗的原生渲染证据；未声称通过管理员真实网关的完整安装点击链路，真实逐应用安装归后续46–59实机任务。

测试链接器保留既有macOS __eh_frame性能提示；Clippy通过。早期窗口日志出现系统IMKCFRunLoopWakeUpReliable提示，没有panic；最终窗口日志报告12场景通过。未运行无过滤全量cargo test、fmt检查、wasm检查或Makefile。

最终所有本次Cargo与桌面子进程结束，释放本人slot-2占用记录与锁，保留缓存。差异仅本任务SDK授权通路、服务端Node门控接线、桌面状态/交互、独立测试、窗口目标及证据；既有1–40编号任务删除不纳入提交，plan.md未修改。下一步批次42卸载桌面交互。
