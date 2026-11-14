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
