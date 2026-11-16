# 任务32：配置读取与偏好持久化静态交付

完成范围：偏好作用域、项目读写、编辑保护、凭据清空 patch、保存反馈决策与本地 bootstrap 更新归入已有 desktop-config。宿主只注入当前网关/路径、映射消息并应用跨域效果。未编译、未测试；未执行任何 Cargo 命令。

## 真实调用链与状态

- App.desktop_preferences_persistence 原本已经持有 DesktopPreferencesPersistence；沿用该字段，不复制 App、不新增总控 trait。现有 app_init 初始化 Default 保持有效。
- reload_project_chat_preferences → ProjectChatPreferences::reload → PreferencesMessage::ProjectChatPreferencesLoaded → preferences_message → 宿主加载消息 → ProjectChatPreferences::loaded → 应用项目模型选择。revision、路径、网关三重校验及 pending PUT 快照均在领域。
- save_chat_model_preferences / save_automatic_model_preference → DesktopPreferencesPersistence 对应方法 → 原有全局 PATCH 和项目 PUT 队列；端点在入队前绑定，失败后仍排空后续写入，idle 保持跨两类队列一致。
- retain_unedited_desktop_preferences、desktop_preference_edited、system_settings_edited 转交领域状态。
- queue_gateway_client_snapshot → enqueue_gateway_client_snapshot 构造空凭据清理 patch → 原有系统设置队列；宿主只切换 agent_apps_auth 端点。
- system_settings_saved 先 finish_system_settings_save，再 system_settings_save_feedback；领域决定反馈类型，宿主本地化 Toast、更新 save_error 和派发 DialogueFlowUiSettingsSaved。
- save_task_pet_size → enqueue_companion_size；宿主提供宠物域当前快照并记录窗口 pending 作用域。
- update_gateway_client_config → BootstrapStore::update_gateway_client_config；宿主保留窗口 fixture 禁写断言。
- update_agents_compat_registry_async → ConfigAccess 对应方法 → ConfigSaved → 宿主 AgentConfigSaved。

## 逐文件去向和 splitlines

| 基线文件 | 基线 | 当前 | 去向/保留理由 |
|---|---:|---:|---|
| config.rs | 68 | 68 | 保留 mod/cfg、稳定导出及宿主边界测试注册。 |
| config_agent.rs | 394 | 394 | 全部现有入口是 ConfigAccess 转发；注入当前 endpoint 与本地覆盖路径，映射 ConfigSaved 为宿主消息。 |
| config_cli_bootstrap.rs | 7 | 7 | 仅注入 bootstrap 配置与本机缓存路径，生产实现已在 config_cli_bootstrap。 |
| config_cron_jobs.rs | 98 | 98 | 仅提供真实 GatewayClient；生产算法在 config_cron_jobs。 |
| config_desktop.rs | 338 | 320 | 领域队列/patch/编辑策略已迁；保留宠物跨域快照、窗口 pending、Toast、PreferencesMessage 映射及当前端点注入。 |
| config_desktop_tests.rs | 820 | 606 | 只保留下表列明的跨域契约及被 settings/channels、projects、gateway_client 测试真实调用的临时 HTTP/进程配置端口。领域队列测试无 App、无环境 bootstrap。 |
| config_file_manager.rs | 1 | 1 | 仅重新导出领域系统端口。 |
| config_gateway.rs | 69 | 69 | 仅当前配置端点/真实 Client 装配、消息桥及领域函数导出。 |
| config_gateway_tests.rs | 334 | 50 | 只保留 interactive_config_paths_reject_blocking_gateway_wrappers，读取真实宿主入口验证非阻塞契约。 |
| config_project_chat.rs | 140 | 108 | 仅传入模型/路径/端点并应用领域输出到会话模型，失败 Toast 属跨域。 |
| config_system_settings.rs | 217 | 187 | 领域处理配置、归一化、patch、反馈决策；保留目录资源、窗口测试 bootstrap 所有权及跨域设置消息。 |
| config_system_settings_tests.rs | 278 | 227 | 保留进程目录隔离断言和供 editor/settings 跨域调用的 Iced 消息/HTTP 同步端口；保存算法与源码覆盖哨兵已迁出。 |
| config_tests.rs | 13 | 13 | 保留宿主 mod 声明哨兵，验证真实装配注册。 |

宿主总量：2777 → 2148；净减少 629 行。含测试与空行，不将其冒充纯生产减量。

## 旧断言完整映射

- `config_gateway_tests.rs::interactive_config_paths_reject_blocking_gateway_wrappers` → 原宿主同名测试，全部断言保留；理由见下。
- `config_gateway_tests.rs::gateway_client_endpoint_trims_auth_and_clamps_port` → host_module_32_tests::gateway，同名完整断言。
- `config_gateway_tests.rs::gateway_client_endpoint_uses_trimmed_bearer_token_when_skey_is_blank` → host_module_32_tests::gateway，同名完整断言。
- `config_gateway_tests.rs::gateway_client_endpoint_omits_auth_key_when_credentials_are_blank` → host_module_32_tests::gateway，同名完整断言。
- `config_gateway_tests.rs::gateway_client_builds_from_configured_endpoint` → host_module_32_tests::gateway，同名完整断言。
- `config_gateway_tests.rs::load_tools_list_via_gateway_returns_normalized_tools` → host_module_32_tests::gateway，同名完整断言。
- `config_gateway_tests.rs::load_tools_list_via_gateway_returns_empty_vec_on_request_error` → host_module_32_tests::gateway，同名完整断言。
- `config_desktop_tests.rs::config_desktop_tests_keeps_planned_coverage_targets` → host_module_32_tests::desktop，同名；原 HTTP 内容、请求数、队列输出数、失败/成功、原端点、idle 和最终存储断言均保留。Toast 拆到宿主 preference_save_failure_reaches_host_toast / project_save_failure_reaches_host_toast；SendPressed 拆到 send_behavior_change_emits_send_before_preferences_complete。模型最终值断言改验领域 defaults，模型应用另由宿主项目路由覆盖。
- `config_desktop_tests.rs::app_update_source_patch_and_restart_read_use_gateway_preferences` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::app_update_source_gateway_save_failure_reaches_unsaved_toast` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::preference_save_failure_keeps_new_edits_and_preserves_unrelated_fields` → host_module_32_tests::desktop，同名；原 HTTP 内容、请求数、队列输出数、失败/成功、原端点、idle 和最终存储断言均保留。Toast 拆到宿主 preference_save_failure_reaches_host_toast / project_save_failure_reaches_host_toast；SendPressed 拆到 send_behavior_change_emits_send_before_preferences_complete。模型最终值断言改验领域 defaults，模型应用另由宿主项目路由覆盖。
- `config_desktop_tests.rs::project_puts_keep_the_original_gateway_and_drain_after_a_failed_save` → host_module_32_tests::desktop，同名；原 HTTP 内容、请求数、队列输出数、失败/成功、原端点、idle 和最终存储断言均保留。Toast 拆到宿主 preference_save_failure_reaches_host_toast / project_save_failure_reaches_host_toast；SendPressed 拆到 send_behavior_change_emits_send_before_preferences_complete。模型最终值断言改验领域 defaults，模型应用另由宿主项目路由覆盖。
- `config_desktop_tests.rs::reasoning_and_send_preferences_preserve_unrelated_sessions_and_remove_legacy_key` → host_module_32_tests::desktop，同名；原 HTTP 内容、请求数、队列输出数、失败/成功、原端点、idle 和最终存储断言均保留。Toast 拆到宿主 preference_save_failure_reaches_host_toast / project_save_failure_reaches_host_toast；SendPressed 拆到 send_behavior_change_emits_send_before_preferences_complete。模型最终值断言改验领域 defaults，模型应用另由宿主项目路由覆盖。
- `config_desktop_tests.rs::mini_companion_preferences_round_trip_preserves_other_fields` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::mini_companion_late_load_keeps_local_edit` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::mini_companion_failed_save_keeps_edit_and_can_retry` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::mini_companion_old_endpoint_snapshot_cannot_restore_visibility` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::mini_companion_queued_edits_save_latest_without_reopening_window` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::mini_companion_defaults_and_loaded_scale_are_bounded` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::mini_companion_size_failure_keeps_draft_and_retry_commits_once` → 原宿主同名测试，全部断言保留；理由见下。
- `config_desktop_tests.rs::mini_companion_size_queued_save_waits_for_its_own_response` → 原宿主同名测试，全部断言保留；理由见下。
- `config_system_settings_tests.rs::config_system_settings_tests_keeps_planned_coverage_targets` → host_module_32_tests::system_settings，同名同断言，使用真实 BootstrapStore 和临时目录。
- `config_system_settings_tests.rs::update_gateway_client_config_persists_without_a_gateway` → host_module_32_tests::system_settings，同名同断言，使用真实 BootstrapStore 和临时目录。
- `config_system_settings_tests.rs::agent_unified_selection_config_override_isolates_window_bootstrap` → 原宿主同名测试，全部断言保留；理由见下。
- `config_tests.rs::config_tests_keeps_config_module_exports_explicit` → 原宿主同名测试，全部断言保留；理由见下。

## 每组保留契约与夹具理由

- app_update_source 两例：desktop_release 来源编码/恢复与 ViewMessage 保存结果进入 Toast，跨发布域、配置域和宿主更新路由；不迁入配置库造成反向依赖。
- mini_companion round_trip：配置传输→StartupAppConfigLoaded→宠物状态→窗口打开；保留窗口 id 断言。
- mini_companion late_load：StartupAppConfigLoaded 在本地宠物编辑后不得覆盖窗口/头像/scale/visibility；验证真实宿主启动应用顺序。
- mini_companion failed_save：ViewMessage 头像选择→保存失败→Toast→重试；窗口不重开。
- mini_companion old_endpoint：StartupGatewaySnapshotLoaded 的代际隔离由启动域负责，断言宠物窗口不可由旧快照开启。
- mini_companion queued_edits：宿主各宠物字段快照接入配置队列，最新头像/比例/可见性落盘但不重开窗口。
- mini_companion defaults_and_loaded_scale：真实 StartupAppConfigLoaded 调用 desktop_task_board 的宠物解析，断言未知头像回退及 scale 上下界；既有底层实现已在 pet_state，不在配置宿主重复算法。
- mini_companion size_failure / size_queued：ViewMessage 弹窗草稿、重复点击抑制、pending 消息归属、Toast、保存后提交与窗口状态，属于真实跨域效果；保留全部请求及草稿断言。
- agent_unified_selection_config_override_isolates_window_bootstrap：BootstrapStore 与宿主 project_dirs 数据目录在同一显式目录根下隔离，是真实系统资源边界。修正原已不存在的 legacy getter 调用为实际 BootstrapStore 方法。
- HTTP 宿主夹具仍被其它领域测试调用，保留最小端口实现；领域队列采用直接 endpoint 的精简 HTTP fixture，不复制 App 或进程配置夹具。

## cfg、依赖与静态验证

lib.rs 用 cfg(test) 注册 host_module_32_tests；gateway、desktop、system_settings 子模块用非 wasm cfg 注册。领域测试无 crate::app、反向 path/include 或宿主源码引用；源码哨兵只读本领域生产文件。未修改 Cargo 清单或新增依赖。

所有修改 Rust 文件独立 rustfmt --emit stdout 解析，输出丢弃以避免扩大格式改动；git diff --check 按本任务文件检查。未编译、未测试，不声称通过数。后续仅用户另行授权验证：
`python3 scripts/cargo_slot.py cargo test -p desktop-config --lib host_module_32_tests -- --test-threads=1`

新增覆盖保留编辑字段保护、旧 revision/其它项目拒绝、保存反馈 scope 隔离。宿主跨域用例后续分别使用 vw-desktop --lib config_desktop_tests / config_system_settings_tests / config_gateway_tests 过滤，仍需用户另行授权。
