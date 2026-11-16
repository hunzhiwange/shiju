# 任务25：静态交付证据

状态：完整静态交付；未编译、未测试、未绘制、未执行回归。原部分实施提交为 04f709e4d，本次补齐目录转换、页面族、原生窗口入口、调用者与旧挂载清理。

## 实现与真实接线

- apps_catalog 的查询、凭证失效、观察请求代际、观察回包、连接回包、刷新决策、详情展开和离线错误均接入 settings-apps::system_settings_apps::catalog_update::CatalogInput。输入仅借用局部字段，不借用 App；CatalogState 提供相同真实输入给独立页面夹具。App 继续持有已有 desktop-apps-ui 组合状态及领域 LaunchState/PreferencesState/LifecycleState/DragState，未复制 App 或引入 Deref。
- state::agent_apps 的 begin、finish、reset_if_context_changed 调用同一 CatalogInput；自身只组装跨领域失效。移除 desktop-apps-ui::catalog_state 中不再使用的 begin、finish、cache 旧实现，避免双份状态算法。
- launch、preferences、lifecycle_update 的既有领域入口继续负责启动身份、参数、预检、确认、事务提交、取消及不确定结果恢复。安装提交不重放、请求代际及主机/上下文检查原样保留。
- usage 生产入口已归 desktop-usage::refresh，原事件测试整组随真实归属迁至该库。plugin_install 已归 desktop-settings-skills，宿主保留 session_gateway 外部事务和跨页刷新装配，不另建应用安装算法。
- 原目录、详情、拖拽、生命周期、卸载、偏好、启动及 UI refresh 测试独立挂载 host_module_25_tests；原宿主文件及挂载已删除。五个 native window 验收入口保留可执行调用链，宿主只调用 native_acceptance::{catalog,launch,lifecycle,preferences,uninstall}::run。
- 原生验收夹具使用局部 CatalogState 与实际领域 view，跨页内容槽提供空 Element，不复制其它页面控制器；相关断言仍覆盖本领域布局、绘制、字体、事件与截图。原生窗口创建/帧订阅/截图实现随验收场景完整保留，现轮不调用。
- 书签页面仍调用的 verify_help 迁至领域 native_acceptance::help，成为泛型消息辅助；真实直接调用者已改接，未遗留到已删除的 preferences_tests 路径。

## 底层渲染依赖

直接依赖 chat-view 会形成 settings-apps → chat-view → desktop-session → settings-apps 循环，已静态发现并消除。将通用 markdown_selection、markdown_links、selection_menu 按原实现提取到 desktop-ui-controls::selectable_text；chat-view 保留兼容重导出，chat-tools 选择颜色重导出同一底层实现。保留原背景 RGB 阈值、选择颜色、字体、事件与 URL 校验，未复制算法；Fragments 的既有绘制测试直接调用者所需只读几何入口仍可访问。新增 url 仅是已存在锁定依赖的新直接使用；截图及本地化依赖从 dev 移至验收入口实际使用位置。未新增特性开关。

## 宿主残留逐职责说明

- catalog/local/presentation/uninstall：稳定类型或函数重导出；local::available 只从宿主取当前网关。
- details::view：组合项目组合、路由、提示词、OpenClaw/Hermes、高可用等其它领域页面槽，实际安装详情由领域绘制。
- domains::view：将领域导航 Target 映射宿主其它设置页，组合 external_sessions。
- list::view：向领域列表提供跨页详情、领域导航、路由及宿主本地化帮助工厂。
- mod::message/domain_event：生产消息与其它领域导航、URL 外部效果映射；view_state：只借用领域视图输入；view_overlays：组合其它页面模态层。
- drag::event/update/subscription：映射消息、应用领域返回的局部结果及保存任务，主窗口 ID 由宿主持有。
- launch/preferences 的 update/event/view：取当前网关和项目上下文、借用领域输入、映射 Task 与 Toast；visible_rows 仅转发领域函数。
- lifecycle::action_button/progress/overlay：领域绘制转发与指南装配；apps_lifecycle::update：宿主当前网关/项目上下文、跨页目录缓存失效与 Toast 映射。
- apps_catalog::update：跨域分发、跨域失效、当前网关/项目上下文与异步 SDK Task 调度、加载后的路由/偏好刷新及托盘导航；所有局部目录状态策略走 CatalogInput，catalog_input 只装配字段借用。
- plugin_install::message/update：宿主 session_gateway prepare/activate/discard 外部事务、其它页面刷新与 Toast 装配；领域更新仍调用 settings-skills。
- 五个 *_window_tests::run：仅转发显式原生验收二进制生命周期入口；不在单元测试中启动窗口。
- state::agent_apps 的类型字段组：既有跨页组合存储；reset/begin 只失效其它领域，目录字段转换调用领域方法；finish 直接转发。

## 断言与夹具核对

基线所有31个命名测试均有新归属；逐函数计数 assert!/assert_eq!/assert_ne! 宏数量与原函数一致，无删除断言。原宿主 Toast 文案断言改为同文案领域 Notice::Error；原 HostInput 触发局部状态失效改为实际 CatalogInput::credentials 后检查迟到回复，所用实现就是生产宿主接线。拖拽仍执行真实 HTTP PATCH、响应归一化和失败回滚；子进程 --exact 路径已改为新模块。原生五组场景保留全部 screenshot/draw 和主题/中文状态输入。

既有 desktop_split_13 和 system_settings_apps 下的 catalog/launch/preferences/lifecycle/drag 测试分别覆盖纯状态函数、SDK或单次对象结果；本次对应的是控制器消息代际、事务流程和页面绘制，逐函数测试名及断言语境不同，未再复制旧宿主实现。原整组控制器测试仅保留领域新挂载。

| 原测试来源 | 测试名 | 新归属 |
|---|---|---|
| `catalog_tests.rs` | `cc_apps_desktop_catalog_state_retry_and_stale_response` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_tests.rs` |
| `catalog_tests.rs` | `cc_apps_desktop_catalog_state_context_switch_releases_pending_load` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_tests.rs` |
| `catalog_tests.rs` | `local_apps_credentials_only_apply_to_current_connection_and_generation` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_tests.rs` |
| `catalog_tests.rs` | `catalog_cache_expires_and_invalidates_after_failure_or_context_change` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_tests.rs` |
| `catalog_tests.rs` | `catalog_reopening_reuses_cache_but_manual_refresh_and_credentials_invalidate_it` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_tests.rs` |
| `catalog_ui_refresh_06_tests.rs` | `ui_refresh_06_renders_catalog_states_and_narrow_connection` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs` |
| `catalog_ui_refresh_06_tests.rs` | `ui_refresh_06_layout_and_interaction` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs` |
| `catalog_ui_refresh_06_tests.rs` | `ui_refresh_06_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs` |
| `catalog_ui_refresh_06_tests.rs` | `settings_help_gap_14_app_catalog` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/catalog_ui_refresh_06_tests.rs` |
| `details_tests.rs` | `cc_apps_desktop_manual_guide_click_opens_only_official_client_url` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/details_tests.rs` |
| `drag_tests.rs` | `apps_drag_saves_only_after_drop_and_keeps_last_confirmed_order_on_rejection` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/drag_tests.rs` |
| `launch_tests.rs` | `cc_apps_desktop_launch_state_message_scope_and_redacted_diagnostics` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/launch_tests.rs` |
| `launch_tests.rs` | `cc_apps_desktop_launch_state_identity_reply_scope_and_no_forged_selection` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/launch_tests.rs` |
| `launch_ui_refresh_07_tests.rs` | `ui_refresh_07_busy_terminal_does_not_open_a_menu` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/launch_ui_refresh_07_tests.rs` |
| `launch_ui_refresh_07_tests.rs` | `ui_refresh_07_renders_modal_states` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/launch_ui_refresh_07_tests.rs` |
| `launch_ui_refresh_07_tests.rs` | `ui_refresh_07_layout_and_interaction` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/launch_ui_refresh_07_tests.rs` |
| `launch_ui_refresh_07_tests.rs` | `ui_refresh_07_preserves_launch_identity_and_literal_parameters` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/launch_ui_refresh_07_tests.rs` |
| `launch_ui_refresh_07_tests.rs` | `ui_refresh_07_preserves_preferences_and_disabled_states` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/launch_ui_refresh_07_tests.rs` |
| `launch_ui_refresh_07_tests.rs` | `help_rollout_04_launch_renders_contextual_help` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/launch_ui_refresh_07_tests.rs` |
| `lifecycle_tests.rs` | `cc_apps_desktop_lifecycle_state_uncertain_submit_blocks_retry_and_invalidates_generation` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/lifecycle_tests.rs` |
| `lifecycle_tests.rs` | `cc_apps_desktop_lifecycle_state_authorized_preview_submit_disconnect_recover_cancel` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/lifecycle_tests.rs` |
| `lifecycle_tests.rs` | `cc_apps_desktop_lifecycle_state_message_confirmation_scope_cancel_and_toast` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/lifecycle_tests.rs` |
| `lifecycle_tests.rs` | `cc_apps_desktop_lifecycle_state_message_unknown_submit_cannot_replay` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/lifecycle_tests.rs` |
| `preferences_tests.rs` | `cc_apps_desktop_preferences_state_message_scopes_and_atomic_domains` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/preferences_tests.rs` |
| `preferences_tests.rs` | `help_rollout_04_renders_contextual_help` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/preferences_tests.rs` |
| `uninstall_tests.rs` | `cc_apps_desktop_uninstall_state_selected_instance_requires_new_preview_after_rejection` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/uninstall_tests.rs` |
| `uninstall_tests.rs` | `cc_apps_desktop_uninstall_state_unknown_missing_and_environment_targets_never_execute` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/uninstall_tests.rs` |
| `uninstall_tests.rs` | `cc_apps_desktop_uninstall_state_mismatched_preview_and_unknown_source_rejected` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/uninstall_tests.rs` |
| `uninstall_tests.rs` | `cc_apps_desktop_uninstall_state_http_selected_instance_stale_submit_and_no_replay` | `crates/libs/desktop-settings-apps/src/host_module_25_tests/uninstall_tests.rs` |
| `apps_usage_events_tests.rs` | `usage_usage_events_returns_expected_result` | `crates/libs/desktop-usage/src/apps_usage_cards/usage_events_25_tests.rs` |
| `apps_usage_events_tests.rs` | `usage_usage_events_rejects_invalid_input` | `crates/libs/desktop-usage/src/apps_usage_cards/usage_events_25_tests.rs` |

## 固定基线行数审计

同 baseline.json 的 pathlib.read_text().splitlines()，包含注释与空行。无本任务新建宿主替代文件；native 窗口旧文件仅为两行转发，仍计入来源。

来源总量 4670 → 631，宿主净减少 4039 行。生产候选 683 → 621，候选净减少 62 行；不把测试搬迁算作生产减少。此为固定基线至最终版本，包含此前领域迁移和共享调用者字段变更，不将净值全部归因本次新增方法。

| 来源 | 基线 | 当前 | 生产候选 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/components/system_settings_apps/catalog.rs` | 5 | 1 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/catalog_tests.rs` | 200 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/catalog_ui_refresh_06_tests.rs` | 651 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/details.rs` | 31 | 31 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/details_tests.rs` | 84 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/domains.rs` | 8 | 8 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/drag.rs` | 13 | 13 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/drag_tests.rs` | 185 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/launch.rs` | 19 | 16 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/launch_tests.rs` | 144 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/launch_ui_refresh_07_tests.rs` | 679 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/launch_view.rs` | 3 | 3 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/launch_window_tests.rs` | 181 | 2 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/lifecycle.rs` | 12 | 12 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/lifecycle_tests.rs` | 291 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/lifecycle_window_tests.rs` | 210 | 2 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/list.rs` | 5 | 5 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/local.rs` | 4 | 4 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/mod.rs` | 116 | 102 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/preferences.rs` | 15 | 15 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/preferences_tests.rs` | 269 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/preferences_view.rs` | 3 | 3 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/preferences_window_tests.rs` | 208 | 2 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/presentation.rs` | 1 | 1 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/uninstall.rs` | 1 | 1 | True |
| `crates/vw-desktop/src/app/components/system_settings_apps/uninstall_tests.rs` | 341 | 0 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/uninstall_window_tests.rs` | 221 | 2 | False |
| `crates/vw-desktop/src/app/components/system_settings_apps/window_tests.rs` | 233 | 2 | False |
| `crates/vw-desktop/src/app/message/settings/apps_catalog.rs` | 381 | 340 | True |
| `crates/vw-desktop/src/app/message/settings/apps_lifecycle.rs` | 32 | 32 | True |
| `crates/vw-desktop/src/app/message/settings/apps_usage_events.rs` | 2 | 2 | True |
| `crates/vw-desktop/src/app/message/settings/apps_usage_events_tests.rs` | 90 | 0 | False |
| `crates/vw-desktop/src/app/message/settings/plugin_install.rs` | 32 | 32 | True |

## 静态检查与风险

- 本任务差异 git diff --check 无空白错误；独立 rustfmt 仅处理新增/修改领域文件，未调用 cargo fmt。
- 使用路径清单和源码查找核对旧 mod、cfg、--exact、native binary 的路径入口与 verify_help 调用者；领域没有 crate::app/宿主路径 include!。
- 静态读取本地 Cargo.toml 运行依赖路径图，settings-apps 可达415个本地清单，修正后无循环。仅源码路径图，不等同 Cargo 依赖解析；首次 Python tomllib 不可用，改用依赖段路径读取，未安装工具。
- 保留原生入口的 not(wasm32) 范围；host_module_25_tests 页面族跟随 native 验收依赖限定非 wasm。没有进行 wasm 验证。
- 未编译或执行，因此类型/链接、实际控件树路径、GPU/字体整形及系统权限仍须以后按用户授权验证；不声称通过数、无 panic 或性能提升。

仅以后明确授权时：

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-settings-apps --lib host_module_25_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-usage --lib apps_usage_cards::tests::usage_events_25_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib
```

共享控件原有直接调用者的绘制测试后续过滤：desktop-chat-view 的 host_module_5_tests::markdown_selection_tests；原生五个二进制仍需用户单独要求才可运行。
