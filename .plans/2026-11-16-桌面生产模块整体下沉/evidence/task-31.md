# 任务31：宿主状态初始化与网关薄化（部分交付）

状态：已阻塞。前置任务38仍明确标记已阻塞，混合 App 弹窗/反馈夹具与领域状态尚未收口。公共规则要求任务31等待前置领域补齐后收口；本次只完成可独立实施的网关转发清理、导航构造和导航偏好转换。不声称整体验收完成，保留任务卡。

## 已实现的生产路径

- App::new → desktop_navigation::app_init::{work_workspace,workspace_mode,layout_drag,panels,tabs,sidebar} → 既有领域 State；companion 使用既有 Default。保留首页中文标题、控件 ID、宽度边界和全部初始字段值。
- App::apply_bootstrap_app_config → desktop_navigation::app_init::apply_preferences；导航面板、工作模式、侧栏、外部应用选择和拖拽尺寸回填归导航库。缺失或无效配置保留当前值，正在拖拽时不回填宽度。宿主仍负责先过滤已编辑偏好。
- 删除 session_gateway.rs 的114个请求/作用域/错误分类转发函数。全部真实调用者直接使用 desktop_session::session_gateway 的既有公开入口；需要端点者显式传入 config::gateway_client_endpoint()，不需要者直接复用 client 或纯函数。
- Task::perform 原来接收异步转发函数的地方以 async move 保留端点在任务被轮询时解析的时机；未增加缓存端点、客户端、重试或权限分支。
- 网关壳仅保留活动租户配置装配及当前端点契约测试注册，测试专用 gateway_client 仍委托既有 config 入口。
- 没有新增依赖或循环依赖，没有将宿主源码 path/include 到库中，没有引入 App 替代体或 Deref 状态隐藏。

## 初始化与更新尚未完成项

- app_state.rs：导航已有领域 State 继续持有；聊天呈现缓存、会话输入/权限、项目预览、任务看板、反馈状态仍为散落字段。本次未修改字段所有权，不将它们全都解释为系统资源；需在前置38完成其反馈/弹窗边界后逐组归入真实领域状态。
- new.rs：除上述导航构造外，聊天缓存、任务看板、设置、主题和系统资源初始化仍留宿主；主题/进程资源和跨域顺序可保留装配，领域初始值仍需归库。
- app_update.rs：apply_project_chat_preferences、load_startup_sessions、apply_startup_gateway_snapshot、apply_bootstrap_app_config 的非导航部分、apply_system_settings_snapshot、复制反馈和主 update 分支还没有完成逐域转换/效果分离；不能声称 update 已仅路由。
- new/settings.rs：model_routes/skills/autonomy 的局部构造与编辑保护应用仍有待归库；现有其余构造重导出不重复实现。
- startup.rs：原生 StartupConfiguration 的宿主包装及跨域 worktree 合并仍需审计，未用搬文件代替职责收口。
- 本次不越权修改任务38独占来源、删除其测试或将其缺口转给任务43；恢复条件是前置38交付真实领域边界，再继续本卡余下实施。用户禁测仅限制验证，不是上述实施缺口的原因。

## 基线同口径统计（pathlib.read_text().splitlines）

| 来源 | 基线 | 当前 | 减少 |
| --- | ---: | ---: | ---: |
| `crates/vw-desktop/src/app/impls/app_init/external_apps.rs` | 2 | 2 | 0 |
| `crates/vw-desktop/src/app/impls/app_init/new/load_tests.rs` | 6 | 6 | 0 |
| `crates/vw-desktop/src/app/impls/app_init/new/settings.rs` | 146 | 146 | 0 |
| `crates/vw-desktop/src/app/impls/app_init/new/settings_tests.rs` | 63 | 63 | 0 |
| `crates/vw-desktop/src/app/impls/app_init/new/state_tests.rs` | 6 | 6 | 0 |
| `crates/vw-desktop/src/app/impls/app_init/new.rs` | 1099 | 919 | 180 |
| `crates/vw-desktop/src/app/impls/app_init/settings_builders.rs` | 7 | 7 | 0 |
| `crates/vw-desktop/src/app/impls/app_init/startup.rs` | 129 | 129 | 0 |
| `crates/vw-desktop/src/app/impls/app_init/startup_tests.rs` | 260 | 260 | 0 |
| `crates/vw-desktop/src/app/impls/app_update.rs` | 1054 | 1028 | 26 |
| `crates/vw-desktop/src/app/session_gateway.rs` | 1008 | 14 | 994 |
| `crates/vw-desktop/src/app/state/app_state.rs` | 1251 | 855 | 396 |

- 来源基线 5031 行，当前 3435 行，累计减少 1596 行；生产候选 4696 → 3100，累计减少 1596 行。包含前置任务已有变更，不将全部差值归本任务。
- 本会话相对实施前提交 `255d5064337c0f4fadb7775f6618ad27b75d9fa3`，所有实际修改宿主文件合计净减少 1121 行；本任务新建宿主替代文件 0 个。本次源码减量均为生产入口转发/构造/转换，未删除测试源码。
- 领域库新增代码与测试独立统计，不作为仓库总代码减少，也不承诺构建提速。

## 测试和断言归属

- host_module_31_tests 已在 desktop-navigation/lib.rs 用 cfg(test) 独立注册，新增三个真实入口用例：首屏/中文首页/几何默认值、配置宽度边界/显式开关、缺失或无效偏好/拖拽保护。只是新增源码，未运行。
- 下列原始启动测试全部原位保留，本次没有迁移/删除断言。App::new 非阻塞、跨域配置回填/本地编辑保护、实际 headless 布局等仍属于待完成宿主契约审计，未冒称领域测试已迁完：
  - `crates/vw-desktop/src/app/impls/app_init/new/load_tests.rs::load_tests_module_is_wired` → 原模块原名；断言保留。
  - `crates/vw-desktop/src/app/impls/app_init/new/settings_tests.rs::settings_tests_module_is_wired` → 原模块原名；断言保留。
  - `crates/vw-desktop/src/app/impls/app_init/new/settings_tests.rs::legacy_task_board_switches_are_ignored_after_capability_cutover` → 原模块原名；断言保留。
  - `crates/vw-desktop/src/app/impls/app_init/new/state_tests.rs::state_tests_module_is_wired` → 原模块原名；断言保留。
  - `crates/vw-desktop/src/app/impls/app_init/startup_tests.rs::startup_tests_module_is_wired` → 原模块原名；断言保留。
  - `crates/vw-desktop/src/app/impls/app_init/startup_tests.rs::build_startup_task_constructs_native_bootstrap_task` → 原模块原名；断言保留。
  - `crates/vw-desktop/src/app/impls/app_init/startup_tests.rs::app_new_does_not_wait_for_gateway_configuration` → 原模块原名；断言保留。
  - `crates/vw-desktop/src/app/impls/app_init/startup_tests.rs::native_startup_backfills_once_and_retries_only_failed_reads` → 原模块原名；断言保留。

- 直接调用者中的 account_tests、shiju_apps_tests、stream_events_tests、session_gateway_tests 仅修改领域入口路径/端点参数；身份校验、空项目拒绝、作用域、保存重试及错误分支断言保留。后续整体收口仍需逐断言核对，不能用本节代替完整迁移清单。

## 静态检查与未验证风险

- 本任务37个修改 Rust 文件完成独立 rustfmt 解析（skip_children=true，既有调用者未批量格式化）；本任务限定路径 git diff --check 无输出。全工作区 diff --check 曾报告其他会话 about_modal/task_pet/toast 的 EOF 空行，未擅自修改。
- 已读取旧转发函数完整签名/函数体和真实调用点，静态定位全部114个对应领域公开函数；域类型使用既有公开类型。没有运行 Cargo metadata/check/test/Clippy、单元测试、绘制或实机。类型检查、目标 cfg、运行和布局结果均未知。
- 新增导航用例仅待以后明确授权：`python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_31_tests -- --test-threads=1`。
- 保留宿主契约以后授权命令：`python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::impls::app_init -- --test-threads=1`、`python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::session_gateway -- --test-threads=1`；宿主目标检查：`python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。以上本轮均未执行。

## 删除转发入口清单

以下名称均直接复用 `desktop_session::session_gateway` 同名入口；未用替代壳保存转发。无真实调用者的旧入口同时删除：

- `gateway_task_plan_run_stop_with_client`
- `gateway_task_plan_run_stop_async`
- `gateway_shiju_account_login_async`
- `gateway_shiju_account_login_poll_async`
- `gateway_shiju_account_profile_async`
- `gateway_shiju_account_monthly_quota_async`
- `gateway_shiju_account_me_async`
- `gateway_shiju_account_update_async`
- `gateway_shiju_account_password_update_async`
- `gateway_shiju_app_catalog_async`
- `gateway_shiju_apps_async`
- `gateway_shiju_app_api_key_binding_async`
- `gateway_shiju_app_api_keys_async`
- `gateway_shiju_app_save_api_key_binding_async`
- `gateway_shiju_app_prepare_launch_async`
- `gateway_shiju_skill_search_async`
- `gateway_shiju_skill_installed_async`
- `gateway_shiju_system_skills_async`
- `gateway_shiju_maintained_skills_async`
- `gateway_shiju_skill_tags_async`
- `gateway_shiju_tenant_skill_tags_async`
- `gateway_shiju_skill_detail_async`
- `gateway_shiju_skill_readme_async`
- `gateway_shiju_skill_versions_async`
- `gateway_shiju_skill_remote_install_async`
- `gateway_shiju_skill_remote_install_version_async`
- `gateway_shiju_skill_remote_uninstall_async`
- `active_project_scope`
- `active_agent_scheduler_scope`
- `gateway_load_session_async`
- `gateway_bounded_session_open_with_client`
- `gateway_session_message_page_before_with_client`
- `gateway_load_session_any_async`
- `gateway_load_session_any_with_raw_async`
- `gateway_load_sessions_scoped_async`
- `gateway_load_archived_session_ids_async`
- `gateway_session_preview_meta_async`
- `gateway_session_file_path_async`
- `gateway_session_step_llm_raw_async`
- `gateway_resolve_session_scope_id_async`
- `gateway_permission_list_owned_async`
- `gateway_save_session_async`
- `gateway_session_message_delta_async`
- `gateway_set_session_scope_async`
- `gateway_current_session_scope_async`
- `gateway_question_list_async`
- `gateway_session_todo_list_async`
- `gateway_question_reply_async`
- `gateway_question_reject_async`
- `gateway_permission_list_async`
- `gateway_permission_reply_async`
- `gateway_save_agent_session_scoped_async`
- `gateway_external_apps_async`
- `gateway_external_open_async`
- `gateway_external_reveal_async`
- `gateway_skills_async`
- `gateway_skill_detail_async`
- `gateway_skill_create_async`
- `gateway_skill_install_builtin_async`
- `gateway_skill_set_enabled_async`
- `gateway_skill_delete_async`
- `gateway_skill_market_sources_async`
- `gateway_skill_market_recommended_async`
- `gateway_skill_market_skills_async`
- `gateway_skill_market_categories_async`
- `gateway_skill_market_skill_detail_async`
- `gateway_skill_market_skill_file_async`
- `gateway_skill_market_install_async`
- `gateway_skill_market_install_to_targets_async`
- `gateway_skill_market_skill_sets_async`
- `gateway_skill_market_skill_set_detail_async`
- `gateway_skill_market_skill_set_install_async`
- `gateway_plugins_async`
- `gateway_hooks_async`
- `gateway_hook_trust_async`
- `gateway_hook_revoke_async`
- `gateway_hook_audit_async`
- `gateway_plugin_set_enabled_async`
- `gateway_plugin_delete_async`
- `gateway_project_change_records_async`
- `gateway_load_session`
- `gateway_load_session_any`
- `gateway_load_sessions_scoped`
- `gateway_load_archived_session_ids`
- `gateway_session_preview_meta`
- `gateway_session_file_path`
- `gateway_resolve_session_scope_id`
- `gateway_save_session_result`
- `is_permanent_session_save_error`
- `gateway_save_session`
- `gateway_set_session_scope`
- `gateway_current_session_scope`
- `gateway_question_list`
- `gateway_question_reply`
- `gateway_question_reject`
- `gateway_permission_list`
- `gateway_permission_reply`
- `gateway_save_agent_session_scoped`
- `gateway_project_change_records`
- `gateway_plugin_prepare_async`
- `gateway_plugin_activate_async`
- `gateway_plugin_discard_async`
- `gateway_skill_repos_scope`
- `gateway_skill_repos_request`
- `gateway_claude_config_request`
- `gateway_skills_market_request`
- `gateway_skills_install_view`
- `gateway_skills_import_request`
- `gateway_skills_job`
- `gateway_skills_submit`
- `gateway_skill_bindings_request`
- `gateway_skills_mutation`
- `gateway_skill_storage_request`
- `gateway_skill_backups_request`
