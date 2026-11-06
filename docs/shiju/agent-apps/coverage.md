# 全应用能力交付索引

审查日期：2026-11-06（Asia/Shanghai）。本索引记录已有证据，不代表母任务3完整验收通过。本轮仅文档核对，未运行Cargo、安装或GUI验收；历史测试数量、退出码和槽位见各证据，不累加重复回归。

## 来源与需求

[来源清单](../../../.plans/2026-11-05-CC能力完整融合/来源清单.json)固定提交为`ca58c0313360eae7c851fbd7553c1614c7af03e7`，主责3共3份文件、15条命令；[来源与接口](../../../.plans/2026-11-05-全应用注册与安装运行管理/来源与接口.md)保留交叉归属。两份plan.md均只读。

| 来源文件 | 所有者 |
| --- | --- |
| src-tauri/src/commands/misc.rs | 下表逐命令归属 |
| src-tauri/tests/app_config_load.rs | 有效应用集合01/02；旧结构、v1拒绝及迁入错误归母任务18 |
| src/lib/api/types.ts | 目录01、实例04、动作18、偏好27、启动28；其他字段保留原母任务所有者 |

| misc.rs命令 | 子任务/母任务 |
| --- | --- |
| open_external | 母任务20 |
| copy_text_to_clipboard | 母任务20 |
| check_for_updates | 母任务20 |
| open_portable_update_download | 母任务20 |
| is_portable_mode | 母任务20 |
| get_init_error | 母任务20 |
| get_migration_result | 母任务18 |
| get_skills_migration_result | 母任务18 |
| get_tool_versions | 10–12、33、37、40 |
| prepare_tool_uninstall | 20–21、34、38、42 |
| run_tool_lifecycle_action | 13–26、34、38、41–42 |
| probe_tool_installations | 05–10、19、33、37 |
| list_tool_installations | 05–10、33、37、40 |
| open_provider_terminal | 28–32、36、39、44 |
| set_window_theme | 母任务20 |

| 母任务3需求 | 证据链 |
| --- | --- |
| 1：全部14 AppId | 01–03→catalog/catalog_with_acp→33/37/40→下方14应用记录 |
| 2：发现、多实例、版本、依赖、生命周期、终端、身份、错误 | 04–26、28–32→33–39→40–44→46–59 |
| 3：平台差异、来源锚定、安全卸载 | 05–07、19–25、29–30；macOS实机与其他平台规则分开 |
| 4：原生/ACP/审查运行时/桌面/编辑器 | 01–03、08；ACP仍由vw-acp执行原版本/平台门控，注册不证明可运行 |
| 5：可见性、排序、终端、重启提示、远程隔离 | 27、35、39、43、45；所选Gateway授权本地执行，binding=remote明确拒绝 |

## API、SDK与Iced

API均位于现有管理API（SDK使用`/v1`），绑定targetHostId及认证/租户/主机授权；SDK不探测远端路径。[正式处理器](../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/)由真实Gateway装配接入，生产装配补验见[Claude](macos/claude.md)。

| API | GatewayClient方法 | Iced/证据 |
| --- | --- | --- |
| GET catalog/installations/diagnostics/versions | agent_apps_catalog/installations/diagnostics/versions及授权变体 | [33](task-33.md)→[37](task-37.md)→[40](task-40.md) |
| POST actions/preview、actions | agent_apps_action_preview/submit及授权变体 | [34](task-34.md)→[38](task-38.md)→[41](task-41.md)/[42](task-42.md) |
| GET operations/{id}、events；DELETE operations/{id} | agent_apps_operation/events/cancel及授权变体 | 同上；events为当前快照长轮询，不是持久重放 |
| GET/PATCH preferences | agent_apps_preferences/update及授权变体 | [35](task-35.md)→[39](task-39.md)→[43](task-43.md) |
| POST launch | agent_apps_launch/authorized/identity_authorized | [36](task-36.md)→[39](task-39.md)→[44](task-44.md) |
| GET launch-identities、launch-sessions/{id} | agent_apps_launch_identities、agent_apps_launch_session | [44](task-44.md)，只有身份元数据和运行阶段 |

SDK源码：[读取](../../../crates/vw-gateway-client/src/client/agent_apps_read_api.rs)、[生命周期](../../../crates/vw-gateway-client/src/client/agent_apps_lifecycle_api.rs)、[设置/启动](../../../crates/vw-gateway-client/src/client/agent_apps_settings_api.rs)。Iced源码：[应用页](../../../crates/vw-desktop/src/app/components/system_settings_apps/)、[消息分派](../../../crates/vw-desktop/src/app/message/settings/agent_apps.rs)。44的身份/PTY实装已取代36/39早期501描述；59的[主机配置终端补验](macos/omp.md)不倒填其他应用缺失的实机证据。

## 应用逐项矩阵

P＝对应macOS记录含该场景实证；R＝共享规则/夹具证据，不是本应用实机通过；待＝尚待应用回归；L＝源码明确限定CLI不适用；W＝用户明确豁免的未验证项；Q＝实际官方CLI对专用loopback协议夹具访问通过。P不表示全部安装渠道通过。依赖R指12/41；权限R指33–36/44；设置R指27/35/43；远程R指45，不冒充14次实机验收。

| AppId/实机记录 | 发现 | 多实例 | 版本/运行 | 依赖 | 安装 | 升级变化 | 指定卸载 | 终端实态 | 供应商/账号 | 权限 | 取消进度 | 设置 | 远程 | Iced对应 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [claude](macos/claude.md) | P | P | P | R | P | P | P | 待 | 待 | R | 待 | R | R | 待 |
| [claude-desktop](macos/claude-desktop.md) | P | P | 待 | L | L | L | L | L | L | R | L | R | R | P |
| [codex](macos/codex.md) | P | P | P | R | P | P | P | 待 | 待 | R | 待 | R | R | P |
| [gemini](macos/gemini.md) | P | P | P | R | P | P | P | 待 | 待 | R | 待 | R | R | 待 |
| [grokbuild](macos/grokbuild.md) | P | P | P | R | P | P | P | 待 | 待 | R | P | R | R | 待 |
| [opencode](macos/opencode.md) | P | P | P | R | P | P | P | 待 | 待 | R | P | R | R | 待 |
| [openclaw](macos/openclaw.md) | P | P | P | R | P | P | P | 待 | 待 | R | P | R | R | 待 |
| [hermes](macos/hermes.md) | P | 待 | P | R | 待 | 待 | 待 | 待 | 待 | R | 待 | R | R | 待 |
| [pi](macos/pi.md) | P | P | P | R | P | P | P | 待 | 待 | R | P | R | R | 待 |
| [mcode](macos/mcode.md) | P | P | P | R | P | P | P | 待 | 待 | R | P | R | R | 待 |
| [kimi](macos/kimi.md) | P | P | P | R | P | P | P | 待 | 待 | R | P | R | R | 待 |
| [dsh](macos/dsh.md) | P | P | P | R | P | P | P | 待 | 待 | R | P | R | R | P |
| [agy](macos/agy.md) | P | P | P | R | P | 待 | P | 待 | 待 | R | P | R | R | P |
| [omp](macos/omp.md) | P | P | P | R | P | P | P | P | Q/W | R | P | R | R | P |

每格仅表达该项观察；真实版本、渠道、API/SDK输出与截图见各行记录。`grokbuild`的CLI名为`grok`，不是两个应用。Desktop的L依据[CatalogApp::capability](../../../crates/vw-agent-apps/src/apps/catalog.rs)及[独立桌面探测](../../../crates/vw-agent-apps/src/apps/discovery/claude_desktop.rs)，不表示桌面产品全部功能完成；欢迎界面不改变API只做包观察的边界。Hermes官方安装器成功不等同正式安装API通过。Node列未逐应用实机制造依赖缺失。

## ACP、身份与平台

[03](task-03.md)及[acp_registry/catalog_with_acp](../../../crates/vw-agent-apps/src/apps/acp_projection.rs)直接委托`vw_acp::built_in_agent_specs`。[独立测试](../../../crates/vw-agent-apps/src/apps/acp_projection_tests.rs)断言22项完整相等，旧claude-legacy兼容key保留。8个应用（claude/codex/gemini/opencode/openclaw/hermes/pi/kimi）追加ACP形态；目录外ACP仍在完整视图，未缩减为14项。reviewed_command与verify_reviewed_executable继续委托原审查门控，Ok(None)不代表已安装或审查通过。本轮未重跑历史测试。

母任务4/6后置启动证据：[31](task-31.md)供应商投影、[32](task-32.md)官方账号材料/租约/轮换、[44](task-44.md)API/SDK/Iced/PTY，均含实际子进程夹具；真实厂商账号仍以应用矩阵为准。[供应商实现](../../../crates/vw-agent-apps/src/providers/mod.rs)区分已迁入与未实现分支；AGY固定OS钥匙串路径的安全拒绝依据在32，缺账号不判不支持。

### OMP最终补验与豁免范围

提交`e5205438a`及[OMP最终记录](macos/omp.md)已补齐官方18.8.9→18.17.0真实升级、Omp供应商投影、生产app_providers路由及私有材料清理。佐证：[最终API](macos/omp-final-api.json)、[实机运行](macos/omp-final-runtime.json)、[验证输出](macos/omp-final-validation.txt)。15项不同有效测试及全仓Clippy退出0属于59的执行证据，本轮未重跑。此前59的Git/ACP编译限制已解除，不再列作当前限制。

本轮通过read_thread直接核实原会话`01a1afdb-07a3-7f21-8f57-5bc5a0a32c0c`的用户消息`01a1b037-9420-76e3-82a1-edfb5cc02904`：问题明确询问“omp 官方账号实机验收”，用户答复“无账号，算通过，只要大体能够访问，以后专门来配置再说。”因此Q表示真实官方OMP消费生产材料并向loopback的`/v1/responses`请求/接收SSE；W表示真实账号及外部厂商认证未验证、按本应用用户口径豁免。没有把协议夹具标为真实登录，也不将该答复扩展到其他13应用、缺失功能或UI场景。

原46–58会话与当前持久记录已重新核对：尚无与上述OMP答复等价的范围调整或完整补验证据。编号文件不存在仅是文件状态，不是新的实机证据。具体原会话定位和恢复范围保留在任务61；不能据此更新母任务3为完整通过。

macOS实机记录为27.0（26A428）、arm64；未覆盖Intel/x86_64，不外推其他CPU。Node v26.13.0/npm 11.20.0是记录中的环境，不等于任意版本兼容。Windows/WSL/Linux只引用05–07、19–25、29–30的规则/拒绝测试，均未实机验证；按用户要求不构成本轮完成阻塞。

[60渲染](macos/rendering.md)记录46种原生场景、29项自动行为测试、16项人工截图审查。补强重跑不算92种场景，harness=false窗口不算46条Rust单元测试。截图：[生命周期](macos/rendering/extended/lifecycle/)、[卸载](macos/rendering/extended/uninstall/)、[设置](macos/rendering/extended/preferences/)、[启动](macos/rendering/extended/launch/)。通用表单截图不代替各厂商实机闭环。

## 本轮核对

索引覆盖5项母需求、3份主责文件、15条命令、45份实现证据、14份应用记录、1份渲染记录与22项ACP保留契约。具体收口状态、缺口及恢复条件仅记[任务61](../../../.plans/2026-11-05-全应用注册与安装运行管理/61-macOS能力证据收口.md)，母任务3状态不变。

## 01–45证据、实现与测试定位

| Evidence | Implementation | Test filter |
| --- | --- | --- |
| [任务 01：应用目录共享类型](task-01.md) | [catalog](../../../crates/vw-api-types/src/agent_apps/catalog.rs) | `cc_apps_catalog_roundtrip` |
| [任务02：十四应用能力目录](task-02.md) | [catalog](../../../crates/vw-agent-apps/src/apps/catalog.rs) | `cc_apps_catalog_capabilities` |
| [任务03：现有ACP注册投影](task-03.md) | [acp_projection](../../../crates/vw-agent-apps/src/apps/acp_projection.rs) | `cc_apps_acp_projection_preserves_registry` |
| [任务04：安装实例共享类型](task-04.md) | [installations](../../../crates/vw-api-types/src/agent_apps/installations.rs) | `cc_apps_installation_roundtrip` |
| [任务05：POSIX安装路径发现](task-05.md) | [posix](../../../crates/vw-agent-apps/src/apps/discovery/posix.rs) | `cc_apps_posix_discovery_paths` |
| [任务06 Windows安装路径发现](task-06.md) | [windows](../../../crates/vw-agent-apps/src/apps/discovery/windows.rs) | `cc_apps_windows_discovery_paths` |
| [任务07 WSL环境与Shell解析](task-07.md) | [wsl](../../../crates/vw-agent-apps/src/apps/discovery/wsl.rs) | `cc_apps_wsl_target_resolution` |
| [任务08 Claude桌面安装探测](task-08.md) | [claude_desktop](../../../crates/vw-agent-apps/src/apps/discovery/claude_desktop.rs) | `cc_apps_claude_desktop_presence` |
| [任务09 安装实例去重与冲突](task-09.md) | [report](../../../crates/vw-agent-apps/src/apps/discovery/report.rs) | `cc_apps_installation_conflicts` |
| [任务10 本地版本与运行诊断](task-10.md) | [version](../../../crates/vw-agent-apps/src/apps/discovery/version.rs) | `cc_apps_version_probe_broken_install` |
| [任务11 最新版本查询](task-11.md) | [latest](../../../crates/vw-agent-apps/src/apps/versions/latest.rs) | `cc_apps_latest_version_selection` |
| [任务12 Node升级依赖门控](task-12.md) | [node](../../../crates/vw-agent-apps/src/apps/versions/node.rs) | `cc_apps_node_requirement_gap` |
| [任务13 npm安装动作](task-13.md) | [npm](../../../crates/vw-agent-apps/src/apps/lifecycle/npm.rs) | `cc_apps_npm_install_allowlist` |
| [任务14 Claude与OpenCode原生安装](task-14.md) | [native_primary](../../../crates/vw-agent-apps/src/apps/lifecycle/native_primary.rs) | `cc_apps_primary_native_install` |
| [任务15 Grok与MiniMax原生安装](task-15.md) | [native_grok_mcode](../../../crates/vw-agent-apps/src/apps/lifecycle/native_grok_mcode.rs) | `cc_apps_grok_mcode_install` |
| [任务16 Hermes与Antigravity安装](task-16.md) | [native_hermes_agy](../../../crates/vw-agent-apps/src/apps/lifecycle/native_hermes_agy.rs) | `cc_apps_hermes_agy_install` |
| [任务17 Kimi与Oh My Pi安装](task-17.md) | [native_kimi_omp](../../../crates/vw-agent-apps/src/apps/lifecycle/native_kimi_omp.rs) | `cc_apps_kimi_omp_install` |
| [任务18 安装升级预览契约](task-18.md) | [actions](../../../crates/vw-api-types/src/agent_apps/actions.rs) | `cc_apps_action_contract_roundtrip` |
| [任务19 升级锚定与预检](task-19.md) | [update](../../../crates/vw-agent-apps/src/apps/lifecycle/update.rs) | `cc_apps_update_anchors_installation` |
| [任务20 包管理器卸载预览](task-20.md) | [packages](../../../crates/vw-agent-apps/src/apps/uninstall/packages.rs) | `cc_apps_package_uninstall_preview` |
| [任务21：原生卸载安全边界](task-21.md) | [native](../../../crates/vw-agent-apps/src/apps/uninstall/native.rs) | `cc_apps_native_uninstall_scope` |
| [任务22：生命周期写入互斥](task-22.md) | [coordinator](../../../crates/vw-agent-apps/src/apps/lifecycle/coordinator.rs) | `cc_apps_lifecycle_lock_survives_disconnect` |
| [任务23：安装进程执行与结果](task-23.md) | [process](../../../crates/vw-agent-apps/src/apps/lifecycle/process.rs) | `cc_apps_lifecycle_process_result` |
| [任务24：卸载确认执行与后验](task-24.md) | [execute](../../../crates/vw-agent-apps/src/apps/uninstall/execute.rs) | `cc_apps_uninstall_revalidates_plan`, `cc_apps_lifecycle_process_result` |
| [任务25：WSL生命周期执行](task-25.md) | [wsl](../../../crates/vw-agent-apps/src/apps/lifecycle/wsl.rs) | `cc_apps_wsl_lifecycle_target`, `cc_apps_lifecycle_process_result` |
| [任务26：操作进度与取消](task-26.md) | [operations](../../../crates/vw-agent-apps/src/apps/lifecycle/operations.rs) | `cc_apps_operation_cancel_reaps_process`, `cc_apps_lifecycle_process_result` |
| [任务27：应用设置归一化](task-27.md) | [preferences](../../../crates/vw-agent-apps/src/apps/preferences.rs) | `cc_apps_preferences_normalization` |
| [任务28：终端启动输入边界](task-28.md) | [request](../../../crates/vw-agent-apps/src/apps/launch/request.rs) | `cc_apps_launch_request_validation` |
| [任务29：POSIX终端启动](task-29.md) | [posix](../../../crates/vw-agent-apps/src/apps/launch/posix.rs) | `cc_apps_posix_terminal_launch` |
| [任务30：Windows与WSL终端启动](task-30.md) | [windows](../../../crates/vw-agent-apps/src/apps/launch/windows.rs) | `cc_apps_windows_terminal_launch` |
| [任务31：供应商API凭据启动](task-31.md) | [provider](../../../crates/vw-agent-apps/src/apps/launch/provider.rs) | `cc_apps_provider_launch_isolation` |
| [任务32：官方账号终端启动](task-32.md) | [account](../../../crates/vw-agent-apps/src/apps/launch/account.rs) | `cc_apps_account_launch_isolation`, `cc_apps_launch_request_validation_rejects_stale_ambiguous_and_foreign_instances` |
| [任务33：授权主机的应用读取与诊断API](task-33.md) | [read](../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/read.rs) | `cc_apps_read_api_authorization` |
| [任务34：生命周期API](task-34.md) | [lifecycle](../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs) | `cc_apps_lifecycle_api_scope`, `cc_apps_read_api_authorization` |
| [任务35：应用设置API](task-35.md) | [preferences](../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/preferences.rs) | `cc_apps_preferences_api_scope`, `cc_apps_read_api_authorization` |
| [任务36：终端启动API](task-36.md) | [launch](../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/launch.rs) | `cc_apps_launch_api_target_scope`, `cc_apps_read_api_authorization` |
| [任务37：读取诊断SDK](task-37.md) | [agent_apps_read_api](../../../crates/vw-gateway-client/src/client/agent_apps_read_api.rs) | `cc_apps_read_sdk_roundtrip` |
| [任务38：生命周期SDK](task-38.md) | [agent_apps_lifecycle_api](../../../crates/vw-gateway-client/src/client/agent_apps_lifecycle_api.rs) | `cc_apps_lifecycle_sdk_roundtrip` |
| [任务39：设置启动SDK](task-39.md) | [agent_apps_settings_api](../../../crates/vw-gateway-client/src/client/agent_apps_settings_api.rs) | `cc_apps_settings_launch_sdk_roundtrip` |
| [任务40：应用列表桌面入口](task-40.md) | [catalog](../../../crates/vw-desktop/src/app/components/system_settings_apps/catalog.rs) | `cc_apps_desktop_catalog_state` |
| [任务41：安装升级桌面进度](task-41.md) | [lifecycle](../../../crates/vw-desktop/src/app/components/system_settings_apps/lifecycle.rs) | `cc_apps_desktop_lifecycle_state`, `cc_apps_lifecycle_sdk_roundtrip` |
| [任务42：卸载选择与确认弹窗](task-42.md) | [uninstall](../../../crates/vw-desktop/src/app/components/system_settings_apps/uninstall.rs) | `cc_apps_desktop_uninstall_state`, `cc_apps_desktop_lifecycle_state` |
| [任务43：应用设置桌面表单](task-43.md) | [preferences](../../../crates/vw-desktop/src/app/components/system_settings_apps/preferences.rs) | `cc_apps_desktop_preferences_state`, `cc_apps_preferences_normalization` |
| [任务44：终端启动桌面表单](task-44.md) | [launch](../../../crates/vw-desktop/src/app/components/system_settings_apps/launch.rs) | `cc_apps_desktop_launch_state`, `cc_apps_terminal_session` |
| [任务45：远程隔离闭环回归](task-45.md) | [remote_tests_tests](../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/remote_tests_tests.rs) | `cc_apps_remote_host_isolation` |
