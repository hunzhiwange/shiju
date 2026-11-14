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
