# 数据管理接入契约

2026-11-14，配置迁入导出备份与恢复子任务 1。本文是静态接入合同，不表示迁入、备份恢复或 macOS 验收已经完成。本轮按用户覆盖要求不执行 Cargo、单元测试、实机测试；只读取源码、文档和 Git 信息。母计划只读，来源映射不修改。

## 来源与依赖基线

来源 `/Users/xiangmin/worker/code/cc-switch` 的 `git rev-parse HEAD` 返回 `ca58c0313360eae7c851fbd7553c1614c7af03e7`；冻结版本 4.0.25，`src-tauri/src/database/mod.rs::SCHEMA_VERSION` 为 24。`database/schema.rs` 定义下面源表，`database/migration.rs` 是旧 JSON 到源库的桥，不是目标宿主迁移器。`database/dao/` 的领域 CRUD 和 `commands/import_export.rs` 是语义来源；`database/backup.rs` 的整库替换不得照搬到宿主。

母任务 1 的冻结副本在 `crates/vw-agent-apps/upstream/`，只作参考，不注册进运行模块。母任务 2 已有正式包、DTO、SDK、租户存储和图标竖切，见[底座交付映射](../../vibewindow/agent-apps/底座交付映射.md)；不能因此宣布母任务 1–17 全部完成。业务包 `Cargo.toml` 无 features 声明，`src/lib.rs` 已注册 providers、mcp、skills、prompts、project_profiles、usage 等领域；macOS 文件能力带平台条件。当前工作区有其它任务未提交代码，下表仅证明路径与符号可读，不证明这些实现已经编译、验收或提交。

## 七类唯一所有者与字段

以下代码路径省略公共前缀 `crates/`。来源表名及字段是输入映射，不允许客户端直接指定目标表或 namespace。

| 类别 / 依赖母任务 | 源表与关键字段 | 目标唯一所有者 / 接入限制 |
| --- | --- | --- |
| 供应商 / 4、6、8 | providers：id、app_type、name、settings_config、meta、is_current、in_failover_queue、排序、图标；provider_endpoints：provider_id、app_type、url、added_at | `vw-agent/src/app_providers.rs::AppProviderStore` 保存正式租户记录；`vw-agent-apps/src/providers/` 负责校验和原生配置。凭据依赖账号所有者，current/队列必须经领域操作，保存记录不能冒充激活。 |
| MCP / 11 | mcp_servers：id、name、server_config、description、homepage、docs、tags、enabled_* | `vw-agent/src/agent_apps/mcp/mod.rs::McpCatalogStore`；原生文件与分发为 `vw-agent-apps/src/mcp/{native_file,distribution,adapters}/` 对应模块。enabled 是来源意图，不是目标已部署证明。 |
| 技能 / 12 | skills：id、directory、repo_owner/name/branch、content_hash、installed_at、updated_at、enabled_*；skill_repos：owner、name、branch、enabled | `vw-agent/src/gateway/agent_apps_skills_store.rs::SkillReposStore/SkillInstallsStore`；`vw-agent-apps/src/skills/` 安装、inventory、部署、备份恢复。目录是来源定位信息，不能复制成目标绝对路径；必须另校验资源并经安装所有者物化。 |
| 规则 / 13 | prompts：id、app_type、name、content、description、enabled、created_at、updated_at | `vw-agent/src/agent_apps/prompts_store.rs::PromptsStore`；`vw-agent-apps/src/prompts/` 管理激活、模板及 live 文件。正文保真；创建条目与激活分开，见[持久化契约](prompts/persistence.md)。 |
| 组合 / 16 | profiles：id、name、payload、sort_order、created_at、updated_at；settings 的 current_profile_id_* | `vw-agent/src/gateway/agent_apps/profile_store.rs::ProjectProfileStore` 与 `vw-agent-apps/src/project_profiles/`。payload 四槽引用需在前四类 ID 映射完成后重写；当前指针不可直接移植，应用必须走领域编排。 |
| 偏好 / 3、15、20 | settings：key/value；只选择已说明语义的键，不整表复制 | `vw-agent-apps/src/apps/preferences.rs::normalize_preferences`；`vw-agent/src/gateway/api/handlers/agent_apps/preferences.rs` 经 GatewaySettingsStore 写 agent_apps_user（principal）、agent_apps_host（host）。工作区/系统集成键回各领域，不塞入通用偏好。 |
| 用量 / 17 | proxy_request_logs、usage_daily_rollups、session_log_sync、session_usage_dedup、model_pricing；日志、汇总、扫描位置、去重、价格分别处理 | `vw-agent-apps/src/usage/store.rs::initialize_usage_store/save_usage_record/save_usage_record_in_transaction`、UsageSourceIdentity；配置引用由 `vw-agent/src/agent_apps/usage_settings.rs::UsageSettingsStore` 管理。现有记录写入接口接 rusqlite Connection，不据此承诺 PostgreSQL 用量迁入；不可同时迁入明细和同源汇总造成重复计费。 |

源 proxy_config、provider_health、stream_check_logs、proxy_live_backup 不作为第八个通用类别直接复制。涉及路由、健康、诊断和文件回退时回各领域核对；会话内容、账号令牌、宿主运行表不因上述表存在而自动获得迁入授权。每个类别的具体字段转换及支持应用由后续类别任务交付；未知字段应记录排除原因，不静默宣称保留。

## 身份、事务与回退合同

目标租户使用 `vw-api-types/src/id` 的 TenantScope（platform/company），项目使用 ProjectScope，主机是授权策略中的执行主机 ID；来源无这些身份，必须由认证请求显式绑定。来源 app_type 转既有 AppId，不造别名。项目、用户 principal 与 host 各有作用域，不能用一个租户键抹平。主机路径来自授权根，不能拿桌面 cwd/HOME 代替远端根。

`vw-agent/src/gateway/settings_store.rs::GatewaySettingsStore` 支持 SQLite/PostgreSQL；`update_with_preimages` 在同一事务核对引用并更新一个配置记录，SQLite 使用 Immediate，PostgreSQL 走对应适配。该方法是内部入口，不是跨七领域公共事务。PromptsStore 使用 revision/CAS；ProjectProfileStore 管组合状态事务；MCP native_file 使用供应商配置事务；技能文件安装/部署由技能所有者恢复。任何一个局部事务都不能证明七类数据库记录、资源目录、Keychain、原生配置文件可原子回滚。

后续写入必须先建立与本次选定作用域闭合的回退点，记录所触及对象的前镜像、版本和文件身份；恢复前同样建立回退点。失败只恢复本次拥有且版本仍匹配的数据，外部修改需明确冲突，不能覆盖其他租户、项目或用户后续写入。提交完成后投影/缓存失败单独报告并提供重试，不声称数据未写入。**当前尚无本计划的跨类别回退实现**；消费该能力的提交/恢复任务须等待实际交付，不能用 CC 整库覆盖绕过。

完整本机回退与可迁移导出必须分型。前者可包含该操作恢复所需敏感值，0700 目录/0600 文件、受控保留且不默认远程下载；后者默认排除凭据、主机绝对路径、系统集成及其它租户数据。恢复只接受显式范围、版本兼容且资源校验完整的归档。备份调度不接管日志清理、用量汇总或 vacuum。

## API、SDK、Iced 与任务协议

现有 API 外层 `/v1`：供应商 `/app-providers`，MCP `/agent-apps/mcp`，偏好 `/agent-apps/preferences`，组合 `/agent-apps/project-profiles`。handler 注册在 `vw-agent/src/gateway/api/handlers/mod.rs`，路由通过 `gateway/router.rs` 与 `gateway/bootstrap.rs` 挂载；SDK 从 `vw-gateway-client/src/client/mod.rs` 注册，DTO 从 `vw-api-types/src/lib.rs` 注册。Iced 由 `vw-desktop/src/app/` 调 SDK，不直接持有数据库或运行迁入调度。

数据管理模块、DTO、SDK 和对应 API **尚未由本任务实现**。后续固定接入到上述父模块；拟用 `/v1/agent-apps/data-management`，仅作为本计划保留前缀，不称为已注册接口。具体任务交付时定义显式识别、预览、执行、查询、取消、导出、创建/列举/重命名/删除/恢复备份操作，不允许任意 command、SQL、namespace 或文件路径后门。新增模块复用七类所有者，不另开永久 CC 库。

后续 DTO 至少承载：授权目标身份、来源/归档身份和摘要、格式版本、所选类别、冲突决策、预览版本、幂等键、任务 ID、阶段/进度、逐项结果及回退状态；响应只回安全摘要，不回凭据/原始载荷。预览不是写授权，执行时重新核验身份、版本和资源。相同幂等键不同载荷必须冲突。

错误需区分非法输入、版本不支持、权限拒绝、未找到、并发冲突、存储/主机不可用、取消及恢复失败。沿用领域真实错误与 HTTP 映射，不把 unsupported/unreachable 转空成功；稳定错误码、任务持久化和取消接口仍需后续交付。既有 SSE 不保证持久重放，重连以授权任务查询为准。

取消临界点：读取/解析/预览/准备回退阶段可在有界项间取消并清理临时数据；开始提交后不能宣称立即撤销，必须由事务决定回滚或完成再恢复；进入原生投影后消费领域恢复协议，终态区分 cancelled、failed、partial 与成功，保留提交事实。任务查询和取消必须绑定原租户/项目/主机，不能仅凭任务 ID 操作。

主机动作沿 `handlers/agent_apps/access.rs` 的 policy 与 x-agent-apps-token 授权；现有 launch、preferences 或安装授权不自动赋予迁入、备份读取/恢复或任意路径权。后续须在首次敏感读写前增加对应显式授权；远端不可达不回退桌面本机。

## 隔离准备与后续验证入口

静态确认的测试入口：`vw-agent/src/agent_apps/{sqlite_tests,postgres_tests}.rs` 已由模块注册；图标测试过滤族 `cc_base_icon_sqlite_`、`cc_base_icon_postgres_atomic_` 是底座事务证据入口，不是数据迁入测试。PostgreSQL 夹具 `global_database/test_support.rs::PostgresTestDatabase::from_environment` 使用 `VIBEWINDOW_TEST_POSTGRES_URL` 创建唯一 `vw_test_<uuid>` schema 和临时 home，退出清理自身 schema；无测试 URL 的宏可能跳过，不能把退出 0 当执行成功。SQLite 用 TempDir 和显式 GlobalDatabase 路径，匿名夹具不得打开真实用户库。用量当前 SQLite 事务不能外推为跨后端合同。

后续只在用户恢复验证授权后使用仓库 `scripts/cargo_slot.py` 包住有效包/目标/过滤条件；默认 features 足以发现上述已有入口。本轮没有调用 Cargo、构建、单元或实机测试，故无测试数量、编译成功或后端连通结论。数据管理专属测试需随对应行为新增并由父模块注册。

macOS 已确认 CLI `vw-cli/src/cli.rs` 的真实语法（以下仅展示，未执行）：

```sh
shiju --config-dir /absolute/isolated/config gateway --host 127.0.0.1 --port 19381
```

**这不是完整隔离启动命令**：`GlobalDatabase::for_home` 仍按 `vw-config-types/src/paths.rs::global_database_path(home)` 定位共享数据库及后端配置；桌面 `config_cli_bootstrap.rs` 同样以真实 home 构建配置根和服务启动，未确认支持完整数据/凭据根覆盖的生产启动参数。因此不能安全提供“隔离 Gateway + 桌面”的可执行验收流程，也不能通过改 HOME、猜测环境变量或普通调试构建绕过。`VW_MODEL_RECOVERY_TEST_ROOT` 仅在 cfg(test) 生效，不适用于生产程序。后续实机验收需已有受支持隔离入口或专用测试程序显式注入临时路径，确认数据库、凭据、配置、应用发现根都隔离后再启动；此项未就绪，不阻止本合同交付，但阻止宣称隔离实机通过。

本轮无自有服务进程，无退出清理动作。未来前台进程应记录自身 PID，退出只停止自有进程、删除本任务临时目录，不清理真实 HOME 或其他会话资源。本机文件由 rfd 选择后交给受控上传；远程以绑定身份的传输 ID 获取文件，桌面不能把本机绝对路径直接传给远端打开。传输与目录定位接口尚待对应任务交付，不能假设已存在；定位远程目录只返回远端身份与位置，不调用本机文件管理器冒充远端定位。

## 交付边界

七类现有所有者和父模块已登记；缺口为数据管理专属 API/SDK/DTO/任务实现、跨类别回退、敏感操作授权、远程传输与完整隔离启动入口。逐类别任务只等待自己的实际接口，不等待所有母任务整体完结。本文锁定消费约束，不补造前置实现，不将静态阅读等同功能验收。仅面向 macOS，未运行任何平台验收。

## 子任务 2：传输类型落点（2026-11-14）

唯一 DTO 落点为 `crates/vw-api-types/src/agent_apps/data_management.rs`，由 `agent_apps/mod.rs` 注册并公开导出；独立测试由同一父模块注册。此项补齐上文的 DTO 缺口，不表示业务包、API 路由、SDK、授权、任务调度或跨类别回退已经实现。

- `DataManagementTarget` 复用 `ProjectScope`（含 TenantScope），同时显式绑定执行主机与 principal。请求字段仅是待鉴权声明，网关必须与认证身份核对。任务查询与取消仍绑定原目标，复用 `TaskId`、`TaskStatus`，不创建第二套调度器。
- 识别使用受控上传的 `DataTransferId`；`DataSourceSummary` 承载源版本、schema、格式版本、归档格式与 SHA-256 摘要。版本和摘要须由服务端检查后产生，不由客户端声明代替校验。`DataCategory` 封闭为供应商、MCP、技能、规则、组合、偏好、用量。
- `PreviewDataImportRequest` / `DataImportPreview` / `ExecuteDataImportRequest` 分离预览与执行；选择条目使用服务端签发的安全句柄，冲突只允许 skip、keep_both、replace，且每项声明允许的决策。`revision` 必须绑定源摘要、选择、冲突和目标观察版本；提交按幂等键核对完整输入并重新鉴权。首次识别后的具体句柄枚举由读取/预览任务交付，不允许直接把原始配置值或来源路径当句柄。
- `ExportDataRequest` 只代表可迁移导出；`DataArtifactKind` 区分 portable_export 与 local_rollback；备份创建、列举、改名、删除、恢复使用独立请求类型与 `DataBackupId`。本机回退不得返回下载引用，恢复须检查原主机、范围、摘要、版本并先创建新回退点。名称是用户自定义备份标签，不能从源凭据或配置正文生成。
- `DataOperationProgress` 复用任务状态，额外提供阶段、计数、取消意图、提交事实、逐项结果和回退状态。`DataOperationResult::Partial` 表示部分失败；提交后投影失败不能抹掉 `committed`。取消请求不承诺立即撤销，终态以所有者完成或回滚的事实为准。
- 响应以计数、封闭枚举和不透明句柄表达预览/错误，无任意 JSON、原始载荷、SQL、异常消息或路径字段。拒绝未知操作与未知请求字段。DTO 无法自动净化任意字符串：来源版本、摘要和所有句柄均由所有者校验/生成，禁止把密钥放入这些字段；跨字段一致性、非空选择及版本兼容规则属于后续业务实现。

按本轮明确覆盖要求，未调用 Cargo、未编译、未运行单元或实机测试；静态阅读不能证明序列化行为测试通过。后续验证入口见 `data-management-macos.md` 子任务 2 小节。

## 子任务 3：来源只读暂存入口

`vw-agent-apps/src/data_management/source.rs::read_selected_source` 已实现用户选定副本的 SQLite/JSON 识别与只读暂存，私有文件捕获/JSON 解析分别在 `source_files.rs` / `source_json.rs`；父模块注册到业务包 `data_management`，不另建永久 CC 存储。调用方提供已授权根与 SourceScope（ProjectScope、host、principal），原始载荷通过同作用域的 `SourceSnapshot::records` 供后续领域预览校验，不调用类别写入。SQLite schema 上限 24、JSON 版本 2；摘要不包含源路径、原始字段或发行版本猜测。DTO 的传输 ID 授权及摘要转换仍由后续网关任务交付，不将业务函数误报为已注册 API。实现边界及待回归过滤条件见 [macOS 交付记录](data-management-macos.md)。本轮未运行 Cargo 或任何测试，不能据此宣称读取一致性或源不变断言已通过。

## 子任务 8：迁入前回退点（2026-11-14）

`vw-agent-apps/src/data_management/safety_snapshot.rs` 交付 macOS 回退归档核心，私有存储在 `safety_snapshot_files.rs`，父模块 `data_management/mod.rs` 注册。`create_safety_snapshot` / `open_safety_snapshot` / `with_safety_snapshot` 绑定 ProjectScope、principal、执行主机及精确 SnapshotTarget 集合；OwnerPreimage 保存所有者编码的完整字节和 revision/文件身份，None 表示原先不存在。不同类别、重复、多余或缺失对象不能作为完整回退点。归档不是可迁移导出，不暴露原始载荷给 DTO，不执行任意 SQL、namespace 或路径命令。

调用方必须先授权并在现有领域事务/锁内捕获所选对象及其依赖资源，保持锁至提交，或提交前对全部版本做 CAS。该模块只校验捕获集合，不自行读取七类存储，也不能把顺次读取变成跨数据库/Keychain/原生文件的原子快照。消费任务必须交付对应所有者捕获与恢复接线；尚未接线的类别不支持执行，不能以空集合或仅 ID 代替资源。`with_safety_snapshot` 只在持久化和完整回读成功后调用写入闭包；写入闭包失败仍返回回执与独立错误，供所有者按写后版本恢复，不能谎报零写入。回退点准备错误才阻止该闭包。

0700 根目录由执行主机既有授权目录所有者提供；模块要求其已存在、归当前用户且无符号链接，不自动选择 HOME、下载目录或远端路径。descriptor-relative openat/O_NOFOLLOW 逐级打开目录，UUID 子目录 0700、文件 0600，拒绝硬链接；文件与目录 sync_all 后重新打开，以最多 64 MiB、SHA-256、格式版本、身份与完整选择集合校验。回执应随拥有任务受控持久化，摘要不是针对本机所有者篡改的认证。保留清理、跨类别迁入/恢复、网关授权及任务回执存储仍由后续消费任务实现。本轮没有 Cargo、编译、单元或实机测试通过证据。
