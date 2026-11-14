# 工作区与应用专属设置 macOS 证据

## 任务 1：前置契约与交叉归属（2026-11-14）

静态核对完成，正式结果见 [workspace-contracts.md](workspace-contracts.md)。记录了 32 个来源命令的唯一所有者、已有供应商 API/SDK/Iced 接点，以及应用身份、路径、文件授权、主机、配置锁、错误与任务取消边界。

只读证据：来源 `git rev-parse HEAD` 退出 0，提交为 ca58c0313360eae7c851fbd7553c1614c7af03e7；RustCodeGraph status 退出 0，索引 7988 文件；explore 正常完成，但索引不可替代当前共享 checkout，关键路径和正在变更的 access/mod 源码另行读取。源码检索曾遇不存在的 transaction.rs、auth *_api.rs 与 shell 空 glob，已改用实际 paths/live/commit、auth_flow/accounts 文件定位，不把检索失败当缺失实现证明。

缺项：工作区项目级授权、敏感 env 读权限、本领域搜索/dashboard 取消动作、模型选项去重/名称回退/中文排序的专用投影、Pi/OMP 提示词与会话完整闭环；逐项详见合同。现有供应商事务并不覆盖全部工作区动作。

用户明确要求本轮不执行 Cargo、任何测试或最终回归，因此仅有源码/文档审查证据，没有新的编译、权限拒绝、持久化、系统动作、GUI/字体或 macOS 运行通过结论。Windows/Linux 未验收。下一步按实际生产接口推进后续卡，统一验证另行执行。

## 任务 13：Hermes 记忆接口类型（2026-11-14）

新增 `vw_api_types::hermes_memory`：`HermesMemoryKind` 严格限定 memory/user；内容响应区分缺失与空文件；写入和开关请求拒绝额外字段；`HermesMemoryLimits` 保留预算及两个独立开关（预算为 UTF-16 单元，不截断内容）；`HermesWriteOutcome` 保留可选 backupPath。`HermesModelSnapshot` 仅包含 default/provider/contextLength/maxTokens，无凭据、任意扩展或可能夹带凭据的 URL，模型写入继续归供应商领域。所有结构使用 camelCase wire 字段。

静态核对：已读取任务 1 正式合同、来源 commands/hermes.rs、src/lib/api/hermes.ts、hermes_config.rs 类型与 TS 定义；RustCodeGraph status/explore 定位现有所有者；API 模块只登记模块及独立测试。新增五个测试函数，覆盖未知 kind 拒绝、预算/开关往返、中文长文本与缺失状态、模型敏感字段拒绝、备份 outcome 往返；仅写入测试源码，未运行。

按用户本轮最高优先要求，不执行 Cargo、单元测试、实机测试或最终回归；没有编译或运行通过结论。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，后续统一验证。

## 任务 3：执行主机文件边界（2026-11-14）

新增 `workspace_paths.rs::WorkspaceDirectory`，消费执行端解析的 OpenClaw workspace 或 Hermes 应用根，限定 HOME 内的 workspace、memory、memories。调用者须先授权 tenant/project/action，并传递可信分派的授权主机及当前执行端身份；身份不匹配或为空直接拒绝，不回退本机。目录逐级 openat/O_NOFOLLOW，按需 mkdirat 后重新安全打开；文件只接受单段相对名称和普通单链接文件。读写删除共用保留的目录描述符，写入复用现有 AppWriteGuard 物理锁和私有文件原子替换，删除使用 unlinkat。没有新增远程协议或路径解析所有者。

静态审查来源 workspace 命令、OpenClaw/Hermes 目录解析、前置 workspace-contracts、providers/paths 和 live/write_guard；补充独立测试源码，覆盖父目录/文件链接、主机不匹配、绝对路径/穿越、缺失目录与正常读写删除，未执行。模块仅在 macOS 注册，其他平台未验收。

遵照本轮要求未执行任何 Cargo、单元/实机/其它测试或最终回归；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 不适用。仅以源码审查交付，不声称编译或行为验证通过；后续统一验证。编号任务文件依用户要求删除。

## 任务 2：工作区文件接口类型（2026-11-14）

已实现 `vw_api_types::workspace_memory`：九个大小写敏感的白名单文件、私有值封装的合法公历 `DailyMemoryFilename`（0001–9999）、workspace/memory 目录枚举、缺失与空内容区分、写入/搜索/打开目录请求、每日列表与搜索 DTO。元数据保持 u64 字节大小和 Unix 秒时间；列表/搜索保留来源 camelCase 字段。内容、预览与查询类型不派生 Debug，避免普通调试日志输出记忆。无新增依赖、路由或权限扩展。

静态核对来源 workspace.rs、workspace.ts、接口边界及任务 1 workspace-contracts.md；前置授权/主机/取消缺项留给对应生产任务，本 DTO 不冒充其实现。独立测试源码覆盖九文件往返、路径与未知值拒绝、世纪闰年、非法日期、null/空字符串、中文与大文件元数据。

按本轮最高优先要求，未运行 Cargo、单元测试、实机测试或其它测试，未启动回归。未领取槽位，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 未设置；没有编译或测试通过结论。后续统一验证。

## 任务 11：Hermes 双记忆文件（2026-11-14）

新增 `hermes_memory.rs::HermesMemory`，复用任务 3 的 `WorkspaceDirectory`：严格将 memory/user 映射到 Hermes 根下 memories/MEMORY.md、memories/USER.md。缺失内容为空并保留 exists；读错误原样传播。写入复用目录描述符、物理锁与私有原子替换，不读取开关、不截断内容、不覆盖另一文件。调用者须先授权并传入可信主机、HOME 与解析后的应用根。

静态核对来源 MemoryKind/read_memory/write_memory、任务 1 合同、任务 3 文件边界和现有 DTO；新增两个独立测试源码，覆盖缺失与空文件、中文长文本、两文件隔离、禁用配置保持、主机拒绝、符号链接拒绝与无效 UTF-8。按本轮要求未运行 Cargo 或任何测试，不声明验证通过；未领取槽位，CARGO_TARGET_DIR 未设置。后续统一验证时运行任务约定的 hermes_memory_roundtrips_kinds 过滤测试。

## 任务 4：白名单工作区原子读写（2026-11-14）

新增 macOS `workspace_files::{read_workspace_file,write_workspace_file}`，消费任务 2 的九文件强类型白名单及任务 3 的执行主机路径边界。读缺失返回 None，空文件保持 Some("")；写按需创建父目录，复用物理写锁、私有暂存及原子替换，不接触全局记忆。调用者仍须先授权项目和动作，传入可信执行主机及原生解析的 workspace 根，不回退本机。

已静态核对来源 commands/workspace.rs、接口边界、任务 1 正式合同、任务 3 实现与底层 write_guard；RustCodeGraph status/explore 已读取。独立测试源码覆盖九文件中文/空文本往返、缺失读取无副作用、主机拒绝、原子替换保持旧 inode、链接拒绝保留原内容及非法文件名，共五个函数，未运行。底层准备/rename 失败清理沿用既有实现，本轮没有故障注入或运行证据。

按用户本轮要求仅编码与静态审查，未运行 Cargo、单元测试、实机或其它测试，未启动最终回归；没有编译或行为通过结论。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号任务文件按要求删除，后续统一验证。

## 任务 5：每日记忆读取与保存（2026-11-14）

新增 daily_memory_files.rs 的 read_daily_memory_file/write_daily_memory_file，复用 DailyMemoryFilename 有效公历日期类型和任务 3 的 WorkspaceDirectory。仅操作解析后的 OpenClaw workspace/memory，读取缺失返回 None，中文及空文本通过既有物理锁、私有文件与原子替换保存，拒绝主机不匹配及不安全链接。调用者须先授权内容读写并提供可信执行主机、HOME 和工作区根；不新增列表、搜索或权限入口。

已静态核对来源、前置合同、日期 DTO 与文件 IO/写锁实现；新增四个 daily_roundtrip_valid_dates 前缀测试源码，未执行。依本轮要求未运行 Cargo、单元测试、实机或其他测试，未启动最终回归，不声明验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一验证该过滤测试及写入失败场景。本任务卡按用户指令删除。

## 任务 17：OpenClaw 模型白名单分节（2026-11-14）

新增 openclaw_catalog::{get_model_catalog,set_model_catalog}，读取 agents.defaults.models 的 alias 与未知字段；缺失返回 None，显式空目录保留为空对象。候选编辑仅替换 models，保留 model、其他 defaults 及非目标文本；畸形祖先拒绝覆盖。复用 providers/live/patch_json 的 round-trip 层，发布须由调用者沿用 OpenClawFiles 与共享事务锁；没有新增配置 IO 所有者。当前底层仅支持 JSON/JSONC，完整 JSON5 明确拒绝，未扩展语法能力。

已静态核对来源 get_model_catalog/set_model_catalog、任务 1 合同、任务 3 文件边界、现有 OpenClaw 与补丁层；新增三个 openclaw_catalog_preserves_defaults 前缀测试源码，覆盖 alias/未知字段、非目标 defaults 与注释、缺失/空目录及非法形状。按本轮要求未执行 Cargo 或任何测试，不声明编译或行为验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一验证；本任务卡按要求删除。

## 任务 18：OpenClaw 代理默认设置（2026-11-14）

新增 openclaw_defaults.rs：完整 agents.defaults 读写保留扩展字段；四个表单字段投影、空白清除、有限数字校验；读取优先 timeoutSeconds，已知数字 timeout 保存迁移。未知旧字段不迁移。保存消费 OpenClawFiles 快照及既有 apply 事务，复用供应商/模型物理锁，冲突及恢复错误原样传播。调用方须先完成配置和执行主机授权。

静态核对来源 get_agents_defaults/set_agents_defaults、AgentsDefaultsPanel.tsx、utils.ts，任务 1 合同、任务 3 文件边界及 provider 配置事务。共享解析器当前仅支持 JSON/JSONC，完整 JSON5 的单引号/裸键明确拒绝，不能声称完整 JSON5 已验收；此限制与现有 provider 一致，未另造解析器。新增三个独立测试源码覆盖投影、扩展保留、清空、有限数拒绝、旧 timeout、缺失及损坏区分，未执行。

依用户本轮要求未运行 Cargo、任何测试或最终回归，不声明编译或运行验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除，建议后续统一验证 openclaw_defaults_projects_fields 过滤场景与事务集成。

## 任务 19：OpenClaw 环境设置（2026-11-14）

新增 openclaw_env::{OpenClawEnvConfig,get_env_config,prepare_env_config,set_env_config}，严格 JSON 对象编辑，保留 vars/shellEnv 嵌套及未知值，拒绝空白、坏 JSON、数组与 null，{} 替换清空。缺失与空对象区分；读取先调用独立敏感授权策略，无 Debug 派生或原文错误。调用方仍须绑定认证范围和执行主机，后续 API 负责提供真实敏感权限策略，不将普通配置读权限冒充敏感授权。发布沿用 OpenClawFiles snapshot/apply 的物理锁、私有原子写和冲突事务，非目标分节字节保留。底层仅支持 JSON/JSONC，完整 JSON5 仍明确拒绝。

静态核对来源 get_env_config/set_env_config、utils.ts、正式合同、任务 3 路径与共享配置事务。新增四个 openclaw_env_preserves_nested_values 前缀测试源码，覆盖嵌套往返、非目标注释、清空、缺失、非法输入和敏感读取拒绝；未执行。按用户本轮要求未运行 Cargo、任何测试或最终回归，不声明编译或行为验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。本编号卡按要求删除；后续统一验证。

## 任务 20：OpenClaw 工具设置（2026-11-14）

新增 openclaw_tools::{get_tools_config,prepare_tools_config,set_tools_config}：profile 保留原字符串并显式区分未设置、minimal/coding/messaging/full 和未知值；allow/deny 只过滤空白项，非空文本与顺序保留；未知扩展随完整分节读写。拒绝损坏分节及扩展覆盖已知字段。配置仅供 OpenClaw 消费，不改变识矩工具授权。发布复用 OpenClawFiles 快照、供应商物理锁及事务冲突检查，调用方先授权配置与执行主机。

静态核对来源 get_tools_config/set_tools_config、ToolsPanel.tsx、任务 1 正式合同、任务 3 文件边界及共享补丁层。新增三个 openclaw_tools_preserves_unknown_profile 前缀测试源码，未运行。共享解析器仅支持 JSON/JSONC，完整 JSON5 仍明确拒绝。本轮依用户要求未运行 Cargo、单元/实机/其它测试或最终回归，不声称编译或验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续统一验证，任务卡按指令删除。

## 任务 21：OpenClaw 配置健康扫描（2026-11-14）

新增 openclaw_health 的配置只读扫描和纯值诊断，迁移 invalid_tools_profile、legacy_agents_timeout、stringified_env_vars、stringified_env_shell_env 四分支；缺文件空列表，解析失败 config_parse_failed，读取失败保留错误。结构化 code/path 与固定 message 不包含环境变量、令牌、未知 profile 或解析器原文。文件读取复用 providers::openclaw::read 的执行主机 HOME 受控边界，不创建目录，不写配置。调用者须先授权配置与主机。现有共享解析器仅支持 JSON/JSONC，完整 JSON5 尚不支持，超出支持范围返回解析警告。

已静态核对任务 1 合同、任务 3 文件边界、来源扫描函数及 HealthBanner。新增三个 openclaw_health_reports_without_mutation 前缀测试源码，覆盖四分支、固定路径、敏感值不输出、合法 profile、缺失/损坏、只读字节与链接拒绝，未执行。依用户本轮要求未运行 Cargo、任何测试或最终回归，不声称验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，后续统一验证。

## 任务 27：Hermes 面板探测（2026-11-14）

新增 macOS hermes_dashboard::probe_dashboard：先拒绝空或不匹配的可信执行主机身份，再读取执行端 HERMES_WEB_PORT，trim 后有效 u16 使用原值，否则 9119。no_proxy、1200ms 总超时，仅探测 127.0.0.1 的 /api/status；任意 HTTP 响应（含 401/500/302）为在线，网络失败为离线，客户端构建失败独立报错。禁用 HTTP 重定向，避免探测越过 loopback。只接受页面路径，拒绝协议、双斜杠 authority、反斜杠、编码分隔符、控制字符、路径穿越及 query/fragment；不读取或注入 session token，不启动浏览器。调用者须先完成项目/动作授权并可信分派；远程 loopback 不可直接当桌面可用地址。

静态阅读来源 commands/hermes.rs::open_hermes_web_ui、TS 调用者、接口边界、任务 1 正式合同与任务 3 主机边界；RustCodeGraph status/explore 已读取，宽查询截断不作为不存在的证明。新增三个 hermes_probe_distinguishes_offline 前缀独立测试源码，覆盖端口回退、页面拒绝、本地 HTTP 状态、无 Authorization 请求头、超时、连接失败及主机拒绝，均未执行。

遵照本轮最高优先要求，未执行任何 Cargo、单元测试、实机或其它测试，未启动回归，不声称编译或行为验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除，后续统一验证上述测试族。


## 任务 16 补齐：共享 JSON5 解析与 round-trip（2026-11-14）

本条取代上文任务 16 的 JSON5 缺项结论。新增 providers/live/patch_json5：复用既有叶节点协调算法，以 json-five 0.3.1 的 JSON5 tokenizer/serde 解码定位原文 byte spans，只替换变化值，不通过 AST formatter 重印文件。因此单引号、裸键/转义键、Unicode 空白、行续接、十六进制、小数、Infinity/NaN、注释及尾逗号等非目标原文保留；解码后重复键仍拒绝，原有 1MiB 输入边界和解析深度限制保留。JSONC 通用模块合同不变。

OpenClaw model/catalog/defaults/env/tools、provider 配置及 health 统一改用共享 JSON5 模块，配置发布仍复用 OpenClawFiles/ProviderTransactions，未新增写入所有者。新增独立 JSON5 测试源码并调整旧裸键拒绝断言；只读审查了来源配置函数及 json-five 0.3.1 原始 tokenizer/parser/de 源码，未运行任何测试、Cargo 或实机检查。不声称编译或行为验证通过。新增轻量解析依赖；按本轮禁令未调用 Cargo 更新 Cargo.lock，后续统一验证阶段需先解析依赖更新锁文件。未领取槽位，CARGO_TARGET_DIR 未设置。


## 任务 7：每日记忆列表元数据（2026-11-14）

新增 daily_memory_list::list_daily_memory_files，复用任务 5 的日期 DTO 和任务 3 的 WorkspaceDirectory；仅合法公历日期普通单链接文件参与列表，按日期降序，返回字节大小、Unix 秒修改时间与前 200 个 Unicode 字符。缺目录空列表且不创建，非法名字、符号/硬链接及特殊文件不读，读取和枚举错误显式返回。底层新增 descriptor 相对目录枚举与普通文件打开，逐级 nofollow 和执行主机校验保持；内容与元数据来自同一打开文件。

静态核对来源 list_daily_memory_files、调用者、接口边界、正式合同与任务 5 源码；新增三个 daily_list_preserves_metadata 前缀独立测试源码，覆盖日期排序、中文 emoji 预览、元数据、空文件、无效日期、链接、目录、缺失、主机拒绝与非法 UTF-8。按本轮要求未运行 Cargo、任何测试或最终回归，不声明编译或验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按指令删除；后续统一验证。


## 任务 6：每日记忆幂等删除（2026-11-14）

新增 daily_memory_files::delete_daily_memory_file，复用合法公历 DailyMemoryFilename 和 WorkspaceDirectory::delete。缺目录不创建、缺文件和重复删除成功；执行主机校验、不跟随链接、普通单链接文件限制、物理锁及 unlinkat/目录同步错误传播沿用底层边界。调用者须先授权内容写入并提供可信执行主机、HOME 与解析后的工作区根，不新增权限或路由。

静态读取来源删除函数、workspace.ts、DailyMemoryPanel 调用者、任务 1 正式合同、任务 5 读写和底层实现。新增两个 daily_delete_is_idempotent 前缀测试源码，覆盖缺失、重复删除、邻文件保留、主机拒绝、目录与越界链接拒绝；非法日期复用既有日期类型断言。未运行任何测试，没有权限失败运行证据，不声明编译或行为通过。

依本轮最高优先要求未运行 Cargo、单元/实机/其它测试或最终回归；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，建议后续统一验证。

## 任务 12：Hermes 记忆预算与开关（2026-11-14）

新增 hermes_memory_limits 的 read_memory_limits/prepare_memory_enabled/set_memory_enabled，消费现有 HermesMemoryKind/HermesMemoryLimits DTO；预算默认 2200/1375，两个开关默认 true，单字段无效回退，损坏 YAML 明确返回不含载荷的错误。user 映射 user_profile_enabled。编辑复用 patch_yaml 的既有 block mapping 补丁算法，仅增加限定两开关的内部入口，保留预算、外部 provider、未知字段和非目标文本；复杂 flow/reference/tag 布局拒绝修改。发布复用 HermesFiles snapshot/apply 与供应商物理锁、冲突事务，不另建 IO 所有者。调用者须先授权配置和执行主机。

已静态核对来源 hermes_config.rs、任务 1 合同、任务 11 双记忆文件及现有 DTO、YAML 补丁和 Hermes 配置事务。新增三个 hermes_toggle_preserves_yaml 前缀测试源码，覆盖文本保留、双开关隔离、默认/无效字段、缺失和损坏/歧义拒绝；均未执行。依用户最高优先要求未运行 Cargo、任何测试或最终回归，不声称编译或验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除；建议后续统一验证该测试族及配置事务集成。


## 任务 28：Hermes 面板启动动作（2026-11-14）

静态核对任务 27 的 hermes_dashboard::probe_dashboard、来源 launch_hermes_dashboard / hermes_dashboard_command_line 和现有执行主机 command、Operation 取消/进度底座。新增固定 probe/start 及 operations/{id} GET/DELETE；SDK 同步四个动作，保留 HTTP 状态。授权策略在现有 launch 外另需 hermes_dashboard 项目 ID 列表；每次请求重验租户、主机、项目及令牌，任务观察/取消再匹配提交人。远程主机由既有策略拒绝，不把远程 loopback 当本地地址。固定 Hermes dashboard argv，继承底座环境隔离，无客户端命令或 shell 插值；运行进度不宣称 HTTP 在线。取消杀死自有进程组并回收后再确认。

新增权限拒绝与 SDK 路径注入测试源码，均未执行。按本轮最高优先要求未运行 Cargo、单元/实机或其他测试、最终回归，不声称编译或验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按指令删除；后续统一验证。

## 任务 8：每日记忆中文全文搜索（2026-11-14）

新增 daily_memory_search::search_daily_memory_files，复用 WorkspaceDirectory 的执行主机、描述符相对读取及合法普通文件边界；使用现有 lifecycle::operations::Operation 取消信号，分块读取和遍历时检查取消，取消返回独立错误、不返回部分结果。日期及全文小写匹配、非重叠内容计数、日期降序，首处上下文按原文字符生成；日期单独命中返回前 120 字符。小写展开的字节位置映射回原文字位置，不建立持久原文索引。本领域 API 的授权与任务终态仍由后续路由任务接线，不冒用供应商操作。

静态核对任务 1 合同、任务 7 列表、来源 workspace.rs 与 DailyMemoryPanel.tsx。新增三个 daily_search_handles_unicode_offsets 前缀测试源码，覆盖中文/emoji、İ 展开、计数、日期摘要与排序、无效日期/链接/目录跳过、缺失无副作用、主机拒绝、无效 UTF-8 与预取消；均未运行。本轮按用户要求未执行 Cargo、任何测试或最终回归，不声称编译或验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，建议后续统一验证搜索测试族及作用域任务接线。

## 任务 39：桌面 Hermes Web 入口（2026-11-14）

应用详情复用现有目录、项目/租户/主机/令牌上下文，通过 hermes_dashboard SDK 探测、确认启动、查询进度及取消；服务端确认在线后才打开其返回的 HTTP(S) loopback URL。远程网关或非本地主机明确拒绝，无 localhost 回退。异步响应校验上下文、项目、主机、目录代次与请求代次，切换连接失效；启动后持续重新探测，轮询有界，操作 ID 保留供查询或取消。无深层配置编辑器、命令转发或新权限。

静态核对来源 useHermes.ts、api/hermes.ts、App.tsx，接口边界、来源映射及任务 1 正式合同索引，任务 28 正式条目和实际 API/SDK。依用户本轮要求仅写代码和只读审查，未执行 Cargo、任何测试或最终回归，不声称编译、渲染或行为通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除。建议后续统一验证离线重试、取消、过期响应、权限拒绝与明暗主题实际渲染。

## 任务 14：Hermes 记忆与模型路由（2026-11-14）

新增固定 memory/{kind} GET/PUT、memory-limits GET、memory/{kind}/enabled PUT 和 model GET。注册沿现有 Hermes dashboard 的 agent_apps 子模块入口；未增加 model 写端点。新增管理员策略 hermes_memory_read/hermes_memory_write 项目 ID 白名单，复用认证租户、令牌与主机分派，默认拒绝；写权限不隐含读权限。双文件复用 HermesMemory，配置读写复用 Hermes 原生 reader、HermesFiles 与 ProviderTransactions；开关提交前重验授权并保留恢复/冲突状态。供应商领域补充 model_snapshot 白名单投影，排除凭据和 URL。

静态核对任务 1 合同、任务 12/13 产物及来源 get_hermes_model_config；新增 hermes_api_isolates_memory 测试源码，覆盖无权限、跨项目/租户/主机拒绝、真实双记忆和 YAML 回读、模型敏感字段排除。按本轮要求未运行 Cargo 或任何测试，不声明编译或验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除；建议统一验证上述测试族和 HTTP 集成。

## 任务 15：Hermes 记忆客户端 SDK（2026-11-14）

新增 `hermes_memory_api.rs`，公开 HermesMemoryScope/HermesMemoryError；封装任务 14 的 GET/PUT memory/{kind}、GET memory-limits、PUT memory/{kind}/enabled 与只读 GET model。项目与执行主机通过编码查询参数传递，租户沿用 endpoint 上下文，独立授权令牌标记敏感并使用禁止重定向的 native_transport。保留 exists、中文全文、备份 outcome、空模型及 HTTP 状态；错误不包含原始内容/令牌。开关成功后调用方可同作用域刷新限制，写失败直接返回错误。

已静态核对来源 hermes.ts/useHermes.ts、任务 1 正式合同、接口边界、任务 14 已提交路由与 DTO。新增三个 hermes_sdk_preserves_outcome 前缀测试源码，覆盖请求方法/路径/查询/租户/令牌/body、缺失与空文件、长文本不截断、备份、刷新限制、模型缺失与存在、拒绝/冲突/IO 状态、解码失败和传输失败。本轮按用户要求未运行 Cargo 或任何测试，不声明编译或行为验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。建议后续统一运行该过滤测试。本编号任务文件按要求删除。

## 任务 9：工作区领域授权路由（2026-11-14）

固定接入九文件 GET/PUT、每日列表和 GET/PUT/DELETE、POST search 与 GET/DELETE search/{id}。真实认证租户、令牌、项目及目标主机经既有 access dispatcher 核验后才进入 IO；新增 workspace_read/workspace_write 项目白名单，默认拒绝，供应商和 Hermes 权限不隐含内容访问。根目录沿来源 workspace.rs 固定为可信执行主机 HOME/.openclaw/workspace，不接受客户端路径或远程本机回退。注册沿已有 agent_apps 子模块入口，未复制领域实现。

搜索复用 Operation，绑定租户/项目/主机/提交人，查询和取消重新授权，取消等待阻塞读取退出后回收任务，不返回部分结果；最多保留 128 个任务，GET 保留结果供重试，DELETE 释放。缺失文件返回 null，空文本保留；路径、IO 和授权错误返回非成功状态，不含原文。新增 workspace_api_denies_foreign_host_and_project_without_io 测试源码，未执行。

静态读取任务 1 正式合同、接口边界、来源 workspace.rs、任务 2/4/6/7/8 生产类型及实现和相邻 Hermes 路由/授权。按本轮要求未运行 Cargo、任何测试或最终回归；没有编译或行为验证通过结论。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除；后续统一验证权限拒绝无 IO、HTTP 路由和取消竞争。

## 任务 22：OpenClaw 设置接口类型（2026-11-14）

新增 openclaw_settings 的 model/catalog/defaults/env/tools/health/WriteOutcome DTO，保留分节扩展、未知 profile、缺失与空对象及结构化警告和 backupPath。defaults/env 沿用领域对象形状；不新增路径或命令入口，env 独立敏感读取权限明确为调用者合同，载荷不派生 Debug。来源 WriteOutcome 为结构化 warnings，现有领域字符串 warnings 由后续路由适配。

静态阅读任务 1 合同、接口边界、来源映射、来源 commands/openclaw.rs、openclaw.ts、types.ts 与现有领域源码，读取 RustCodeGraph status/explore。新增三个 openclaw_wire_preserves_extra_fields 前缀测试源码，未执行。按用户要求未运行 Cargo 或任何测试，不声明编译或验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，后续统一验证。

## 任务 22：OpenClaw 设置接口类型（2026-11-14）

新增 openclaw_settings 的 model/catalog/defaults/env/tools/health/WriteOutcome DTO，保留分节扩展、未知 profile、缺失与空对象及结构化警告和 backupPath。defaults/env 沿用领域对象形状；不新增路径或命令入口，env 独立敏感读取权限明确为调用者合同，载荷不派生 Debug。来源 WriteOutcome 为结构化 warnings，现有领域字符串 warnings 由后续路由适配。

静态阅读任务 1 合同、接口边界、来源映射、来源 commands/openclaw.rs、openclaw.ts、types.ts 与现有领域源码，读取 RustCodeGraph status/explore。新增三个 openclaw_wire_preserves_extra_fields 前缀测试源码，未执行。按用户要求未运行 Cargo 或任何测试，不声明编译或验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，后续统一验证。

## 任务 23：OpenClaw 模型、目录与默认设置路由（2026-11-14）

新增固定 GET/PUT `/v1/agent-apps/openclaw/settings/{model,catalog,defaults}`；查询使用 projectId/targetHostId，沿用租户上下文及 x-agent-apps-token。独立 `openclaw_settings_read/write` 项目授权在 IO 前检查，写入复用 OpenClawFiles/ProviderTransactions 与提交前授权复核；恢复/冲突返回 409，未知字段由底层分节保留，写警告映射到公开 outcome。注册沿用相邻 Hermes 在 agent_apps/mod.rs 中的路由所有者，未复制 import 或 provider 存储。

已静态核对任务 1 正式合同、任务 16/17/18/22 生产接口、来源 commands/openclaw.rs 与 useOpenClawModelOptions.ts。现有 `/app-providers/openclaw/config/state` 和供应商目录保留原只读所有权；模型选项专用去重/名称回退/中文排序投影仍是前置合同中的缺项，本任务未伪造该能力。新增两个 openclaw_api_roundtrips_defaults 前缀测试源码，均未运行。本轮依用户指令未运行 Cargo、任何测试或最终回归，不声明编译或行为通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除；建议后续统一验证 HTTP 权限隔离与配置事务。

## 任务 24：OpenClaw 环境、工具与健康路由（2026-11-14）

扩展现有 openclaw_settings 路由：GET/PUT env、GET/PUT tools、只读 GET health。env 先校验普通 settings_read，再校验独立项目级 openclaw_env_sensitive_read，授权拒绝发生在读取配置前；写入复用 settings_write、OpenClawFiles 和 ProviderTransactions。health 仅返回领域固定脱敏警告；没有新增模型选项投影或桌面排序。

静态核对任务 1 合同、来源映射、commands/openclaw.rs、utils.ts 和任务 19/20/21/23 真实产物。新增 openclaw_api_protects_sensitive_env 测试源码，覆盖普通读权限拒绝 env、拒绝无目录副作用、独立授权读原值及 health 脱敏不改文件，未执行。按用户最高优先要求未运行 Cargo、任何测试或最终回归，不声明编译或验证通过；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，建议后续统一验证该测试族与 HTTP/事务集成。

## 任务 25：OpenClaw 设置客户端 SDK（2026-11-14）

新增 `openclaw_settings_api`，五个固定分节 model/catalog/defaults/env/tools 的 GET/PUT 与 health GET 使用任务 23–24 实际路由和任务 22 DTO；公开 OpenClawSettingsScope、OpenClawSettingsError。projectId/targetHostId 查询与端点租户头共同传递作用域，敏感凭证头使用原生禁止重定向/重试传输。保留 null、空对象、未知 profile、扩展字段、backupPath 与结构化 warnings；HTTP 状态错误单独保留，错误不含载荷或凭证。供应商读取继续使用已有 app_provider_get/openclaw_state，模型选项使用 model_catalog，不另造业务所有者。

静态核对来源 openclaw.ts/useOpenClaw.ts、正式 workspace-contracts、现有路由与相邻 SDK。新增三个 openclaw_sdk_preserves_warnings 前缀测试源码，覆盖分节往返、缺失、扩展、警告、方法/路径/租户/凭证/请求体、拒绝/冲突/解码及传输错误；仅编写，未运行。按本轮要求未执行 Cargo、任何测试或最终回归；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置，无编译或测试通过结论。任务卡删除，后续统一验证。

## 任务 10：工作区客户端 SDK（2026-11-14）

封装任务 9 已存在的固定 workspace files GET/PUT、daily 列表及 GET/PUT/DELETE、search POST 和 search/{id} GET/DELETE。使用真实 workspace_memory DTO，公开 WorkspaceMemoryScope/WorkspaceMemoryError 与绑定提交客户端及作用域的 WorkspaceSearch；取消请求前禁止本地后续 poll，取消失败可重试，不返回取消响应中的结果。项目/目标主机由 reqwest 编码查询传递，租户沿端点上下文，授权头标为敏感并复用禁止重定向/重试的原生传输。保留 null/空文本及 HTTP 状态，204 写入不尝试 JSON 解码；不开放任意路径。目录动作不在任务 9 路由内，仍由任务 26 所有。

静态核对任务 1 正式合同、接口边界、来源映射、workspace.ts、相邻 desktop/Hermes SDK 测试及任务 9 真实路由。新增 workspace_sdk_preserves_scope 前缀测试源码，覆盖方法/路径/编码/租户/令牌/body、空与缺失、中文、HTTP 拒绝与解码失败、搜索提交/查询和取消失败后停止接收。按用户本轮要求未运行 Cargo、单元/实机或其它测试及最终回归，不声明编译或验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除，建议后续统一验证该过滤测试和正式 Gateway 集成。

## 任务 32：桌面 Hermes 双记忆草稿（2026-11-14）

新增 Hermes 双记忆编辑入口，memory/user 独立草稿、首次水合保留编辑、切标签保留、保存快照确认及失败保留；离开详情、设置页、切换连接或主机时清空并以作用域和代次丢弃旧响应。读取路径由授权执行主机返回可选 path，不推测本机 HOME；未返回时明确提示。复用统一编辑器、按钮及 toast，中文使用默认字体。新增 hermes_drafts_preserve_inflight_edits 两个独立测试源码。

已静态核对任务 15 SDK、任务 1 合同、来源 HermesMemoryPanel/hermesMemoryDrafts 及相邻测试。按本轮要求未运行 Cargo、任何测试或最终回归，不声称编译、渲染或行为验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除；建议后续统一验证上述测试族、旧响应及实际渲染。

## 任务 34：桌面 OpenClaw 模型选择（2026-11-14）

新增 `components/agent_apps/openclaw_model.rs`，从应用详情打开统一设置弹窗；调用任务 25 的 model/catalog/health SDK 与 model PUT。读取失败不开放保存；当前未配置值保留；保存排除 primary、空值和重复 fallback，保留 model 扩展字段。写警告与只读健康提示在弹窗展示，错误使用 toast。网关、租户、项目、执行主机、目录与请求代数共同拒绝过期响应，关闭和切换连接使请求失效。配置 JSON5 原文仍由底层分节事务维护。

静态核对发现母 5 的供应商 provider/model 专用选项投影尚未提供；本次只消费 SDK 白名单键与 alias，不在桌面复制供应商解析或中文排序。白名单以外的当前值显示“未配置”。供应商全集与中文排序等价性仍需后续补齐投影。

补充独立行为测试源码两个场景（未配置值、扩展字段与回退去重），未执行。按本轮最高优先指令，没有运行 Cargo、单元/实机或其它测试、最终回归；没有领取槽位，CARGO_TARGET_DIR 未设置。仅源码审查，不宣称编译或验收通过。任务卡按指令删除，后续统一验证。

## 任务 32：桌面 Hermes 双记忆草稿（2026-11-14）

新增 Hermes 双记忆编辑入口，memory/user 独立草稿、首次水合保留编辑、切标签保留、保存快照确认及失败保留；离开详情、设置页、切换连接或主机时清空并以作用域和代次丢弃旧响应。读取路径由授权执行主机返回可选 path，不推测本机 HOME；未返回时明确提示。复用统一编辑器、按钮及 toast，中文使用默认字体。新增 hermes_drafts_preserve_inflight_edits 两个独立测试源码。

已静态核对任务 15 SDK、任务 1 合同、来源 HermesMemoryPanel/hermesMemoryDrafts 及相邻测试。按本轮要求未运行 Cargo、任何测试或最终回归，不声称编译、渲染或行为验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除；建议后续统一验证上述测试族、旧响应及实际渲染。

## 任务 36：桌面 OpenClaw 代理默认表单（2026-11-14）

已静态核对任务 25 SDK、任务 1 合同、来源 AgentsDefaultsPanel / 模型选项 hook / HealthBanner 与底层 defaults 表单投影。应用详情新增统一设置弹窗，编辑 workspace/timeoutSeconds/contextTokens/maxConcurrent；匹配领域投影与有限数字规则，保留 model/catalog 和未知字段，空值清除字段，数字旧 timeout 别名按已有规则处理，未知旧数据保留。读取失败允许重试，保存失败保留草稿；字段错误可见，健康与写入警告展示，连接/主机/项目/请求代数使旧响应失效。配置发布沿用 SDK 和执行端分节事务，不新增本地 IO。

新增两个独立测试源码，覆盖字段保留、清空及非法数字；依本轮要求未运行 Cargo、单元测试、实机测试或最终回归，不声称编译或行为验证通过。只格式化新增 Rust 文件。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除；建议后续统一验证表单过滤场景、暗黑主题和中文实际渲染。

## 任务 37：桌面 OpenClaw 环境表单（2026-11-14）

新增 `components/agent_apps/openclaw_env.rs`，应用详情打开统一 env JSON 弹窗；复用任务 25 的专属 SDK，支持 vars/shellEnv 嵌套对象。空白、数组、null 与非法 JSON 明确拒绝；本地校验或服务端拒绝均保留草稿和错误。敏感读取 401/403 显示不可访问，未读取成功不能保存；错误不含响应正文、解析片段或敏感值，事件 Debug 不暴露草稿。保存成功保留结构化警告，toast 提示；关闭、连接切换、详情切换清空状态，作用域和请求代数拒绝旧响应。复用统一编辑器及弹窗，无中文等宽字体。

已静态核对来源 EnvPanel、ToolsPanel、utils 测试、任务 1 正式合同及任务 25 SDK/DTO 和现有模型弹窗。独立测试源码覆盖失败保稿、非对象拒绝、成功警告及权限提示，仅编写，未运行。按用户最高优先要求未执行 Cargo、单元测试、实机或任何其它测试及最终回归，不声明编译、渲染或验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除，建议后续统一验证测试族、过期响应和明暗主题实际渲染。


## 任务 35：桌面 OpenClaw 模型白名单（2026-11-14）

- 静态核对任务 25 的 `openclaw_settings_catalog` / `set_openclaw_settings_catalog` 及强类型 DTO；沿用应用详情导航、项目/主机/网关上下文和过期响应隔离。
- 新增 catalog 独立弹窗，编辑 JSON 中 alias 与任意额外字段，缺分节明确提示，`{}` 可显式保存；失败保留草稿，保存只使用 catalog SDK，不写默认模型。使用统一设置编辑器及弹窗、默认字体。
- 添加两个 `openclaw_catalog_editor_preserves_aliases` 前缀测试用例，尚未执行。按本轮最高优先指令，未运行 Cargo、单元测试或实机验证；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。不宣称编译、行为测试或 macOS 验收通过，后续统一验证。

## 任务 38：桌面 OpenClaw 工具表单（2026-11-14）

静态核对任务 1 合同、任务 25 tools GET/PUT SDK 与 DTO、来源 ToolsPanel/EnvPanel/utils 测试及现有设置弹窗。新增工具弹窗：四个已知 profile、未设置和未知值分别展示，未知原值及扩展字段保留；allow/deny 可增删编辑，保存过滤空白条目，失败保留草稿，成功展示结构化写入警告。使用统一表单与可滚动弹窗、默认字体及 toast；连接、主机、详情切换及关闭使旧请求失效。

补充三个独立测试源码，覆盖 profile 区分、未知值/扩展保留、空白条目过滤和失败保稿及写入警告，均未执行。按本轮用户要求未运行 Cargo、任何测试或最终回归，不宣称编译、渲染或行为验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除；建议后续统一验证上述测试族及暗黑主题中文实际渲染。

## 任务 26：工作区目录主机动作（2026-11-14）

仅静态源码审查，未执行 Cargo、单元测试、实机测试或最终回归；未领取槽位，CARGO_TARGET_DIR 未设置。

新增 POST `/v1/agent-apps/openclaw/workspace/open-directory`，沿用 projectId/targetHostId 查询上下文与租户、令牌认证。body 仅接受 `{"subdir":"workspace"}` 或 `{"subdir":"memory"}`，未知字段/目录被 serde 拒绝。要求 workspace_write 项目权限；离线返回 503、远程绑定返回 501，均不回退客户端路径。安全目录所有者以授权执行 HOME 为根，descriptor 相对创建固定目录并拒绝链接；系统 opener 在持有目录描述符对应 cwd 中打开 Finder，失败返回 500。SDK `open_workspace_directory` 复用作用域和错误传递；取消等待不能撤销已接收的系统动作。

注册复用既有 agent_apps 路由所有者，没有重复领域服务。后续统一验证应覆盖权限拒绝、两种目录、符号链接、远程/离线、SDK 上下文与 macOS Finder 实际行为；本文不声称任何运行验证通过。

## 任务 33：桌面 Hermes 预算与开关（2026-11-14）

扩展任务 32 双记忆编辑器，预算/双开关由 SDK 读取，按 UTF-16 单元展示剩余或超限，不截断原文；禁用仍可编辑并提示跳过记忆。开关写入成功后同作用域刷新，失败保留原状态并显示脱敏 SDK 错误及 toast；刷新失败明确提示已保存但状态待刷新。模型仅展示供应商领域快照及修改入口说明。复用统一控件、默认字体与既有页面作用域/代次校验，离页或切换连接使旧响应失效。

静态读取来源 HermesMemoryPanel/hermesMemoryDrafts/相邻测试、任务 1 正式合同、任务 32、SDK/DTO 和统一设置控件；新增两个 hermes_budget_matches_utf16_count 前缀测试源码，覆盖中文、emoji、组合字符、零预算及超限保留。按用户最高优先要求未运行 Cargo、任何测试或最终回归，不声明编译、渲染或验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除；建议后续统一验证该测试族、开关失败/刷新及实际明暗渲染。
