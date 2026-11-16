# 任务7实施证据（未完成）

严格未运行 Cargo、测试、实机或回归。静态 diff --check 已完成。

## 已接线

- lifecycle::handle → session::interaction::update：菜单、展开、双击、重命名和标题同步；宿主保留时钟采样、宠物通知与网关调用。
- SessionOpenPrepared → session::history_open：代际与目标接纳、错误回退、短历史保护、显式重置边界和同内容刷新。复用已有 remote_history_is_incomplete。
- common_tests 两用例全部断言原样迁至 host_module_7_tests，以局部 PickerFixture 取代 App；旧挂载及文件删除。新增历史策略断言源码，未执行。

## 未完成职责

- open_session_draft/open_project_session 的运行态与视图缓存转换。
- apply_prepared_session_open、分页合并、SessionMessagesLoaded 和 SessionUiPrepared 的完整状态转换。
- archive/delete/created/copied 的列表转换与切换。
- edit/open/worktree/lifecycle 原测试仍保留，需逐断言迁移去重，不能统称跨域契约。
- App 状态归属尚未收口。无新增依赖或反向引用。

## 来源统计（pathlib splitlines）

|路径|基线|当前|生产候选|
|---|---:|---:|---|
|crates/vw-desktop/src/app/message/project/session/common.rs|21|18|True|
|crates/vw-desktop/src/app/message/project/session/common_tests.rs|54|0|False|
|crates/vw-desktop/src/app/message/project/session/edit.rs|110|110|True|
|crates/vw-desktop/src/app/message/project/session/edit_tests.rs|339|339|False|
|crates/vw-desktop/src/app/message/project/session/lifecycle.rs|957|901|True|
|crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs|1207|1207|False|
|crates/vw-desktop/src/app/message/project/session/open.rs|418|418|True|
|crates/vw-desktop/src/app/message/project/session/open_tests.rs|1460|1460|False|
|crates/vw-desktop/src/app/message/project/session/worktree.rs|82|82|True|
|crates/vw-desktop/src/app/message/project/session/worktree_tests.rs|487|487|False|

没有新建宿主替代文件。测试减少不充当生产减少。尚未满足完整职责契约，保留任务卡。
