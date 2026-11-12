# 任务45：桌面模型诊断证据

2026-11-12（Asia/Shanghai），macOS 27.0 / arm64；vw-desktop 0.6.0、Iced 0.14。直接在原仓库实施，未创建工作树，未修改只读 plan.md。

状态：已完成，待回归。最终记录时间：2026-11-12 07:59:33 CST。

## 生产接线

- 直接依赖已核实：任务39 SDK `2b5362db5`、任务41候选 SDK `e62adc9ef`。模型诊断调用现有 `GatewayClient::diagnostic_start/status/cancel`；候选采集继续由既有接口和端点测速所有者负责。
- 供应商管理列表的“模型诊断”打开 `components/agent_apps_diagnostics/model_panel.rs`，三个独立按钮分别提交可达性、模型和流式任务；消息经 `app_providers/list.rs::ListMessage::ModelDiagnostics` 转到 `message/agent_apps_diagnostics/model.rs::update`。
- `model_state.rs::{ModelContext, ModelRun, ModelState}` 绑定网关、租户、项目、主机、应用、供应商、模型、探测种类及凭据代次。任务身份与详细证据均校验归属；终态拒绝迟到响应。会话最多保留20条记录，不覆盖仍可能在运行的任务身份。
- `App::update` 在切页、关闭设置或上下文失效时终止轮询；启动响应仍保留已受理的任务ID，重新进入只查询原任务。应用目录刷新不改变诊断凭据代次，令牌/主机变更及连接上下文重置才使旧上下文失效。
- UI复用统一弹窗、输入框、按钮、toast和主题样式，滚动区4px滚动条及两侧7px留白；中文和动态模型文本使用默认字体。发送模型/流式最小请求前显示可能费用提示，不增加权限配置。
- 结果展示进度、任务/关联ID、耗时、来源、HTTP状态和实际模型。可达性明确不验证鉴权/模型/流式；任务完成且无详细证据时保持“未验证”。主动诊断没有生产熔断重置或路由切换调用。

## 验证

桌面生产模块已在库crate，采用真实 `--lib` 入口替代卡片旧 `--bin shi-ju`，保留默认features。独立测试文件为 `components/agent_apps_diagnostics/model_panel_tests.rs`，显式注册在父模块。

聚焦命令：

```sh
cargo test -p vw-desktop --lib diagnostic_desktop_cancel_keeps_terminal_state -- --nocapture
```

| 阶段 | 退出码与有效数量 | 观察 |
| --- | --- | --- |
| 红灯，槽位2 | 101；0通过、1失败、9403 filtered | 真实生产状态方法断言 `late success cannot replace confirmed cancellation` 失败 |
| 首次接线编译，槽位1 | 101；未执行 | 7处本任务强类型ID显示/取值错误，已改用公开AsRef<str> |
| 第二轮，槽位1 | 101；1通过、1失败、9404 filtered | 实际Iced消息路由用例通过；隔离HTTP夹具遇到WouldBlock，已显式将接受连接设为阻塞读取；不计行为红灯 |
| 第三轮，槽位1 | 101；未执行 | 其他会话新增system_settings_mcp_tools.rs的5处E0659宏歧义；未修改该文件，观察到所有者补显式导入后再继续 |
| 第四轮绿灯，槽位1 | 0；2通过、0失败、9407 filtered | 构建5m13s，行为3.19s；包括指定测试和routes_and_resumes用例 |
| 取消事务关闭竞态复验，槽位1 | 0；2通过、0失败、9408 filtered | 构建3m28s，行为2.51s；取消Task未执行即关闭面板仍实际发送取消POST |
| 最终聚焦绿灯，槽位1 | 0；2通过、0失败、9408 filtered | 构建4m26s，行为2.76s；含立即关闭取消事务及未确认取消恢复重试断言，源码哈希此后保持不变 |
| 格式化，槽位1 | 0 | `cargo fmt --all -- --emit stdout --config skip_children=true`；只对本任务新文件及共享入口的自有块回写格式化 |
| Clippy首轮，槽位12 | 101；不执行测试 | `cargo clippy --all-targets --all-features -- -D warnings`；SDK读取模块引用另会话正在增加的McpAcpUsageQuery/McpAcpUsage，API类型在该轮编译后补齐；已核对最新源码，复用热缓存重跑 |
| Clippy第二轮，槽位12 | 101；不执行测试 | 无关mcp_management_tools_tests.rs:49的辅助get遮蔽路由get（E0061/E0308）；所有者随后改名为request_read |
| Clippy最终轮，槽位12 | 101；不执行测试 | 无关messages_thinking_tests.rs:113调用不存在的Model::default()（E0599）；本任务未修改该文件，不将失败归为通过，保留45卡等待统一回归 |

收尾发现并修复取消POST被关闭面板中止的竞态：仅GET进度轮询保存abort句柄，取消事务继续执行；真实消息场景在取消Task尚未执行时立即关闭面板，仍要求观察到POST取消。该场景及服务端新快照覆盖未确认取消意图、允许重试的断言均已通过最终聚焦验证。全仓Clippy三轮均由无关改动阻断，最终轮退出101；未报告本任务新增错误/警告，不声称全仓Clippy通过，保留任务卡待统一回归。

绿灯真实观察：

- 三种kind分别通过生产submit helper与真实SDK，观察一次POST启动、GET进度、POST取消和GET重入结果；每种4次请求，共12次；核对正文provider/model/kind、作用域查询及授权头，服务线程全部join退出，无自动重发。
- 实际Iced `App::update`路由也覆盖三种kind，每种4次请求；关闭面板/导航到普通设置页实际中止轮询句柄，保留原任务ID；目录刷新后重入只GET原任务，迟到成功消息不能替换Cancelled。
- Cancelling不等于清理完成；只有服务端Cancelled确认终态。错误供应商证据被拒绝，受控来源、耗时与401可达但鉴权未验证均有断言。
- tiny-skia真实layout/draw/screenshot路径：三种kind的取消状态和成功证据各在明暗主题绘制，共12次，含中文、emoji及动态实际模型ID，像素缓冲非空，无字形整形panic。
- HTTP服务、模型和令牌均为隔离受控夹具；这些证据不代表真实供应商或实机窗口验收。真实账号与原生窗口按母计划人工验收。
- 红灯和绿灯均已有ld的`__eh_frame`超过16MB链接提示；测试仍正常通过，未修改仓库构建配置规避它。

## Cargo槽位与环境

每轮以原子mkdir动态领取1–20槽位，优先已知热缓存；只清理自己的owner记录及锁目录，保留缓存。实际使用：

- 槽位2：`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`（红灯）。
- 槽位1：`/Users/xiangmin/worker/code/shiju/target/rust-slot-1`（格式化、聚焦编译及绿灯）。
- 槽位12：`/Users/xiangmin/worker/code/shiju/target/rust-slot-12`（Clippy）。

所有本任务Cargo及测试子进程已结束，自己领取的锁均已释放，编译缓存保留。

每轮 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为对应的上述绝对路径。命令统一继承 `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_CODEGEN_UNITS=256 CARGO_PROFILE_TEST_LTO=off CARGO_PROFILE_TEST_OPT_LEVEL=0`，未缩减features，未修改Cargo配置。

## 统一回归交接

本任务不修改无关 `model_relay/messages_thinking_tests.rs`。统一回归先核对该测试的Model构造，再按上述环境运行全仓Clippy；通过后可复用最终2项聚焦绿灯（自有源码哈希未变时），将45卡标记已完成并删除。真实供应商及原生窗口仍由用户人工验收。
