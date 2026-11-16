# 任务21：网络与网关设置整组

静态交付；按用户要求未执行任何 Cargo、测试、窗口绘制或回归。独立 rustfmt 仅解析迁入文件；本任务 diff --check 无空白错误。

## 生产调用链与职责

App 的 browser/gateway/gateway_client 字段使用 desktop-settings-network::state，HTTP/search/proxy/tunnel 使用同库 app_init::state；本轮读取确认既有真实接线，无平行状态或转换实现。页面 view/content/overlay → 对应领域 View；SettingsMessage → 领域 message::update → Effect → 宿主持久化、Task、toast、剪贴板和跨域连接刷新。安全校验、探测及失败回滚仍使用原实现。

生产候选387行原已为装配入口；本轮移除页面测试挂载，没有将测试减量称为生产迁出。browser 保留消息映射和聊天帮助渲染边界；gateway 保留标签搜索装配及跨域 General 标签外观比较；client 保留选端点后的项目偏好、归档和 worktree 刷新；HTTP/proxy/tunnel/search 保留异步配置持久化边界；gateway 保留服务恢复生命周期与 wasm 拒绝路径。browser_tests 的无关消息路由契约留宿主。

## 测试与断言归属

四组 UI 文件完整迁入 host_module_21_tests 对应子模块；所有几何、中文、明暗主题、事件、draw、screenshot 断言保留。仅将 App 构造、全局配置断言与宿主消息外壳替换为局部 NetworkFixture、真实领域 Message 和 View。网络导航比较用 NetworkTab；保存失败直接调用领域 save_feedback。帮助文本使用相同 desktop-chat-view selectable_titled_text 实现，测试专用 OpenUrl 事件承接真实右键菜单边界。

代理 help_rollout_17 全部断言迁入 proxy_tests；宿主仅保留 rollout helper，因为其它任务拥有的 dialogue_flow/projects/hooks/tunnel 仍直接调用；不删除其它任务调用链。网关 settings_tabs_core_appearance_gateway 保留：它比较 General 与网关标签真实跨域外观，不在领域重新实现 General。网关客户端 remote_save 两项保留，验证远端持久化调度、启动陈旧回执、租户请求头与跨域偏好刷新。其余七项客户端状态测试完整迁入 gateway_client_update；toast 断言独立保留为宿主 invalid_tenant_effect_shows_host_toast 契约。

前轮 message/gateway_client_tests 验证 helper、状态错误和保存回滚；本轮保留来源连续交互场景及全部原断言，没有按相同文件名删除已有测试。

- `crates/vw-desktop/src/app/components/system_settings_browser.rs`：基线 57 → 当前 54；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/components/system_settings_browser_ui_refresh_26_tests.rs`：基线 748 → 当前 0；已迁入领域测试。
- `crates/vw-desktop/src/app/components/system_settings_gateway.rs`：基线 76 → 当前 59；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/components/system_settings_gateway_client.rs`：基线 23 → 当前 23；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/components/system_settings_gateway_client_ui_refresh_24_tests.rs`：基线 535 → 当前 0；已迁入领域测试。
- `crates/vw-desktop/src/app/components/system_settings_gateway_tests.rs`：基线 30 → 当前 30；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/components/system_settings_gateway_ui_refresh_23_tests.rs`：基线 858 → 当前 0；已迁入领域测试。
- `crates/vw-desktop/src/app/components/system_settings_http_request.rs`：基线 20 → 当前 14；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/components/system_settings_http_request_ui_refresh_25_tests.rs`：基线 1084 → 当前 0；已迁入领域测试。
- `crates/vw-desktop/src/app/components/system_settings_proxy.rs`：基线 20 → 当前 20；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/components/system_settings_proxy_tests.rs`：基线 366 → 当前 306；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/browser.rs`：基线 20 → 当前 20；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/browser_tests.rs`：基线 15 → 当前 15；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/gateway.rs`：基线 64 → 当前 64；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/gateway_client.rs`：基线 45 → 当前 45；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/gateway_client_tests.rs`：基线 555 → 当前 324；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/http_request.rs`：基线 13 → 当前 13；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/proxy.rs`：基线 23 → 当前 23；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/tunnel.rs`：基线 13 → 当前 13；保留宿主装配/跨域契约（见上文）。
- `crates/vw-desktop/src/app/message/settings/web_search.rs`：基线 13 → 当前 13；保留宿主装配/跨域契约（见上文）。

同口径来源总计 4578 → 1036，宿主净减 3542 行；本任务未新建宿主文件。

### 逐用例映射（用例内原断言随源码迁移）

- `help_rollout_17_renders_contextual_help` → `crates/libs/desktop-settings-network/src/host_module_21_tests/proxy_tests.rs`。
- `ui_refresh_25_layout_and_interaction` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `ui_refresh_25_preserves_values_and_disabled_states` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `ui_refresh_25_provider_keyboard_and_cancel` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `help_rollout_16_renders_contextual_help` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `help_rollout_16_http_preserves_permissions_and_errors` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `help_rollout_16_browser_preserves_native_and_sidecar_controls` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `settings_help_gap_08_brave_credentials_and_result_range` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `settings_help_gap_08_http_limits_and_permissions` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `settings_help_gap_08_browser_backend_and_domain_help` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `settings_help_gap_08_tunnel_examples_and_restart` → `crates/libs/desktop-settings-network/src/host_module_21_tests/http_request_ui_refresh_25.rs`。
- `ui_refresh_24_layout_and_interaction` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_ui_refresh_24.rs`。
- `ui_refresh_24_preserves_values_and_disabled_states` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_ui_refresh_24.rs`。
- `help_rollout_19_renders_contextual_help` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_ui_refresh_24.rs`。
- `ui_refresh_26_layout_and_interaction` → `crates/libs/desktop-settings-network/src/host_module_21_tests/browser_ui_refresh_26.rs`。
- `ui_refresh_26_tunnel_keyboard_cancel_and_confirm` → `crates/libs/desktop-settings-network/src/host_module_21_tests/browser_ui_refresh_26.rs`。
- `ui_refresh_26_preserves_values_and_disabled_states` → `crates/libs/desktop-settings-network/src/host_module_21_tests/browser_ui_refresh_26.rs`。
- `ui_refresh_26_proxy_scope_hover_and_help` → `crates/libs/desktop-settings-network/src/host_module_21_tests/browser_ui_refresh_26.rs`。
- `browser_install_help_renders_chinese_in_light_and_dark_themes` → `crates/libs/desktop-settings-network/src/host_module_21_tests/browser_ui_refresh_26.rs`。
- `ui_refresh_23_layout_and_interaction` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_ui_refresh_23.rs`。
- `ui_refresh_23_preserves_values_and_disabled_states` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_ui_refresh_23.rs`。
- `ui_refresh_23_scrolls_skeys_and_keeps_row_indices` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_ui_refresh_23.rs`。
- `help_rollout_18_renders_contextual_help` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_ui_refresh_23.rs`。
- `settings_help_gap_08_gateway_listener_and_skey_help` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_ui_refresh_23.rs`。
- `gateway_client_tenant_messages_persist_switch_and_reject_blank_inputs` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_update.rs`。
- `gateway_client_update_adds_selects_and_removes_servers` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_update.rs`。
- `gateway_client_update_ignores_other_settings_messages_and_unknown_server` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_update.rs`。
- `gateway_client_update_normalizes_and_persists_editable_fields` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_update.rs`。
- `gateway_client_update_rejects_add_and_switch_when_current_tenant_is_invalid` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_update.rs`。
- `gateway_client_update_requires_valid_remove_request_and_confirmation` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_update.rs`。
- `gateway_client_update_removes_selected_server_health_and_selects_first` → `crates/libs/desktop-settings-network/src/host_module_21_tests/gateway_client_update.rs`。

## 依赖及未验证风险

新增的 renderer、image、tempfile、聊天选择文本和设置帮助依赖仅用于领域测试；领域生产不依赖宿主或宿主源码。Cargo.lock 仅同步已有包的依赖边，未运行 Cargo。局部 renderer 不再加载完整应用设置壳；真实壳与本机权限仍须用户后续授权验收，当前不宣称布局、字体整形或编译通过。

后续授权命令：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-network --lib host_module_21_tests -- --test-threads=1`；宿主检查 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。
