# 任务40：原生验收入口与领域夹具

静态交付完成，未编译、未测试。遵守本计划禁测要求，未执行 Cargo、窗口或实机。

## 归属与调用链

- cc04_provider_window_main → desktop_provider_apps::acceptance::{fixture,create_draft} → acceptance/cc04_*。保留回环地址、停止信号、线程 join、HTTP 断言和原状态机。宿主只负责 App 初始化、消息映射、窗口和字体。
- ACP options、Agent entries、companion items、editor tab、voice setup 分别由 desktop-chat-input、desktop-settings-agents、desktop-task-board、desktop-editor、desktop-settings-voice 持有，原生入口直接调用；无 App 参数或反向路径。
- 工作树入口 → desktop_settings_workspace::acceptance::{setup,view} → 真实 page/editor/overlay；宿主持有窗口、截图、跨域路由及 endpoint 能力判定。4px 滚动条与7px边距、明暗主题与中文不变。
- tray_window → desktop_navigation::acceptance::tray_view；NSApplication/NSMenu/TrayIcon 及恢复 App 窗口契约仍由宿主持有。host_module_40_tests 已 cfg(test) 注册，包含 tray_fixture_builds；它只检查构造，不能证明实际字体绘制。
- 所有 apps_*、archived、cron、plugin、recovery 薄入口原样保留：只注册原生验收与调用各领域已分配模块；没有移动或删除其覆盖。
- local_speech_system_main 的 run/validate 全部为 Apple 主线程、物理语音合成/识别、权限和取消边界，保留。voice 的 live preflight/真人确认、App 设置路由、截图/播放及取消时限为真实系统或跨域契约，保留。
- companion 的 boot/update/view/subscription/scripted_scene 负责 App 窗口/设置/回复/Toast 之间的装配与断言；argument/scripted 是进程参数；纯 items 数据已迁出。editor 的主题切换和消息路由保留 App 装配。ACP 的 Harness 负责 Chat 更新与窗口/截图之间契约。

## 逐文件行数（splitlines）

|来源|迁前|迁后|归属/残留|
|---|---:|---:|---|
|`crates/vw-desktop/src/acp_options_window_main.rs`|200|170|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/agent_profile_window_main.rs`|72|69|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/apps_catalog_window_main.rs`|11|11|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/apps_launch_window_main.rs`|11|11|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/apps_lifecycle_window_main.rs`|11|11|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/apps_preferences_window_main.rs`|11|11|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/apps_uninstall_window_main.rs`|11|11|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/archived_chats_window_main.rs`|26|26|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/cc04_desktop_fixture.rs`|82|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_desktop_fixture.rs`|
|`crates/vw-desktop/src/cc04_endpoints_fixture.rs`|76|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_endpoints_fixture.rs`|
|`crates/vw-desktop/src/cc04_env_fixture.rs`|70|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_env_fixture.rs`|
|`crates/vw-desktop/src/cc04_icons_fixture.rs`|101|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_icons_fixture.rs`|
|`crates/vw-desktop/src/cc04_mode_fixture.rs`|58|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_mode_fixture.rs`|
|`crates/vw-desktop/src/cc04_paths_fixture.rs`|52|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_paths_fixture.rs`|
|`crates/vw-desktop/src/cc04_preview_fixture.rs`|152|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`|
|`crates/vw-desktop/src/cc04_provider_window_main.rs`|679|108|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/cc04_snippets_fixture.rs`|64|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_snippets_fixture.rs`|
|`crates/vw-desktop/src/cc04_universal_fixture.rs`|78|0|`crates/libs/desktop-provider-apps/src/acceptance/cc04_universal_fixture.rs`|
|`crates/vw-desktop/src/companion_window_main.rs`|306|291|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/cron_picker_window_main.rs`|11|11|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/editor_config_acceptance_main.rs`|99|79|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/local_speech_system_main.rs`|117|117|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/plugin_compat_window_main.rs`|11|11|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/recovery_window_main.rs`|12|12|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/src/voice_window_main.rs`|410|401|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/tests/worktree_management_window.rs`|259|202|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|
|`crates/vw-desktop/tests/tray_window.rs`|199|195|原生/跨域入口；领域数据按上文调用链迁出或原为纯入口|

宿主本任务来源合计 3189 → 1747，净减少 1442 行（含额外两个 tests 入口）。不是仓库总代码减少指标。

## 全部旧断言映射（完整表达式去空白/尾逗号匹配）

- `crates/vw-desktop/src/acp_options_window_main.rs:99` → `crates/vw-desktop/src/acp_options_window_main.rs`：`assert!(runtime.acp_options_state.open && runtime.acp_options_state.pending)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/archived_chats_window_main.rs:22` → `crates/vw-desktop/src/archived_chats_window_main.rs`：`assert!(status.success(), "isolated acceptance child: {status}")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_desktop_fixture.rs:34` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert_eq!(value["targetHostId"], "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_endpoints_fixture.rs:52` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_endpoints_fixture.rs`：`assert_eq!(body["targetHostId"], "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_env_fixture.rs:17` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_env_fixture.rs`：`assert_eq!(method, "POST")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_env_fixture.rs:19` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_paths_fixture.rs`：`assert_eq!(request["targetHostId"], "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_env_fixture.rs:46` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_env_fixture.rs`：`assert_eq!(`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_env_fixture.rs:62` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_env_fixture.rs`：`assert_eq!(request, json!({"targetHostId":"fixture-host","backupId":backup_id}))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_env_fixture.rs:63` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_env_fixture.rs`：`assert!(self.cleaned)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_mode_fixture.rs:37` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_mode_fixture.rs`：`assert_eq!(`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_mode_fixture.rs:41` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_mode_fixture.rs`：`assert!(!self.direct, "不能在已恢复后再次提交")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_paths_fixture.rs:20` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_paths_fixture.rs`：`assert_eq!(request["targetHostId"], "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_preview_fixture.rs:37` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(request.target_host_id, "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_preview_fixture.rs:58` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(request.provider_id, "provider-1")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_preview_fixture.rs:66` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(request.target_host_id, "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_preview_fixture.rs:82` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(id, "provider-0")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_preview_fixture.rs:83` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(input.target_host_id, "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_preview_fixture.rs:84` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(input.context, format!("fixture-{}", self.revision))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_preview_fixture.rs:85` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(input.resolution.is_some(), input.choice != EditorChoice::Refuse)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:393` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:406` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:414` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:416` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert_eq!(value["targetHostId"], "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:462` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:464` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert_eq!(value["targetHostId"], "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:466` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(line.starts_with("POST "))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:469` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(line.starts_with("POST "))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:481` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(line.starts_with("POST ") || line.starts_with("PATCH "))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:515` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert_eq!(value["revision"], prior.unwrap()["revision"])`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:546` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:548` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert_eq!(target["targetHostId"], "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:562` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:564` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert_eq!(value["targetHostId"], "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:622` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(value["apiKey"].get("value").is_none())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:628` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_provider_window_main.rs:639` → `crates/libs/desktop-provider-apps/src/acceptance.rs`：`assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_snippets_fixture.rs:27` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(request.target_host_id, "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_snippets_fixture.rs:45` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_preview_fixture.rs`：`assert_eq!(request.target_host_id, "fixture-host")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_universal_fixture.rs:44` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_env_fixture.rs`：`assert_eq!(method, "POST")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_universal_fixture.rs:45` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_universal_fixture.rs`：`assert!(matches!(id, "fixture-0" | "fixture-1"))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/cc04_universal_fixture.rs:47` → `crates/libs/desktop-provider-apps/src/acceptance/cc04_universal_fixture.rs`：`assert!(!request.apps.is_empty())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:31` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!([0, 1, 3, 20].contains(&count), "--tasks must be 0, 1, 3 or 20")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:33` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!([0.5, 1.0, 2.0].contains(&scale), "--scale must be 0.5, 1 or 2")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:199` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_avatar_kind, Avatar::Beauty)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:204` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_avatar_kind, Avatar::Handsome)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:209` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_avatar_kind, Avatar::None)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:214` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_size_draft, Some(committed))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:222` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_size_draft, Some(0.5))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:227` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_size_draft, Some(2.0))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:232` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_size_draft, Some(1.0))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:238` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_scale, committed)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:239` → `crates/vw-desktop/src/companion_window_main.rs`：`assert_eq!(app.task_pet_size_draft, Some(preview))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:240` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.task_pet_size_pending.is_none())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:241` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.active_toast.is_some())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:246` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.task_pet_size_draft.is_none())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:250` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(!app.companion_navigation.task_pet_visible)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:251` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.task_pet_window_id.is_none())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:256` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.companion_navigation.task_pet_visible)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:257` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.task_pet_window_id.is_some())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:262` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(!app.task_pet_collapsed)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:280` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.task_pet_reply_request_id.is_none())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:293` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.companion_navigation.task_pet_context_menu_pos.is_some())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/companion_window_main.rs:299` → `crates/vw-desktop/src/companion_window_main.rs`：`assert!(app.task_pet_collapsed)`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/voice_window_main.rs:143` → `crates/vw-desktop/src/voice_window_main.rs`：`assert!(`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/voice_window_main.rs:166` → `crates/vw-desktop/src/voice_window_main.rs`：`assert!(`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/src/voice_window_main.rs:172` → `crates/vw-desktop/src/voice_window_main.rs`：`assert!(`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/worktree_management_window.rs:159` → `crates/vw-desktop/tests/worktree_management_window.rs`：`assert!(`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/worktree_management_window.rs:183` → `crates/vw-desktop/tests/worktree_management_window.rs`：`assert!(ink > 20, "environment command text was not painted: {ink}")`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:47` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:75` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(app.windows().iter().any(|window| window.isVisible()))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:80` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(self.app.main_window_id.is_none())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:81` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(!app.windows().iter().any(|window| window.isVisible()))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:91` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(app.windows().iter().any(|window| window.isVisible()))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:97` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(self.app.main_window_id.is_none())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:98` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(!app.windows().iter().any(|window| window.isVisible()))`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:117` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(!app.isHidden())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:118` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:132` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(!app.isHidden())`；完整表达式静态匹配，未删除或弱化。
- `crates/vw-desktop/tests/tray_window.rs:133` → `crates/vw-desktop/tests/tray_window.rs`：`assert!(`；完整表达式静态匹配，未删除或弱化。

## 静态证据与限制

- 新增模块均由领域 lib.rs 正向 pub mod 注册；原九个 cc04 mod 已从宿主删除，以依赖库调用替代；无 path/include 指回宿主、无 App 复制、无 test=false 修改。
- serde_yaml 0.9 和 toml 0.9 从原宿主夹具依赖转入实际消费库；手工补充已有锁中该库两条依赖边，未运行 Cargo。
- 独立 rustfmt 解析新增领域源码；仅本任务 git diff --check 检查。全仓 diff --check 曾报告其他任务 worktree_admin.rs/task/mod.rs 的空白，不修改他人文件。
- 后续仅经用户另行授权：`python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_40_tests -- --test-threads=1`。真实窗口/字体/音频仍须原显式入口验收，不用构造测试替代。
- 尚未建立编译、行为或绘制通过证据。
