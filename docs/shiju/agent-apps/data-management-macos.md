# 数据管理 macOS 交付与验证记录

## 子任务 2：迁入与备份传输类型（2026-11-14）

已实现 `vw-api-types/src/agent_apps/data_management.rs` 并由真实父模块导出，独立测试位于 `agent_apps/data_management_tests.rs`。依赖为子任务 1 的 `data-management-contracts.md` 静态合同；没有引入新的包、领域存储、业务执行或 API 路由。

静态交付：七类选择、源版本/schema/格式/摘要、预览修订、逐项冲突决策、幂等键、可迁移导出与本机回退分型、备份 ID 及维护请求、进度/取消/部分失败/回退和封闭错误码。复用 ProjectScope、TaskId、TaskStatus。响应不承载原始凭据或配置；字符串仍由领域所有者确保安全，不能将 DTO 当运行时脱敏器。

已编写但**未运行**：

- `cc_data_02_dto_serializes_redacted_preview`：匿名结构化摘要往返、敏感载荷字段注入拒绝。
- `cc_data_02_dto_rejects_unknown_operation`：封闭操作往返、未知操作/类别/冲突策略/结果及命令字段拒绝。

用户本轮明确要求仅实现，统一延后验证。未执行任何 Cargo、构建、单元或实机测试；无退出码、有效测试数、红灯或通过证据。未领取 Cargo 槽位，CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR 未设置，无自有测试进程或锁需要释放。未读取或修改真实用户数据。任务卡按本轮要求在实现收尾时删除，不代表验证通过。

待恢复验证授权后，在仓库槽位入口内执行定向命令（本轮未执行）：

```sh
python3 scripts/cargo_slot.py cargo test -j 1 -p vw-api-types --lib cc_data_02_ -- --nocapture
```

预期发现上述 2 个测试，必须记录实际有效测试数、退出码和槽位路径后才能声称通过。无需实机即可验证此 DTO 契约；本文件不提供其他功能或平台的验收结论。

## 子任务 3：只读识别 CC 数据源（2026-11-14）

本轮仅完成代码，按用户明确覆盖要求未执行 Cargo、构建、格式化工具、单元测试或实机测试；以下为实现说明与待回归入口，不是验证通过证据。未领取 Cargo 槽位，未设置 CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR，未启动自有服务或构建进程。编号卡按本轮要求删除，plan.md 保持只读。

- 前置：任务 1 的 `data-management-contracts.md`、正式 `vw-agent-apps` 包、ProjectScope；来源冻结 schema 24、旧 MultiAppConfig JSON v2。业务入口为 `data_management::source::read_selected_source`，由 `src/lib.rs` / `data_management/mod.rs` 注册；文件读取及 JSON 解析拆入同目录私有模块，不复用或注册 upstream 数据库迁移器。
- 调用合同：调用方先验证用户选择/传输资源与敏感读取权限，再提供授权目录、绝对所选路径及 SourceScope（租户/项目、执行主机、principal）。不扫描 HOME、不接 HTTP 路径参数、不回退本机、不调用领域写入。scope 不匹配时无法读取摘要或暂存载荷；SourceSnapshot / SourceRecord 不实现 Debug / Serialize，摘要只含格式、版本、主文件/WAL 哈希及类别计数。
- SQLite：仅以文件只读方式捕获主文件和同名 `-wal`，复核哈希、inode、mtime、ctime 与长度后在 0700 临时目录创建 0600 副本；不复制 SHM。SQLite 只打开私有副本，以 query_only、trusted_schema=OFF 和一个读事务读取，自动包含已提交 WAL，源端不建立 SQLite 连接、不迁移、不 checkpoint。读取期间检测变化时报错，需调用方重新选择/重试。源端访问时间不作为不变承诺；待测断言为内容和修改时间不变。
- 识别：主文件 SQLite magic、user_version 不大于 24、providers/settings 表及关键列签名；保留旧 schema 原字段，不承诺下游已支持其转换。JSON v2（缺省版本沿用来源默认 2）保留扁平应用管理器及 mcp/prompts/skills/片段原结构与 JSON pointer。未知格式、结构损坏、未来版本、越界路径、链接、缺失文件与超限显式错误。SQLite 仅读取契约七类及关联表 allowlist；路由/健康/回退/会话/宿主表不读取。JSON 无法分类的根字段拒绝，不静默丢弃。不能从库中可靠识别应用发行版本，因此不编造发行号。
- 边界：每个源文件至多 256 MiB，SQLite 暂存字段预算 256 MiB，最多 200,000 个暂存记录；超过则明确 TooLarge。原始 SQLite 文本（包括内嵌 JSON）保持原样，JSON 按来源 section 暂存。后续预览仍须经过对应领域类型校验和过滤；本项没有创建目标数据或激活任何能力。跨后端写入与其他平台均无结论。
- 独立测试代码：`source_tests.rs` 注册 `cc_data_03_` 七项，覆盖 WAL 提交行、无 SHM 副本、主文件/WAL/SHM 内容与 mtime、过新 schema、JSON 原样暂存与四种身份越界、缺失/未知/过新 JSON、链接/路径越界、变化/大小上限。所有断言均未运行，没有有效测试数、退出码或前后差异实测证据。

待统一回归（仅记录，未执行）：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_03_ -- --nocapture`。需获得实际 ran > 0、失败数、退出码以及匿名 fixture 前后证据，才能宣布行为验收通过；届时记录实际槽位和两个绝对缓存路径。

## 子任务 8：建立迁入前回退点（2026-11-14）

本轮按用户覆盖仅实现代码并交付说明，未执行任何 Cargo、构建、单元或实机测试。未领取槽位，CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR 未设置；没有有效测试数、退出码或实测前后差异证据。仅对三个新增 Rust 文件运行独立 rustfmt 排版，不代表编译或行为验证。编号卡按本轮要求删除，plan.md 未修改。

- 入口：`data_management::safety_snapshot::{create_safety_snapshot, open_safety_snapshot, with_safety_snapshot}`；独立文件存储模块采用 macOS descriptor-relative 私有目录和文件读写。只接收所选对象的所有者完整前镜像，校验版本、作用域、目标集合及回读摘要后才允许写入回调。失败不调用该回调；回调错误保留回执。所有者事务/CAS、类别捕获/恢复和任务持久化接线是消费方合同，不宣称七类已经全部可迁入或回滚。
- 来源静态阅读：CC Switch `database/backup.rs::backup_database_file_from_conn` 在数据库和备份锁内生成副本，`validate_sqlite_integrity` 以 quick_check 校验；本实现保留写前生成/回读/失败拒绝思想，不复制整库或跨租户运行状态。
- 独立测试源码 `safety_snapshot_tests.rs` 由真实父模块注册，过滤族 `cc_data_08_`，共五项：真实匿名 SQLite 用量偏好保存、捕获、修改与通过现有所有者 CAS 恢复，检查无关租户保持不变及权限；捕获/存储/不完整集合失败阻止回调；归档损坏/跨作用域/链接拒绝；写入错误后仍能读取回退点；指定回读校验失败边界清理未交付归档。SQLite 测试仅在内存匿名库运行，临时目录显式 0700，不读真实用户数据。这些断言尚未运行。

待统一回归（仅记录，未执行）：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_08_ -- --nocapture`。应发现五项测试并记录实际运行数、退出码、槽位和绝对缓存路径；另需消费任务验证真实类别前镜像覆盖完整、一致性锁/CAS及失败恢复。macOS 以外无验收结论。

## 子任务 22：创建一致性备份（2026-11-14）

已实现 `backup_create.rs` 的 `create_local_backup` / `open_local_backup`，由真实 `data_management/mod.rs` 以 macOS 条件注册；私有原子文件发布位于 `backup_create_files.rs`。先前记录的快照接口缺项已由同批任务 8 的真实接口解除，本项直接复用其完整集合捕获、作用域绑定和回读校验，不另建领域存储或盲拷宿主数据库。调用方仍须在领域事务/锁内提供一致快照；跨领域生产捕获与 API 接线没有在本项伪报完成。

归档类型固定本机回退；用户标签仅存为元数据，文件名使用随机 ID。隐藏临时文件完整写入、fsync、重新打开、校验摘要与完整反序列化后才以 macOS RENAME_EXCL 无覆盖发布；同名竞争重试，失败清理自有临时产物。全部目标明确不存在返回 None。读回绑定原租户/项目/principal/主机，并拒绝摘要损坏。发布完成后的父目录同步错误会保留已提交文件并返回错误，不能将其描述为发布前失败。

已编写但未运行的独立测试：

- `cc_data_22_backup_publishes_readable_snapshot`：匿名 SQLite 真实 `usage::source_settings` 所有者，在同一读取事务内捕获，后续修改真实记录后完整回读原值与版本；校验 0600、无暂存遗留、租户/主机/principal 不匹配与损坏拒绝。
- `cc_data_22_backup_collision_never_overwrites`：四线程在真实私有临时目录并发竞争同一发布 ID，断言 ID 唯一且各自完整字节未被覆盖。
- `cc_data_22_backup_failure_and_absence_leave_no_archive`：全体明确不存在、捕获不完整、所有者失败、发布前校验失败和已删除目标目录均不产生可见归档或遗留临时文件；故障注入只作用于发布边界，不模拟领域存储。

本轮仅实现并格式化本项三个 Rust 文件，未执行 Cargo、构建、单元、实机或其它运行验证，无测试退出码、有效测试数或通过结论。未领取 Cargo 槽位，CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR 未设置，无自有测试进程和槽位锁。按本轮覆盖要求删除任务卡，不代表验证通过。恢复验证授权后，可在仓库槽位入口执行下列定向命令并记录实际槽位、路径、退出码及有效测试数（本轮未执行）：

```sh
python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_22_ -- --nocapture
```


## 子任务 23：列出目标备份（2026-11-14）

实现 `data_management::backup_list::list_local_backups`，复用任务 22 的 LocalBackup 格式、校验与私有文件打开。通过 pinned descriptor 枚举目录并相对打开，不跟随符号链接，拒绝硬链接及不安全权限。仅返回与完整 ProjectScope、principal、host 匹配的 ID、类型、格式版本、实际字节数、创建时间和恢复资格状态；按时间降序、ID 排序。结构合法仅表示 RequiresOwnerValidation，恢复仍须原回执摘要、独立授权与领域校验，不宣称已可恢复。可归属的未来版本和损坏结构返回明确状态；不可解析、超限、不可读或无法安全确认身份的条目隐藏，不向其它租户泄露文件名或诊断。根目录及枚举失败明确报错，不将不可达存储当空列表。

来源静态阅读：CC Switch `commands/import_export.rs::list_db_backups` 调用 `database/backup.rs::list_backups`（任务卡旧路径 backup.rs 已迁入 database），原实现读取大小、时间并倒序。本项沿用元数据列举语义，增加现有目标格式与身份安全边界，不引入整库备份体系。未接 API/SDK/UI（由后续任务负责）。

独立测试源码 backup_list_tests.rs 由真实父模块按 macOS 注册：`cc_data_23_backup_list_returns_visible_metadata` 使用匿名内存 SQLite 真实用量偏好所有者捕获，覆盖元数据、列举前后字节不变、未来版本、结构损坏及不可归属损坏隐藏；`cc_data_23_backup_list_hides_other_tenant` 覆盖租户、项目、principal、host 和符号/硬链接边界。两项测试均未运行。本轮遵循用户覆盖仅实现，不执行 Cargo、构建、单元或实机测试，无通过证据、退出码或有效测试数；未领取槽位，CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR 未设置。编号卡按用户要求删除，plan.md 保持只读。

待统一回归（未执行）：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_23_ -- --nocapture`，届时记录实际槽位、绝对缓存路径、退出码及有效测试数。

## 子任务 4：SQL 隔离暂存实现（2026-11-14）

`data_management/sql_stage.rs::stage_sql` 接受已授权上传字节与 SourceScope，不接受目标数据库或路径。只在一次性内存 SQLite 执行 SQL；authorizer 正向白名单拒绝 ATTACH/DETACH、VACUUM、虚拟表、函数及除 foreign_keys/user_version 外的 PRAGMA，未知动作默认拒绝，不输出 SQL 错误或载荷。输入上限 64 MiB，数据库上限 256 MiB，单值上限 16 MiB，VM progress handler 有界；临时存储固定内存。校验精确首行导出头、事务闭合、七张来源必需表及 integrity_check，再复用 source::read_connection 校验版本、关键列并提取有界记录。返回原作用域绑定的 SourceSnapshot，不迁移 schema、不写目标、不授予领域执行权限。此入口未接入上传 API，调用者必须先授权输入及目标身份。

独立测试注册于 data_management/mod.rs，过滤族 `cc_data_04_`，覆盖匿名导出形状、隔离作用域、跨文件攻击无磁盘副作用、危险 PRAGMA、空头、截断事务、关键列缺失与未来版本。夹具依照来源 dump_sql 格式手工构造，未调用来源程序导出真实文件，不作为真实导出兼容性已验证证据。

按本轮用户覆盖要求，仅完成代码和测试代码，未运行 Cargo、编译、单元或实机测试；无退出码、有效测试数或通过结论。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 均未设置。后续统一验证使用仓库 cargo_slot.py 入口执行 `cargo test -j 1 -p vw-agent-apps --lib cc_data_04_ -- --nocapture`，另补来源真实匿名导出兼容验证。

## 子任务 24：无覆盖重命名备份（2026-11-14）

`backup_rename::rename_local_backup` 复用任务 22 的私有根目录、备份格式和回执摘要校验以及任务 23 的 ID 语义。输入原回执、完整租户/项目/principal/host 与目标 ID；先校验归档摘要及作用域，再在固定目录描述符内以 macOS renameatx_np(RENAME_EXCL) 原子移动，不改内容、内嵌显示标签和摘要，返回新 ID 回执。目标沿用现有小写 ASCII ID 限制，拒绝空值、分隔符、控制字符和同名；源拒绝符号/硬链接，目标链接与既存文件不覆盖。大小写等价目标由文件系统原子判定。目录同步失败可能发生于提交后，明确报错并保留目标供调用者核对。调用者先授权执行主机根目录与作用域；未接 API/SDK/UI。

来源静态核对：CC Switch commands/import_export.rs::rename_db_backup → database/backup.rs::rename_backup；本项沿用文件重命名语义，目标使用已有 ID 合同，不引入第二份备份格式。独立测试 backup_rename_tests.rs 注册到 macOS 父模块，过滤 cc_data_24_，覆盖选中项移动、原字节和其它文件保留、碰撞、非法 ID、同名、外部主机、源/目标符号链接、大小写等价和四线程竞争唯一成功。

遵循本轮用户覆盖：仅实现和静态审阅，未运行 Cargo、构建、单元或实机测试，无退出码、有效测试数或通过结论。未领取槽位；CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除，plan.md 未修改。后续统一回归命令（未执行）：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_24_ -- --nocapture`。

## 子任务 5：迁入副本 schema 升级（2026-11-14，未验证）

`source_upgrade.rs::upgrade_staged_copy` 接入 SQLite 私有临时副本和 SQL 内存暂存入口，按冻结来源 `database/schema.rs` 的 18→24 步骤新增应用启用列与 parser_state；单事务失败回滚，不接受目标数据库连接。原文件及摘要不变，暂存摘要 version 表示升级后的 schema。当前支持 18–24，0–17 涉及数据重写的迁移尚不支持，显式返回 UnsupportedVersion；未来版本拒绝。旧版本缺必需源表/列直接失败，不调用 create_tables 补造数据。24 维持已有只读解析合同。

匿名 fixture `data_management/fixtures/schema_18.sql` 摘取冻结来源实际建表定义并移除 v18 之后的列，不读取用户库。新增独立注册测试 `cc_data_05_upgrade_migrates_supported_copy`、`cc_data_05_upgrade_failure_preserves_source`，包含游标保留、幂等、失败事务回滚、源字节保留、缺表及版本拒绝断言。这些断言尚未执行。

按用户本轮覆盖要求，未执行 Cargo、编译、格式化、单元或实机测试；无退出码、有效通过数或运行前后差异证据。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一回归入口：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_05_ -- --nocapture`，并回归 `cc_data_03_`、`cc_data_04_` 的暂存调用行为。

## 子任务 30：备份分类占用（2026-11-14）

实现 `data_management::locations::list_locations`，执行端提供已授权且仅含当前租户/项目/主机数据的四个根目录及作用域；不从 HOME 自动发现，不接收客户端路径，不返回本机路径。十四类白名单沿用来源分类及文件匹配规则，缺失类别保留零值。目录分类统计顶层条目、递归普通文件字节与最新文件修改时间，忽略 `.DS_Store` 和子项符号链接；绝对路径逐级打开、描述符相对遍历，不跟随祖先或类别目录链接，权限/枚举/读取元数据失败显式错误，递归深度上限 128。数据库分类沿用任务 23 的原生作用域过滤归档列表、大小与归档创建时间，不另扫旧数据库。该列表原有的无法确定归属归档省略语义继续保留；其它分类只允许执行端已隔离目录，不扫描混合租户旧根目录。

新增独立测试 `cc_data_30_locations_measures_whitelisted_categories`、`cc_data_30_locations_does_not_follow_symlink`，通过 macOS 父模块注册。按本轮用户覆盖要求，未运行 Cargo、单元测试或实机验收；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置。仅完成代码与静态审阅，不能视为验证通过。统一回归需执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_30_ -- --nocapture`，并补权限失败、全部分类匹配与真实备份列表集成场景。

## 子任务 7：选择与冲突预览（2026-11-14，仅实现）

新增 `data_management/preview.rs::{build_preview,ImportPreview::resolve}`，父模块注册独立 `preview_tests.rs`。只将选中类别传给只读领域规范化适配器；复用 portable_policy 的 CrossDevice 过滤后比较，输出新增、不同、相同、无效引用，不输出载荷、名称或字段指针。不透明 UUID 将决策限制在当前预览；不同项必须显式 skip/replace，无效引用只能 skip；重复、多余决策拒绝。来源完整摘要（含 WAL）、目标修订和 SourceScope 绑定，准备执行时重新核对。无文件或数据库写入。

这是供后续逐类别迁入任务消费的预览核心，并非已接通 API 或七类目标存储。领域适配器负责解析、身份匹配与引用校验，必须在同一目标一致性边界捕获 revision，执行者在锁/CAS 内重验并重新授权；未支持类别返回 UnsupportedCategory。未实现 keep_both，不将 DTO 可表达能力当作业务支持。

按本轮用户覆盖要求：未执行 Cargo、编译、单元测试、实机测试；未领取槽位，CARGO_TARGET_DIR 未设置，无退出码或有效测试数通过证据。新增两项静态测试代码，待统一回归执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_07_ -- --nocapture`。静态实现不能证明运行行为，任务卡按本轮要求删除。

## 子任务 28：按数量清理备份（2026-11-14）

`data_management/retention.rs::cleanup_local_backups` 复用 `backup_list` 与 `backup_delete`，仅清理同租户/项目/principal/主机、结构有效的本机回退归档。按创建时间升序及 ID 升序裁决相同时间；保护源与回退点沿用 LocalBackup 持有的 inode 文件锁，受保护项可以使最终数量超过策略。不可解析或不支持的归档不参与计数、不自动删除；存储错误立即返回，先前删除可能已提交，调用方应重新列举。调用方应串行调度同一根的保留清理；此模块不提供批次原子事务。

来源 `database/backup.rs::cleanup_db_backups` 与 `settings.rs::effective_backup_retain_count`：默认 10，u32 配置的 0 归一为 1；负数及超过 u32 的输入明确拒绝。独立 `retention_tests.rs` 已注册，包含保护源/回退点、外作用域保持、0/1、非法值及同时间稳定排序测试代码，大小写别名分支仅在文件系统存在该别名时适用。

按用户覆盖要求，本轮只实现，未运行 Cargo、编译、单元或实机测试，无通过证据、退出码或有效测试数；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置。待统一回归命令（未执行）：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_28_ -- --nocapture`。仅面向 macOS，网关、调度及恢复调用接线不在本项范围。

## 子任务 20：可迁移数据包导出（2026-11-14）

`data_management/export.rs::export_portable_package` 接收已授权 SafetySnapshotScope、显式类别和逐类别所有者捕获闭包；闭包必须按原租户/项目/principal/主机读取规范化对象并保持一致性锁，不支持类别返回错误。此项交付业务核心，未接网关、传输或七类生产捕获，不宣称端到端导出已验收。包内不记录来源作用域或路径，复用 portable_policy 的 CrossDevice 默认过滤（含凭据、路径、系统集成、执行权限与未知字段）；正文中的用户自由文本仍需所有者确认可公开，过滤器不是任意正文秘密检测器。

复用既有 zip 依赖，固定 manifest.json/data.json 两条目，清单绑定格式版本 1、portable_export、完整类别与数据 SHA-256；回执另绑定整个包 SHA-256。verify_portable_package 拒绝额外条目、版本/类型/摘要不符、类别不一致、不安全字段及超限（64 MiB、10000 条）。不复制本机回退前镜像，不执行 SQL。

macOS export_files.rs 在已授权现有 0700 私有根目录描述符内创建 0600 隐藏临时文件，写入、同步、完整回读校验后 RENAME_EXCL 发布 UUID.shiju-package；拒绝覆盖，不接受客户端文件名。调用者必须在整个操作期间独占该导出目录；错误清理自有临时文件，发布后的目录同步错误清理自有结果。父模块已注册独立 export_tests.rs，覆盖成功脱敏/完整校验、损坏包、捕获失败、校验失败清理及覆盖冲突。

遵从本轮用户覆盖要求：未运行 Cargo、编译、单元或实机测试；无退出码或有效通过数，不声明验证通过。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。待统一验证：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_20_ -- --nocapture`；还需生产所有者捕获和下载授权集成验证。

## 子任务 28：按数量清理备份（2026-11-14）

`data_management/retention.rs::cleanup_local_backups` 复用 `backup_list` 与 `backup_delete`，仅清理同租户/项目/principal/主机、结构有效的本机回退归档。按创建时间升序及 ID 升序裁决相同时间；保护源与回退点沿用 LocalBackup 持有的 inode 文件锁，受保护项可以使最终数量超过策略。不可解析或不支持的归档不参与计数、不自动删除；存储错误立即返回，先前删除可能已提交，调用方应重新列举。调用方应串行调度同一根的保留清理；此模块不提供批次原子事务。

来源 `database/backup.rs::cleanup_db_backups` 与 `settings.rs::effective_backup_retain_count`：默认 10，u32 配置的 0 归一为 1；负数及超过 u32 的输入明确拒绝。独立 `retention_tests.rs` 已注册，包含保护源/回退点、外作用域保持、0/1、非法值及同时间稳定排序测试代码，大小写别名分支仅在文件系统存在该别名时适用。

按用户覆盖要求，本轮只实现，未运行 Cargo、编译、单元或实机测试，无通过证据、退出码或有效测试数；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置。待统一回归命令（未执行）：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_28_ -- --nocapture`。仅面向 macOS，网关、调度及恢复调用接线不在本项范围。

## 子任务 31：按分类清理（2026-11-14）

`location_delete::delete_location` 复用任务 30 的分类白名单与文件匹配规则；拒绝未知 ID、database 和作用域不符。仅使用执行端已授权隔离根目录，不发现 HOME。描述符相对遍历不跟随链接，拒绝硬链接和特殊文件，深度限 128；文件模式不递归同名目录。使用独占非阻塞 inode 锁保留持有共享锁的恢复源，返回实际已 unlink 普通文件字节与相对路径未删条目，保留分类根目录。执行端仍须串行调度同根写操作，文件锁为协作协议。

技能/历史所有者尚未暴露清理锁和备份缓存刷新生产接口；`skills` 及三个 `codexHistory*` 分类明确返回 OwnerRequired，不绕过领域所有者。本项只交付其余分类核心，不宣称这些领域清理闭环完成。独立测试代码覆盖匹配文件、数据库与未知类别拒绝、作用域、所有者缺失、保护文件与符号链接及部分释放，已在 macOS 父模块注册。

按用户本轮覆盖要求，只写代码，未运行 Cargo、单元或实机测试，未领取槽位，CARGO_TARGET_DIR 未设置，无退出码或有效测试数通过证据。待统一回归：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_31_ -- --nocapture`；领域分类待真实所有者接口就绪后接通并验收。按本轮要求删除任务卡，不表示验证通过。


## 子任务 34：备份维护 HTTP API（2026-11-14）

`agent_apps_data/backup_crud_api.rs` 通过既有 agent_apps 路由挂载四个 POST 入口 `/agent-apps/data/backups/{create,list,rename,delete}`，请求沿用 data_management DTO。复用受保护 access policy 文件与 token 校验，新增显式项目许可 `data_backups`（默认空）与执行主机 `data_backup_root`（默认无）；根必须为已存在的绝对 0700 私有目录。请求不接受根路径；租户与 Extension 身份一致，principal 与 token 身份一致，主机须在线本地，所有归档按完整作用域过滤。不复用导出、技能或生命周期权限扩大访问。

沿用原 local_rollback 格式；`backup_api` 从真实归档读取标签、类别、摘要及大小，摘要作为 revision。目录 flock 串行化本 API 的跨进程操作；标签更新保留 ID、完整 owner 内容，持有源独占锁检查 revision 后以隐藏临时文件 fsync/readback/rename 原子替换。删除在同一源独占锁内验证 revision，再 unlink 与目录同步；受保护源返回冲突。底层 ID 重命名也补持有源独占锁，避免绕过保护。响应错误只有状态码，统一 no-store，不返回原始配置、路径或异常信息。

创建的生产捕获范围目前明确为 **Preferences**：从显式注入 GlobalDatabase 的 SQLite 只读事务中捕获当前 principal 的 agent_apps_user 与当前执行主机的 agent_apps_host 完整原值/明确不存在，保持事务到归档持久化。其它类别、portable_export 与非 SQLite 捕获明确返回 501，不用可迁移 SharedSnapshot 冒充完整本机前镜像；本项未补其它领域捕获。全体不存在返回 null，无可见备份。请求键与完整作用域哈希生成稳定归档 ID，同一键现存备份的名称/类别不一致返回冲突；标签改名后重放原创建请求也返回冲突。首次发布与 ID 移动为两阶段，后阶段失败可能留下随机 ID 的有效归档，需重新列举核对，不承诺跨归档/数据库事务或自动回滚。现有旧归档可列举维护，无第二套回执库。

新增并注册 `cc_data_34_backup_api_maintains_isolated_backups` 与 `cc_data_34_backup_api_rejects_cross_tenant_id` 源码，使用匿名临时 SQLite、私有归档根与本地动态端口真实 HTTP，覆盖创建/列举/改中文标签/旧 revision 拒绝/删除及外租户拒绝。按用户要求**未运行** Cargo、构建、单元或实机测试；无通过证据、测试数或退出码。未领取 Cargo 槽位，CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR 未设置。仅对新增文件运行独立 rustfmt 排版，不代表编译或行为验证。统一回归需执行仓库槽位包装下的 `cargo test -j 1 -p vw-agent --lib cc_data_34_ -- --nocapture`，并补鉴权拒绝、保护源、并发与发布后失败场景。

## 子任务 10：MCP 迁入实现（2026-11-14，未验证）

- `data_management/mcp.rs::normalize` 严格解析 SQLite mcp_servers 与统一 JSON mcp.servers，整类先校验、拒绝重复 ID、非法定义及需另行授权的字段；保留元数据与各应用启用意图。旧版分应用 JSON 明确拒绝。
- `McpCatalogStore::{preview_data_migration,commit_data_migration}` 为唯一目录所有者接线，固定来源绑定项目、主机与 principal，接收执行主机已授权的安装目标；预览以完整私有定义摘要比较，避免通用可迁移过滤忽略 command/args/enabled 导致误判相同。来源摘要、冲突决策与整个目录快照均重新核对。
- 先通过任务 8 的 `with_safety_snapshot` 持久化所改目录条目的完整前镜像（含不存在标记），再在 SQLite Immediate 事务中比较完整 namespace 并批量写入；任一错误回滚整类。回执与写入结果分别返回，过期快照不覆盖并发变更。相同内容重新预览后跳过，替换推进 revision；墓碑不复活。原生状态固定 NotVerified，不执行 MCP、不授予 runtime/ACP 工具能力、不分发原生文件。
- 当前批量事务只支持 SQLite；PostgreSQL 明确拒绝，无逐条降级。此为内部所有者入口，网关授权、任务回执持久化与多类别恢复调度仍由对应后续任务接入，不宣称 API 已发布。
- 新增业务包 `cc_data_10_mcp_preserves_scope_and_enabled_state`、`cc_data_10_mcp_import_never_executes_command` 和所有者 `cc_data_10_owner_add_skip_replace_and_repeat_without_duplicate_rows`；全部仅编写、未运行。统一回归应分别运行 vw-agent-apps / vw-agent 的 `--lib cc_data_10_` 定向测试，并补 SQLite 故障注入、主机/项目隔离及授权目标边界。
- 本轮按用户覆盖要求未执行任何 Cargo、单元或实机测试，无退出码、有效测试数或验证通过证据；未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。只格式化本次独立新增 Rust 文件。按覆盖要求实现后删除编号任务卡，不能据卡片删除推断验收通过。

## 子任务 9：供应商迁入代码交付（2026-11-14）

新增 `data_management/providers.rs::{prepare,ProviderPreview::resolve}`，封装来源摘要、目标记录摘要、身份与随机预览句柄到保留载荷的映射；SQLite/JSON 来源按应用 ID 和供应商 ID 关联端点，重复 ID、非法端点、孤立引用和非法图标使整批准备失败。元数据复用 presentation/icons/endpoints 所有者校验，不复制 is_current 或 failover 队列作为激活动作。仅支持 Claude、Gemini、Codex；其它应用、Bedrock/Vertex 和无法解析的配置明确拒绝。内联自定义图标沿用既有校验，外部图标 ID 不伪装成有效图片。

`AppProviderStore::{preview_cc_providers,import_cc_providers}` 是现有所有者的新业务入口，非 HTTP 路由。调用方必须先授权 SourceScope；执行时 SQLite Immediate 事务内重新读取同租户完整供应商集合，校验预览版本和决策，捕获完整原记录/不存在状态，通过任务8创建回退点后在唯一 namespace 原子提交。失败由 SQLite 回滚类别，不逐行补偿，不触碰 live；绑定原生发布/账号的记录拒绝。PostgreSQL 明确不支持此入口，不切换后端。回退归档保留完整前镜像；恢复入口与 API 调度仍由后续任务接线。

凭据按 portable_policy 不从源复制。Claude/Gemini 第三方配置须有相同目标身份的既有领域凭据，否则拒绝；Codex 只保留模型/路由 TOML 白名单及目标已有 key，不搬源 auth。官方记录仅入目录，不改变本机登录。测试文件 `providers_tests.rs` 注册四个 `cc_data_09_` 行为测试，涵盖重复执行、skip、跨主机/非法批次、源凭据拒绝；本轮均未运行。

用户要求本轮只写代码，未运行 Cargo、编译、单元或实机测试；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置，无退出码或有效测试数。统一验证应执行 `cargo test -j 1 -p vw-agent-apps --lib cc_data_09_ -- --nocapture`，并补充 vw-agent 所有者真实 SQLite 事务、回退文件失败与并发版本场景。本节为交付记录，不是 macOS 验收通过证据。

## 子任务 11：技能类别迁入实现（2026-11-14）

入口为 data_management/skills.rs::{SkillsPreview,SkillMigration}；所有者接线为 vw-agent/src/gateway/agent_apps_skill_migration.rs::{preview_migration,migrate_skills}，复用 SkillInstallsStore 的项目/主机 settings 事务。技能资源由已授权目录提供，先复制到私有暂存并校验清单与内容摘要；非法目录、链接、元数据不符、重复身份、未知启用目标和未受管冲突均拒绝。SQLite skills 行保留来源 ID、名称、描述、仓库引用、时间、readme_url、内容摘要与启用标志；skill_repos 不另建安装记录。未支持的 JSON skills 布局显式拒绝。

预览绑定来源、作用域、目标版本与文件观察摘要；逐项 skip/replace，重复输入不增行。共享根锁经 dup 复用，在同一所有者事务准备所有 SSOT/应用复制和禁用，写前调用任务 8 创建含 settings/资源/目录/链接的安全快照，提交失败反向回滚；提交后隔离目录清理失败明确报告。现有符号链接部署的覆盖可能因既有 CopyDeploy 模式约束被拒绝，不强行转为复制或绕过所有权校验。不运行安装脚本。

独立测试 skills_tests.rs 注册三个 cc_data_11_ 行为测试，覆盖元数据/启用保留、skip/replace/重复、路径穿越及链接拒绝、资源发布回滚。按用户要求均未执行；未调用 Cargo、编译或实机测试，无退出码、有效测试数或通过结论；未领取槽位，未设置 CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR。仅对本任务三个新增 Rust 文件直接 rustfmt（退出 0），不代表编译或行为验证。后续统一验证入口为 scripts/cargo_slot.py 内的 cargo test -j 1 -p vw-agent-apps --lib cc_data_11_ -- --nocapture，另需覆盖网关所有者事务失败场景。

共享工作区接线：agent_apps_skills_store.rs 增加 migrated/migration_snapshot 默认字段与 migration 子模块；skills/local_install.rs 放开 crate 内复制复用；copy_deploy.rs、disable_deploy.rs 增加 crate 内共享锁准备入口。以上既有文件本轮开工即未跟踪，包含其他任务的大量实现，故未整体纳入本项提交，接线变动保留工作区待集成。source.rs 的 principal_id 只读访问器已随共享工作区基线进入 HEAD。plan.md 未修改。按本轮覆盖要求实现后删除编号任务卡。

## 子任务 13：项目组合迁入（2026-11-14）

实现 `data_management/profiles.rs::{prepare,ProfilePreview::resolve}` 与唯一所有者 `ProfileOperation::import_profiles`。SQLite profiles payload 的供应商、MCP、技能与规则 ID 按应用和类别映射，技能转换为 installationId；None、空集合与空规则停用标记保留。未选中或目标缺失的引用显示 InvalidReference，只允许 skip；执行再次检查目标存在性。整个类别在 ProjectProfileStore 同一 settings 事务中写入，保留当前选择，不触发应用；revision 防止陈旧预览，重复预览相同内容默认跳过。来源 sort_order/created_at/updated_at 不迁入：目标无排序字段，版本和更新时间由所有者产生。

调用者仍须完成鉴权、回退点捕获，并持有所选领域依赖锁/CAS 至提交；此入口不代表 API 执行编排已接线。匿名独立测试 `cc_data_13_profiles_remap_selected_references` 和 `cc_data_13_profiles_reject_dangling_reference` 已注册，覆盖映射、幂等、skip/replace、悬空及提交前引用失效。

按用户最新覆盖要求，仅写代码与静态阅读：未运行 Cargo、构建、单元或实机测试，不能声称通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 均未设置。待统一验证：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_13_ -- --nocapture`，并补充真实存储事务的原子失败与作用域验收。

## 任务 12：规则与提示词迁入（实现，未验证）

新增 `data_management/prompts.rs::normalize`，严格解析 SQLite prompts 与 JSON /prompts；校验应用、身份、正文、单一活动条目，保留正文和创建时间。目标唯一所有者为 `PromptsStore`，`preview_data_migration` 与 `commit_data_migration` 按认证 SourceScope 绑定源及目标快照，每应用目录整体处理 skip/replace；保留活动选择但不发布原生文件。SQLite 类别批量 CAS 写前保存 Prompt 回退快照，拒绝陈旧预览及不支持的 PostgreSQL 批量写入；重复迁入无新增行。源 updated_at 不在现有 PromptEntry 合同中，不扩展存储模型。

新增 apps 解码边界测试及 agent 所有者新增、重复、skip、replace、陈旧预览和原生文件不变测试代码。遵照本轮用户覆盖要求，未运行 Cargo、单元测试或实机测试，无通过证据；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置。后续统一回归运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_12_ -- --nocapture` 及相同入口 `-p vw-agent --lib cc_data_12_`，并补非法源整类别不写入、跨租户/项目/主机拒绝及 SQLite 源回归。

## 子任务 29：后端周期备份（2026-11-14）

网关 bootstrap 的 `periodic_backups` 生命周期每 60 秒读取受保护访问策略，退出时停止轮询；不依赖桌面页面。管理员在 grant 中显式配置 `periodic_data_backups: [{"project_id":"<project>","interval_hours":24}]`，且项目必须同时有 `data_backups` 许可；默认空，零小时关闭，仅在线本地主机和已授权绝对私有备份根。使用现有 SQLite 偏好完整事务捕获接口，其它领域和数据库不伪支持。错误使用固定脱敏日志。

`data_management::periodic::periodic_backup_if_needed` 在目录锁内按完整租户/项目/用户/主机筛选有效归档，以最新归档创建时间决定到期；并发执行端重新检查，重启不会重置已有归档的周期。时间参数可控，倒退不提前创建。关闭在访问文件前返回；不接入或改变用量清理。无数据时不创建归档，失败下次轮询重试。

用户本轮明确要求只写代码：未运行 Cargo、编译、单元或实机测试，不声称验证通过；未领取槽位，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 未设置。待统一回归覆盖：无页面到期创建一次、零间隔零创建、重启/并发、权限撤销、失败日志与重试；macOS 限定。


## 子任务 32：来源识别与只读预览 API（2026-11-14）

`migration_read_api.rs` 注册 POST `/agent-apps/data/sources/identify` 与 `/agent-apps/data/import/preview`。认证在读取请求体前执行，随后核对租户、项目、principal 与执行主机；响应 no-store，无来源路径、配置正文或错误载荷。管理员策略新增项目权限 `data_migration_read`、主机 `data_migration_root` 和 `data_migration_sources`（来源 ID 到绝对路径映射）；默认空即拒绝，既有备份权限不隐含迁入权限。

本项选择任务允许的主机授权来源分支，不提供上传接口。identify 请求的 transferId 首次为管理员登记的来源 ID；返回新的服务端随机暂存引用，preview 只接受该返回引用。识别沿用只读 source 所有者，不接受客户端路径；最多保留两份暂存，生命周期五分钟，原始数据不返回客户端。授权撤销在预览时重新检查；来源和预览按原租户/项目/principal/主机绑定，过期引用在后续请求时清理。进程重启丢弃全部引用。格式准确标记 sqlite/json/sql，未知发行版本明确为 unknown，不将 schema 猜作发行版本。

预览目前接通真实供应商领域 `providers::prepare` 与 AppProviderStore 读取，另需原有 provider_config_read 授权；仅支持供应商 All 选择，其他类别返回 501，条目选择返回 400。目标观察、来源与规范化值保留在服务端有界缓存（最多16份预览）；revision 是绑定该记录的不透明随机引用。无写入或执行路由；后续执行必须重新授权、核对源及目标 revision 并取得回退点，不能仅凭 DTO 提交。响应只含计数、UUID 和允许的冲突决策，不回显供应商名称、配置、路径或凭据。读取并非跨条目事务快照，后续执行须使用领域 resolve/CAS 拒绝目标变化。

按最新用户要求仅实现并静态阅读，未运行 Cargo、单元测试或实机测试，无通过结论；未领取槽位，两个 Cargo 缓存路径未设置。后续统一验证需补实际 HTTP 成功、401/403、跨身份引用、过期/限额及凭据脱敏场景。


## 子任务16：选中类别提交（2026-11-14）

业务核心 `data_management/apply.rs::apply_selected` 校验作用域、预览保留的来源摘要、选择集合与已准备类别的完整对应关系，拒绝重复和替换选择；空集合不调用写入者。固定按供应商、MCP、技能、规则、偏好、用量、组合分派，组合在引用所有者之后。来源为不可变 SourceSnapshot，不将 CC 当作运行存储。

macOS 执行端 `vw-agent/src/agent_apps/data_import.rs::commit_selected` 用封闭 PreparedCategory 枚举接通七类既有提交入口，各所有者在自己的事务/CAS 内重新核验预览修订与冲突决策；不新增任意 SQL、namespace 或原生命令。供应商/MCP/规则收集所有者返回的回退回执；技能保留状态中的回执；组合、偏好、用量要求调用方先完成授权和持久前镜像捕获，显式传入 recovery 回执。此入口是内部执行核心，不代表 API 已开放；回执不是授权。失败停止后续类别，返回此前已完成类别和可用回执，不宣称全局事务或已回滚。失败类别自身可能已提交（例如技能清理失败），恢复层必须核对所有者状态；跨类别恢复是后续任务17的范围，并非本任务前置条件。

独立 `apply_tests.rs` 注册了 `cc_data_16_apply_commits_only_selected_categories` 与 `cc_data_16_apply_rejects_stale_preview`：匿名内存 SQLite 真实用量写入、重导零写、空选择、类别替换、源摘要及目标修订变化。仅新增测试代码，没有执行。后续统一回归需编译 vw-agent 的 macOS 执行端并执行 vw-agent-apps 的 cc_data_16_ 定向测试。

按用户覆盖要求未执行 Cargo、单元测试或实机验证；未领取槽位，两个 Cargo 缓存变量均未设置，无退出码、有效测试数或通过结论。仅对新增 Rust 文件运行独立 rustfmt（非 Cargo）。此前卡片中的跨类别事务前置判断已更正，任务17仍单独实施。

## 子任务 17：失败恢复编排（2026-11-14）

`data_management/rollback.rs::apply_with_rollback` 复用任务 16 的分发和任务 8 的归档校验。写前校验授权 scope、摘要、精确目标集合与类别覆盖；失败后先恢复可能已部分提交的失败类别，再逆序恢复已完成类别，逐类别保留失败，返回原回执及 recovery_required，停止剩余导入。所有者回调必须覆盖引用/资源并持锁或校验本次写后版本；不支持恢复的所有者不得进入执行。调用方需持久化回执与恢复失败类别，并在修复前拒绝同作用域后续任务；此核心未接入网关任务持久化或七类所有者自动恢复，不宣称已具备全链路写入阻断。

新增独立 `rollback_tests.rs` 两个 `cc_data_17_` 场景，由父模块注册；使用隔离 SQLite 引用关系和真实回退归档，指定边界注入晚期提交与恢复失败。它们覆盖编排，不冒充七类业务适配器验收。

按本轮覆盖要求未运行 Cargo、编译、单元或实机测试，测试结果未知。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。统一回归待运行：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent-apps --lib cc_data_17_ -- --nocapture`，另需核验所有者恢复与任务持久化接线。

## 子任务 18：迁入进度与安全取消（2026-11-14）

新增 macOS `data_management/operations.rs::ImportOperation`，复用共享 `Operation` 执行器、`DataOperationProgress` 和任务 17 的 `apply_with_rollback`。查询与取消核对完整目标身份；提交锁与取消锁排序，提交临界区明确拒绝取消，类别间接受取消后停止派发并逆序恢复已尝试写入的所有者。未执行的类别不调用恢复；恢复失败报告 Partial/RecoveryFailed，不冒报 Cancelled。重复取消和终态查询保留原结果，终态先通过保存回调持久化再发布。

接线合同：授权任务所有者提供持久化保存回调、保存回退回执，持有目标级迁入/恢复互斥锁直至 run 返回，并以共享执行器调度 run；本项不实现 API 或第二套调度器。保存失败必须向调用者报告并保留恢复责任。计数以类别为单位；committed 保守保留尝试写入事实。尚未证明七类业务接线或进程重启恢复行为。

按本轮用户覆盖要求，仅实现并静态阅读，未执行 Cargo、编译、单元或实机测试；没有验证通过证据，未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置。统一回归需覆盖取消后恢复、提交临界区拒绝、重复取消、跨作用域拒绝、保存失败、恢复冲突与目标互斥。

## Task 26: backup restore (implementation only)

`backup_restore::restore_local_backup` validates the source and every owner bundle before capture, creates and reads back an exact task-8 safety snapshot, persists its receipt before writing, and invokes task-19 projections after successful restoration. The source guard spans retention cleanup; safety snapshots live in the independent task-8 store and cannot be pruned by backup retention. Owner failure returns the safety receipt without cleanup; projection and retention errors remain separate from the committed restore.

The authorized caller must retain the shared import/restore/delete exclusion guard and owner transaction/CAS guards; owner validation, restoration and receipt persistence use explicit callbacks, following existing rollback orchestration. This is a core entry point, not seven-owner or gateway integration. Independent registered tests cover retain=1 and exact current-state capture using the real usage-source settings owner with SQLite, and corrupt-source rejection before callbacks.

Per user override: no Cargo, compilation, unit tests or device tests were run. No passing evidence, exit code or test count is claimed. No slot was acquired; CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR were not set. Deferred test filter: `cargo test -j 1 -p vw-agent-apps --lib cc_data_26_ -- --nocapture`, through the repository slot entry. Static source comparison: commands/import_export.rs::restore_db_backup and database/backup.rs::restore_with_retain_one_keeps_source_and_exact_safety_snapshot.


## 子任务 33：迁入写入与操作 API（2026-11-14）

在现有 migration_read_api 路由内共享服务端来源与预览，新增 macOS migration_write_api。POST `/agent-apps/data/import/execute` 提交确认预览；`/agent-apps/data/operations/get`、`cancel`、`retry-projections` 分别查询、提交前取消、重试投影。查询/取消使用现有请求 DTO，重试使用 GetDataOperationRequest 的完整目标与 operationId。来源读取沿用管理员来源引用；读请求需要迁入读取授权，写入、取消和重试额外要求供应商配置写入与同项目备份权限。身份核对包含租户、项目、主机和 principal，跨目标 ID 返回 404，错误不回显底层载荷。

仅接现有预览实际支持的供应商类别，不扩展其它类别的预览或恢复。以共享 Operation 后台执行，返回 queued；SQLite gateway_settings 固定命名空间保存操作及回退回执。目标+幂等键生成稳定操作 ID，输入摘要防止同键异参；终态可脱离已过期预览重放。目录锁复用 backup_api，租户级活动任务排斥覆盖共享供应商目录。取消与 worker 领取使用 SQLite Immediate 事务排序：queued 可取消，已提交临界区返回冲突，终态取消幂等。

AppProviderStore::import_cc_providers_recorded 在既有真实供应商事务内保存终态和回退回执；保存失败一并回滚目录写入，避免另建事务造成提交事实丢失。未成功提交的进程残留操作仅在 PID 已确认不存在时由查询标为 HostUnavailable；不自动重新迁入，不删除其它进程任务。PID 重用或存活状态不明时保持保守占用。

投影能力边界：本入口不扩大原生发布权限，供应商提交后保留 provider_live 的 Partial/ProjectionFailed；其它未选择类别不报告虚构刷新失败。重试调用任务19的 retry_projections，保留尚不可用的原生投影失败，不重放目录导入，不声称原生刷新成功。完整原生投影适配仍需领域发布入口交付，本项没有以成功空回调掩盖该边界。

新增独立 migration_write_api_tests.rs 并注册两个 cc_data_33_ HTTP 场景：修订拒绝、幂等重放/冲突、真实目录提交、部分失败和不重放写入的重试；已授权其它项目的查询、取消与重试拒绝，以及终态取消幂等。夹具仅使用匿名 JSON、隔离 SQLite、临时目录和动态 HTTP 端口。

按本轮用户覆盖要求未运行任何 Cargo、编译、单元或实机测试；仅静态阅读与对修改文件运行独立 rustfmt，没有验证通过证据。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 均未设置，无测试退出码及有效测试数。统一验证待运行：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib cc_data_33_ -- --nocapture`，另需覆盖真实原生投影适配及进程中断边界。

## 子任务 37：迁入导出客户端 SDK（2026-11-14）

唯一实现为 `vw-gateway-client/src/client/agent_apps_data/migration_sdk.rs`，由 client 父模块注册；公开 `DataMigrationError`、`DataExportTransfer` 及 `GatewayClient::data_migration_*` 方法。沿用任务 32 的管理员授权来源 ID，识别返回暂存 transferId 后预览；没有上传 API，SDK 不伪造上传或传递客户端路径。消费任务 33 的执行、查询、取消和显式 retry-projections，原样保留 committed、rollback、partial、逐项结果及取消意图。消费任务 36 的导出句柄响应和 POST 下载（携带完整 target），下载返回有界字节，不擅自落盘。

请求沿用 native_transport 与 apply_request_context，继承租户/网关身份，应用 grant 标记 sensitive，禁自动重试与重定向；超时只返回 Timeout，不自动重放写入。HTTP 保留状态及可解析的封闭 DataManagementError，不保留原始错误载荷、URL 或令牌。成功体限 64 MiB、错误体限 16 KiB。不将未支持类别的 501 转为成功。后续 UI 必须按操作身份查询歧义结果，不把网络超时当作未提交。

新增并注册独立 `migration_sdk_tests.rs`：cc_data_37_migration_sdk_preserves_operation_status、cc_data_37_migration_sdk_does_not_retry_write、cc_data_37_source_preview_and_portable_download，使用本地 TCP HTTP 夹具，覆盖部分提交状态、协议、授权头、结构化失败及导出下载；尚未执行。按用户覆盖本轮没有 Cargo、编译、单元或实机测试，没有通过证据、退出码或有效测试数；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。统一验证入口：`python3 scripts/cargo_slot.py cargo test -j 1 -p vw-gateway-client --lib cc_data_37_ -- --nocapture`；另待验证超时、传输断开、超限响应与跨作用域服务端拒绝。


## 子任务 35：恢复与诊断 API（实现，未验证）

新增 macOS `agent_apps_data/backup_restore_api.rs`：POST `/agent-apps/data/backups/restore/preview`、`restore`、`restore/get`、`diagnose`，挂载既有 agent-apps 路由。每个入口先核对租户、项目、主机与 principal 的 data_backups 授权，再接触归档或数据库；响应禁止缓存，不接受客户端文件路径。诊断以只读 SQLite quick_check 和既有 recovery::diagnose_recovery 读取独立备份目录，不创建、迁移或修复目标库；列表故障与空列表保持可区分。

仅支持现有 backup_capture 的完整偏好归档：严格核对两个固定对象及 AppUserPreferences/AppHostPreferences 编码和内容摘要，其他 owner 不进入写入。预览返回归档修订，确认要求相同修订及显式 confirmed。目标和幂等键生成持久操作 ID，同键异参冲突；后台执行期间持有与备份维护共用的跨进程目录锁及源归档保护。恢复使用任务27的 recover_from_backup，持久化安全快照回执后，SQLite Immediate 事务重新核对两个当前前镜像，原子恢复存在/不存在状态，并在同一事务发布 committed 终态。版本冲突或事务失败不产生部分 owner 写入。孤儿任务仅在执行 PID 确认退出后报告失败，查询不自动重放恢复。安全快照使用既有独立快照格式；归档保留不会清理快照。原生投影尚未接通，明确报告 refresh_required；保留清理失败单独报告。

新增并注册 `backup_restore_api_tests.rs` 的两个 `cc_data_35_` HTTP 场景代码，覆盖实际偏好变更后的恢复、回退 ID、异步终态、幂等重放、诊断及四个入口未授权拒绝。没有运行这些测试。按用户覆盖要求未调用 Cargo、构建、单元或实机测试，无通过证据、测试数量或退出码；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。仅对新增 Rust 文件执行独立 rustfmt。待统一回归使用仓库槽位入口执行 `cargo test -j 1 -p vw-agent --lib cc_data_35_ -- --nocapture`，并补充损坏归档/目标、跨作用域、并发更新、进程中断与保留保护场景。

## 子任务 38：备份恢复客户端 SDK（2026-11-14，实现未验证）

新增 agent_apps_data/backup_sdk.rs 与独立 backup_sdk_tests.rs，父模块公开类型和 GatewayClient::data_backup_*：创建（保留 null）、列表、重命名、删除、恢复预览/执行/查询、诊断、分类列表/清理、策略读写。复用任务37的有界原生 HTTP 传输；无自动重试、重定向、客户端路径或真实数据访问。ID 在 JSON 中传输，完整租户/项目/主机/principal 原样保留；分类统计仅描述执行主机，不能作为本机可打开目录。按任务34/35/36实际接口解码：409无正文只报告冲突，不推测保护或修订原因；恢复独立状态保留 committed、refresh_required 与 retentionFailed；部分清理保留 undeletedCount。恢复查询使用 restore/get，不能误用迁入 operations/get。

新增并注册 cc_data_38_backup_sdk_decodes_protected_conflict 与 cc_data_38_backup_sdk_preserves_remote_host_identity 本地动态端口 HTTP 测试代码，覆盖不安全字符ID、裸409、授权头、主机身份、恢复后保留失败、部分清理和策略读写。仅静态核对前置生产代码和来源 useBackupManager.ts，未运行测试，不能作为行为通过证据。

按最新用户覆盖未执行任何 Cargo、编译、单元或实机测试；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置，无退出码或有效测试数。后续统一回归执行 python3 scripts/cargo_slot.py cargo test -j 1 -p vw-gateway-client --lib cc_data_38_ -- --nocapture，并补完整端点响应、诊断 snake_case/SystemTime 解码、断连/超时、鉴权与真实保护源集成。按要求实现后删除任务卡。


## 子任务 41：桌面导出与保存文件（2026-11-14）

桌面 Apps 设置接入 system_settings_data/export_ui 与 settings/data_management/export_ui。复用任务 37 的 data_migration_export/download：显式选择七类数据、JSON 格式及授权主体 ID，目标沿用网关租户、项目和执行主机，网关重新鉴权；不推测主体身份。说明可迁移脱敏导出与本机完整回退备份的区别。不支持类别保留 SDK 明确错误。异步响应核对上下文与代次。

下载成功才打开 rfd 保存对话框，取消不创建文件；显式目标使用 create_new，拒绝同名覆盖，Unix 新文件权限 0600。写入与 sync_all 成功后才显示成功 toast；写入失败清理本次新建文件并报告清理失败。路径不进入日志，中文不使用等宽字体。设置表单复用统一输入框、复选框和按钮样式、固定标题与底部及滚动内容 4px/7px 布局。

新增并注册 export_ui_tests：cc_data_41_export_ui_saves_downloaded_bytes（中文名及同名拒绝）、cc_data_41_export_ui_cancel_creates_no_file、cc_data_41_export_ui_reports_missing_directory。按用户最新覆盖要求，本轮仅实现，未运行 Cargo、单元测试、实机或字体整形测试；没有通过证据、退出码或有效测试数。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一执行聚焦测试与 macOS 对话框、磁盘写失败和明暗主题字体渲染验收。


## 子任务 42：桌面备份列表维护（2026-11-14，实现未验证）

新增 system_settings_data/backup_list_ui 与 settings/data_management/backup_list_ui，注册 Apps 入口、状态和消息。复用任务38的创建/列表/重命名/删除/分类/策略 SDK，完整目标保留网关租户、项目、执行主机与显式主体；上下文和请求代次拦截过期响应。重命名和删除显式确认，携带 expected_revision，保护/冲突保留失败反馈；成功后重新读取列表、分类和策略。周期0关闭自动备份，数值范围及保护规则仍由后端校验。创建明确使用本机回退类别，不混作可迁移导出。复用统一设置输入框、按钮、toast及固定标题底部的4px滚动条/7px内容留白，无中文等宽字体。

按最新用户覆盖仅写代码，未运行 Cargo、编译、单元测试、实机或字体整形；没有通过证据、退出码或有效测试数。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一回归需覆盖真实匿名备份创建、重命名刷新、受保护删除、策略错误、分类刷新、过期响应及macOS明暗主题长列表字体渲染。本轮按要求删除任务卡。

### 任务 43：桌面恢复与损坏诊断向导（实现，未验证）

- 复用任务 38 的 `data_backup_diagnose`、`data_backup_restore_preview`、`data_backup_restore` 和 `data_backup_restore_operation`；SDK 相关 DTO 补充根导出。
- 应用设置入口使用现有网关、租户、项目、执行主机和授权主体；异步响应核对 catalog identity 与请求代数。诊断与预览不写数据，明确确认才提交，提交后禁止再次恢复。
- 未来版本显示保留数据提示并禁用恢复；损坏与不可用目标仍可诊断/预览。恢复结果显示任务 ID、写前回退 ID、提交/投影/保留警告；网络失败提示结果不确定，不自动重试。
- 复用设置弹窗固定标题/操作区、内容滚动 4px 与两侧 7px 留白、主题样式和默认字体；恢复提交错误通过 toast 提示。
- 用户本轮明确要求仅实现：未运行 Cargo、单元测试、实机或字体整形渲染；未领取槽位，CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR 未设置，无测试退出码与有效测试数。不得据此认定 macOS 验收通过。
- 后续统一回归需检查确认/取消写入边界、身份切换丢弃旧响应、损坏/未来 schema、回退 ID 与投影失败，以及实际中文字体渲染。只读静态核对不能代替上述验证。
