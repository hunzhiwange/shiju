# TUI 桌面能力对齐计划

目标：让活动 V3 TUI 覆盖有稳定 Gateway 契约的核心桌面能力，并明确标记桌面专属与后端未支持能力。

范围：审计并接通活动 TUI 的动作执行、流取消与排队、权限/提问/待办、命令面板、会话生命周期、模型与会话设置、附件、技能/插件/MCP/记忆，以及工作流/调度/目标只读与安全操作；同步真实能力文档。

范围外：不复制桌面端文件树、图形编辑器、LSP、预览、原生菜单、宠物、账户商城和拖拽看板；不新增 Gateway 契约或绕过权限确认。

假设：`crates/vw-cli/src/cli/interactive/mod.rs` 当前默认进入 V3 Gateway TUI；`vw-gateway-client` 已提供本计划引用的会话、权限、提问、配置、插件、技能、MCP、工作流、Cron 与调度接口。

## 设计决策

- 采用“同能力、不同交互”，复用 Gateway 真值，不移植桌面 Iced 状态。
- 先修活动宿主闭环，再逐层开放命令；失败必须留在 TUI 内显示。
- 危险写操作需要类型化确认，未接通能力显示原因，不伪装成功。

## 架构说明

- `vw-tui` 只负责渲染与 `V3Action`；`vw-cli` 负责状态、Gateway 调用和副作用。
- 保持目录、租户、会话作用域与默认拒绝边界；动态条目只接受网关返回的稳定 ID。

## 现状结论

| 能力域 | 当前状态 | 代码证据与判断 |
|---|---|---|
| 基础聊天、工具增量、历史快照 | 部分可用 | `app.rs::run_event_loop` 可提交聊天并渲染流；启动仅恢复最近会话或新建，会在流结束后保存 UI 快照。 |
| 动作执行 | 明确漏接 | `bridge.rs::controller_command` 忽略 `V3Action::Select`；外部编辑器直接报未接线；转录查看被转换为控制器不识别的按键。 |
| 排队与取消 | 明确漏接 | `Queue` 仍走立即提交，活动流存在时宿主返回错误；取消只停止客户端消费，没有使用可恢复流请求 ID 调用服务端 stop。 |
| 权限、提问、待办 | 明确漏接 | 展示模型和 `show_approval/show_user_input` 已存在，但活动循环不处理 `TuiControllerCommand::Overlay`；`TaskStateChanged` 被 reducer 丢弃。 |
| 用量与会话元数据 | 部分可用 | 标题更新会落状态；`UsageUpdated` 被忽略，只能依赖终态 usage。 |
| 命令面板 | 未进入活动树 | `CommandPopup`、通用选择组件和大量 overlay 模型存在，但 `BottomPane` 实际只持有 composer 与排队预览；斜杠文本会作为普通提示发送。 |
| 会话生命周期 | 仅最小能力 | `runtime/session_store.rs` 只封装 create、UI load/save/previews、scope get；resume picker、rename、fork、reset、compact、archive 未接活动宿主。 |
| 模型与设置 | 数据模型休眠 | `SessionCapabilityMatrix`、`SettingsPanel` 和 ACP 枚举存在；启动只把 `CliSetup.model_name` 写入状态，未加载 provider/config 能力或打开设置面板。 |
| 附件 | UI 可收集但不可提交 | composer 能产生附件动作，但 bridge 忽略附件动作，并在 submit 时明确拒绝非空附件。 |
| 技能、插件、MCP、记忆 | 模型/API 存在但未接 | Gateway client 与 TUI overlay 模型存在，活动命令注册、加载、选择和安全操作路径缺失。 |
| 工作流、计划/目标、调度 | 适配器/模型存在但未接 | `adapt_task_plan_run_event`、Agents/Workflows/Schedules/BackgroundTasks 模型存在，活动宿主仍只启动 chat stream。 |
| 桌面工作区功能 | 桌面专属 | 文件树、编辑器、LSP、预览、Git 图形面板、终端面板、系统设置全表单、账户/应用市场不应机械移植进交互 TUI。 |
| 能力文档 | 已漂移 | `docs/vibewindow/TUI命令.md` 把多项休眠模型或旧路径标成“已实现”，与当前 V3 活动调用链不符。 |

## 方案权衡

- 方案一：直接复用桌面 `App/Message`。功能看似齐全，但会把 Iced、窗口状态和本地副作用耦合进 TUI，否决。
- 方案二：恢复旧 TUI 全量宿主。已有代码多，但会重新引入双运行时和不清晰边界，否决。
- 方案三：保留 V3 纯渲染边界，由 `vw-cli` 逐项接 Gateway 能力。改动可验证、权限边界清楚，采用。

## 开发策略

- 对会改变行为的代码，先写聚焦回归测试，再实现最小闭环。
- 仓库禁止未经明确授权运行 `cargo test`；先用 Clippy 编译全部测试目标，未获准执行聚焦测试时状态只能记为 `已完成，待回归`。
- 每个代码任务结束都运行 `cargo clippy --all-targets --all-features -- -D warnings`；无关既有失败只记录，不顺手修复。
- 使用本地 Gateway 测试服务模拟网络边界，不模拟 reducer、bridge 或渲染行为。

## 执行顺序

所有批次按 1 至 11 线性执行，无并行批次。任务会连续修改 `app.rs`、`bridge.rs`、`controller.rs`、命令目录和 runtime 模块，并行会产生文件争用和不完整路由。
