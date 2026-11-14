# 桌面来源与接口契约盘点（任务 1）

状态：盘点文档已交付；行为验证未执行。日期：2026-11-14。

本轮用户要求不运行 Cargo、单元测试、实机测试或最终回归，统一留待未来验证；因此本文不是功能验收通过证明。未运行任务卡的 JSON 校验命令。只读取清单、映射及源码，整理交接入口；不修改 Rust、只读 plan.md 或来源快照。

## 输入版本与范围

- 来源快照：`ca58c0313360eae7c851fbd7553c1614c7af03e7`；输入为[来源清单](../../../.plans/2026-11-05-CC能力完整融合/来源清单.json)。路径与摘要以清单为准，本文不声称重新校验来源内容或哈希。
- ShiJu 基线：`a28a3a0cc011fac0cb332154995607fdbdb62f17`，共享 checkout 含其他会话未提交实现。以下接口定位反映阅读时的工作区，不代表全部已提交或编译。
- 无直接任务依赖。母任务 2—19 为领域契约所有者，本任务只整理入口，不要求其整条实施线交付。
- 母任务 20 共 405 个候选：356 个文件、41 个命令、8 个事件。全源清单的命令/事件还含其他母任务，不能将其总数当作桌面范围。
- 405 项逐条列于下表，结论均限于已有任务归属。命令是词法候选，事件可能有多个发出者/消费者，不据此声称已注册或 macOS 可用。

## 接口与权限交接

|桌面任务|领域与现有入口（相对仓库根目录）|消费边界|
|---|---|---|
|3 应用|`vw-api-types/src/agent_apps/catalog.rs`、`read.rs`、`actions.rs`；SDK `agent_apps_read_api.rs`、`agent_apps_lifecycle_api.rs`|类型路径位于 `crates/` 对应包，SDK 位于 `crates/vw-gateway-client/src/client/`。目录、安装、诊断、版本按目标主机读取；操作先 preview 再 submit，失败恢复读取 operation，不自动重放写入。|
|4 供应商|SDK `app_providers_api.rs`、`app_providers/`|消费领域配置读写与事务；客户端不能代目标主机直接写文件。|
|5 授权|SDK `agent_apps_auth_read.rs`、`agent_apps_auth_accounts.rs`、`agent_apps_auth_flow.rs`|复用账号所有者；不把令牌或敏感载荷写入日志、证据或 UI 错误。|
|6 用量|SDK `apps_usage_read_api.rs`、`apps_usage_write_api.rs`|区分读取、刷新及写权限，不把空数据解释为不支持。|
|7 MCP|SDK `mcp_management_read.rs`、`mcp_management_write.rs`、`mcp_management_distribution.rs`|目录和目标分发分属权限，实际应用矩阵消费领域合同。|
|8 Skills|SDK `agent_apps_skills/`|安装、备份、分发与目标路径使用领域实现；未提交模块需后续任务确认合入状态。|
|9 提示词|SDK `agent_app_prompts_api.rs`、`agent_app_prompt_resources_api.rs`|激活、导入、模板及同步由领域负责，桌面不再实现第二套文件写入。|
|10 会话|类型 `crates/vw-api-types/src/external_session/`；领域 `crates/vw-agent-apps/src/sessions/`|外部会话与运行时 `session_api.rs` 不可混同；消费前确认对应外部会话路由和 SDK，本文不虚构已就绪 SDK。|
|11 工作区|SDK `workspace_memory_api.rs`|项目与目标主机上下文保持一致。|
|12 组合|SDK `project_profile_api.rs`|组合预览与单次应用消费领域快照及并发写契约。|
|13 设置及配置迁移/同步|`crates/vw-desktop/src/app/config_system_settings.rs`；母任务18、19的配置与同步领域|各行为消费已交付接口；本文未确立备份恢复、WebDAV/S3桌面 SDK 就绪，后续任务读取其正式证据，不能用空壳接口替代。|
|21—37 原生偏好、生命周期与托盘|`crates/vw-desktop/src/app/desktop_lifecycle.rs`、`desktop_lifecycle/native.rs`、`desktop_lifecycle/window.rs`|平台条件分支须分别阅读；非 macOS 占位实现不能当作 macOS 行为证据。|
|38—46 深链与系统动作|来源命令/事件见逐项表；Skills 已有 `system_settings_skills/deeplink.rs`|先解析并展示确认，系统动作属于客户端；目标主机业务交网关，不信任深链扩大权限。|
|47—51 更新与版本|`crates/vw-desktop/src/app/desktop_update.rs`、`installer_update.rs`|沿用识矩更新源，保留下载、取消、安装交接状态，不启用 CC 更新源。|

应用读取 SDK 的真实符号为 `GatewayClient::agent_apps_catalog`、`agent_apps_installations`、`agent_apps_diagnostics`、`agent_apps_versions`，参数带目标主机；读错误为 `AgentAppsReadError::Transport/Http/Decode`。生命周期 SDK 提供 `agent_apps_action_preview`、`agent_apps_action_submit`、`agent_apps_operation`，错误为 `AgentAppsLifecycleError::Transport/Http/Decode`，Http 保留状态与业务体。其他领域错误须由消费任务按实际类型核对，不能推定全部使用这两个枚举。

授权入口位于 `crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs`：管理员策略声明 principal、host、grant，头部不能建立主机所有权；不同操作具有独立授权项。保留默认拒绝、租户和项目边界，不以界面可见性代替服务端授权。

## 14 应用声明

`crates/vw-api-types/src/agent_apps/catalog.rs` 的 `AppId` 声明为 claude、claude-desktop、codex、gemini、grokbuild、opencode、openclaw、hermes、pi、mcode、kimi、dsh、agy、omp。每个应用均为“身份已声明、各操作能力待对应任务核对”，本任务不标记任何应用全能力支持或不支持。`grok` 是可执行名称而非公开应用 ID；NativeCli、AcpAdapter、ReviewRuntime、DesktopApp、EditorIntegration 互不隐含支持。目录注册也不承诺安装或启动。

## 映射修正与后续关口

`update_tray_menu` 改归 34—35，`deeplink-pending` 改归 38—42，`configLoadError` 改归 19，`update-download-progress` 改归 48。其余“2（核对后分配对应领域）”保留为导航任务的入口清单所有权，不代表任务2实现所有资源对应的业务；图标、静态资源、通用查询工具由消费页面继续细分，禁止迁移时替换仓库品牌。无来源链接指向开发者旧绝对路径，输入链接均指向本仓库清单/映射。

后续按[任务入口](../../../.plans/2026-11-05-原生桌面系统集成/prompt.md)进入具体行为卡。卡删除后仍以正式文档及提交为依据。405 项的行为源码、相邻测试、路由、权限、SDK 和实际渲染由各行为任务确认；未来统一验证补齐，不把本盘点计作测试通过。macOS 人工验收继续由用户执行。

## 逐项交接清单

“已映射”仅指存在任务归属；“待细分”保留任务2入口盘点责任。所有条目的行为状态均为未验证。序号稳定采用来源映射的现有顺序。

|序号|形态|来源路径|符号|任务归属|盘点结论|
|---:|---|---|---|---|---|
|1|文件|`docs/guides/deeplink-site-integration-guide-en.md`|文件级入口|38—42|已映射，行为未验证|
|2|文件|`docs/guides/deeplink-site-integration-guide-ja.md`|文件级入口|38—42|已映射，行为未验证|
|3|文件|`docs/guides/deeplink-site-integration-guide-zh.md`|文件级入口|38—42|已映射，行为未验证|
|4|文件|`docs/user-manual/en/1-getting-started/1.5-settings.md`|文件级入口|13|已映射，行为未验证|
|5|文件|`docs/user-manual/en/5-faq/5.3-deeplink.md`|文件级入口|38—42|已映射，行为未验证|
|6|文件|`docs/user-manual/ja/1-getting-started/1.5-settings.md`|文件级入口|13|已映射，行为未验证|
|7|文件|`docs/user-manual/ja/5-faq/5.3-deeplink.md`|文件级入口|38—42|已映射，行为未验证|
|8|文件|`docs/user-manual/zh/1-getting-started/1.5-settings.md`|文件级入口|13|已映射，行为未验证|
|9|文件|`docs/user-manual/zh/5-faq/5.3-deeplink.md`|文件级入口|38—42|已映射，行为未验证|
|10|文件|`src-tauri/Info.plist`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|11|文件|`src-tauri/capabilities/default.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|12|文件|`src-tauri/capabilities/tray-panel.json`|文件级入口|34—35|已映射，行为未验证|
|13|文件|`src-tauri/icons/128x128.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|14|文件|`src-tauri/icons/128x128@2x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|15|文件|`src-tauri/icons/32x32.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|16|文件|`src-tauri/icons/64x64.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|17|文件|`src-tauri/icons/Square107x107Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|18|文件|`src-tauri/icons/Square142x142Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|19|文件|`src-tauri/icons/Square150x150Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|20|文件|`src-tauri/icons/Square284x284Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|21|文件|`src-tauri/icons/Square30x30Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|22|文件|`src-tauri/icons/Square310x310Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|23|文件|`src-tauri/icons/Square44x44Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|24|文件|`src-tauri/icons/Square71x71Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|25|文件|`src-tauri/icons/Square89x89Logo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|26|文件|`src-tauri/icons/StoreLogo.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|27|文件|`src-tauri/icons/android/mipmap-hdpi/ic_launcher.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|28|文件|`src-tauri/icons/android/mipmap-hdpi/ic_launcher_foreground.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|29|文件|`src-tauri/icons/android/mipmap-hdpi/ic_launcher_round.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|30|文件|`src-tauri/icons/android/mipmap-mdpi/ic_launcher.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|31|文件|`src-tauri/icons/android/mipmap-mdpi/ic_launcher_foreground.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|32|文件|`src-tauri/icons/android/mipmap-mdpi/ic_launcher_round.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|33|文件|`src-tauri/icons/android/mipmap-xhdpi/ic_launcher.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|34|文件|`src-tauri/icons/android/mipmap-xhdpi/ic_launcher_foreground.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|35|文件|`src-tauri/icons/android/mipmap-xhdpi/ic_launcher_round.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|36|文件|`src-tauri/icons/android/mipmap-xxhdpi/ic_launcher.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|37|文件|`src-tauri/icons/android/mipmap-xxhdpi/ic_launcher_foreground.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|38|文件|`src-tauri/icons/android/mipmap-xxhdpi/ic_launcher_round.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|39|文件|`src-tauri/icons/android/mipmap-xxxhdpi/ic_launcher.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|40|文件|`src-tauri/icons/android/mipmap-xxxhdpi/ic_launcher_foreground.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|41|文件|`src-tauri/icons/android/mipmap-xxxhdpi/ic_launcher_round.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|42|文件|`src-tauri/icons/dmg-background.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|43|文件|`src-tauri/icons/icon.icns`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|44|文件|`src-tauri/icons/icon.ico`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|45|文件|`src-tauri/icons/icon.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|46|文件|`src-tauri/icons/ios/AppIcon-20x20@1x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|47|文件|`src-tauri/icons/ios/AppIcon-20x20@2x-1.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|48|文件|`src-tauri/icons/ios/AppIcon-20x20@2x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|49|文件|`src-tauri/icons/ios/AppIcon-20x20@3x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|50|文件|`src-tauri/icons/ios/AppIcon-29x29@1x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|51|文件|`src-tauri/icons/ios/AppIcon-29x29@2x-1.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|52|文件|`src-tauri/icons/ios/AppIcon-29x29@2x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|53|文件|`src-tauri/icons/ios/AppIcon-29x29@3x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|54|文件|`src-tauri/icons/ios/AppIcon-40x40@1x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|55|文件|`src-tauri/icons/ios/AppIcon-40x40@2x-1.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|56|文件|`src-tauri/icons/ios/AppIcon-40x40@2x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|57|文件|`src-tauri/icons/ios/AppIcon-40x40@3x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|58|文件|`src-tauri/icons/ios/AppIcon-512@2x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|59|文件|`src-tauri/icons/ios/AppIcon-60x60@2x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|60|文件|`src-tauri/icons/ios/AppIcon-60x60@3x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|61|文件|`src-tauri/icons/ios/AppIcon-76x76@1x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|62|文件|`src-tauri/icons/ios/AppIcon-76x76@2x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|63|文件|`src-tauri/icons/ios/AppIcon-83.5x83.5@2x.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|64|文件|`src-tauri/icons/tray/macos/statusTemplate.png`|文件级入口|34—35|已映射，行为未验证|
|65|文件|`src-tauri/icons/tray/macos/statusTemplate@2x.png`|文件级入口|34—35|已映射，行为未验证|
|66|文件|`src-tauri/icons/tray/macos/statusbar_template_3x.png`|文件级入口|34—35|已映射，行为未验证|
|67|文件|`src-tauri/src/agy_smoke_tests.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|68|文件|`src-tauri/src/auto_launch.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|69|文件|`src-tauri/src/commands/deeplink.rs`|文件级入口|38—42|已映射，行为未验证|
|70|文件|`src-tauri/src/commands/lightweight.rs`|文件级入口|31|已映射，行为未验证|
|71|文件|`src-tauri/src/commands/settings.rs`|文件级入口|13|已映射，行为未验证|
|72|文件|`src-tauri/src/deeplink/codex_link.rs`|文件级入口|38—42|已映射，行为未验证|
|73|文件|`src-tauri/src/deeplink/grok_link.rs`|文件级入口|38—42|已映射，行为未验证|
|74|文件|`src-tauri/src/deeplink/icon_fetch.rs`|文件级入口|38—42|已映射，行为未验证|
|75|文件|`src-tauri/src/deeplink/mod.rs`|文件级入口|38—42|已映射，行为未验证|
|76|文件|`src-tauri/src/deeplink/parser.rs`|文件级入口|38—42|已映射，行为未验证|
|77|文件|`src-tauri/src/deeplink/tests.rs`|文件级入口|38—42|已映射，行为未验证|
|78|文件|`src-tauri/src/deeplink/utils.rs`|文件级入口|38—42|已映射，行为未验证|
|79|文件|`src-tauri/src/error.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|80|文件|`src-tauri/src/init_status.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|81|文件|`src-tauri/src/kimi_session.rs`|文件级入口|10|已映射，行为未验证|
|82|文件|`src-tauri/src/lib.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|83|文件|`src-tauri/src/lightweight.rs`|文件级入口|31|已映射，行为未验证|
|84|文件|`src-tauri/src/linux_fix.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|85|文件|`src-tauri/src/main.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|86|文件|`src-tauri/src/panic_hook.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|87|文件|`src-tauri/src/resources/codex_deepseek_catalog_template.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|88|文件|`src-tauri/src/resources/codex_native_responses_template.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|89|文件|`src-tauri/src/resources/gpt5_5_template.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|90|文件|`src-tauri/src/services/config.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|91|文件|`src-tauri/src/services/mod.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|92|文件|`src-tauri/src/services/pi_state.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|93|文件|`src-tauri/src/services/sql_helpers.rs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|94|文件|`src-tauri/src/settings.rs`|文件级入口|13|已映射，行为未验证|
|95|文件|`src-tauri/src/tray.rs`|文件级入口|34—35|已映射，行为未验证|
|96|文件|`src-tauri/src/tray_panel.rs`|文件级入口|36—37|已映射，行为未验证|
|97|文件|`src-tauri/tauri.conf.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|98|文件|`src-tauri/tauri.windows.conf.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|99|文件|`src-tauri/tests/deeplink_import.rs`|文件级入口|38—42|已映射，行为未验证|
|100|文件|`src-tauri/tests/golden/snapshots/rows/deeplink-claude.txt`|文件级入口|38—42|已映射，行为未验证|
|101|文件|`src-tauri/tests/golden/snapshots/rows/deeplink-codex-config.txt`|文件级入口|38—42|已映射，行为未验证|
|102|文件|`src-tauri/tests/golden/snapshots/rows/deeplink-codex.txt`|文件级入口|38—42|已映射，行为未验证|
|103|文件|`src-tauri/tests/golden/snapshots/rows/deeplink-gemini.txt`|文件级入口|38—42|已映射，行为未验证|
|104|文件|`src-tauri/tests/golden/snapshots/rows/deeplink-grokbuild-config.txt`|文件级入口|38—42|已映射，行为未验证|
|105|文件|`src-tauri/tests/golden/snapshots/rows/deeplink-grokbuild.txt`|文件级入口|38—42|已映射，行为未验证|
|106|文件|`src-tauri/wix/per-user-main.wxs`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|107|文件|`src/App.tsx`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|108|文件|`src/assets/icons/chatgpt.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|109|文件|`src/assets/icons/claude.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|110|文件|`src/assets/icons/logo.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|111|文件|`src/contexts/UpdateContext.tsx`|文件级入口|47—51|已映射，行为未验证|
|112|文件|`src/hooks/useDarkMode.ts`|文件级入口|21|已映射，行为未验证|
|113|文件|`src/hooks/useDebouncedValue.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|114|文件|`src/hooks/useDelayedFlag.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|115|文件|`src/hooks/useDirectorySettings.ts`|文件级入口|13|已映射，行为未验证|
|116|文件|`src/hooks/useDragSort.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|117|文件|`src/hooks/useLastValidValue.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|118|文件|`src/hooks/useLatestRef.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|119|文件|`src/hooks/useModeActions.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|120|文件|`src/hooks/useNativeConfigPath.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|121|文件|`src/hooks/useProxyStatus.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|122|文件|`src/hooks/useSettings.ts`|文件级入口|13|已映射，行为未验证|
|123|文件|`src/hooks/useSettingsForm.ts`|文件级入口|13|已映射，行为未验证|
|124|文件|`src/hooks/useSettingsMetadata.ts`|文件级入口|13|已映射，行为未验证|
|125|文件|`src/hooks/useSidebarCollapsed.ts`|文件级入口|2、26|已映射，行为未验证|
|126|文件|`src/hooks/useSidebarStatus.ts`|文件级入口|2、26|已映射，行为未验证|
|127|文件|`src/hooks/useTauriEvent.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|128|文件|`src/hooks/useTrayNavigation.ts`|文件级入口|34—35|已映射，行为未验证|
|129|文件|`src/i18n/index.ts`|文件级入口|22|已映射，行为未验证|
|130|文件|`src/i18n/locales/en.json`|文件级入口|22|已映射，行为未验证|
|131|文件|`src/i18n/locales/ja.json`|文件级入口|22|已映射，行为未验证|
|132|文件|`src/i18n/locales/tr.json`|文件级入口|22|已映射，行为未验证|
|133|文件|`src/i18n/locales/zh-TW.json`|文件级入口|22|已映射，行为未验证|
|134|文件|`src/i18n/locales/zh.json`|文件级入口|22|已映射，行为未验证|
|135|文件|`src/icons/extracted/88api.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|136|文件|`src/icons/extracted/ClaudeApi.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|137|文件|`src/icons/extracted/TeamoRouter-icon-dark.png`|文件级入口|21|已映射，行为未验证|
|138|文件|`src/icons/extracted/a6-icon.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|139|文件|`src/icons/extracted/aicodemirror.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|140|文件|`src/icons/extracted/aicoding.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|141|文件|`src/icons/extracted/aihubmix-color.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|142|文件|`src/icons/extracted/algocode.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|143|文件|`src/icons/extracted/alibaba.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|144|文件|`src/icons/extracted/alltokens-si.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|145|文件|`src/icons/extracted/alltokens.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|146|文件|`src/icons/extracted/amuxapi-icon.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|147|文件|`src/icons/extracted/anthropic.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|148|文件|`src/icons/extracted/antigravity.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|149|文件|`src/icons/extracted/apikeyfun.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|150|文件|`src/icons/extracted/apinebula_icon.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|151|文件|`src/icons/extracted/astron.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|152|文件|`src/icons/extracted/atlascloud_icon.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|153|文件|`src/icons/extracted/aws.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|154|文件|`src/icons/extracted/azure.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|155|文件|`src/icons/extracted/baidu.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|156|文件|`src/icons/extracted/bailian.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|157|文件|`src/icons/extracted/bailing.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|158|文件|`src/icons/extracted/bytedance.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|159|文件|`src/icons/extracted/byteplus.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|160|文件|`src/icons/extracted/catcoder.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|161|文件|`src/icons/extracted/ccsub.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|162|文件|`src/icons/extracted/chatglm.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|163|文件|`src/icons/extracted/cherryin.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|164|文件|`src/icons/extracted/claude.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|165|文件|`src/icons/extracted/claudecn.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|166|文件|`src/icons/extracted/claw.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|167|文件|`src/icons/extracted/cloudflare.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|168|文件|`src/icons/extracted/code0.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|169|文件|`src/icons/extracted/cohere.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|170|文件|`src/icons/extracted/crazyrouter.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|171|文件|`src/icons/extracted/cubence.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|172|文件|`src/icons/extracted/deepseek.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|173|文件|`src/icons/extracted/dmxapi.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|174|文件|`src/icons/extracted/doubao.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|175|文件|`src/icons/extracted/eflowcode.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|176|文件|`src/icons/extracted/etok.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|177|文件|`src/icons/extracted/everyapi.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|178|文件|`src/icons/extracted/fenno-icon.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|179|文件|`src/icons/extracted/fluxa.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|180|文件|`src/icons/extracted/gemini.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|181|文件|`src/icons/extracted/gemma.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|182|文件|`src/icons/extracted/github.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|183|文件|`src/icons/extracted/google.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|184|文件|`src/icons/extracted/googlecloud.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|185|文件|`src/icons/extracted/grok.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|186|文件|`src/icons/extracted/grooroute.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|187|文件|`src/icons/extracted/huawei.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|188|文件|`src/icons/extracted/huggingface.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|189|文件|`src/icons/extracted/hunyuan.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|190|文件|`src/icons/extracted/huoshan.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|191|文件|`src/icons/extracted/ikuncode.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|192|文件|`src/icons/extracted/index.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|193|文件|`src/icons/extracted/kimi.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|194|文件|`src/icons/extracted/lioncc.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|195|文件|`src/icons/extracted/longcat-color.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|196|文件|`src/icons/extracted/meta.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|197|文件|`src/icons/extracted/metadata.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|198|文件|`src/icons/extracted/micu.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|199|文件|`src/icons/extracted/midjourney.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|200|文件|`src/icons/extracted/minimax.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|201|文件|`src/icons/extracted/mistral.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|202|文件|`src/icons/extracted/modelflare.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|203|文件|`src/icons/extracted/modelscope-color.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|204|文件|`src/icons/extracted/nekocode-icon.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|205|文件|`src/icons/extracted/newapi.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|206|文件|`src/icons/extracted/notion.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|207|文件|`src/icons/extracted/novita.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|208|文件|`src/icons/extracted/nvidia.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|209|文件|`src/icons/extracted/ollama.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|210|文件|`src/icons/extracted/openai.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|211|文件|`src/icons/extracted/opencode-logo-light.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|212|文件|`src/icons/extracted/openmodel.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|213|文件|`src/icons/extracted/openrouter.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|214|文件|`src/icons/extracted/ospreyai.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|215|文件|`src/icons/extracted/packycode.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|216|文件|`src/icons/extracted/palm.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|217|文件|`src/icons/extracted/pateway.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|218|文件|`src/icons/extracted/perplexity.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|219|文件|`src/icons/extracted/pipellm.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|220|文件|`src/icons/extracted/ppio.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|221|文件|`src/icons/extracted/qianwenai.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|222|文件|`src/icons/extracted/qiniu.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|223|文件|`src/icons/extracted/qwen.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|224|文件|`src/icons/extracted/qwencloud.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|225|文件|`src/icons/extracted/rc.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|226|文件|`src/icons/extracted/relaxcode.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|227|文件|`src/icons/extracted/runapi.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|228|文件|`src/icons/extracted/runtoken.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|229|文件|`src/icons/extracted/shengsuanyun.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|230|文件|`src/icons/extracted/siliconflow.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|231|文件|`src/icons/extracted/soruxgpt.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|232|文件|`src/icons/extracted/soshow.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|233|文件|`src/icons/extracted/sssaicode.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|234|文件|`src/icons/extracted/stability.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|235|文件|`src/icons/extracted/stepfun.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|236|文件|`src/icons/extracted/streamlake.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|237|文件|`src/icons/extracted/sub2api.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|238|文件|`src/icons/extracted/subrouter.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|239|文件|`src/icons/extracted/sudocode-us.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|240|文件|`src/icons/extracted/sudocode.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|241|文件|`src/icons/extracted/tencent.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|242|文件|`src/icons/extracted/tokenflux.jpg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|243|文件|`src/icons/extracted/tuzi.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|244|文件|`src/icons/extracted/ucloud.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|245|文件|`src/icons/extracted/unity2.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|246|文件|`src/icons/extracted/vercel.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|247|文件|`src/icons/extracted/wenxin.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|248|文件|`src/icons/extracted/xai.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|249|文件|`src/icons/extracted/xiaomimimo.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|250|文件|`src/icons/extracted/xycai-icon.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|251|文件|`src/icons/extracted/yi.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|252|文件|`src/icons/extracted/zeroone.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|253|文件|`src/icons/extracted/zetaapi-icon.png`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|254|文件|`src/icons/extracted/zhipu.svg`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|255|文件|`src/index.css`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|256|文件|`src/index.html`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|257|文件|`src/lib/api/customIcons.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|258|文件|`src/lib/api/deeplink.ts`|文件级入口|38—42|已映射，行为未验证|
|259|文件|`src/lib/api/dsh.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|260|文件|`src/lib/api/index.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|261|文件|`src/lib/api/model-fetch.test.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|262|文件|`src/lib/api/model-fetch.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|263|文件|`src/lib/api/pi.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|264|文件|`src/lib/api/proxy.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|265|文件|`src/lib/api/settings.ts`|文件级入口|13|已映射，行为未验证|
|266|文件|`src/lib/api/vscode.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|267|文件|`src/lib/claudePlan.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|268|文件|`src/lib/clipboard.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|269|文件|`src/lib/errors/liveEditConflict.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|270|文件|`src/lib/frontendLogger.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|271|文件|`src/lib/localCache.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|272|文件|`src/lib/navigation.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|273|文件|`src/lib/platform.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|274|文件|`src/lib/query/customIcons.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|275|文件|`src/lib/query/index.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|276|文件|`src/lib/query/mutations.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|277|文件|`src/lib/query/pi.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|278|文件|`src/lib/query/proxy.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|279|文件|`src/lib/query/queries.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|280|文件|`src/lib/query/queryClient.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|281|文件|`src/lib/query/usage.ts`|文件级入口|6|已映射，行为未验证|
|282|文件|`src/lib/toast.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|283|文件|`src/lib/unsavedChanges.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|284|文件|`src/lib/updater.ts`|文件级入口|47—51|已映射，行为未验证|
|285|文件|`src/lib/utils.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|286|文件|`src/lib/utils/base64.test.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|287|文件|`src/lib/utils/base64.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|288|文件|`src/lib/utils/sequentialBulkAction.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|289|文件|`src/lib/version.test.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|290|文件|`src/lib/version.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|291|文件|`src/lib/whatsNew.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|292|文件|`src/lib/windowActivity.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|293|文件|`src/main.tsx`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|294|文件|`src/tray-panel.html`|文件级入口|34—35|已映射，行为未验证|
|295|文件|`src/tray-panel/TrayPanel.tsx`|文件级入口|34—35|已映射，行为未验证|
|296|文件|`src/tray-panel/UsageTab.tsx`|文件级入口|34—35|已映射，行为未验证|
|297|文件|`src/tray-panel/api.ts`|文件级入口|34—35|已映射，行为未验证|
|298|文件|`src/tray-panel/main.tsx`|文件级入口|34—35|已映射，行为未验证|
|299|文件|`src/tray-panel/menuAim.ts`|文件级入口|34—35|已映射，行为未验证|
|300|文件|`src/tray-panel/tray-panel.css`|文件级入口|34—35|已映射，行为未验证|
|301|文件|`src/tray-panel/usePanelTheme.ts`|文件级入口|34—35|已映射，行为未验证|
|302|文件|`src/types.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|303|文件|`src/utils/claudeEditorOverlay.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|304|文件|`src/utils/customIcon.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|305|文件|`src/utils/deepClone.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|306|文件|`src/utils/deepLinkConfigPreview.ts`|文件级入口|38—42|已映射，行为未验证|
|307|文件|`src/utils/deeplinkRisk.test.ts`|文件级入口|38—42|已映射，行为未验证|
|308|文件|`src/utils/deeplinkRisk.ts`|文件级入口|38—42|已映射，行为未验证|
|309|文件|`src/utils/domUtils.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|310|文件|`src/utils/errorUtils.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|311|文件|`src/utils/formatters.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|312|文件|`src/utils/grokBuildConfig.test.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|313|文件|`src/utils/grokBuildConfig.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|314|文件|`src/utils/postChangeSync.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|315|文件|`src/utils/routeSwitchNotification.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|316|文件|`src/utils/routingReason.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|317|文件|`src/utils/textNormalization.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|318|文件|`src/utils/tomlUtils.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|319|文件|`src/utils/usageDisplay.ts`|文件级入口|6|已映射，行为未验证|
|320|文件|`src/utils/uuid.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|321|文件|`src/vite-env.d.ts`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|322|文件|`src/whats-new/4.0.0.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|323|文件|`src/whats-new/4.0.10.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|324|文件|`src/whats-new/4.0.11.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|325|文件|`src/whats-new/4.0.12.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|326|文件|`src/whats-new/4.0.13.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|327|文件|`src/whats-new/4.0.14.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|328|文件|`src/whats-new/4.0.15.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|329|文件|`src/whats-new/4.0.16.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|330|文件|`src/whats-new/4.0.17.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|331|文件|`src/whats-new/4.0.18.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|332|文件|`src/whats-new/4.0.19.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|333|文件|`src/whats-new/4.0.2.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|334|文件|`src/whats-new/4.0.20.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|335|文件|`src/whats-new/4.0.21.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|336|文件|`src/whats-new/4.0.22.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|337|文件|`src/whats-new/4.0.23.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|338|文件|`src/whats-new/4.0.24.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|339|文件|`src/whats-new/4.0.25.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|340|文件|`src/whats-new/4.0.3.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|341|文件|`src/whats-new/4.0.4.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|342|文件|`src/whats-new/4.0.5.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|343|文件|`src/whats-new/4.0.6.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|344|文件|`src/whats-new/4.0.7.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|345|文件|`src/whats-new/4.0.8.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|346|文件|`src/whats-new/4.0.9.json`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|347|文件|`src/whats-new/README.md`|文件级入口|2（核对后分配对应领域）|待细分，入口归属已记录|
|348|文件|`tests/hooks/useDirectorySettings.test.tsx`|文件级入口|13|已映射，行为未验证|
|349|文件|`tests/hooks/useSettings.test.tsx`|文件级入口|13|已映射，行为未验证|
|350|文件|`tests/hooks/useSettingsForm.test.tsx`|文件级入口|13|已映射，行为未验证|
|351|文件|`tests/hooks/useSettingsMetadata.test.tsx`|文件级入口|13|已映射，行为未验证|
|352|文件|`tests/hooks/useTrayNavigation.test.tsx`|文件级入口|34—35|已映射，行为未验证|
|353|文件|`tests/i18n/syncLanguageFromSettings.test.ts`|文件级入口|22|已映射，行为未验证|
|354|文件|`tests/integration/SettingsDialog.test.tsx`|文件级入口|13|已映射，行为未验证|
|355|文件|`tests/tray-panel/menuAim.test.ts`|文件级入口|34—35|已映射，行为未验证|
|356|文件|`tests/utils/deepLinkConfigPreview.test.ts`|文件级入口|38—42|已映射，行为未验证|
|357|命令候选|`src-tauri/src/commands/deeplink.rs`|peek_pending_deeplink|38—42|已映射，行为未验证|
|358|命令候选|`src-tauri/src/commands/deeplink.rs`|ack_pending_deeplink|38—42|已映射，行为未验证|
|359|命令候选|`src-tauri/src/commands/deeplink.rs`|parse_deeplink|38—42|已映射，行为未验证|
|360|命令候选|`src-tauri/src/commands/deeplink.rs`|merge_deeplink_config|38—42|已映射，行为未验证|
|361|命令候选|`src-tauri/src/commands/deeplink.rs`|fetch_deeplink_icon|38—42|已映射，行为未验证|
|362|命令候选|`src-tauri/src/commands/deeplink.rs`|import_from_deeplink|38—42|已映射，行为未验证|
|363|命令候选|`src-tauri/src/commands/deeplink.rs`|import_from_deeplink_unified|38—42|已映射，行为未验证|
|364|命令候选|`src-tauri/src/commands/lightweight.rs`|enter_lightweight_mode|31|已映射，行为未验证|
|365|命令候选|`src-tauri/src/commands/lightweight.rs`|exit_lightweight_mode|31|已映射，行为未验证|
|366|命令候选|`src-tauri/src/commands/lightweight.rs`|is_lightweight_mode|31|已映射，行为未验证|
|367|命令候选|`src-tauri/src/commands/settings.rs`|get_settings|13|已映射，行为未验证|
|368|命令候选|`src-tauri/src/commands/settings.rs`|save_settings|13|已映射，行为未验证|
|369|命令候选|`src-tauri/src/commands/settings.rs`|has_codex_unify_history_backup|13|已映射，行为未验证|
|370|命令候选|`src-tauri/src/commands/settings.rs`|codex_forces_multi_agent_v2|13|已映射，行为未验证|
|371|命令候选|`src-tauri/src/commands/settings.rs`|get_codex_session_compression|13|已映射，行为未验证|
|372|命令候选|`src-tauri/src/commands/settings.rs`|set_codex_session_compression|13|已映射，行为未验证|
|373|命令候选|`src-tauri/src/commands/settings.rs`|get_codex_sessions_disk_usage|13|已映射，行为未验证|
|374|命令候选|`src-tauri/src/commands/settings.rs`|restore_codex_unified_history|13|已映射，行为未验证|
|375|命令候选|`src-tauri/src/commands/settings.rs`|restart_app|13|已映射，行为未验证|
|376|命令候选|`src-tauri/src/commands/settings.rs`|install_update_and_restart|13|已映射，行为未验证|
|377|命令候选|`src-tauri/src/commands/settings.rs`|check_app_update_available|13|已映射，行为未验证|
|378|命令候选|`src-tauri/src/commands/settings.rs`|get_app_config_dir_override|13|已映射，行为未验证|
|379|命令候选|`src-tauri/src/commands/settings.rs`|set_app_config_dir_override|13|已映射，行为未验证|
|380|命令候选|`src-tauri/src/commands/settings.rs`|set_auto_launch|13|已映射，行为未验证|
|381|命令候选|`src-tauri/src/commands/settings.rs`|get_auto_launch_status|13|已映射，行为未验证|
|382|命令候选|`src-tauri/src/commands/settings.rs`|get_rectifier_config|13|已映射，行为未验证|
|383|命令候选|`src-tauri/src/commands/settings.rs`|set_rectifier_config|13|已映射，行为未验证|
|384|命令候选|`src-tauri/src/commands/settings.rs`|get_optimizer_config|13|已映射，行为未验证|
|385|命令候选|`src-tauri/src/commands/settings.rs`|set_optimizer_config|13|已映射，行为未验证|
|386|命令候选|`src-tauri/src/commands/settings.rs`|get_copilot_optimizer_config|13|已映射，行为未验证|
|387|命令候选|`src-tauri/src/commands/settings.rs`|set_copilot_optimizer_config|13|已映射，行为未验证|
|388|命令候选|`src-tauri/src/commands/settings.rs`|get_log_config|13|已映射，行为未验证|
|389|命令候选|`src-tauri/src/commands/settings.rs`|set_log_config|13|已映射，行为未验证|
|390|命令候选|`src-tauri/src/lib.rs`|update_tray_menu|34—35|已映射，行为未验证|
|391|命令候选|`src-tauri/src/tray.rs`|tray_app_page_seen|34—35|已映射，行为未验证|
|392|命令候选|`src-tauri/src/tray.rs`|take_tray_navigation|34—35|已映射，行为未验证|
|393|命令候选|`src-tauri/src/tray_panel.rs`|tray_panel_state|36—37|已映射，行为未验证|
|394|命令候选|`src-tauri/src/tray_panel.rs`|tray_panel_placement|36—37|已映射，行为未验证|
|395|命令候选|`src-tauri/src/tray_panel.rs`|tray_panel_layout|36—37|已映射，行为未验证|
|396|命令候选|`src-tauri/src/tray_panel.rs`|tray_panel_hide|36—37|已映射，行为未验证|
|397|命令候选|`src-tauri/src/tray_panel.rs`|tray_panel_action|36—37|已映射，行为未验证|
|398|事件候选|`src-tauri/src/commands/settings.rs`|update-download-progress|48|已映射，行为未验证|
|399|事件候选|`src-tauri/src/lib.rs`|deeplink-pending|38—42|已映射，行为未验证|
|400|事件候选|`src-tauri/src/tray.rs`|tray-navigate|34—35|已映射，行为未验证|
|401|事件候选|`src-tauri/src/tray.rs`|profile-applied|34—35|已映射，行为未验证|
|402|事件候选|`src-tauri/src/tray.rs`|proxy-flags-changed|34—35|已映射，行为未验证|
|403|事件候选|`src-tauri/src/tray.rs`|provider-switched|34—35|已映射，行为未验证|
|404|事件候选|`src/hooks/useTrayNavigation.ts`|tray-navigate|34—35|已映射，行为未验证|
|405|事件候选|`src/main.tsx`|configLoadError|19|已映射，行为未验证|
