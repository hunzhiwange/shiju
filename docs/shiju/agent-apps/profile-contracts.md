# 项目配置组合领域接入契约

2026-11-14，任务 1 静态盘点。仅 macOS；本文为后续实现合同，源码存在不等于通过验证。本轮按用户覆盖要求未运行 Cargo、构建、检查、单元测试、实机测试或最终回归。共享 checkout 中其他会话的未提交产物不构成稳定交付。

## 来源与范围

来源 CC Switch 4.0.25，提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`；同级 cc-switch 的 `git rev-parse HEAD` 返回该值（退出 0）。源码 `src-tauri/src/services/profile.rs::ProfileScope` 实际为 claude、claude-desktop、codex 三组；`snapshot_current/create/update/delete/apply` 是来源编排入口。`database/dao/profiles.rs` 所有 CRUD 与 current 标记属于来源数据库，不直接接入目标运行时。

| 来源 | 接入职责 / 后续任务 |
| --- | --- |
| commands/profile.rs、lib/api/profiles.ts | 19–23 的 API 和独立 SDK 方法 |
| services/profile.rs | 2、4、7–18 的三组快照、最小应用与自动保存 |
| database/dao/profiles.rs | 3、5、6 的实体和分组标记 |
| tests/profile_roundtrip.rs | 28–30 的四类恢复、悬空引用、Desktop 独立和空快照场景 |
| components/profiles/{ProfileManageDialog,ProfileSwitcher}.tsx、scope.ts、lib/query/profiles.ts | 24–27 的 Iced 状态、搜索、弹窗、取消和定向刷新 |
| config/piThinkingProfiles.ts 与对应测试 | 31 交叉核对模型领域，不复制模型配置所有者 |

上游未拍组仅选为当前并提示；切走时补拍。自动保存只覆盖旧组合当前组，不能覆盖另外两组。只清 current 标记不撤销原生配置。上游 warnings 后设置 current 不能照搬为“全部应用成功”。

## 实体、项目、主机和存储

组合实体在同租户内共享，不以应用组复制实体；组合 ID 是新领域 ID，不能使用现有 AgentProfile 或 ProjectId 冒充。组合名 trim 后非空。三个组独立 captured 状态；未拍和已拍但全空必须区分。快照只存领域引用和启用状态，不保存密钥、原始令牌、连接敏感载荷或提示词正文副本。

`crates/vw-api-types/src/id.rs::ProjectScope` 是 TenantScope + ProjectId。`gateway/api/handlers/project.rs::project_scope/load_project/project_context_directory` 为现有项目解析入口，项目目录取已加载项目的 worktree/sandboxes，不信任请求提供的任意目录。组合实体不是文件系统项目；应用时必须另带真实 ProjectScope。

`gateway/api/handlers/desktop.rs::project_preferences_put_in_store` 复用 GatewaySettingsStore、TenantScope、project_path；现有 DTO 仅 model、auto_model、acp_agent。它不包含 host，也不证明路径已授权，不能据此创建第二项目设置表或把请求路径当执行端路径。若后续需要项目组合偏好，先由该所有者扩展并保留原字段。

组合实体由 agent 存储适配写入现有 GlobalDatabase/GatewaySettingsStore，不另开数据库。标记和操作结果按租户、授权目标主机、应用组隔离；应用动作同时绑定 ProjectScope，并拒绝不同项目占有同一物理槽位。实体的跨主机引用必须在目标主机重新解析，缺失显式报告，不能回退本机。实体删除与全部引用标记清理需同一存储事务或明确可恢复方案；任务 3/5 实现前核对底层原子能力，不能把逐条 upsert 当原子事务。

## 当前生产入口与缺项

以下路径相对 crates。表中仅表示静态存在，各槽位尚需本计划适配。

| 母任务 | 当前产物 / 可用边界 | 消费前条件 |
| --- | --- | --- |
| 1 | vw-agent-apps/upstream 保留来源树；同级来源 HEAD 已核对 | 保留冻结来源映射，不能将 upstream 模块当原生库 |
| 2 | vw-agent-apps/src/lib.rs 已注册原生模块；agent 有 GlobalDatabase、GatewaySettingsStore | 旧来源映射“无 vw-agent-apps”已过时；组合编排和存储尚不存在 |
| 3 | vw-api-types/src/agent_apps/catalog.rs::AppId；vw-agent-apps/src/apps/catalog.rs | 复用既有三个 AppId，不引入别名 |
| 4 | vw-agent/src/app_providers.rs::AppProviderStore；vw-agent-apps/src/providers；live/write_guard.rs::lock_app/lock_directory，live/commit.rs::ProviderTransactions | 任务 11 接各应用真实 state/切换和路由模式入口，不能仅更改供应商数据库 current；代理模式只切路由 |
| 5 | vw-agent-apps/src/models/preset_pi.rs::apply_thinking_profiles | 模型领域负责 Pi thinking，组合不重复实现 |
| 6 | vw-agent/src/app_providers/*_binding.rs 及 agent_apps/auth 账号所有者 | 快照仅引用供应商；不复制 OAuth grant，授权与原生独占槽仍由账号领域判断 |
| 7–8 | vw-agent-apps/src/routing、vw-agent/src/gateway/api/handlers/app_routing | 常驻路由生命周期由原领域管理；本任务未验收协议完整性 |
| 9 | vw-agent-apps/src/high_availability、diagnostics | 不纳入四类快照；不得因组合应用隐式启动探测或故障转移 |
| 10 | vw-agent-apps/src/aggregation、providers/claude_desktop.rs | Desktop 保持独立；不扩张 MCP/技能支持矩阵 |
| 11 | vw-agent-apps/src/mcp/import.rs、distribution.rs::NativeMcpWrite::{validate_target,apply}；vw-agent/src/agent_apps/mcp/binding.rs::set_app_binding | 使用目录所有者解析引用，再走绑定服务；NativeMcpWrite 仅原生投影，不能绕过目录授权。当前 native_capability 明确 Desktop 不支持，逐项返回不可用 |
| 12 | vw-agent/src/gateway/agent_apps_skills_store.rs 的 inventory、enable_with_method、disable_deploy；vw-agent-apps/src/skills/bulk_sync.rs::SkillInstallTarget | 安装身份包含 installation_id + app_id；不能把目录名当全局 ID。仅恢复已安装引用，不隐式下载。Desktop 不支持时显式跳过/失败 |
| 13 | vw-agent/src/agent_apps/prompt_activation_store.rs::PromptsStore::activate_ordinary；prompt_deactivation_store.rs::reconcile_ordinary；vw-agent-apps/src/prompts/activation.rs::publish | PromptScope 含项目、主机、应用；revision 和原生内容前像核对不可省略。无激活引用调用已有停用语义，不能随意删除用户文件 |
| 14 | vw-agent-apps/src/sessions | 会话浏览和历史迁移不属于快照写入；不引用其 profile 名称作为组合身份 |
| 15 | 项目解析和 project-preferences 见上一节；workspace_paths/workspace_files 已有独立模块 | 工作区目录、主机授权及设置字段不得由 Iced 自行推断；不依赖母任务整体完成 |

缺少的是本计划要交付的组合 DTO、存储、四类编排适配、操作锁及 API/SDK/UI，并非需要重建上述底座。任务 7 在能读取对应领域真实状态后拍摄；任务 11–14 分别落实原生调用和权限合同。现有模块不承诺统一的 snapshot/apply 四类函数，不虚构函数名。跨主机远程协议、Desktop 不支持能力、未知物理槽归属都必须返回明确错误。

## API、SDK 与执行安全合同（待实现）

组合 API 拟采用 `/v1/project-profiles`：GET 列表、POST 创建；`/{id}` PATCH 改名/重拍、DELETE 删除；分组标记清理、preview、apply 及 operations 查询/取消由任务 19–21 明确子路由。当前没有已交付组合路由，不得将此表当可调用接口。

租户取 Gateway 鉴权上下文，不能采纳 body 的租户值；目标主机通过 `gateway/api/handlers/agent_apps/access.rs::authorize_provider_config/authorize_provider_editor` 的现有策略和主机分派模式解析。该 wrapper 不接 project，任务 19–21 必须额外校验 ProjectScope、项目目录和物理槽归属，并逐项执行原领域读写权限检查；组合维护权不能扩大原领域权限。离线/remote 不支持保留错误。

任务 2 的共享 DTO 至少表达组合 ID/名称、三组可选快照、领域引用与启用位、selected 和应用结果。请求明确 project_id、target_host_id、group；修改携带版本前提，预览与执行重新核对版本。响应逐项返回成功、缺失、冲突、拒绝、失败、取消或不支持，不含秘密。selected 只表达选择；applied 仅四类所需步骤全部确认后成立；partial、uncaptured、cancelled 均不能显示为 applied，取消保留已提交项。

SDK 由 GatewayClient 提供维护、预览、应用、结果查询和取消的独立公开方法，参数/返回值复用 DTO；无需构造 Iced 状态即可调用。传输、HTTP、解码与领域失败保持可诊断。Iced 仅通过 SDK，使用统一设置弹窗和 toast，按目标组刷新，不直接写配置。

任务 17 的操作锁必须复用领域锁秩：外层组合操作串行化后逐项调用领域入口，不跨 await 持有同步锁，不预持 AppWriteGuard 再调用会取得同一锁的存储入口。MCP binding.rs 明确先持 catalog record lock，再由适配取得物理应用/目录锁；提示词适配内部取得 lock_app 再核对目录版本。各领域锁顺序不能倒置；逐项释放后进入下一领域，不同时攥住四类物理锁。底层没有已确认的统一“组合锁秩 API”，任务 17 需核对真实调用链后落地，不能新增绕过底层锁的快捷路径。部分失败不整组回滚；取消只在安全提交边界生效。

## 后续文件和注册入口

以下均为拟新增落点；同职责已有实现优先复用。B = `crates/vw-agent-apps/src/project_profiles/`；G = `crates/vw-agent/src/gateway/agent_apps/`；H = `crates/vw-agent/src/gateway/api/handlers/`；S = `crates/vw-gateway-client/src/client/`；U = `crates/vw-desktop/src/app/`。

- B/mod.rs 尚不存在，需在 vw-agent-apps/src/lib.rs 声明；子模块及独立 *_tests.rs 在 B/mod.rs 注册。
- G 目录尚不存在，需新增 G/mod.rs 并在 gateway/mod.rs 声明 agent_apps；不要与现有 `vw-agent/src/agent_apps/mod.rs` 所有的 MCP/提示词存储混淆。若实施时选择已有 agent_apps 根，应整体调整落点，仅维护一个所有者。
- DTO 的 project_profiles.rs 在 vw-api-types/src/lib.rs 注册，独立 project_profiles_tests.rs 用 cfg(test) 注册。
- H/project_profiles.rs 在 handlers/mod.rs 声明，router.rs 和 bootstrap.rs 的生产路由构造均按既有模式 merge；不能只在测试 router 接线。
- S/project_profile_api.rs 在 client/mod.rs 声明，独立测试同根 cfg(test) 注册。
- U/components/project_profiles/mod.rs 接 components/mod.rs；message/project_profiles.rs 接 message/mod.rs，消息分派接 app 根 update；state/project_profiles.rs 接 state.rs 和初始化。设置视图接真实 system_settings 入口；测试在 components/project_profiles/mod.rs 注册。
- H/profile_macos_tests.rs 在 handlers/mod.rs 使用 cfg(all(test, target_os = "macos")) 注册，28–30 共用时逐项追加，不能重复声明模块。

| 任务 | 拟文件（独立测试与业务同目录） |
| --- | --- |
| 2 | `crates/vw-api-types/src/project_profiles.rs`、`crates/vw-api-types/src/lib.rs`、`crates/vw-api-types/src/project_profiles_tests.rs` |
| 3 | `crates/vw-agent/src/gateway/agent_apps/profile_store.rs`、`crates/vw-agent/src/gateway/agent_apps/profile_store_tests.rs` |
| 4 | `crates/vw-agent-apps/src/project_profiles/rename.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/rename_tests.rs` |
| 5 | `crates/vw-agent/src/gateway/agent_apps/profile_store.rs`、`crates/vw-agent/src/gateway/agent_apps/profile_store_tests.rs` |
| 6 | `crates/vw-agent/src/gateway/agent_apps/profile_store.rs`、`crates/vw-agent/src/gateway/agent_apps/profile_store_tests.rs` |
| 7 | `crates/vw-agent-apps/src/project_profiles/snapshot.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/snapshot_tests.rs` |
| 8 | `crates/vw-agent-apps/src/project_profiles/create.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/create_tests.rs` |
| 9 | `crates/vw-agent-apps/src/project_profiles/resnapshot.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/resnapshot_tests.rs` |
| 10 | `crates/vw-agent-apps/src/project_profiles/preview.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/preview_tests.rs` |
| 11 | `crates/vw-agent-apps/src/project_profiles/apply_provider.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/apply_provider_tests.rs` |
| 12 | `crates/vw-agent-apps/src/project_profiles/apply_mcp.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/apply_mcp_tests.rs` |
| 13 | `crates/vw-agent-apps/src/project_profiles/apply_skills.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/apply_skills_tests.rs` |
| 14 | `crates/vw-agent-apps/src/project_profiles/apply_prompt.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/apply_prompt_tests.rs` |
| 15 | `crates/vw-agent-apps/src/project_profiles/autosave.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/autosave_tests.rs` |
| 16 | `crates/vw-agent-apps/src/project_profiles/apply.rs`、`crates/vw-agent-apps/src/project_profiles/mod.rs`、`crates/vw-agent-apps/src/project_profiles/apply_tests.rs` |
| 17 | `crates/vw-agent/src/gateway/agent_apps/profile_operations.rs`、`crates/vw-agent/src/gateway/agent_apps/profile_operations_tests.rs` |
| 18 | `crates/vw-agent/src/gateway/agent_apps/profile_operations.rs`、`crates/vw-agent/src/gateway/agent_apps/profile_operations_tests.rs` |
| 19 | `crates/vw-agent/src/gateway/api/handlers/project_profiles.rs`、`crates/vw-agent/src/gateway/api/handlers/project_profiles_tests.rs` |
| 20 | `crates/vw-agent/src/gateway/api/handlers/project_profiles.rs`、`crates/vw-agent/src/gateway/api/handlers/project_profiles_tests.rs` |
| 21 | `crates/vw-agent/src/gateway/api/handlers/project_profiles.rs`、`crates/vw-agent/src/gateway/api/handlers/project_profiles_tests.rs` |
| 22 | `crates/vw-gateway-client/src/client/project_profile_api.rs`、`crates/vw-gateway-client/src/client/mod.rs`、`crates/vw-gateway-client/src/client/project_profile_api_tests.rs` |
| 23 | `crates/vw-gateway-client/src/client/project_profile_api.rs`、`crates/vw-gateway-client/src/client/mod.rs`、`crates/vw-gateway-client/src/client/project_profile_api_tests.rs` |
| 24 | `crates/vw-desktop/src/app/components/project_profiles/selection.rs`、`crates/vw-desktop/src/app/message/project_profiles.rs`、`crates/vw-desktop/src/app/state/project_profiles.rs`、`crates/vw-desktop/src/app/components/project_profiles/selection_tests.rs` |
| 25 | `crates/vw-desktop/src/app/components/project_profiles/management.rs`、`crates/vw-desktop/src/app/message/project_profiles.rs`、`crates/vw-desktop/src/app/state/project_profiles.rs`、`crates/vw-desktop/src/app/components/project_profiles/management_tests.rs` |
| 26 | `crates/vw-desktop/src/app/components/project_profiles/apply.rs`、`crates/vw-desktop/src/app/message/project_profiles.rs`、`crates/vw-desktop/src/app/state/project_profiles.rs`、`crates/vw-desktop/src/app/components/project_profiles/apply_tests.rs` |
| 27 | `crates/vw-desktop/src/app/components/project_profiles/progress.rs`、`crates/vw-desktop/src/app/message/project_profiles.rs`、`crates/vw-desktop/src/app/state/project_profiles.rs`、`crates/vw-desktop/src/app/components/project_profiles/progress_tests.rs` |
| 28 | `crates/vw-agent/src/gateway/api/handlers/profile_macos_tests.rs` |
| 29 | `crates/vw-agent/src/gateway/api/handlers/profile_macos_tests.rs` |
| 30 | `crates/vw-agent/src/gateway/api/handlers/profile_macos_tests.rs` |

## 下一步及证据限制

任务 2 先落实 DTO，任务 3 沿存储合同实现实体/标记；四个槽位按表中的实际领域入口独立接入。缺少原生能力时只阻塞相关槽位，不阻塞独立类型和存储。所有拟模块注册都须在实施时核对最新共享 checkout。本任务未新增运行代码，也未确认任何应用闭环成功。后续恢复验证时执行任务 2–30 各卡的聚焦验证，再由 28–31 收集 macOS 和来源覆盖证据。

## 任务 3：共享实体和分组指针存储合同

`gateway/agent_apps/profile_store.rs::ProjectProfileStore` 复用 GatewaySettingsStore，在 `project_profiles/state_v1` 的租户记录内保存共享实体和主机/组指针；单次修改通过 `update_with_preimages` 的底层事务完成，不开第二数据库。实体按 ID 字典序返回；新建版本为 1，更新要求预期版本并递增，名称 trim 后不能为空。同 ID 可存在于不同租户，读取与修改均携带 TenantScope。

select/current 接收既有 ProjectScope，记录 ProjectId 而不复制项目设置；相同主机/组由其它项目占用时报冲突。构造器的执行主机 ID 必须来自执行端身份；目标主机不匹配时拒绝，调用方必须先走授权和主机分派并解析真实项目身份。selected 仅代表指针，不代表配置已应用。删除实体、清理标记由后续任务 5/6 在同一状态事务内扩展。

本轮仅实现代码与契约，按用户要求未运行构建、检查、单元测试、实机测试或最终回归；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置。重启读回、租户/主机/组隔离、版本并发和项目槽位冲突仍待后续定向验证，不作通过声明。

## 任务 5：删除实体与引用标记合同

`ProjectProfileStore::delete(tenant, id, expected_version)` 在既有 `mutate` 事务内核对实体存在及版本，再删除实体并移除当前租户内所有指向该 ID 的选择记录，覆盖三组、所有项目及主机。其他实体及其标记、其他租户的同名 ID 不变；缺失实体、版本冲突或存储错误返回失败。实体和标记作为同一状态记录提交，沿用 SQLite Immediate / PostgreSQL 行锁事务，不新增逐条写入路径。共享实体删除需调用方授权；清理跨主机元数据不触发远端配置变更，也不撤销已经应用的配置。后续 API 任务需接此入口并传入版本前提。

本轮仅代码与必要契约静态审阅，未编写/执行测试，未运行 Cargo、构建、检查、实机测试或回归；未领取槽位，CARGO_TARGET_DIR 未设置，不作验证通过声明。待后续验证三组匹配标记清理、其他实体/租户/主机隔离、版本冲突与写入失败原子性。

## 任务 7：指定组快照读取合同

`snapshot_current` 已落在 `vw-agent-apps/src/project_profiles/snapshot.rs`，通过四个惰性读取闭包接收同一 `SnapshotScope`（ProjectScope、授权目标主机、指定组）。供应商消费领域 `ProviderModeStatus.in_use_provider`，不使用 direct_provider 回退；没有有效供应商返回错误。MCP/技能只传启用引用集合，提示词只传激活 ID；成功读取无提示词用 None 表达，输出为空串。返回值仅填指定组，其余组保持 null；任一读取错误都不返回可保存的部分载荷。集合排序去重，不复制密钥、正文或原生配置。

执行端接线要求：由现有授权主机分派组装四个闭包，复用 mode_state 观察、MCP 原生绑定、技能 inventory 和提示词激活所有者。闭包必须核对原生状态和记录一致性，不得仅把目录的期望状态当实际部署状态；不支持能力、读取失败、未托管引用、远端不可达均返回明确错误。该函数本身不授予权限、不解析路径、不持久化，也不声称四领域读取原子一致。当前尚无组合 API 调用者及四类统一生产读取适配，后续任务接线时必须补齐，不能将本编排入口视作已完成平台闭环。

按本轮用户要求未编写/运行测试，未执行 Cargo、构建、检查、实机或回归；未领取槽位，CARGO_TARGET_DIR 未设置。实现未经编译和行为验证，后续先补至少四个 profile_t07_ 场景（实际路由、指定组与空集合、各类失败、权限/主机隔离）再做平台验收。

## 任务 7：指定组快照读取合同

`snapshot_current` 已落在 `vw-agent-apps/src/project_profiles/snapshot.rs`，通过四个惰性读取闭包接收同一 `SnapshotScope`（ProjectScope、授权目标主机、指定组）。供应商消费领域 `ProviderModeStatus.in_use_provider`，不使用 direct_provider 回退；没有有效供应商返回错误。MCP/技能只传启用引用集合，提示词只传激活 ID；成功读取无提示词用 None 表达，输出为空串。返回值仅填指定组，其余组保持 null；任一读取错误都不返回可保存的部分载荷。集合排序去重，不复制密钥、正文或原生配置。

执行端接线要求：由现有授权主机分派组装四个闭包，复用 mode_state 观察、MCP 原生绑定、技能 inventory 和提示词激活所有者。闭包必须核对原生状态和记录一致性，不得仅把目录的期望状态当实际部署状态；不支持能力、读取失败、未托管引用、远端不可达均返回明确错误。该函数本身不授予权限、不解析路径、不持久化，也不声称四领域读取原子一致。当前尚无组合 API 调用者及四类统一生产读取适配，后续任务接线时必须补齐，不能将本编排入口视作已完成平台闭环。

按本轮用户要求未编写/运行测试，未执行 Cargo、构建、检查、实机或回归；未领取槽位，CARGO_TARGET_DIR 未设置。实现未经编译和行为验证，后续先补至少四个 profile_t07_ 场景（实际路由、指定组与空集合、各类失败、权限/主机隔离）再做平台验收。

## 任务 8：从当前快照创建组合

`project_profiles/create.rs::create_from_current` 先 trim 并拒绝空名称，再调用任务 7 的四类快照入口，成功后生成独立 UUID 并执行一次插入。`ProjectProfileStore::create_from_current` 核对执行主机，通过既有 `save(..., None)` 事务写入同租户实体；不调用 select、apply，不改当前指针。快照失败不调用存储，存储失败不返回实体。其余组保留未拍状态。调用者仍须先授权项目、主机及四类读取；生产读取适配与 API 接线尚待后续任务。

本轮按用户覆盖要求仅实现代码及契约，未编写或运行测试，未执行 Cargo、构建、检查、实机测试和最终回归；无 Cargo 槽位，CARGO_TARGET_DIR 未设置。静态核对了任务 3 存储、任务 7 快照及来源 create 调用顺序，未经编译和行为验证。后续补充并执行 profile_t08_ 场景：创建不激活、空名称不读取/写入、快照失败无实体、事务失败不改变指针。

## 任务 4：共享组合重命名合同

`ProjectProfileStore::rename(tenant, id, name, expected_version)` 在既有租户状态事务内读取实体并调用 `project_profiles::rename::rename`；不接收客户端快照或时间。名称 trim 后非空，缺失实体、版本不符或版本耗尽明确报错，失败不保存。成功仅修改 name、updatedAt（执行端 UTC Unix 秒）及递增的并发版本；payload 和全部主机/组/project 指针保持原值。同名请求也按一次元数据写入处理。DTO 新增可空 updatedAt，缺省反序列化为 None；不增加第二存储所有者。后续 API 必须先执行租户鉴权及执行主机授权/分派再调用该入口。

本轮按用户覆盖要求仅实现代码与必要合同，未编写或运行测试，未运行 Cargo、构建、检查、实机测试或最终回归；未领取槽位，CARGO_TARGET_DIR 未设置。实现未验证；后续需补 profile_t04_ 的快照/三组标记保留、空白拒绝、租户隔离、版本冲突/溢出及重启读回场景，再执行任务原定聚焦测试。

## 任务 10：目标组最小只读预览

`project_profiles/preview.rs::preview` 接收目标 payload、指定组和执行端授权读取的 `PreviewState`（四类现存引用及实际启用位）。按供应商、MCP、技能、提示词以及各类别 ID 字典序输出；null 跳过，空 MCP/技能集合停用已启用项，空提示词停用当前激活项。集合去重，重排不改变结果；缺失引用逐项返回 Missing，不产生执行动作。供应商/提示词使用领域 Select 操作，不拆成先停用再启用；悬空单选保留现有激活项。

该函数仅借用输入，无配置、快照或 current 标记写入入口。PreviewState 必须来自同一授权项目、执行主机与应用组的实际领域观察，读取失败和不支持不能用空集合代替；技能键为 installationId。当前未接 API/四类读取适配，后续任务须完成授权分派与观察接线，应用前重新校验版本、权限和状态，不能把预览当执行凭据。

本轮按用户覆盖要求仅写代码及必需契约，未编写或运行测试，未运行 Cargo、构建、检查、实机测试或最终回归；未领取槽位，CARGO_TARGET_DIR 未设置。实现未验证；后续补 profile_t10_ 场景，验证零写入、null/空集区别、逐项悬空、重复与重排最小差异、三组隔离及提示词停用。

## 任务 9：指定组重拍与事务合并合同

`ProjectProfileStore::resnapshot` 必须接收 `SnapshotScope`，没有缺省组；先核对执行主机、租户内实体和版本并通过原有严格 decode 拒绝损坏存储，再调用任务 7 的四类读取。读取失败不写入。读取期间不持有设置事务锁；成功后在既有 mutate 事务重新检查实体和版本，调用 `project_profiles/resnapshot.rs::merge_snapshot`，仅替换指定组的供应商、MCP、技能、提示词四槽，保留 ID、名称、其它组及全部选择标记，递增版本并写执行端时间。版本耗尽、不完整快照、无效引用直接失败，不回退空快照。空集合与无激活提示词仍是有效观察。生产 API 与领域读取适配由后续任务接线，调用方必须先授权项目、主机及四类读取，不把组合维护权当领域权限。

本轮按用户覆盖要求仅编写代码和必要合同；未运行 Cargo、构建、检查、单元测试、实机或最终回归，未领取槽位，CARGO_TARGET_DIR 未设置，不作通过声明。后续需验证指定组四槽覆盖及其它组/名称/标记保留、缺 scope 请求拒绝、损坏 payload 拒绝、读取失败不写入，以及并发版本冲突和失败原子性。

## 任务 15：切换前自动保存合同

`ProjectProfileStore::prepare_switch` 在执行主机校验后先读取/解析目标实体，再查询授权项目当前组指针；目标缺失、载荷无法解析、目标引用非法或槽位被其他项目占用均返回错误，不执行自动保存。未拍组和 null 类别仍允许交给应用入口按原合同处理。首次选择和同组合选择不读取原生配置、不覆盖快照。切到不同组合时，调用任务 7 的快照及任务 9 的版本检查重拍事务，只更新旧组合当前组；其余组和指针不变。旧组合读取、快照或保存失败返回 `PreviousProfileSaveFailed` warning，保留已解析目标供继续应用，不暴露底层错误或密钥。

后续任务 16 必须消费 `PreparedProfileSwitch.target` 和 warning，不能绕过准备入口；任务 17/18 必须串行化同一租户/主机/项目/组的整个 prepare/apply/select 生命周期，并在应用前执行既有权限和版本校验。本入口不应用配置、不移动指针；四领域实际读取适配及生产应用调用者尚未交付，不能声称平台闭环。

按用户覆盖要求仅实现代码与必需合同，未编写/执行测试，未运行 Cargo、构建、检查、实机或最终回归；无 Cargo 槽位，CARGO_TARGET_DIR 未设置。实现未经编译和行为验证。后续需补 profile_t15_ 场景：跨组合保存当前组、同组合不覆盖、无当前组合、目标缺失/非法载荷零写入、快照/事务失败 warning 继续、其他组保留和并发版本冲突。

## 任务 13：技能槽位应用合同

`project_profiles/apply_skills.rs::apply_skills` 对指定组执行惰性读取与逐项分发：null 不读不写，空集合停用当前启用项；仅启用位不同才调用分发，缺失引用逐项返回 Missing，单项失败继续，结果按引用排序。已启用项不重物化。读取失败整体返回错误，不伪造空集合；结果不更新快照或 selected/applied 指针，也不下载或安装技能。

macOS 执行端适配 `gateway/agent_apps/profile_skills.rs::apply_resolved_skills` 接母任务 12 的 `SkillInstallsStore::enable_with_method/disable_deploy`，保留其事务、版本检查和失败恢复；每项重新读取当前版本，沿用保存的同步模式，只传指定应用实例。profile 的 installationId 是不透明技能引用，底层 distribution 的 installation_id 则是应用实例身份，两者不可混用。执行端必须将技能引用解析成唯一目录、拒绝重复目录别名，并从该应用实例的实际部署观察 enabled；不能直接把客户端目录或期望绑定状态传入。未完成此解析、项目/主机/逐项领域授权及外层串行化前，不得调用适配器。Claude Desktop 沿底层能力明确返回 Unsupported。

当前提供编排与分发适配，生产 API、原生观察/引用解析接线由后续应用任务完成；未声称平台闭环。按用户本轮覆盖要求仅写代码与必需契约，未编写或运行测试，未执行 Cargo、构建、检查、实机或最终回归；未领取槽位，CARGO_TARGET_DIR 未设置。后续补充并执行 profile_t13_ 场景：组隔离、重复应用不重物化、null/空集合区别、缺失引用、单项失败后继续、权限拒绝及 Desktop 不支持。
