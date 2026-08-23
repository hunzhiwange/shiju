# TUI v3 Codex 整体复制移植计划

目标：完整复制 Codex TUI 源码后原地裁剪改写，最大限度保留真实布局、组件、终端行为和快照，再接入 VibeWindow Gateway。

范围：先把 `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui` 整体复制为独立 `crates/vw-tui-v3`，保留目录、测试、快照和资源；随后按编译闭环逐组替换 Codex 协议与业务模块；在 `crates/vw-cli/src/cli/tui_v3` 建立 Gateway 适配并切换默认入口。

范围外：不把 Codex app-server、登录、模型供应商、插件市场、遥测或配置系统接入 VibeWindow；不重新手绘已有 Codex 组件；不修改 VibeWindow Gateway API、安全决策和会话语义；不删除 `tui_v2`。

假设：Codex 源目录当前含 1550 个文件、483 个 Rust 文件和约 270810 行 Rust；源码快照没有可读取的 Git 元数据，因此用来源路径、文件清单和内容校验记录版本；两个项目使用兼容的 Ratatui 0.30/Crossterm 0.29。

## 设计决策

- 用户明确选择“整体复制后改写、删除”，放弃上一计划的界面重绘路线。
- 完整副本作为独立 crate，保留 `crate::` 模块结构；CLI 目录只承载 Gateway 适配。
- 先复制全部文件，再缩小编译模块；删除 UI 文件前必须有对应上游快照通过。
- 未改文件保持上游内容；改动文件标记 VibeWindow 适配并保留 Apache-2.0/NOTICE。

## 架构说明

- `vw-tui-v3` 拥有 Codex 派生的终端、ChatWidget、history cell、composer 和 overlay。
- `vw-cli::tui_v3` 把 `GatewayChatStreamEvent` 转为 UI 事件，把 UI 动作转回现有请求与 controller 行为。
- `vw-tui-v3` 不持有 `GatewayClient`，不构造 Gateway 请求，不决定审批结果。
- 删除门禁以“无引用 + 对应快照通过 + Gateway 回归通过”为准，不以文件看似无用为准。

## 上游源码分区

| 分区 | Codex 绝对来源 | 处理方式 |
| --- | --- | --- |
| crate 与全部资源 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui` | 第一批完整复制，之后原地修改 |
| 渲染基础 | `tui/src/render/`、`style.rs`、`color.rs`、`width.rs`、`wrapping.rs`、`ui_consts.rs` | 优先激活，尽量原样保留 |
| Markdown/流式文本 | `markdown*.rs`、`streaming/`、`table_detect.rs` | 保留算法，替换 Codex 事件类型 |
| 历史消息/工具 | `history_cell/`、`exec_cell/`、`diff_render.rs` | 保留布局与快照，改接 v3 UI 事件 |
| 输入区 | `bottom_pane/textarea*`、`chat_composer*`、`footer.rs` | 保留组件和键位，替换命令/模型来源 |
| 弹窗与审批 | `bottom_pane/list_selection_view.rs`、`approval_overlay.rs`、`request_user_input/` | 保留渲染，动作改接 VibeWindow |
| 主聊天面 | `chatwidget.rs`、`chatwidget/` | 保留布局/活动 cell，删除 Codex 业务处理 |
| 终端与记录 | `tui.rs`、`custom_terminal.rs`、`insert_history.rs`、`pager_overlay*` | 移植真实终端行为，不再近似重画 |
| 必须删除 | `app_server_session*`、登录、Codex 配置/插件/模型业务 | 对应替代链路通过后删除 |

## 验收口径

- 完整复制批次的源/目标目录递归比较无差异，483 个 Rust 文件和全部快照均存在。
- 最终界面直接运行复制并改写后的 Codex 组件，不以新写的相似 renderer 替代。
- 主要上游快照按原尺寸通过；必要文案差异单独列入允许差异文件。
- 所有 Codex app-server/protocol/login/config 生产依赖从最终依赖图消失。
- VibeWindow prompt、session、cancel、permission、terminal event 和 Gateway request 回归不变。

## 开发策略

- 每批先固定一个上游快照或编译失败，再只替换该依赖组。
- 使用“复制保持 → 编译隔离 → 逐组激活 → Gateway 接桥 → 删除死模块”的顺序。
- 每次删除前运行引用搜索与对应快照；禁止一开始批量清空源目录。
- 代码任务执行聚焦测试和 `cargo clippy --all-targets --all-features -- -D warnings`。
- 不运行仓库禁止的全量 `cargo test`、fmt check、wasm check 或 Makefile 命令。
