# 任务20 静态交付

用户禁止 Cargo、测试、绘制与回归；本次仅源码审查、独立 rustfmt 与 diff 检查。未编译、未测试。

## 生产入口与真实接线

本轮基线已经将完整生产实现放入 desktop-settings-runtime，未创建平行算法：
- state + app_init 持有/构造 Hooks、Runtime、Reliability、Multimodal、Observability、QueryClassification；宿主 App 字段直接持有这些类型。
- system_settings_hooks/runtime/reliability 的 View、view、help/view_overlays 属领域。宿主对应组件仅输入装配、局部消息映射、聊天文本选择及跨页帮助弹窗装配。
- settings::{runtime,multimodal,observability,reliability} 调用领域 update，将配置结果交给已有异步保存入口。
- hooks 调用领域 update，并将 Effect 映射到 gateway_hooks/audit/trust/revoke、保存和 toast。
- query_classification 调用领域 update，将 Refresh/Save/Invalid 接入模型路由公共持久化队列。
- settings/mod.rs 的真实消息分支调用这些适配器，保存反馈使用领域 save_feedback；无 App 反向依赖或宿主源码 include/path。

生产候选基线255行均为必要装配，本次生产净减量0；不把测试减量算生产迁出。

## 断言归属

新增 host_module_20_tests 独立注册：
- reliability_ui::ui_refresh_21_layout_and_interaction：原同名全部断言，真实领域 view/update；选择器对齐、高度、菜单截图差异、键盘选择、未知存储值兼容。
- reliability_ui::ui_refresh_21_preserves_values_and_disabled_states：原同名全部断言，中文输入、挂载与只读开关、滑块边界、弹窗滚动/关闭与禁用。
- hooks_ui::ui_refresh_28_preserves_values_and_disabled_states：原同名全部断言，空态/加载、信任/撤销、忙态、审计滚动、中英明暗布局与截图。
- *_unrelated：原 multimodal/observability/query_classification 的无关消息全部状态断言；领域使用存在的异域消息 ReliabilityHelpClose。宿主外层枚举的过滤仍由适配分支静态审查。
- hooks_update::plugin_hooks_toggle_persists_config：原开关状态、清错断言及真实 Save 效果；宿主同名保留实际任务调度断言。

现有 runtime/reliability/multimodal/observability/query_classification 的 update 测试与所有页面树测试仍在领域独立注册，无重复复制。desktop_split_29 的 enabled/command_logger 状态测试仅覆盖另两个开关，新增 plugins_enabled 不重复它们。

## 宿主残留逐组理由

- runtime/multimodal/observability/reliability 的 schedule_persistence 测试：真实跨域保存任务。
- query_classification 的 refresh_failure：公共模型路由刷新、版本隔离与草稿保护，跨域契约。
- hooks 更新测试：设置页切换产出两个消息、网关 Task、toast、保存任务、异域宿主枚举过滤；不能由局部 HooksMessage 替代。
- hooks UI layout_and_interaction：观测页经宿主设置窗口及 SELECT_FOCUS_SCOPE 的焦点、共享帮助弹窗接线。
- reliability UI renders_pages_and_narrow_help/runtime_select_is_reachable：Runtime 与 Agents 内嵌入口、全设置窗口和共享帮助选择接线。
- runtime rollout 与各 help_rollout：真实宿主聊天文本选择/复制/外部搜索菜单，跨页 Runtime/Observability/Scheduler/Cron/Hooks 入口；保留其真实 renderer、字体、draw/screenshot，未删断言。
- 上述宿主共享几何、截图、隔离进程夹具仍由保留的跨域用例使用；新增领域夹具只含三个页面局部状态，无 App、网关或全局生命周期。

## 静态验收及后续

按 baseline.json 的18个来源、pathlib splitlines 同口径：基线2551行，现2192行，宿主净减359行；无新增宿主替代文件。全部来源生产函数与测试模块按以上分组审查。领域测试依赖新增 tempfile 和 desktop-chat-view（仅 dev），使用真实字体和选择文本控件，无生产反向依赖。

独立 rustfmt 仅新增文件；本任务 diff --check 已检查。尚无编译、运行、字体整形或截图验证证据。以后须用户授权才能执行：
`python3 scripts/cargo_slot.py cargo test -p desktop-settings-runtime --lib host_module_20_tests -- --test-threads=1`
以及统一阶段宿主 `cargo check -p vw-desktop --lib`（同槽位入口）。
