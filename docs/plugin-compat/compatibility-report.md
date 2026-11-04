# 插件兼容报告与指南来源

任务 65，2026-11-04。产物为 [用户指南](user-guide.md) 与本报告；本任务不改变运行行为，不运行 Cargo、不领取槽位。基线登记 **299 项**，不是 299 项通过。任务 64 的 macOS 授权范围已完成（提交 `c22fe02a`），其 Linux/Windows/OAuth 跳过与任务 63 的真实账号豁免均不计通过。任务66已完成获准范围的固定快照回归；严格完整覆盖仍未通过，详见末节。

## 来源更新授权与边界

用户明确授权：“以当前 codex-main 源码核对这六处差异并更新来源基线，保留变更依据，不修改 plan.md 和 prompt.md”。实际只读参考根为 `/Users/xiangmin/worker/code/codex-main`，无 Git 元数据；不虚构版本号。初始基线提交 `8532fa46`，旧散列保留于下表及 baseline.json 的 source_revisions。

旧参考根不存在；参考目录无 Git 历史。相关 worker/Downloads/Shared/tmp 文件扫描、两个 Codex 压缩包、已登记 sources.json 和 Time Machine 查询均未提供六个旧文件完整内容；候选源码 55 个、冻结目标匹配 0，系统备份不可用。临时目录的 .vbox-root-ipc 无读取权限，未越权。此排查不证明全机没有备份。

因此本次进行**当前完整文件散列固定及登记符号行为复核**，不能还原或声称旧/新完整文本 diff。更新 6 个 sources 和 11 个 item 来源引用散列/行号，原有 expected、fields、tasks、basis、证据状态不变；旧验收记录仍描述各自当时运行，未伪装成针对新参考快照重跑。其余 49 个 Codex 源码散列仍匹配。新源码的额外 API 不自动加入兼容范围。

| 参考文件 | 原 SHA-256 | 当前 SHA-256 | 本次源码审阅结论 |
| --- | --- | --- | --- |
| `codex-rs/exec-server-protocol/src/protocol.rs` | `9fcea3d88e4d2e07cfb6611723e5242571cec3c0071c769dc712b74d9f2881fe` | `b34b6e3ddd50841ef15e24b64f582bef790090df9719aca0c9c464030437d6c9` | DISCOVERABLE_PLUGIN_MANIFEST_PATHS 的三入口顺序不变；常量现位于第 55 行。 |
| `codex-rs/core-plugins/src/loader.rs` | `4563a09ab7ad1f2400cfc71a1d0640d165f321f8c0f8c97e0a24c6494a74aaa7` | `0c16f0a25e4cb0c210c0d3cad21069d52b12a123273d1fe12bfbf54903b33c39` | load_plugin_skill_inventory 按 Legacy/AgentPlugin 选择 Recursive/DirectChildren；plugin_skill_roots 排序去重并仅为 legacy 附加迁移目录；默认 .mcp.json；load_plugin_hooks 显式路径/内联优先，缺省文件，读/解析失败保留 warning。顶层 agents 执行器仍无此入口依据。 |
| `codex-rs/codex-mcp/src/connection_pool.rs` | `8a39feca53cd9bdf9115b29f51e16c9e01d7d50ea9a5a42b0b7a82e9072655fe` | `451ec056a762648c06f332747663ef394090639ea68f43c4c4b5c4607e038e8d` | ConnectionKey 显式比较 server identity、account、启动超时、目录限制和 auth 三类 generation；chat 请求与 lease 分离，legacy/启动期输入保持私有。只登记已有隔离要求，不新增新协议承诺。 |
| `codex-rs/codex-mcp/src/rmcp_client.rs` | `096694dd52c0bbd4134de487f74f3e0d018cddc20adacfca46048ca2ddf191fd` | `441575e72e8a8ceeba7fac4513792f3b4b6e23c21515b55a380fc5f99afcf94c` | mcp_initialize_request_params 仍发送 V_2025_06_18，elicitation 和扩展独立构造，过滤仅 Core 管理的 lease capability；2026-07-28 不作为全局初始化版本。 |
| `codex-rs/codex-mcp/src/runtime.rs` | `120282b123e782efa44260cc126df0ee340544cf0f7fa5d2b5bcf0cf478936e9` | `1f9a71206ccb016d6ef70450ceca09bf449da1c1ed7b869013f17b636ce930b4` | invalidate_resource_caches 增加 generation；updated_oauth_credentials_after_auth_failure 从当前 connections/config 检查新凭据。不能由此推断宿主真实 OAuth 验收通过。 |
| `codex-rs/codex-mcp/src/resource_client.rs` | `e656d163cc12c970266c0346bfa2ba9099b02c3ca1f35adf78df1b8aeece3058` | `2f95a1b94ade62fa2883ffe699046c077aad2e56448518599c5116eb00844b40` | McpResourcePage 保留 next_cursor，McpResourceReadResult 保留 Vec<ResourceContent>；不由资源类型推断 prompts 或新增 event API 兼容。 |

## 已有证据与限制

| 能力 | 负责批次 | 可追溯证据与解释 |
| --- | --- | --- |
| 清单/路径/身份/安装 | 3–8 | 下方行为行链接对应任务记录；清单解析不替代真实运行 |
| 技能/命令/代理声明/onboarding | 9–13 | 仅实际加载/调用的声明；没有任意 agents 执行承诺 |
| MCP 配置、鉴权头、连接/目录/结果 | 14–23 | [t17](evidence/t17.json) 记录主动拒绝非法头、端点和重定向等安全差异 |
| 表单/OAuth/刷新/Apps | 24–31 | [t27](evidence/t27.json)、[t28](evidence/t28.json)、[t31](evidence/t31.json) 是分层证据；[真实账号范围豁免](public-mcp-oauth-evidence.md) 保留 |
| Hooks 事件与 handler | 32–46 | 对应逐项证据；prompt/agent 跳过、模型事件只观察等见 [scope](scope.md)，不是所有 handler 通用执行 |
| 市场/更新/CLI/Gateway/Iced | 47–56 | [t53](evidence/t53.json) 含三项目标测试与九项相邻 CLI 测试；本地目录登记限制不变 |
| 会话刷新/依赖/宿主工具 | 57–60 | [t58](evidence/t58.json) 是只读存在性与隔离检查，未知动态模块/远端就绪不能计通过 |
| 原包与匿名公开服务 | 61–62 | [t61](evidence/t61.json)、[t62](evidence/t62.json)、[公开服务说明](public-mcp-evidence.md)；Context7 原包锁定，服务部署版本未锁定 |
| OAuth 与平台 | 63–64 | [平台实际渲染证据](platform-evidence.md)、[t64](evidence/t64.json)；仅授权 macOS 范围，跳过不是测试通过 |
| 指南与最终验收 | 65–66 | 本报告是文档核查证据，不能替代任务 66 覆盖关闭 |

排除理由：托管连接器、私有目录/账号后端、专属运行时、品牌/令牌/私有白名单为用户明确排除；未知未来 schema 与模型逐字输出不在冻结行为承诺内。带 .app.json 的独立技能或公开 MCP 仍按范围内部分核验，不按包名整体排除。Windows/Linux 和真实账号验收属于用户跳过/豁免，不混同上述产品范围排除。

## 本任务审阅与验证

审阅文件包括六个上表参考文件的登记定义及其局部调用路径；CLI 顶层 vw-cli/src/cli.rs 的 Commands::Plugins 和 lib.rs 的实际 handler 接线（用户命令为 `shiju plugins`）；宿主 cli_commands.rs 的 PluginCommands/PluginMarketplaceCommands、plugins/cli.rs 的 handle_command、install.rs 的 install_dir_from_config、t53_tests.rs 的命令序列和同名来源隔离断言、t61_tests.rs 的原包散列断言、桌面 system_settings_skills/plugins.rs、system_settings_hooks.rs、system_settings_mcp.rs，以及 mcp_environment.rs、mcp_oauth_flow.rs、mcp_oauth_tokens.rs。此前和本次仅阅读相邻行为断言，没有把阅读记录当执行成功。

以下为 2026-11-04 本任务检查（退出码均为 0）：

- `python3 -m json.tool docs/plugin-compat/baseline.json`：JSON 可解析。
- Python hashlib 校验 baseline.sources 中全部 Codex 文件：55/55 当前散列匹配；212 个 Codex item 来源引用 的来源路径、散列和符号可定位。
- Python 核验 299 个 ID 唯一、任务映射与逐行索引一致，11 个受影响 item 的预期/字段/任务/basis/原证据状态相对 HEAD 未改变。
- Python 检查新增文档的502 个相对文件链接存在；命令逐项对照现有 CLI enum/handler 与已通过 t53/t61 证据。示例不执行网络、安装或 OAuth；不以文档核查声称运行验收重跑。
- `git diff --check` 与提交前 `git diff --cached --check`：无空白错误；只提交本任务文档及六处授权来源更新。

不运行 Cargo 是任务 65 的文档验证契约。历史证据各自保留实际平台、测试数、退出码和槽位；本报告没有新 Cargo 运行目录。

## 逐项来源与任务索引

每行对应 baseline.json 的一个稳定 ID；字段及精确预期以该 JSON 为准。以下来源索引可在 JSON 的 sources 找到完整路径及 SHA-256；登记后的源码证据不自动升级为执行通过。t65/t66 的当前记录分别见对应 evidence 文件；任务66按获准范围收尾，未验收项继续保留；其余链接只用于追溯记录，不表示其中没有限制、豁免或待验收。

| ID | 基线来源（symbol） | 负责任务与记录 |
| --- | --- | --- |
| `discovery.legacy-order` | `codex:codex-rs/exec-server-protocol/src/protocol.rs` / `DISCOVERABLE_PLUGIN_MANIFEST_PATHS` | [t3](evidence/t03.json), [t7](evidence/t07.json) |
| `discovery.portable-precedence` | `codex:codex-rs/utils/plugins/src/plugin_namespace.rs` / `find_plugin_manifest_path` | [t4](evidence/t04.json), [t7](evidence/t07.json) |
| `manifest.legacy.name` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.legacy.version` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.legacy.description` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.legacy.keywords` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.legacy.skills` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.legacy.mcpServers` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.legacy.apps` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.legacy.hooks` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.legacy.extensions` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifest` | [t3](evidence/t03.json) |
| `manifest.interface.displayName` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.shortDescription` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.longDescription` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.developerName` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.category` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.capabilities` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.websiteUrl` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.privacyPolicyUrl` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.termsOfServiceUrl` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.brandColor` | `codex:codex-rs/core-plugins/src/manifest.rs` / `RawPluginManifestInterface` | [t3](evidence/t03.json), [t55](evidence/t55.json) |
| `manifest.interface.composerIcon` | `codex:codex-rs/core-plugins/src/manifest.rs` / `resolve_interface_asset_path` | [t3](evidence/t03.json), [t5](evidence/t05.json), [t55](evidence/t55.json) |
| `manifest.interface.logo` | `codex:codex-rs/core-plugins/src/manifest.rs` / `resolve_interface_asset_path` | [t3](evidence/t03.json), [t5](evidence/t05.json), [t55](evidence/t55.json) |
| `manifest.interface.logoDark` | `codex:codex-rs/core-plugins/src/manifest.rs` / `resolve_interface_asset_path` | [t3](evidence/t03.json), [t5](evidence/t05.json), [t55](evidence/t55.json) |
| `manifest.interface.screenshots` | `codex:codex-rs/core-plugins/src/manifest.rs` / `resolve_interface_asset_path` | [t3](evidence/t03.json), [t5](evidence/t05.json), [t55](evidence/t55.json) |
| `manifest.interface.defaultPrompt` | `codex:codex-rs/core-plugins/src/manifest.rs` / `resolve_default_prompts` | [t3](evidence/t03.json) |
| `paths.manifest` | `codex:codex-rs/core-plugins/src/manifest.rs` / `resolve_manifest_path` | [t5](evidence/t05.json), [t64](evidence/t64.json) |
| `manifest.portable.$schema` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.name` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.version` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.description` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.author` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.homepage` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.repository` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.license` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.keywords` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.extensions` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json) |
| `manifest.portable.fixed-components` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `parse_agent_plugin_manifest_uri` | [t4](evidence/t04.json), [t9](evidence/t09.json), [t15](evidence/t15.json) |
| `manifest.portable.overlay` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `apply_codex_agent_plugin_extension` | [t4](evidence/t04.json), [t17](evidence/t17.json) |
| `manifest.portable.unknown` | `codex:codex-rs/core-plugins/src/agent_plugin_manifest.rs` / `AGENT_PLUGIN_FIELDS` | [t4](evidence/t04.json) |
| `onboarding.path` | `codex:codex-rs/core-plugins/src/manifest.rs` / `resolve_openai_onboarding_skill` | [t3](evidence/t03.json), [t4](evidence/t04.json), [t13](evidence/t13.json) |
| `skills.roots` | `codex:codex-rs/core-plugins/src/loader.rs` / `plugin_skill_roots` | [t9](evidence/t09.json) |
| `skills.discovery` | `codex:codex-rs/core-plugins/src/loader.rs` / `load_plugin_skill_inventory` | [t9](evidence/t09.json) |
| `skills.frontmatter` | `codex:codex-rs/skills/src/parser.rs` / `parse_skill_frontmatter_metadata` | [t9](evidence/t09.json) |
| `skills.mentions` | `codex:codex-rs/core/src/plugins/mentions.rs` / `collect_explicit_plugin_ids` | [t10](evidence/t10.json) |
| `commands.sources` | `codex:codex-rs/core-plugins/src/command_migration/plugin.rs` / `plugin_command_sources` | [t11](evidence/t11.json) |
| `commands.render` | `codex:codex-rs/core-plugins/src/command_migration.rs` / `render_command_skill` | [t11](evidence/t11.json) |
| `commands.size` | `codex:codex-rs/core-plugins/src/command_migration/plugin.rs` / `MAX_MIGRATED_COMMAND_SKILL_BYTES` | [t11](evidence/t11.json) |
| `agents.no-invented-executor` | `codex:codex-rs/core-plugins/src/loader.rs` / `load_plugin_skill_inventory` | [t12](evidence/t12.json) |
| `skill-agent-metadata.interface` | `codex:codex-rs/skills/src/metadata.rs` / `parse_skill_metadata` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `skill-agent-metadata.dependencies` | `codex:codex-rs/skills/src/metadata.rs` / `parse_skill_metadata` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `skill-agent-metadata.policy` | `codex:codex-rs/skills/src/metadata.rs` / `parse_skill_metadata` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `skill-dependency.type` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_dependency_tool` | [t9](evidence/t09.json), [t12](evidence/t12.json), [t58](evidence/t58.json) |
| `skill-dependency.value` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_dependency_tool` | [t9](evidence/t09.json), [t12](evidence/t12.json), [t58](evidence/t58.json) |
| `skill-dependency.description` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_dependency_tool` | [t9](evidence/t09.json), [t12](evidence/t12.json), [t58](evidence/t58.json) |
| `skill-dependency.transport` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_dependency_tool` | [t9](evidence/t09.json), [t12](evidence/t12.json), [t58](evidence/t58.json) |
| `skill-dependency.command` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_dependency_tool` | [t9](evidence/t09.json), [t12](evidence/t12.json), [t58](evidence/t58.json) |
| `skill-dependency.url` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_dependency_tool` | [t9](evidence/t09.json), [t12](evidence/t12.json), [t58](evidence/t58.json) |
| `skill-dependency.oauth.callbackPort` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_dependency_tool` | [t9](evidence/t09.json), [t12](evidence/t12.json), [t58](evidence/t58.json) |
| `skill-dependency.oauth.callback_port` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_dependency_tool` | [t9](evidence/t09.json), [t12](evidence/t12.json), [t58](evidence/t58.json) |
| `skill-policy.allow_implicit_invocation` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_policy` | [t10](evidence/t10.json), [t12](evidence/t12.json) |
| `skill-policy.products` | `codex:codex-rs/skills/src/metadata.rs` / `resolve_policy` | [t10](evidence/t10.json), [t12](evidence/t12.json) |
| `skill-interface.display_name` | `codex:codex-rs/skills/src/interface.rs` / `resolve_skill_interface` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `skill-interface.short_description` | `codex:codex-rs/skills/src/interface.rs` / `resolve_skill_interface` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `skill-interface.icon_small` | `codex:codex-rs/skills/src/interface.rs` / `resolve_skill_interface` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `skill-interface.icon_large` | `codex:codex-rs/skills/src/interface.rs` / `resolve_skill_interface` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `skill-interface.brand_color` | `codex:codex-rs/skills/src/interface.rs` / `resolve_skill_interface` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `skill-interface.default_prompt` | `codex:codex-rs/skills/src/interface.rs` / `resolve_skill_interface` | [t9](evidence/t09.json), [t12](evidence/t12.json) |
| `mcp.legacy.shape` | `codex:codex-rs/codex-mcp/src/plugin_config.rs` / `PluginMcpFile` | [t14](evidence/t14.json) |
| `mcp.legacy.aliases` | `codex:codex-rs/codex-mcp/src/plugin_config.rs` / `normalize_plugin_mcp_server_value` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.enterprise-auth` | `codex:codex-rs/codex-mcp/src/plugin_config.rs` / `normalize_plugin_mcp_server` | [t17](evidence/t17.json) |
| `mcp.legacy.cwd` | `codex:codex-rs/codex-mcp/src/plugin_config.rs` / `normalize_plugin_mcp_server_value` | [t16](evidence/t16.json) |
| `mcp.portable.file` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `AgentPluginMcpFile` | [t15](evidence/t15.json) |
| `mcp.portable.stdio.command` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `normalize_agent_plugin_stdio_server` | [t15](evidence/t15.json), [t16](evidence/t16.json) |
| `mcp.portable.stdio.args` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `normalize_agent_plugin_stdio_server` | [t15](evidence/t15.json), [t16](evidence/t16.json) |
| `mcp.portable.stdio.env` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `normalize_agent_plugin_stdio_server` | [t15](evidence/t15.json), [t16](evidence/t16.json) |
| `mcp.portable.stdio.cwd` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `normalize_agent_plugin_stdio_server` | [t15](evidence/t15.json), [t16](evidence/t16.json) |
| `mcp.portable.path-boundary` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `resolve_existing_path_prefix` | [t5](evidence/t05.json), [t16](evidence/t16.json), [t64](evidence/t64.json) |
| `mcp.portable.http-url` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `validate_agent_plugin_url` | [t15](evidence/t15.json), [t17](evidence/t17.json) |
| `mcp.portable.http-headers` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `validate_agent_plugin_headers` | [t15](evidence/t15.json), [t17](evidence/t17.json) |
| `mcp.portable.sse` | `codex:codex-rs/codex-mcp/src/agent_plugin_config.rs` / `normalize_agent_plugin_mcp_server` | [t15](evidence/t15.json) |
| `mcp.portable.env-overlay` | `codex:codex-rs/core-plugins/src/agent_plugin_mcp_overlay.rs` / `apply_codex_env_overlay` | [t15](evidence/t15.json), [t17](evidence/t17.json) |
| `mcp.legacy.default` | `codex:codex-rs/core-plugins/src/loader.rs` / `DEFAULT_MCP_CONFIG_FILE` | [t14](evidence/t14.json) |
| `mcp.session-isolation` | `codex:codex-rs/codex-mcp/src/connection_pool.rs` / `McpConnectionPool` | [t18](evidence/t18.json) |
| `mcp.initialize` | `codex:codex-rs/codex-mcp/src/rmcp_client.rs` / `mcp_initialize_request_params` | [t23](evidence/t23.json) |
| `mcp.tool-identity` | `codex:codex-rs/codex-mcp/src/tools.rs` / `normalize_tools_for_model_with_prefix` | [t19](evidence/t19.json) |
| `mcp.tool-filter` | `codex:codex-rs/codex-mcp/src/tools.rs` / `ToolFilter` | [t19](evidence/t19.json), [t23](evidence/t23.json) |
| `mcp.pagination` | `codex:codex-rs/codex-mcp/src/pagination.rs` / `collect_paginated_with_limit` | [t19](evidence/t19.json), [t20](evidence/t20.json), [t22](evidence/t22.json) |
| `mcp.refresh` | `codex:codex-rs/codex-mcp/src/runtime.rs` / `invalidate_resource_caches` | [t20](evidence/t20.json) |
| `mcp.result.content` | `codex:codex-rs/protocol/src/mcp.rs` / `CallToolResult` | [t21](evidence/t21.json) |
| `mcp.result.structuredContent` | `codex:codex-rs/protocol/src/mcp.rs` / `CallToolResult` | [t21](evidence/t21.json) |
| `mcp.result.isError` | `codex:codex-rs/protocol/src/mcp.rs` / `CallToolResult` | [t21](evidence/t21.json) |
| `mcp.result._meta` | `codex:codex-rs/protocol/src/mcp.rs` / `CallToolResult` | [t21](evidence/t21.json) |
| `mcp.resources` | `codex:codex-rs/codex-mcp/src/resource_client.rs` / `McpResourceReadResult` | [t22](evidence/t22.json) |
| `mcp.capability.openai/form` | `codex:codex-rs/codex-mcp/src/client_capabilities.rs` / `client_mcp_extensions` | [t23](evidence/t23.json) |
| `mcp.capability.openai/elicitation` | `codex:codex-rs/codex-mcp/src/client_capabilities.rs` / `client_mcp_extensions` | [t23](evidence/t23.json) |
| `mcp.capability.openai/standard-form-input` | `codex:codex-rs/codex-mcp/src/client_capabilities.rs` / `client_mcp_extensions` | [t23](evidence/t23.json) |
| `mcp.capability.io.modelcontextprotocol/ui` | `codex:codex-rs/codex-mcp/src/client_capabilities.rs` / `client_mcp_extensions` | [t23](evidence/t23.json) |
| `mcp.capability.privileged` | `codex:codex-rs/codex-mcp/src/client_capabilities.rs` / `server_mcp_extensions` | [t23](evidence/t23.json), [t24](evidence/t24.json) |
| `mcp.elicitation` | `codex:codex-rs/codex-mcp/src/elicitation.rs` / `ElicitationRequestRouter` | [t24](evidence/t24.json), [t25](evidence/t25.json) |
| `mcp.oauth.discovery` | `codex:codex-rs/codex-mcp/src/mcp/auth.rs` / `oauth_login_support` | [t26](evidence/t26.json) |
| `mcp.oauth.scopes` | `codex:codex-rs/codex-mcp/src/mcp/auth.rs` / `resolve_oauth_scopes` | [t17](evidence/t17.json), [t26](evidence/t26.json) |
| `mcp.oauth.registration` | `codex:codex-rs/rmcp-client/src/oauth_client_registration.rs` / `start_authorization` | [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `mcp.oauth.refresh` | `codex:codex-rs/codex-mcp/src/runtime.rs` / `updated_oauth_credentials_after_auth_failure` | [t28](evidence/t28.json) |
| `mcp.apps.declaration` | `codex:codex-rs/protocol/src/mcp.rs` / `MCP_APP_UI_EXTENSION_ID` | [t29](evidence/t29.json), [t30](evidence/t30.json), [t31](evidence/t31.json) |
| `hooks.file` | `codex:codex-rs/config/src/hook_config.rs` / `HooksFile` | [t32](evidence/t32.json) |
| `hooks.default` | `codex:codex-rs/core-plugins/src/loader.rs` / `load_plugin_hooks` | [t32](evidence/t32.json) |
| `hooks.command.command` | `codex:codex-rs/protocol/src/hooks.rs` / `HookHandlerConfig` | [t32](evidence/t32.json), [t34](evidence/t34.json), [t35](evidence/t35.json) |
| `hooks.command.commandWindows` | `codex:codex-rs/protocol/src/hooks.rs` / `HookHandlerConfig` | [t32](evidence/t32.json), [t34](evidence/t34.json), [t35](evidence/t35.json) |
| `hooks.command.command_windows` | `codex:codex-rs/protocol/src/hooks.rs` / `HookHandlerConfig` | [t32](evidence/t32.json), [t34](evidence/t34.json), [t35](evidence/t35.json) |
| `hooks.command.timeout` | `codex:codex-rs/protocol/src/hooks.rs` / `HookHandlerConfig` | [t32](evidence/t32.json), [t34](evidence/t34.json), [t35](evidence/t35.json) |
| `hooks.command.async` | `codex:codex-rs/protocol/src/hooks.rs` / `HookHandlerConfig` | [t32](evidence/t32.json), [t34](evidence/t34.json), [t35](evidence/t35.json) |
| `hooks.command.statusMessage` | `codex:codex-rs/protocol/src/hooks.rs` / `HookHandlerConfig` | [t32](evidence/t32.json), [t34](evidence/t34.json), [t35](evidence/t35.json) |
| `hooks.command.additionalContextLimit` | `codex:codex-rs/protocol/src/hooks.rs` / `HookHandlerConfig` | [t32](evidence/t32.json), [t34](evidence/t34.json), [t35](evidence/t35.json) |
| `hooks.mcp.server` | `codex:codex-rs/protocol/src/hooks.rs` / `McpTool` | [t32](evidence/t32.json), [t45](evidence/t45.json) |
| `hooks.mcp.tool` | `codex:codex-rs/protocol/src/hooks.rs` / `McpTool` | [t32](evidence/t32.json), [t45](evidence/t45.json) |
| `hooks.mcp.input` | `codex:codex-rs/protocol/src/hooks.rs` / `McpTool` | [t32](evidence/t32.json), [t45](evidence/t45.json) |
| `hooks.mcp.timeout` | `codex:codex-rs/protocol/src/hooks.rs` / `McpTool` | [t32](evidence/t32.json), [t45](evidence/t45.json) |
| `hooks.mcp.statusMessage` | `codex:codex-rs/protocol/src/hooks.rs` / `McpTool` | [t32](evidence/t32.json), [t45](evidence/t45.json) |
| `hooks.handler.prompt-agent` | `codex:codex-rs/hooks/src/engine/discovery.rs` / `HookHandlerConfig::Prompt` | [t32](evidence/t32.json), [t45](evidence/t45.json) |
| `hooks.matcher` | `codex:codex-rs/hooks/src/engine/matcher.rs` / `HookMatcher` | [t33](evidence/t33.json), [t64](evidence/t64.json) |
| `hooks.matcher-events` | `codex:codex-rs/hooks/src/engine/dispatcher.rs` / `select_handlers_for_matcher_inputs` | [t33](evidence/t33.json) |
| `hooks.timeout` | `codex:codex-rs/hooks/src/engine/discovery.rs` / `normalize_command_hook` | [t34](evidence/t34.json), [t39](evidence/t39.json), [t44](evidence/t44.json) |
| `hooks.command-process` | `codex:codex-rs/hooks/src/engine/command_runner.rs` / `run_command` | [t34](evidence/t34.json) |
| `hooks.shell` | `codex:codex-rs/hooks/src/engine/command_runner.rs` / `build_command` | [t34](evidence/t34.json), [t64](evidence/t64.json) |
| `hooks.async` | `codex:codex-rs/hooks/src/engine/command_runner.rs` / `MAX_CONCURRENT_ASYNC_HOOKS` | [t35](evidence/t35.json) |
| `hooks.session-end-exceptions` | `codex:codex-rs/hooks/src/engine/discovery.rs` / `let runs_async` | [t35](evidence/t35.json), [t39](evidence/t39.json) |
| `hooks.event.PreToolUse` | `codex:codex-rs/hooks/src/events/pre_tool_use.rs` / `PreToolUseRequest` | [t36](evidence/t36.json) |
| `hooks.event.PermissionRequest` | `codex:codex-rs/hooks/src/events/permission_request.rs` / `PermissionRequestRequest` | [t37](evidence/t37.json) |
| `hooks.event.PostToolUse` | `codex:codex-rs/hooks/src/events/post_tool_use.rs` / `PostToolUseRequest` | [t38](evidence/t38.json) |
| `hooks.event.SessionStart` | `codex:codex-rs/hooks/src/events/session_start.rs` / `SessionStartRequest` | [t39](evidence/t39.json) |
| `hooks.event.SessionEnd` | `codex:codex-rs/hooks/src/events/session_end.rs` / `SessionEndRequest` | [t39](evidence/t39.json) |
| `hooks.event.UserPromptSubmit` | `codex:codex-rs/hooks/src/events/user_prompt_submit.rs` / `UserPromptSubmitRequest` | [t40](evidence/t40.json) |
| `hooks.event.PreModelRequest` | `codex:codex-rs/hooks/src/events/model_request.rs` / `ModelRequestHookInput` | [t41](evidence/t41.json) |
| `hooks.event.PostModelRequest` | `codex:codex-rs/hooks/src/events/model_request.rs` / `ModelRequestHookInput` | [t41](evidence/t41.json) |
| `hooks.event.PreCompact` | `codex:codex-rs/hooks/src/events/compact.rs` / `PreCompactRequest` | [t42](evidence/t42.json) |
| `hooks.event.PostCompact` | `codex:codex-rs/hooks/src/events/compact.rs` / `PostCompactRequest` | [t42](evidence/t42.json) |
| `hooks.event.SubagentStart` | `codex:codex-rs/hooks/src/events/session_start.rs` / `SessionStartRequest` | [t43](evidence/t43.json) |
| `hooks.event.SubagentStop` | `codex:codex-rs/hooks/src/events/stop.rs` / `StopRequest` | [t43](evidence/t43.json) |
| `hooks.event.Stop` | `codex:codex-rs/hooks/src/events/stop.rs` / `StopRequest` | [t44](evidence/t44.json) |
| `hooks.event.Interrupt` | `codex:codex-rs/hooks/src/events/interrupt.rs` / `InterruptRequest` | [t44](evidence/t44.json) |
| `hooks.mcp-templates` | `codex:codex-rs/hooks/src/engine/mcp_runner.rs` / `expand_mcp_argument_template` | [t45](evidence/t45.json) |
| `hooks.trust` | `codex:codex-rs/hooks/src/engine/plugin_trust.rs` / `PluginHookTrust` | [t46](evidence/t46.json) |
| `hooks.fingerprint` | `codex:codex-rs/hooks/src/engine/plugin_fingerprint.rs` / `plugin_content_hash` | [t46](evidence/t46.json) |
| `marketplace.field.name` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifest` | [t47](evidence/t47.json) |
| `marketplace.field.interface.displayName` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifest` | [t47](evidence/t47.json) |
| `marketplace.field.plugins[].name` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifest` | [t47](evidence/t47.json) |
| `marketplace.field.plugins[].source` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifest` | [t47](evidence/t47.json) |
| `marketplace.field.plugins[].category` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifest` | [t47](evidence/t47.json) |
| `marketplace.field.plugins[].policy.installation` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifest` | [t47](evidence/t47.json) |
| `marketplace.field.plugins[].policy.authentication` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifest` | [t47](evidence/t47.json) |
| `marketplace.field.plugins[].policy.products` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifest` | [t47](evidence/t47.json) |
| `marketplace.paths` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `MARKETPLACE_MANIFEST_RELATIVE_PATHS` | [t47](evidence/t47.json) |
| `marketplace.local` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `resolve_local_plugin_source_path` | [t47](evidence/t47.json) |
| `marketplace.git.url` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifestPluginSourceObject` | [t48](evidence/t48.json) |
| `marketplace.git.path` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifestPluginSourceObject` | [t48](evidence/t48.json) |
| `marketplace.git.ref` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifestPluginSourceObject` | [t48](evidence/t48.json) |
| `marketplace.git.sha` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `RawMarketplaceManifestPluginSourceObject` | [t48](evidence/t48.json) |
| `marketplace.git-policy` | `codex:codex-rs/core-plugins/src/git_policy.rs` / `PluginGitMode` | [t48](evidence/t48.json) |
| `marketplace.npm.package` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `normalize_optional_npm_registry` | [t49](evidence/t49.json) |
| `marketplace.npm.version` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `normalize_optional_npm_registry` | [t49](evidence/t49.json) |
| `marketplace.npm.registry` | `codex:codex-rs/core-plugins/src/marketplace.rs` / `normalize_optional_npm_registry` | [t49](evidence/t49.json) |
| `marketplace.npm.pack` | `codex:codex-rs/core-plugins/src/npm_source.rs` / `pack_npm_package` | [t49](evidence/t49.json) |
| `marketplace.npm.archive` | `codex:codex-rs/core-plugins/src/npm_source.rs` / `materialize_npm_plugin_source_with_command` | [t50](evidence/t50.json) |
| `marketplace.npm.unpack` | `codex:codex-rs/core-plugins/src/plugin_bundle_archive.rs` / `unpack_plugin_bundle_tar` | [t50](evidence/t50.json) |
| `identity.key` | `codex:codex-rs/core-plugin-common/src/plugin_id.rs` / `PluginId` | [t6](evidence/t06.json) |
| `store.layout` | `codex:codex-rs/core-plugins/src/store.rs` / `PluginStore` | [t8](evidence/t08.json), [t51](evidence/t51.json) |
| `store.toggle` | `codex:codex-rs/core-plugins/src/toggles.rs` / `collect_plugin_enabled_candidates` | [t51](evidence/t51.json) |
| `session.selection` | `codex:codex-rs/core/src/session/plugin_selection.rs` / `activate_plugin_selection` | [t57](evidence/t57.json) |
| `scope.hosted-apps` | `codex:codex-rs/core-plugins/src/app_mcp_routing.rs` / `apps_route_available` | [t17](evidence/t17.json), [t58](evidence/t58.json) |
| `acceptance.t02` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/2-建立行为夹具入口.md` / `目的：` | [t2](evidence/t02.json) |
| `acceptance.t03` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/3-兼容旧版插件清单.md` / `目的：` | [t3](evidence/t03.json) |
| `acceptance.t04` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/4-兼容便携插件清单.md` / `目的：` | [t4](evidence/t04.json) |
| `acceptance.t05` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/5-审计兼容组件路径.md` / `目的：` | [t5](evidence/t05.json) |
| `acceptance.t06` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/6-建立插件来源标识.md` / `目的：` | [t6](evidence/t06.json) |
| `acceptance.t07` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/7-统一插件发现顺序.md` / `目的：` | [t7](evidence/t07.json) |
| `acceptance.t08` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/8-原包事务安装.md` / `目的：` | [t8](evidence/t08.json) |
| `acceptance.t09` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/9-加载多技能目录.md` / `目的：` | [t9](evidence/t09.json) |
| `acceptance.t10` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/10-保持技能调用语义.md` / `目的：` | [t10](evidence/t10.json) |
| `acceptance.t11` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/11-兼容插件命令.md` / `目的：` | [t11](evidence/t11.json) |
| `acceptance.t12` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/12-兼容代理声明.md` / `目的：` | [t12](evidence/t12.json) |
| `acceptance.t13` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/13-提供安装引导入口.md` / `目的：` | [t13](evidence/t13.json) |
| `acceptance.t14` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/14-兼容旧版-MCP-声明.md` / `目的：` | [t14](evidence/t14.json) |
| `acceptance.t15` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/15-兼容便携-MCP-声明.md` / `目的：` | [t15](evidence/t15.json) |
| `acceptance.t16` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/16-解析-MCP-运行目录.md` / `目的：` | [t16](evidence/t16.json) |
| `acceptance.t17` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/17-配置-MCP-公开鉴权.md` / `目的：` | [t17](evidence/t17.json) |
| `acceptance.t18` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/18-保持-MCP-连接生命周期.md` / `目的：` | [t18](evidence/t18.json) |
| `acceptance.t19` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/19-保持-MCP-工具标识.md` / `目的：` | [t19](evidence/t19.json) |
| `acceptance.t20` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/20-刷新-MCP-动态目录.md` / `目的：` | [t20](evidence/t20.json) |
| `acceptance.t21` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/21-保留-MCP-调用结果.md` / `目的：` | [t21](evidence/t21.json) |
| `acceptance.t22` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/22-读取-MCP-资源与提示.md` / `目的：` | [t22](evidence/t22.json) |
| `acceptance.t23` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/23-协商-MCP-客户端能力.md` / `目的：` | [t23](evidence/t23.json) |
| `acceptance.t24` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/24-接入-MCP-表单请求.md` / `目的：` | [t24](evidence/t24.json) |
| `acceptance.t25` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/25-显示-MCP-交互表单.md` / `目的：` | [t25](evidence/t25.json) |
| `acceptance.t26` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/26-发现公开-MCP-授权配置.md` / `目的：` | [t26](evidence/t26.json) |
| `acceptance.t27` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/27-完成-MCP-授权交互.md` / `目的：` | [t27](evidence/t27.json) |
| `acceptance.t28` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/28-管理-MCP-授权有效期.md` / `目的：` | [t28](evidence/t28.json) |
| `acceptance.t29` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/29-识别-MCP-应用资源.md` / `目的：` | [t29](evidence/t29.json) |
| `acceptance.t30` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/30-隔离-MCP-应用桥接.md` / `目的：` | [t30](evidence/t30.json) |
| `acceptance.t31` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/31-渲染-MCP-应用面板.md` / `目的：` | [t31](evidence/t31.json) |
| `acceptance.t32` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/32-解析-Codex-Hooks.md` / `目的：` | [t32](evidence/t32.json) |
| `acceptance.t33` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/33-匹配-Hooks-执行条件.md` / `目的：` | [t33](evidence/t33.json) |
| `acceptance.t34` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/34-执行同步命令钩子.md` / `目的：` | [t34](evidence/t34.json) |
| `acceptance.t35` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/35-管理异步命令钩子.md` / `目的：` | [t35](evidence/t35.json) |
| `acceptance.t36` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/36-适配工具前置事件.md` / `目的：` | [t36](evidence/t36.json) |
| `acceptance.t37` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/37-适配权限请求事件.md` / `目的：` | [t37](evidence/t37.json) |
| `acceptance.t38` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/38-适配工具后置事件.md` / `目的：` | [t38](evidence/t38.json) |
| `acceptance.t39` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/39-适配会话生命周期.md` / `目的：` | [t39](evidence/t39.json) |
| `acceptance.t40` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/40-适配用户输入事件.md` / `目的：` | [t40](evidence/t40.json) |
| `acceptance.t41` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/41-适配模型请求事件.md` / `目的：` | [t41](evidence/t41.json) |
| `acceptance.t42` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/42-适配压缩生命周期.md` / `目的：` | [t42](evidence/t42.json) |
| `acceptance.t43` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/43-适配委托代理生命周期.md` / `目的：` | [t43](evidence/t43.json) |
| `acceptance.t44` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/44-适配停止与中断事件.md` / `目的：` | [t44](evidence/t44.json) |
| `acceptance.t45` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/45-执行公开-MCP-钩子.md` / `目的：` | [t45](evidence/t45.json) |
| `acceptance.t46` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/46-绑定-Hooks-内容信任.md` / `目的：` | [t46](evidence/t46.json) |
| `acceptance.t47` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/47-读取插件市场清单.md` / `目的：` | [t47](evidence/t47.json) |
| `acceptance.t48` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/48-物化-Git-插件来源.md` / `目的：` | [t48](evidence/t48.json) |
| `acceptance.t49` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/49-获取-NPM-插件包.md` / `目的：` | [t49](evidence/t49.json) |
| `acceptance.t50` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/50-安全解包-NPM-插件.md` / `目的：` | [t50](evidence/t50.json) |
| `acceptance.t51` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/51-持久化市场安装状态.md` / `目的：` | [t51](evidence/t51.json) |
| `acceptance.t52` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/52-事务更新插件版本.md` / `目的：` | [t52](evidence/t52.json) |
| `acceptance.t53` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/53-提供命令行市场入口.md` / `目的：` | [t53](evidence/t53.json) |
| `acceptance.t54` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/54-暴露网关插件操作.md` / `目的：` | [t54](evidence/t54.json) |
| `acceptance.t55` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/55-展示插件兼容状态.md` / `目的：` | [t55](evidence/t55.json) |
| `acceptance.t56` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/56-提供插件安装弹窗.md` / `目的：` | [t56](evidence/t56.json) |
| `acceptance.t57` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/57-更新会话插件快照.md` / `目的：` | [t57](evidence/t57.json) |
| `acceptance.t58` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/58-检查插件运行依赖.md` / `目的：` | [t58](evidence/t58.json) |
| `acceptance.t59` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/59-适配宿主命令工具.md` / `目的：` | [t59](evidence/t59.json) |
| `acceptance.t60` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/60-适配宿主文件工具.md` / `目的：` | [t60](evidence/t60.json) |
| `acceptance.t61` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/61-验证原包端到端运行.md` / `目的：` | [t61](evidence/t61.json) |
| `acceptance.t62` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/62-联调匿名公开-MCP.md` / `目的：` | [t62](evidence/t62.json) |
| `acceptance.t63` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/63-联调公开-MCP-授权.md` / `目的：` | [t63](evidence/t63.json) |
| `acceptance.t64` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/64-验证平台行为矩阵.md` / `目的：` | [t64](evidence/t64.json) |
| `acceptance.t65` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/65-编写插件兼容指南.md` / `目的：` | [t65](evidence/t65.json) |
| `acceptance.t66` | `vibewindow:.plans/2026-10-30-Codex-插件兼容/66-关闭兼容覆盖缺口.md` / `目的：` | [t66](evidence/t66.json) |
| `official.portable` | `official:packaging` / `plugin-structure` | [t4](evidence/t04.json), [t15](evidence/t15.json) |
| `official.oauth` | `official:packaging` / `oauth-configuration-reference` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.api-key` | `official:packaging` / `fixed-bearer-tokens-and-api-keys` | [t17](evidence/t17.json) |
| `official.distribution` | `official:packaging` / `marketplace-metadata` | [t47](evidence/t47.json), [t53](evidence/t53.json) |
| `official.hooks` | `official:packaging` / `bundled-mcp-servers-and-lifecycle-hooks` | [t32](evidence/t32.json), [t39](evidence/t39.json), [t46](evidence/t46.json) |
| `mcp.legacy.transport.command` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t16](evidence/t16.json) |
| `mcp.legacy.transport.args` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t16](evidence/t16.json) |
| `mcp.legacy.transport.env` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t16](evidence/t16.json) |
| `mcp.legacy.transport.env_vars` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t16](evidence/t16.json), [t17](evidence/t17.json) |
| `mcp.legacy.transport.cwd` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t16](evidence/t16.json) |
| `mcp.legacy.transport.url` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.transport.bearer_token_env_var` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.transport.http_headers` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.transport.env_http_headers` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.transport.http_headers_helper` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerTransportConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json), [t58](evidence/t58.json) |
| `mcp.legacy.config.enabled` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t51](evidence/t51.json) |
| `mcp.legacy.config.required` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t18](evidence/t18.json) |
| `mcp.legacy.config.startup_readiness` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t18](evidence/t18.json) |
| `mcp.legacy.config.supports_parallel_tool_calls` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t19](evidence/t19.json) |
| `mcp.legacy.config.tool_input_schema_max_bytes` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t19](evidence/t19.json) |
| `mcp.legacy.config.omit_tools_from` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t19](evidence/t19.json) |
| `mcp.legacy.config.startup_timeout_sec` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t18](evidence/t18.json) |
| `mcp.legacy.config.tool_timeout_sec` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t21](evidence/t21.json) |
| `mcp.legacy.config.default_tools_approval_mode` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.config.enabled_tools` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t19](evidence/t19.json) |
| `mcp.legacy.config.disabled_tools` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t19](evidence/t19.json) |
| `mcp.legacy.config.scopes` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json), [t26](evidence/t26.json) |
| `mcp.legacy.config.oauth` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `mcp.legacy.config.oauth_resource` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json), [t26](evidence/t26.json) |
| `mcp.legacy.config.tools` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.config.auth` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.config.environment_id` | `codex:codex-rs/config/src/mcp_types.rs` / `McpServerConfig` | [t14](evidence/t14.json), [t18](evidence/t18.json) |
| `hooks.output-limit.pre-tool` | `codex:codex-rs/hooks/src/engine/output_parser.rs` / `unsupported_pre_tool_use_universal` | [t36](evidence/t36.json) |
| `hooks.output-limit.permission` | `codex:codex-rs/hooks/src/engine/output_parser.rs` / `unsupported_permission_request_hook_specific_output` | [t37](evidence/t37.json) |
| `hooks.output-limit.post-tool` | `codex:codex-rs/hooks/src/engine/output_parser.rs` / `unsupported_post_tool_use_hook_specific_output` | [t38](evidence/t38.json) |
| `official.auth.type` | `official:packaging` / `extensions.com.openai.auth.type` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.client.mode` | `official:packaging` / `extensions.com.openai.auth.client.mode` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.client.clientId` | `official:packaging` / `extensions.com.openai.auth.client.clientId` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.client.tokenEndpointAuthMethod` | `official:packaging` / `extensions.com.openai.auth.client.tokenEndpointAuthMethod` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.authorizationUrl` | `official:packaging` / `extensions.com.openai.auth.authorizationUrl` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.tokenUrl` | `official:packaging` / `extensions.com.openai.auth.tokenUrl` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.registrationUrl` | `official:packaging` / `extensions.com.openai.auth.registrationUrl` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.authorizationServerBase` | `official:packaging` / `extensions.com.openai.auth.authorizationServerBase` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.resource` | `official:packaging` / `extensions.com.openai.auth.resource` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.baseScopes` | `official:packaging` / `extensions.com.openai.auth.baseScopes` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.defaultScopes` | `official:packaging` / `extensions.com.openai.auth.defaultScopes` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.oidcEnabled` | `official:packaging` / `extensions.com.openai.auth.oidcEnabled` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.oidcConfigurationUrl` | `official:packaging` / `extensions.com.openai.auth.oidcConfigurationUrl` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.oidcUserinfoEndpoint` | `official:packaging` / `extensions.com.openai.auth.oidcUserinfoEndpoint` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.oidcScopesSupported` | `official:packaging` / `extensions.com.openai.auth.oidcScopesSupported` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.headerScheme` | `official:packaging` / `extensions.com.openai.auth.headerScheme` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `official.auth.headerName` | `official:packaging` / `extensions.com.openai.auth.headerName` | [t17](evidence/t17.json), [t26](evidence/t26.json), [t27](evidence/t27.json) |
| `mcp.legacy.extra.startup_timeout_ms` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.name` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.bearer_token` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.oauth.client_id` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.oauth.client_secret` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.oauth.callback_url` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.oauth.callback_port` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.oauth.token_endpoint_auth_method` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.oauth.authorization_server_issuer` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.oauth.ema_registration` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.tools.<name>.approval_mode` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |
| `mcp.legacy.extra.tools.<name>.output_token_limit` | `codex:codex-rs/config/src/mcp_types.rs` / `RawMcpServerConfig` | [t14](evidence/t14.json), [t17](evidence/t17.json) |

## 任务66：初轮记录（历史）

2026-11-04 用户明确继续豁免 Windows、Linux 和真实 OAuth，要求先完成汇总、拒绝/隔离测试、macOS 回归及 clippy，并保留未验收项。此次不改变 baseline.json、plan.md、prompt.md，也不改严格检查器的任何要求。**当时未通过后端回归并保留待回归；最新固定快照结果见末节。不宣称完全兼容。**

[汇总 evidence.json](evidence.json) 包含 299 个稳定 ID、66 份来源记录的路径/散列与逐项任务引用。它按完整基线标准显示 298 项 unverified、1 项 excluded；0 项严格完整 verified。这里的 unverified 不表示 298 项实现均失败：原始任务级通过记录仍保留，逐项完整证据、任务联合映射和三平台/真实外部验收没有被批量补成通过。`scope.hosted-apps` 是基线唯一 excluded 项，用户平台/账号豁免没有改写为 excluded 或 verified。任务65的文档核验另见 [t65](evidence/t65.json)。

复现汇总与完整检查：

```sh
python3 scripts/plugin_compat/summarize_evidence.py --baseline docs/plugin-compat/baseline.json --evidence-dir docs/plugin-compat/evidence --output docs/plugin-compat/evidence.json
python3 scripts/plugin_compat/check_evidence.py --baseline docs/plugin-compat/baseline.json --evidence docs/plugin-compat/evidence.json
```

第一条退出 0 仅代表汇总成功；第二条当前退出 1，拒绝 `discovery.legacy-order: conclusion is pending or unverified`，不是获准完成全面兼容验收。汇总器不覆盖原输入或包内证据，不让未知 ID 满足其他能力，不以任务状态、豁免或 mock 填造 green/平台证据；输出先完成全部解析，再原子替换。

| 当前命令/阶段 | 结果 |
| --- | --- |
| `cargo fmt --all` | 退出0，检查并隔离其他工作区变更；无需恢复无关格式差异 |
| `cargo test -p vw-agent --lib --all-features codex_plugin_compat_t66_ -- --nocapture` | skeleton 三项目标断言失败，退出101；实现后3 passed、0 failed；最终再次3 passed、0 failed |
| `cargo test -p vw-agent --lib --all-features codex_plugin_compat_ -- --nocapture` | 退出101；计划448项，观察250个 ok 后在 t27 取消测试监听端口断言处中止，无完整通过汇总 |
| 同上追加 `--test-threads=1` | 退出101；计划448项，观察271个 ok 后在 t32 旧的“执行不受支持”断言处中止，无完整通过汇总 |
| `cargo test -p vw-desktop --bin shi-ju --all-features codex_plugin_compat_ -- --nocapture` | 退出0，18 passed、0 failed、3 ignored；ignored 不计通过，无本次手工 UI 重验声明 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 退出0 |
| `python3 -m unittest discover -s scripts/plugin_compat -p test_check_evidence.py` | 退出0，11 tests |
| JSON、来源散列、链接及差异检查 | 通过，确切命令和源文件见 [t66](evidence/t66.json) |

初始阶段槽位10，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-10`；串行与最终目标阶段槽位3，对应绝对目录 `target/rust-slot-3`。各自 `CARGO_BUILD_BUILD_DIR` 为该绝对目录下的 build；自有锁已释放、缓存保留。确切路径、退出码、日志散列和部分结果计数见 t66.json。红灯后与两次回归失败后均出现 panic 初始化失败/SIGABRT，故不把 scheduled 数量当作已执行通过数量；红灯依据是已经观察到的三项目标断言。

待回归事实：t27 取消测试在串行阶段通过，但并行失败原因未证实；不声称已修复。t32 测试仍期待解析阶段的“不支持执行”错误，实际为 `failed to resolve hook host directory`；已只读核对该断言与后续 Hook 执行入口，未修改安全断言或生产行为来获取绿灯。**TODO：独立复核 t32 测试的宿主目录前提与当前信任/执行契约，以及 t27 并行端口断言，再运行完整后端过滤集。** 当前 Mac 后端回归不能计通过，不能以 clippy 或目标三项通过替代。

其他未验收项继续保留：Windows/Linux；真实公开 OAuth 完整宿主授权/调用/退出与服务端撤销；原始记录中未通过严格逐项覆盖的联合任务及安全差异审查。没有擅自新建补充任务或改写用户豁免。

## 任务66收尾：固定快照与验证边界

2026-11-04 按后续授权完成延后回归。最终验证固定为已提交源码 `07f349fecbb0449c0d604fae580a907ab37ebdd7` 加本任务修复；不追逐此后的无关提交。运行目录为受管理的隔离检出 `/Users/xiangmin/.codex/worktrees/plugin-compat-revalidation/shiju`；源码基点、任务补丁 SHA-256、文件散列、实际命令和日志摘要见 [t66.json](evidence/t66.json) 的 revalidation。该快照通过不等于对共享工作区全部未提交源码作出通过声明。

本轮只修正仓库自编测试夹具，不改变生产权限或运行行为：

- t18/t23：端口文件存在且可解析为非零端口才视为就绪；HTTP/SSE 会话复用及版本协商用例预先放置空文件，保留真实服务启动、工具调用、关闭/请求头断言与原等待上限。
- t32：移除解析阶段遗留的“执行不受支持”前提，核对当前工厂的未信任不准入、显式审批后准入、内容变化后失效；保留包外信任状态与包字节断言。
- t33：合成 matcher 夹具把输出写到宿主 `PLUGIN_DATA/result`，保留精确 `abbc` 顺序断言并验证包目录未被写入；没有修改公开原包或放松内容信任检查。

最终当前快照结果：

| 命令 | 退出码与有效结果 |
| --- | --- |
| `cargo fmt --all` | 0；排除隔离检出中无关的纯格式差异 |
| `RUST_TEST_THREADS=1 cargo test -p vw-agent --lib --all-features codex_plugin_compat_ -- --nocapture` | 0；441 passed、0 failed、7 ignored |
| `cargo test -p vw-desktop --bin shi-ju --all-features codex_plugin_compat_ -- --nocapture` | 0；18 passed、0 failed、3 ignored |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 |

该过滤集包括任务66三项拒绝/隔离测试。顺序运行没有跳过其他测试，也没有改变单个测试内部的并发、取消、超时或隔离断言。此前并行组合运行失败仍保留，不计为通过；其根因未被充分确认。未证实有效的 t27 端口隔离尝试已撤回，最终未修改生产 OAuth 或原 t27 测试。

最终阶段领取槽位3：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，`CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3/build`。此前阶段按实际领取了其他槽位，逐条见 t66.json；自有锁均释放、缓存保留，未另建任务构建目录。忽略项名单与原因保存在最新 revalidation，7个后端和3个桌面忽略项均不计通过，没有本轮手工 UI 或真实账号验收声明。

共享工作区的 resume3 尝试因无关 `workflow/video_edit/assets.rs` 的摘要格式化/租户 ID 显示类型错误而编译失败；这次失败独立保留，未在本任务修改该模块，也没有用隔离通过覆盖它或宣称当前脏工作区整体通过。

Windows、Linux、真实 OAuth 继续沿用用户豁免且不计通过。严格检查器未修改，完整检查仍退出1，汇总保留298个范围内 unverified 与1个原范围外 excluded；完整兼容没有成立。编号任务66仅按获准的证据汇总与固定 macOS 快照回归范围收尾；全平台、真实账号、opt-in 原包/公开服务、手工 UI 及严格逐项覆盖中尚缺的证据继续保留。

临时验证检出的归档已尝试；应用拒绝移除本会话主检出，因此保留供复核。日志保留于原仓库，未删除共享构建缓存；该清理限制不计为兼容验收通过或失败。
