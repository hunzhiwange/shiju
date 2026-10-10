# 长会话列表卡顿分析

## 实际路径

工作模式使用 `views/project/layout/work.rs::build_work_layout`。它同一帧组合 `work_sidebar::view` 与 `chat_panel::view`；后者按虚拟窗口选消息，但可见的一条长助手消息仍整体进入 `message_view`。项目模式使用另一套 `sessions_panel.rs`，上一版计划混淆了两者。

## 采样证据

2026-10-10 13:39（中国时区），对正在运行的 `target-dir/debug/shi-ju` 做了 5 秒、1 毫秒间隔的只读 `sample`。采样总共约 2949 个栈样本；`App::view` 子树约 2351，`chat_panel::view` 约 1596，`message_view` 约 1333。工作模式 `work_sidebar::model` 在可见栈中仅少量样本。`render_assistant_body`、`deduped_tool_last_indices`、`tool_name_from_raw` 及 `serde_json::from_str` 出现在聊天面板热点调用链。该采样表明当时主线程主要忙于重建长消息，不能据此单独判定所有请求的网络耗时。

## 源码证据与因果链

1. `app_subscription.rs::map_global_event` 为每次鼠标移动派发 `PointerMoved`；工作模式空间按钮还在 `work_sidebar.rs::space_button` 的 `on_move` 派发 `RecentHovered`。`open.rs::RecentHovered` 即使目标项目已缓存，仍派发 `LoadInputPanelTodos`，而 `todos.rs::handle_load_input_panel_todos` 发起网关请求。频繁消息让列表交互伴随更多主视图重建和 TODO 响应更新。
2. `chat_panel/mod.rs::view` 只按 `ChatMessage` 索引虚拟化；一条工具输出很长的助手消息无法在内部再虚拟化。`message_view.rs::message_view` 每次构建对原文多次哈希，执行 `assistant_blocks.into_owned()`，克隆 `special_text_blocks` 与 `display_text`，并提前构造复制按钮和右键菜单所用的完整文本副本。
3. `assistant_body.rs::deduped_tool_last_indices` 与后续块遍历重复调用 `tool_name_from_raw`、`tool_identity_from_raw`；两者都对工具块 JSON 做完整解析，工具输出越长成本越高。渲染缓存已有 `blocks`，但未缓存这类工具派生元数据。
4. 网关 `/v1/session` 确有全租户元数据读取后按目录过滤，以及任务摘要缺失时重新读取全部消息的问题。这些影响列表首次加载，采样中的主线程热点却不在网关；不能把它们称为当前交互卡顿的已证实主因。

## 结论与限制

当前最强的证据指向“高频列表交互/动画消息 × 可见长助手消息重复构建”。先优化消息视图，后减少无效 hover 请求，再在相同场景采样。若用户指的是首次打开列表后的等待，而不是拖动/悬停卡帧，需要单独计时 `/v1/session` 及摘要刷新。采样时进程随后退出，未直接测到端到端点击延迟；因此不写未经测量的毫秒改进承诺。
