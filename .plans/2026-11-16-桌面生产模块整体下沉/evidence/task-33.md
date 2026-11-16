# 任务33：语音交互与录音装配

静态交付完成，未编译、未测试。遵守用户禁令，没有执行任何 Cargo、测试、实机或回归。

## 生产调用链

`App.voice` 已持有 `desktop_voice_ui::message::voice::State`，初始化继续使用该领域 Default。宿主 Voice 路由的 Event 现直接重导出领域事件；UI map 在领域内生成相同事件，宿主仅包 Message::Voice。录音 capture_event/finish_capture/complete、授权 dictation、insertion、screen/update/receive 和 archive/update 均调用已有领域入口。新增 paste 接收调用者选定的 editor 与可选 cursor，宿主只维护新草稿 editor 镜像。clear_pending/fail/cloud_error 持有清理和错误去重策略，停止聊天的真实请求归属仍由宿主匹配。

没有反向引用宿主、复制 App 或新增总控 trait。显式屏幕分享/授权/取消和代际语义不变。

## 文件与残留责任清单（splitlines）

| 原文件 | 基线 | 当前 | 去向或必要边界 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/components/system_settings_voice.rs` | 20 | 20 | App 选择局部领域视图并包装 Message；设置页组合设置、归档与宿主本地化 |
| `crates/vw-desktop/src/app/components/voice_chat.rs` | 61 | 54 | App 选择局部领域视图并包装 Message；设置页组合设置、归档与宿主本地化 |
| `crates/vw-desktop/src/app/components/voice_chat_tests.rs` | 245 | 38 | 3项绘制/字体/事件测试迁到 host_module_33_tests；仅保留本机语音不遮挡普通聊天的宿主装配契约 |
| `crates/vw-desktop/src/app/components/voice_onboarding.rs` | 22 | 22 | App 选择局部领域视图并包装 Message；设置页组合设置、归档与宿主本地化 |
| `crates/vw-desktop/src/app/components/voice_recordings.rs` | 21 | 21 | App 选择局部领域视图并包装 Message；设置页组合设置、归档与宿主本地化 |
| `crates/vw-desktop/src/app/message/voice.rs` | 287 | 301 | 选择当前会话/editor、普通聊天发送与请求停止、模型能力及宿主 Toast/剪贴板效果装配；状态算法通过 voice-ui 调用 |
| `crates/vw-desktop/src/app/message/voice_composer_tests.rs` | 142 | 142 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/message/voice_dictation.rs` | 6 | 6 | 选择当前会话/editor、普通聊天发送与请求停止、模型能力及宿主 Toast/剪贴板效果装配；状态算法通过 voice-ui 调用 |
| `crates/vw-desktop/src/app/message/voice_dictation_tests.rs` | 24 | 24 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/message/voice_draft.rs` | 2 | 2 | 选择当前会话/editor、普通聊天发送与请求停止、模型能力及宿主 Toast/剪贴板效果装配；状态算法通过 voice-ui 调用 |
| `crates/vw-desktop/src/app/message/voice_hotkey_lifecycle_tests.rs` | 29 | 29 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/message/voice_hotkeys.rs` | 52 | 52 | 选择当前会话/editor、普通聊天发送与请求停止、模型能力及宿主 Toast/剪贴板效果装配；状态算法通过 voice-ui 调用 |
| `crates/vw-desktop/src/app/message/voice_local_chat.rs` | 211 | 211 | 选择当前会话/editor、普通聊天发送与请求停止、模型能力及宿主 Toast/剪贴板效果装配；状态算法通过 voice-ui 调用 |
| `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` | 215 | 215 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/message/voice_local_screen.rs` | 32 | 32 | 选择当前会话/editor、普通聊天发送与请求停止、模型能力及宿主 Toast/剪贴板效果装配；状态算法通过 voice-ui 调用 |
| `crates/vw-desktop/src/app/message/voice_local_screen_tests.rs` | 178 | 178 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` | 204 | 204 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/message/voice_recordings.rs` | 26 | 26 | 选择当前会话/editor、普通聊天发送与请求停止、模型能力及宿主 Toast/剪贴板效果装配；状态算法通过 voice-ui 调用 |
| `crates/vw-desktop/src/app/message/voice_recordings_tests.rs` | 38 | 38 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/message/voice_screen.rs` | 61 | 61 | 选择当前会话/editor、普通聊天发送与请求停止、模型能力及宿主 Toast/剪贴板效果装配；状态算法通过 voice-ui 调用 |
| `crates/vw-desktop/src/app/message/voice_screen_tests.rs` | 20 | 20 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/message/voice_ui_tests.rs` | 61 | 61 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/voice/acceptance_fixture.rs` | 2 | 2 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/acceptance_tests.rs` | 119 | 119 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/voice/capture.rs` | 19 | 19 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/conversation.rs` | 2 | 2 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/hotkeys.rs` | 15 | 12 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/hotkeys_tests.rs` | 26 | 0 | 整体迁到 desktop-voice-ui::host_module_33_tests::hotkey_edit；不再挂载宿主 |
| `crates/vw-desktop/src/app/voice/insertion.rs` | 2 | 2 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/local.rs` | 2 | 2 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/local_conversation.rs` | 12 | 12 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/local_conversation_tests.rs` | 9 | 9 | 保留真实 App 路由、普通聊天请求/权限、editor 镜像或系统服务释放的跨域契约；纯领域已有测试在 voice-ui/desktop-voice 等库 |
| `crates/vw-desktop/src/app/voice/mod.rs` | 51 | 51 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/permissions.rs` | 5 | 5 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/permissions_macos.rs` | 2 | 2 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/playback.rs` | 13 | 13 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/preferences.rs` | 12 | 12 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/preview.rs` | 2 | 2 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/recordings.rs` | 13 | 13 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/recordings_playback.rs` | 2 | 2 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |
| `crates/vw-desktop/src/app/voice/screen_context.rs` | 2 | 2 | 既有领域/系统库兼容重导出，或 project_dirs 本机存储路径选择；local_conversation 是聊天正文投影契约；测试后端桥接服务保留的跨域生命周期断言 |

基线 2267 行；当前 2038 行。净减少为同一 splitlines 口径之差；领域库增加是实际实现与绘制夹具迁入。

## 逐用例与断言映射

以下列出基线每个测试及断言数。未迁项源码断言原样保留，理由见文件残留清单；迁移项以同名测试定位，每个布局、字体、绘制与事件断言均保留，仅将 App 夹具改为领域 State，Message::Voice 模式拆掉宿主包装。hotkey_edit 的断言字段 app.voice.settings 改为局部 state；设置更新仍走真实 desktop_settings_voice::update。

- `voice_task_17_screen_consent_renders_chinese_and_routes_buttons`：4 个断言 → `crates/libs/desktop-voice-ui/src/host_module_33_tests.rs` 同名用例。
- `voice_task_11_local_overlay_shapes_chinese_in_both_themes`：1 个断言 → `crates/libs/desktop-voice-ui/src/host_module_33_tests.rs` 同名用例。
- `voice_moon_ui_local_keeps_chat_without_modal`：2 个断言 → `crates/vw-desktop/src/app/components/voice_chat_tests.rs` 同名用例。
- `voice_moon_ui_inline_draws_and_routes_controls`：4 个断言 → `crates/libs/desktop-voice-ui/src/host_module_33_tests.rs` 同名用例。
- `voice_task_07_composer_inserts_at_original_cursor_without_sending`：3 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_07_composer_switch_and_cancel_reject_late_reply`：3 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_07_composer_preserves_edit_then_applies_explicitly`：4 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_07_escape_invalidates_before_late_completion`：2 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_14_recordings_cancel_and_failure_discard_audio`：6 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_16_external_failure_preserves_result_without_touching_chat_or_sending`：6 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_16_external_cancel_and_service_error_reject_late_results`：4 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_16_external_pending_remains_scoped_and_prevents_overwrite`：4 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_11_local_start_never_opens_cloud_conversation`：3 个断言 → `crates/vw-desktop/src/app/message/voice_composer_tests.rs` 同名用例。
- `voice_task_07_local_context_change_cancels_native_and_rejects_completion`：3 个断言 → `crates/vw-desktop/src/app/message/voice_dictation_tests.rs` 同名用例。
- `voice_task_15_release_before_device_ready_is_pending_and_cancel_invalidates_start`：4 个断言 → `crates/vw-desktop/src/app/message/voice_hotkey_lifecycle_tests.rs` 同名用例。
- `voice_task_15_foreground_capture_ignores_batched_hotkey_start_and_release`：3 个断言 → `crates/vw-desktop/src/app/message/voice_hotkey_lifecycle_tests.rs` 同名用例。
- `app`：1 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_task_11_local_uses_ordinary_request_and_final_completion_once`：7 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_task_11_local_rejects_commands_and_preserves_draft`：3 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_task_11_local_mute_end_and_switch_discard_late_results`：4 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_task_11_local_interrupt_stops_only_owned_model_request`：5 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_task_11_local_preserves_model_history_and_permission_policy`：9 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_task_11_local_disabled_screen_ignores_stale_permission_or_capture`：5 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_task_19_full_app_routes_local_turn_and_discards_completion_after_end`：8 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_moon_ui_end_cancels_media_and_rejects_late_recognition`：3 个断言 → `crates/vw-desktop/src/app/message/voice_local_chat_tests.rs` 同名用例。
- `voice_task_17_local_sharing_is_explicit_and_disabled_by_default`：6 个断言 → `crates/vw-desktop/src/app/message/voice_local_screen_tests.rs` 同名用例。
- `voice_task_17_local_frame_uses_current_model_history_and_permissions_once`：17 个断言 → `crates/vw-desktop/src/app/message/voice_local_screen_tests.rs` 同名用例。
- `voice_task_17_local_cancel_disable_revoke_end_and_switch_discard_frames`：4 个断言 → `crates/vw-desktop/src/app/message/voice_local_screen_tests.rs` 同名用例。
- `voice_task_17_local_changed_model_busy_draft_invalid_png_and_old_call_send_nothing`：3 个断言 → `crates/vw-desktop/src/app/message/voice_local_screen_tests.rs` 同名用例。
- `voice_task_07_new_draft_start_preserves_keyboard_editor_without_creating_session`：7 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `recognize`：1 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_fills_original_cursor_once_without_sending`：5 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_first_send_uses_original_session_creation_chain`：7 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_first_send_invalidates_inflight_dictation`：5 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_cancel_rejects_late_recognition`：3 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_edit_then_undo_requires_explicit_apply`：5 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_reopen_same_project_rejects_old_generation`：6 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_session_navigation_cancels_before_async_history_load`：3 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_project_switch_cancels_and_preserves_current_editor`：2 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_07_new_draft_cloud_requires_real_session_before_gateway`：2 个断言 → `crates/vw-desktop/src/app/message/voice_new_draft_tests.rs` 同名用例。
- `voice_task_14_recordings_retry_requires_explicit_apply_and_rejects_late_reply`：5 个断言 → `crates/vw-desktop/src/app/message/voice_recordings_tests.rs` 同名用例。
- `voice_task_14_recordings_session_switch_discards_retry_result`：2 个断言 → `crates/vw-desktop/src/app/message/voice_recordings_tests.rs` 同名用例。
- `voice_task_17_end_and_cancel_retire_queued_screen_without_sending`：5 个断言 → `crates/vw-desktop/src/app/message/voice_screen_tests.rs` 同名用例。
- `voice_ui_dictation_preparing_stop_is_safe`：3 个断言 → `crates/vw-desktop/src/app/message/voice_ui_tests.rs` 同名用例。
- `voice_ui_dictation_stop_transcribes_once`：8 个断言 → `crates/vw-desktop/src/app/message/voice_ui_tests.rs` 同名用例。
- `voice_ui_dictation_cancel_rejects_late_result`：1 个断言 → `crates/vw-desktop/src/app/message/voice_ui_tests.rs` 同名用例。
- `voice_ui_dictation_conflict_requires_apply`：4 个断言 → `crates/vw-desktop/src/app/message/voice_ui_tests.rs` 同名用例。
- `voice_task_19_voice_acceptance_cancel_has_no_external_write`：6 个断言 → `crates/vw-desktop/src/app/voice/acceptance_tests.rs` 同名用例。
- `voice_task_19_voice_acceptance_end_closes_all_streams`：3 个断言 → `crates/vw-desktop/src/app/voice/acceptance_tests.rs` 同名用例。
- `voice_task_19_full_app_cancel_releases_local_preview_and_preserves_draft`：6 个断言 → `crates/vw-desktop/src/app/voice/acceptance_tests.rs` 同名用例。
- `voice_task_15_recording_chords_stays_in_edit_until_save_and_cancel_discards`：5 个断言 → `crates/libs/desktop-voice-ui/src/host_module_33_tests.rs (hotkey_edit)` 同名用例。
- `voice_task_11_local_reads_only_final_prose`：1 个断言 → `crates/vw-desktop/src/app/voice/local_conversation_tests.rs` 同名用例。
- `voice_task_11_local_does_not_read_inline_code`：1 个断言 → `crates/vw-desktop/src/app/voice/local_conversation_tests.rs` 同名用例。

## 注册与后续验证命令（未运行）

`desktop-voice-ui/src/lib.rs` 在 test + native cfg 挂载 host_module_33_tests。宿主删除 hotkeys_tests 挂载，voice_chat_tests 仅挂载剩余装配测试。原有库内 dictation/draft/screen/archive/layout 测试注册保留。Cargo.lock 仅补 voice-ui 的既有 image 0.25.9 测试依赖，不新增运行时能力。

`python3 scripts/cargo_slot.py cargo test -p desktop-voice-ui --lib host_module_33_tests -- --test-threads=1`

只在用户另行授权后运行。静态 rustfmt 解析和限定文件 git diff --check 不代表类型、字体渲染或行为通过。
