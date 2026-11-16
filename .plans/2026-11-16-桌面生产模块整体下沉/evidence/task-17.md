# 任务17静态交付证据

本轮未运行任何 Cargo、测试、绘制、实机或回归。静态交付不表示编译或行为通过。

## 生产调用链与状态归属

- 模型：宿主 `models::update` 映射旧消息 → 领域 `models::update` → `model_update::persist_status_effect` → `prepare_status_save`/既有网关任务。保存门控、乐观状态、回滚、完成反馈均归领域；自动模型会话偏好、滚动效果由宿主装配。
- 路由：App 持有领域 `ModelRoutesSettingsState`；宿主消息映射 → `model_routes::edit_draft`（编辑、revision、校验、反馈、有效投影）→ 宿主同步 runtime 查询分类并调用共享 routing persistence。Loaded 同时回写两领域，保留在宿主。
- 页面：宿主组合四个领域子页 → `models_page::view` 统一标签、间距、错误横幅。标签元数据也归 providers；搜索真实调用者适配领域描述符。目录、详情、提供者弹窗与嵌入路由继续调用既有领域视图。
- 聚合：宿主从 App 构造当前 scope/project/access/授权上下文 → `desktop-settings-apps::aggregation::*::update` → 既有 SDK；回包消息和 toast 映射保留宿主。生产算法没有复制。
- 字段：model/provider/embedding 使用既有领域状态；新增归属为 ModelRoutesSettingsState（routes/save_error/persistence）。App 字段类型经原 state 再导出改接，无新增 App 副本、Deref 或反向宿主依赖。
- 密钥：不新增日志或存储路径，继续使用既有草稿、遮蔽控件、网关与凭据存储。遮蔽像素和输入校验测试源码迁入 custom_values。

## 每个宿主生产函数的保留理由

| 来源 | 函数 | 必要装配 |
|---|---|---|
| `crates/vw-desktop/src/app/components/system_settings_embedding_routes.rs` | `map_message` | 将领域 Element 的局部消息映射全局消息。 |
| `crates/vw-desktop/src/app/components/system_settings_embedding_routes.rs` | `embedded_view` | 将领域 Element 的局部消息映射全局消息。 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | `active_tab_save_error` | 组合 providers/runtime 子页及 locale 错误投影；领域事件映射全局消息；selectable 帮助 callback 依赖聊天文本复制能力。 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | `view` | 组合 providers/runtime 子页及 locale 错误投影；领域事件映射全局消息；selectable 帮助 callback 依赖聊天文本复制能力。 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | `main_view` | 组合 providers/runtime 子页及 locale 错误投影；领域事件映射全局消息；selectable 帮助 callback 依赖聊天文本复制能力。 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | `view_overlays` | 组合 providers/runtime 子页及 locale 错误投影；领域事件映射全局消息；selectable 帮助 callback 依赖聊天文本复制能力。 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | `map_view_message` | 组合 providers/runtime 子页及 locale 错误投影；领域事件映射全局消息；selectable 帮助 callback 依赖聊天文本复制能力。 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | `view_style` | 组合 providers/runtime 子页及 locale 错误投影；领域事件映射全局消息；selectable 帮助 callback 依赖聊天文本复制能力。 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | `help_text` | 组合 providers/runtime 子页及 locale 错误投影；领域事件映射全局消息；selectable 帮助 callback 依赖聊天文本复制能力。 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | `map_catalog_event` | 组合 providers/runtime 子页及 locale 错误投影；领域事件映射全局消息；selectable 帮助 callback 依赖聊天文本复制能力。 |
| `crates/vw-desktop/src/app/components/system_settings_providers/catalog.rs` | `view_overlays` | 组合底层设置页、领域卡片与宿主消息；catalog deployments 同时使用 provider app 表单。 |
| `crates/vw-desktop/src/app/components/system_settings_providers/catalog.rs` | `deployments` | 组合底层设置页、领域卡片与宿主消息；catalog deployments 同时使用 provider app 表单。 |
| `crates/vw-desktop/src/app/components/system_settings_providers/connect.rs` | `view_overlays` | 将领域卡片/关闭消息与底层宿主设置页叠加。 |
| `crates/vw-desktop/src/app/components/system_settings_providers/connected.rs` | `view` | 提供者局部视图与宿主事件映射。 |
| `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal.rs` | `view_overlays` | 将领域叠层与宿主 Element/消息接合；响应帮助 callback 连接宿主 selectable 文本。 |
| `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal.rs` | `responsive_field_with_help` | 将领域叠层与宿主 Element/消息接合；响应帮助 callback 连接宿主 selectable 文本。 |
| `crates/vw-desktop/src/app/components/system_settings_providers/custom_provider.rs` | `view_overlays` | 将领域卡片/关闭事件叠加到宿主页面。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_clients.rs` | `message` | 局部结果包入 app provider list 的全局消息。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_desktop.rs` | `current` | 读取授权 scope/project/token/access，连到已有领域控制器并映射 toast；binding_is_current 比对 App 当前上下文。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_desktop.rs` | `message` | 读取授权 scope/project/token/access，连到已有领域控制器并映射 toast；binding_is_current 比对 App 当前上下文。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_desktop.rs` | `update` | 读取授权 scope/project/token/access，连到已有领域控制器并映射 toast；binding_is_current 比对 App 当前上下文。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_desktop.rs` | `binding_is_current` | 读取授权 scope/project/token/access，连到已有领域控制器并映射 toast；binding_is_current 比对 App 当前上下文。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_list.rs` | `current` | 从 App 的已选 scope/project/catalog generation 构造领域上下文；委派更新并映射 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_list.rs` | `message` | 从 App 的已选 scope/project/catalog generation 构造领域上下文；委派更新并映射 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_list.rs` | `update` | 从 App 的已选 scope/project/catalog generation 构造领域上下文；委派更新并映射 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_omo.rs` | `current` | 从 App 读取 scope/project/授权，构造 SDK 边界；委派领域更新并映射消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_omo.rs` | `message` | 从 App 读取 scope/project/授权，构造 SDK 边界；委派领域更新并映射消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_omo.rs` | `update` | 从 App 读取 scope/project/授权，构造 SDK 边界；委派领域更新并映射消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_omp.rs` | `message` | 委派领域更新，借用 App 当前 scope/project/表单并映射消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_omp.rs` | `update` | 委派领域更新，借用 App 当前 scope/project/表单并映射消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_pi.rs` | `message` | 委派领域更新，借用 App 当前 scope/project/表单并映射消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_pi.rs` | `update` | 委派领域更新，借用 App 当前 scope/project/表单并映射消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_publish.rs` | `message` | 局部结果包入 app provider list 的全局消息。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_subagent.rs` | `message` | 组合执行主机、项目、授权上下文，委派领域控制器并装配消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_subagent.rs` | `current` | 组合执行主机、项目、授权上下文，委派领域控制器并装配消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/aggregation_subagent.rs` | `update` | 组合执行主机、项目、授权上下文，委派领域控制器并装配消息与 toast。 |
| `crates/vw-desktop/src/app/message/settings/embedding_routes.rs` | `update` | 将领域有效配置保存效果送到宿主持有的配置异步端口。 |
| `crates/vw-desktop/src/app/message/settings/model_routes.rs` | `queue_valid_routes` | 局部消息映射；同时同步 providers 路由与 runtime 分类；共享 revision 保存完成/加载同时投影两领域。 |
| `crates/vw-desktop/src/app/message/settings/model_routes.rs` | `update` | 局部消息映射；同时同步 providers 路由与 runtime 分类；共享 revision 保存完成/加载同时投影两领域。 |
| `crates/vw-desktop/src/app/message/settings/models.rs` | `persist_status_effect` | 将局部任务映射全局消息；自动模型偏好涉及当前会话；搜索滚动涉及宿主 popover。 |
| `crates/vw-desktop/src/app/message/settings/models.rs` | `models_refresh_task` | 将局部任务映射全局消息；自动模型偏好涉及当前会话；搜索滚动涉及宿主 popover。 |
| `crates/vw-desktop/src/app/message/settings/models.rs` | `update` | 将局部任务映射全局消息；自动模型偏好涉及当前会话；搜索滚动涉及宿主 popover。 |
| `crates/vw-desktop/src/app/message/settings/models.rs` | `map_model_result` | 将局部任务映射全局消息；自动模型偏好涉及当前会话；搜索滚动涉及宿主 popover。 |

## 逐用例断言归属

保持原测试名；同名拆分表示同一原用例的领域部分与宿主契约分别落地，原断言按状态所有者分离，不以名称推断等价。

| 原用例 | 现存归属 |
|---|---|
| `ui_refresh_11_layout_and_interaction` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/routes_ui.rs`; `crates/libs/desktop-settings-runtime/src/host_module_17_tests.rs` |
| `ui_refresh_11_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-runtime/src/host_module_17_tests.rs` |
| `ui_refresh_11_draft_messages_keep_route_indices_and_values` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/routes_ui.rs`; `crates/libs/desktop-settings-runtime/src/host_module_17_tests.rs` |
| `ui_refresh_11_long_menu_scrolls_and_cancel_does_not_click_through` | `crates/libs/desktop-settings-runtime/src/host_module_17_tests.rs` |
| `ui_refresh_11_empty_catalog_and_numeric_limits_keep_existing_actions` | `crates/libs/desktop-settings-runtime/src/host_module_17_tests.rs` |
| `help_rollout_07_renders_contextual_help` | `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs` |
| `help_rollout_07_model_route_renders_contextual_help` | `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs` |
| `help_rollout_07_query_classification_renders_contextual_help` | `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs` |
| `help_rollout_07_embedding_route_renders_contextual_help` | `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs` |
| `help_rollout_27_multimodal_renders_contextual_help` | `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs` |
| `settings_help_gap_04_preserves_multimodal_controls_and_catalog_feedback` | `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs` |
| `settings_help_gap_04_migrates_only_static_multimodal_copy` | `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs` |
| `model_settings_error_banner_is_local_to_tabbed_view` | `crates/vw-desktop/src/app/components/system_settings_models_tests.rs` |
| `ui_refresh_10_layout_and_interaction` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/models_ui.rs`; `crates/vw-desktop/src/app/components/system_settings_models_ui_refresh_10_tests.rs` |
| `ui_refresh_10_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/models_ui.rs` |
| `ui_refresh_10_detail_scrolls_and_closes_without_saving` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/models_ui.rs` |
| `settings_tabs_core_appearance_models` | `crates/vw-desktop/src/app/components/system_settings_models_ui_refresh_10_tests.rs` |
| `settings_help_gap_03_provider_id_and_account_auth` | `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs` |
| `settings_help_gap_03_model_limits_and_modalities` | `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs` |
| `settings_help_gap_03_preserves_masks_and_validation` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/custom_values.rs` |
| `help_rollout_02_renders_contextual_help` | `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs` |
| `help_rollout_02_dismisses_menu_before_card_and_keeps_modal_actions` | `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs` |
| `help_rollout_02_keeps_help_inside_the_window_and_clips_scrolled_triggers` | `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs` |
| `ui_refresh_09_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/custom_values.rs` |
| `settings_tabs_editors_appearance_model` | `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs` |
| `settings_tabs_editors_appearance_provider` | `crates/vw-desktop/src/app/components/system_settings_providers/custom_provider_tests.rs` |
| `aggregation_t63_draws_chinese_mapping_in_light_and_dark` | `crates/libs/desktop-settings-apps/src/host_module_17_tests/desktop.rs` |
| `aggregation_t64_actual_text_rendering_in_both_themes` | `crates/libs/desktop-settings-apps/src/host_module_17_tests/omo.rs` |
| `aggregation_t65_renders_dynamic_chinese_in_light_and_dark` | `crates/libs/desktop-settings-apps/src/host_module_17_tests/pi.rs` |
| `aggregation_t61_aggregation_review_preserves_contract` | `crates/libs/desktop-settings-apps/src/host_module_17_tests/review.rs` |
| `aggregation_t61_actual_text_rendering_in_both_themes` | `crates/libs/desktop-settings-apps/src/host_module_17_tests/review.rs` |
| `aggregation_t60_actual_text_rendering_in_both_themes` | `crates/libs/desktop-settings-apps/src/host_module_17_tests/subagent.rs` |
| `model_routes_update_syncs_query_classification_state` | `crates/vw-desktop/src/app/message/settings/model_routes_tests.rs` |
| `invalid_route_edits_update_fields_and_expose_validation_errors` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/routes_update.rs` |
| `remove_route_updates_existing_rows_and_ignores_out_of_range_indexes` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/routes_update.rs`; `crates/vw-desktop/src/app/message/settings/model_routes_tests.rs` |
| `update_ignores_messages_for_other_settings_sections` | `crates/vw-desktop/src/app/message/settings/model_routes_tests.rs` |
| `route_and_classification_edits_coalesce_without_blocking` | `crates/vw-desktop/src/app/message/settings/model_routes_tests.rs` |
| `model_toggle_updates_locally_and_starts_save_without_reloading` | `crates/libs/desktop-settings-providers/src/host_module_17_tests.rs`; `crates/vw-desktop/src/app/message/settings/models_tests.rs` |
| `disabling_selected_model_switches_back_to_auto_model` | `crates/vw-desktop/src/app/message/settings/models_tests.rs` |
| `enabling_model_updates_locally_and_schedules_save_without_forcing_auto_model` | `crates/libs/desktop-settings-providers/src/host_module_17_tests.rs`; `crates/vw-desktop/src/app/message/settings/models_tests.rs` |
| `provider_set_all_only_changes_models_in_selected_group` | `crates/libs/desktop-settings-providers/src/host_module_17_tests.rs` |
| `provider_reset_optimistically_disables_only_selected_group` | `crates/libs/desktop-settings-providers/src/host_module_17_tests.rs` |
| `models_set_all_updates_every_connected_model_with_one_save` | `crates/libs/desktop-settings-providers/src/host_module_17_tests.rs` |
| `model_status_save_error_rolls_back_local_changes` | `crates/libs/desktop-settings-providers/src/host_module_17_tests.rs` |
| `model_status_save_success_keeps_optimistic_state` | `crates/libs/desktop-settings-providers/src/host_module_17_tests.rs` |
| `provider_changes_update_available_models_without_opening_model_settings` | `crates/libs/desktop-settings-providers/src/host_module_17_tests/provider_projection.rs` |

原模型状态成功/失败/批量/重置断言归 providers；toggle/enabling 的自动模型偏好断言留宿主，移除重复局部模型状态断言。提供者回包到模型摘要完全归 providers。route removal 的行状态断言归领域，分类 enabled/rules/category 断言留跨域宿主。无关消息忽略与两页并发持久化仍验证宿主路由契约。

前轮 desktop_split_11_tests/custom_model 已有几何布局断言继续保留，本轮 custom_values 复用其真实控件场景夹具，未复制同套布局断言。models 页原 layout 用例按目录布局/跨页标签路由分开；embedding 文件 layout/draft 用例按 providers 与 runtime 两组拆分，截图、菜单滚动、数值边界均保留。

## 宿主测试残留审计

- custom_model_modal：仅保留系统设置叠层、上下文帮助的 selectable 菜单/复制/外链/裁剪/关闭顺序、与 General 基准比较的标签样式；输入值、禁用状态、保存完成、密钥遮蔽、输入校验均迁领域。保留 App 仅驱动真实系统设置叠层和全局事件。
- custom_provider：保留 General 基准与 provider 标签切换的整页装配契约；真实 tab appearance 场景跨 settings General 与 providers，不能在 providers 重建 General 基准。
- models：保留四领域错误横幅投影、标签路由与 General 基准比较；目录与详情原生绘制/交互迁出。
- embedding_routes：仅保留提供者/runtime 页面到宿主 selectable 帮助菜单、复制与外链，以及模型目录到多模态列表的真实整页投影。纯控件/布局/草稿/菜单滚动迁出。
- model_routes：仅保留与 runtime 分类同步、共享持久化调度、无关全局消息忽略。
- models update：仅保留自动模型会话偏好同步契约。
- aggregation 五组渲染测试均独立注册于 desktop-settings-apps::host_module_17_tests；Desktop 使用真实 mapping_fields + desktop_view，外部事件仅是无执行的消息边界。

## splitlines 行数

含注释、空行，按 baseline.files[].task=17 口径；无新增宿主替代文件。

| 来源 | 基线 | 当前 |
|---|---:|---:|
| `crates/vw-desktop/src/app/components/system_settings_embedding_routes.rs` | 13 | 13 |
| `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs` | 922 | 604 |
| `crates/vw-desktop/src/app/components/system_settings_models.rs` | 252 | 217 |
| `crates/vw-desktop/src/app/components/system_settings_models_tests.rs` | 42 | 42 |
| `crates/vw-desktop/src/app/components/system_settings_models_ui_refresh_10_tests.rs` | 507 | 216 |
| `crates/vw-desktop/src/app/components/system_settings_providers/catalog.rs` | 12 | 12 |
| `crates/vw-desktop/src/app/components/system_settings_providers/connect.rs` | 7 | 7 |
| `crates/vw-desktop/src/app/components/system_settings_providers/connected.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal.rs` | 28 | 28 |
| `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs` | 1177 | 876 |
| `crates/vw-desktop/src/app/components/system_settings_providers/custom_provider.rs` | 7 | 7 |
| `crates/vw-desktop/src/app/components/system_settings_providers/custom_provider_tests.rs` | 64 | 64 |
| `crates/vw-desktop/src/app/message/settings/aggregation_clients.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/message/settings/aggregation_desktop.rs` | 48 | 48 |
| `crates/vw-desktop/src/app/message/settings/aggregation_desktop_tests.rs` | 277 | 0 |
| `crates/vw-desktop/src/app/message/settings/aggregation_list.rs` | 35 | 35 |
| `crates/vw-desktop/src/app/message/settings/aggregation_omo.rs` | 44 | 41 |
| `crates/vw-desktop/src/app/message/settings/aggregation_omo_tests.rs` | 91 | 0 |
| `crates/vw-desktop/src/app/message/settings/aggregation_omp.rs` | 35 | 35 |
| `crates/vw-desktop/src/app/message/settings/aggregation_pi.rs` | 42 | 42 |
| `crates/vw-desktop/src/app/message/settings/aggregation_pi_tests.rs` | 104 | 0 |
| `crates/vw-desktop/src/app/message/settings/aggregation_publish.rs` | 4 | 4 |
| `crates/vw-desktop/src/app/message/settings/aggregation_review_tests.rs` | 223 | 0 |
| `crates/vw-desktop/src/app/message/settings/aggregation_subagent.rs` | 58 | 55 |
| `crates/vw-desktop/src/app/message/settings/aggregation_subagent_tests.rs` | 160 | 0 |
| `crates/vw-desktop/src/app/message/settings/embedding_routes.rs` | 10 | 10 |
| `crates/vw-desktop/src/app/message/settings/model_routes.rs` | 109 | 110 |
| `crates/vw-desktop/src/app/message/settings/model_routes_tests.rs` | 306 | 263 |
| `crates/vw-desktop/src/app/message/settings/models.rs` | 110 | 106 |
| `crates/vw-desktop/src/app/message/settings/models_tests.rs` | 353 | 73 |

来源合计 5048 → 2916，净减 2132 行。生产候选单列 822 → 778 行；不将测试减少计作生产减少。
额外直接调用者：state/settings/provider_model.rs 再导出领域状态，components/system_settings.rs 适配模型标签描述符。均无新增宿主文件。

## 静态验证与风险

独立 rustfmt 只用于修改源码解析；任务路径 git diff --check。注册核对 providers/models_page 与 providers/apps/runtime 的 host_module_17_tests；移除旧聚合测试挂载；没有 path/include 反向加载宿主。Cargo.lock 仅文本维护已存在依赖项。
未编译、未测试；泛型/lifetime/类型检查、真实字体整形和窗口截图结果须以后用户授权才可验证。
以后可用：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-providers --lib host_module_17_tests -- --test-threads=1`；apps/runtime 同名过滤分别运行；宿主目标检查沿任务卡记录命令，本轮没有执行。
