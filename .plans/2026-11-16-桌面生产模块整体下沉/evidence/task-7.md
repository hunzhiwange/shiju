# 任务7静态交付证据

静态交付完成；未编译、未运行测试、未实机验证、未回归。仅执行源码审查、独立 rustfmt 解析及 git diff --check。

## 实现归属与调用链

- 宿主 lifecycle::handle → desktop-workspace::project_message::session::interaction：菜单、双击、展开、重命名及两份项目列表标题同步。
- SessionOpenPrepared → workspace::session::history_open：活动目标、代际接纳、UiStore 失败回退、远端短历史、显式重置边界与相同内容判定。
- archive/delete/copied/created → workspace::session::lifecycle::SessionMembership：项目归属变更、去重与替代会话选择。
- OpenRecent/SwitchWorkSpace/RecentHovered/OverlayClose/CreateFailed → workspace::session::overlay：浮层状态与具名跨域效果。
- apply_prepared_session_open → desktop-session::session::history_commit::HistoryState::install / PreparedCommit::apply_view：消息、标识、用量、分页初始页、seq_base、步骤、calls、raw 和首屏载荷整体安装。
- SessionHistoryPageLoaded → HistoryState::prepend：活动/代际隔离、失败 cursor 撤销、真实 PagedChatTimeline 合并与消息前插。
- SessionMessagesLoaded → HistoryState::loaded / restore_cached：复用既有 select_loaded_history、same_visible_history，不复制转换算法；保留相同内容热缓存、较短远端保护、步骤及 calls 语义。
- 冷打开、空草稿、关闭、同内容刷新、首屏准备状态 → HistoryState 对应方法。
- 新草稿/fork 权威状态/继承绑定 → session::agent::SessionRuntimeState 的 open_draft、apply_open_execution_state、restore_open_binding。
- 删除缓存 → session::cache::remove_session；流式 UI 预热世代 → StreamTailState::finish_session_open_ui。
- 归档/删除替代历史读取与 TokenUsage 汇总、重命名/归档/删除持久化 → session::loading；宿主仅提供选定 endpoint 和项目 scope。

## 状态与依赖

App 继续持有既有领域类型 ActiveSessionViewState、SessionRuntimeState、SessionViewCache、PagedChatTimeline、HistoryResetBoundaries。history_state 仅在宿主把五个既有字段借给领域，没有复制 App、Deref、反向源码加载或新增运行时依赖。desktop-session 已依赖 desktop-workspace，方向保持不变；workspace 仅为迁入的真实工作流测试启用 desktop-task-model 的既有 test-support 开发特性。

## 宿主保留的生产函数

|入口|保留职责|
|---|---|
|common::reset_new_session_picker_state / clear_new_session_picker_messages|组合调用既有 picker 域清理，并删除宿主本地化错误缓存|
|edit::handle|旧 ProjectMessage 到领域 Event 的枚举映射；文件对话框、系统 Reveal、设置持久化、网关与导航效果装配|
|lifecycle::cache_external_session / cache_project_session / loaded_chat_from_gateway_messages|真实外部调用者兼容入口、字段投影与聊天组件 compaction 文案注入|
|lifecycle::history_state|五个领域字段的窄借用投影|
|lifecycle::apply_prepared_session_open|组合 session 提交、模型与权限偏好、Iced scroll id 和聊天首屏渲染效果，历史数据转换已归 session|
|lifecycle::handle|消息映射、原生 MCP panel 取消、宠物标题、Iced Task 调度、网关 endpoint 获取、跨 workspace/session/chat 的效果与日志装配|
|open::open_session_draft / open_project_session|组合项目归属、session 状态、编辑器、权限、聊天资源快照、滚动与待办刷新；热快照包含真实渲染资源，restore_into 由宿主装配|
|open::request_session_navigation|采集当前项目/会话并调真实导航解析，再装配异步回执|
|open::handle / update_lists|旧消息映射；列表效果到通知、网关、项目能力及任务板字段的跨域投影|
|open::open_existing_checkout|已验证网关项目结果到导航、系统设置面板与新草稿的组合安装|
|worktree::error_field / handle / begin_picker / update_picker|事件映射、本地化字段映射、网关创建删除重置与 session 草稿/偏好效果装配|

所有保留函数均以既有域入口处理状态转换；没有另建宿主替代文件。宿主选中 active_session_id、Iced 控件资源标识、时钟采样以及其它域字段回写保留为效果安装边界。

## 用例与断言归属

完整清单见 task-7-assertion-map.json，逐用例记录原来源、新位置、原/新断言表达式及宿主保留理由。105 个基线用例、495 处原断言均有归属；34 个领域用例迁入独立模块，71 个保留为真实宿主装配契约，数量不是执行结果。闭包等价花括号与已有 navigation/git 字段归属变化单列，不删除断言。

- workspace::host_module_7_tests：common、worktree、interaction、membership、overlay、list、edit 工作流整组领域场景；局部夹具只调用生产状态更新，并解释本地化/通知边界效果，不模拟被测算法。
- session::host_module_7_tests：真实历史安装、缓存回退、空历史就绪、分页及代际隔离场景。
- 现有 workspace/session 的 lifecycle、session_open、session_worktree、runtime、timeline 测试继续覆盖原领域入口；不复制它们的算法与大型 App 夹具。
- 宿主剩余 edit/open/lifecycle/worktree 用例验证真实 App 的效果装配；每项保留原因均写入映射清单。

## 基线统计（pathlib splitlines，含注释空行）

|来源|基线|当前|生产候选|
|---|---:|---:|---|
|crates/vw-desktop/src/app/message/project/session/common.rs|21|18|True|
|crates/vw-desktop/src/app/message/project/session/common_tests.rs|54|0|False|
|crates/vw-desktop/src/app/message/project/session/edit.rs|110|110|True|
|crates/vw-desktop/src/app/message/project/session/edit_tests.rs|339|172|False|
|crates/vw-desktop/src/app/message/project/session/lifecycle.rs|957|756|True|
|crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs|1207|1041|False|
|crates/vw-desktop/src/app/message/project/session/open.rs|418|459|True|
|crates/vw-desktop/src/app/message/project/session/open_tests.rs|1460|1294|False|
|crates/vw-desktop/src/app/message/project/session/worktree.rs|82|82|True|
|crates/vw-desktop/src/app/message/project/session/worktree_tests.rs|487|46|False|

生产候选 1588 → 1425，净减 163 行；测试 3547 → 2553，净减 994 行；总净减 1157 行。没有新增宿主替代文件。生产净减与测试净减分列，不声称编译加速。

## 未验证风险与以后命令

禁测限制下无法确认类型检查、异步消息时序、原生资源释放和实际渲染。独立 rustfmt 只确认 Rust 源码可解析，不代表编译通过。后续仅在用户授权后运行：

```text
python3 scripts/cargo_slot.py cargo test -p desktop-workspace --lib host_module_7_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-session --lib host_module_7_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib
```

以上命令本轮均未执行；宿主保留契约亦未执行。
