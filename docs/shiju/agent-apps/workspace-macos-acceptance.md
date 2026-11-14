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

## 任务 40：其他应用专属入口（2026-11-14）

已完成 AGY/DSH/MiniMax/Pi/OMP 六类能力的静态入口核对，详见 [workspace-special-entrypoints.md](workspace-special-entrypoints.md)。来源命令→唯一领域→目标供应商 API/SDK/Iced 接点、原生提示词路径及尚未核实链路均已登记。特别保留 AGY 绑定 quota、DSH 五授权动作、MiniMax/Hermes key、Pi usage/discovery、OMP builtin default 的专属语义；五应用提示词/会话均有来源，未误写不适用。

只读证据为 RustCodeGraph status/explore（退出 0）及实际源文件、路由、SDK 和历史证据文档审查。完整运行链和错误路径缺项保持明确，不重签历史验收。本轮依用户最高优先要求未运行 Cargo、单元测试、实机或其他测试，也未启动最终回归；无新增运行通过结论。静态盘点完成后删除任务 40 卡，后续按文档缺项统一验证。

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


## 任务 16：OpenClaw 默认模型分节（2026-11-14）

新增 openclaw_model 的 get_default_model/prepare_default_model/set_default_model，复用 OpenClawFiles snapshot/apply 与物理锁、冲突检查和原子事务。保留未知扩展、回退顺序及重复值；结果包含 warnings。已有 JSONC round-trip 支持用于保留注释和非目标字节；底层明确拒绝完整 JSON5（单引号、无引号键），此限制已留源码 TODO，不能宣称 JSON5 完整交付。

静态读取来源默认模型函数、前置正式合同、任务 3 路径边界、供应商事务和 patch_json；新增两个独立测试源码，覆盖字节保留、回退顺序、未知字段、幂等、缺失和非法形状。本轮按用户要求未运行 Cargo、测试或实机验收，不声明验证通过；未领取槽位，CARGO_TARGET_DIR 未设置。后续建议补齐共享 JSON5 round-trip 后统一验证。依指令删除任务卡。

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


## 任务 6：每日记忆幂等删除（2026-11-14）

新增 daily_memory_files::delete_daily_memory_file，复用合法公历 DailyMemoryFilename 和 WorkspaceDirectory::delete。缺目录不创建、缺文件和重复删除成功；执行主机校验、不跟随链接、普通单链接文件限制、物理锁及 unlinkat/目录同步错误传播沿用底层边界。调用者须先授权内容写入并提供可信执行主机、HOME 与解析后的工作区根，不新增权限或路由。

静态读取来源删除函数、workspace.ts、DailyMemoryPanel 调用者、任务 1 正式合同、任务 5 读写和底层实现。新增两个 daily_delete_is_idempotent 前缀测试源码，覆盖缺失、重复删除、邻文件保留、主机拒绝、目录与越界链接拒绝；非法日期复用既有日期类型断言。未运行任何测试，没有权限失败运行证据，不声明编译或行为通过。

依本轮最高优先要求未运行 Cargo、单元/实机/其它测试或最终回归；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，建议后续统一验证。

## 任务 7：每日记忆列表元数据（2026-11-14）

新增 daily_memory_list::list_daily_memory_files，复用任务 5 的日期 DTO 和任务 3 的 WorkspaceDirectory；仅合法公历日期普通单链接文件参与列表，按日期降序，返回字节大小、Unix 秒修改时间与前 200 个 Unicode 字符。缺目录空列表且不创建，非法名字、符号/硬链接及特殊文件不读，读取和枚举错误显式返回。底层新增 descriptor 相对目录枚举与普通文件打开，逐级 nofollow 和执行主机校验保持；内容与元数据来自同一打开文件。

静态核对来源 list_daily_memory_files、调用者、接口边界、正式合同与任务 5 源码；新增三个 daily_list_preserves_metadata 前缀独立测试源码，覆盖日期排序、中文 emoji 预览、元数据、空文件、无效日期、链接、目录、缺失、主机拒绝与非法 UTF-8。按本轮要求未运行 Cargo、任何测试或最终回归，不声明编译或验证通过。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按指令删除；后续统一验证。


## 任务 6：每日记忆幂等删除（2026-11-14）

新增 daily_memory_files::delete_daily_memory_file，复用合法公历 DailyMemoryFilename 和 WorkspaceDirectory::delete。缺目录不创建、缺文件和重复删除成功；执行主机校验、不跟随链接、普通单链接文件限制、物理锁及 unlinkat/目录同步错误传播沿用底层边界。调用者须先授权内容写入并提供可信执行主机、HOME 与解析后的工作区根，不新增权限或路由。

静态读取来源删除函数、workspace.ts、DailyMemoryPanel 调用者、任务 1 正式合同、任务 5 读写和底层实现。新增两个 daily_delete_is_idempotent 前缀测试源码，覆盖缺失、重复删除、邻文件保留、主机拒绝、目录与越界链接拒绝；非法日期复用既有日期类型断言。未运行任何测试，没有权限失败运行证据，不声明编译或行为通过。

依本轮最高优先要求未运行 Cargo、单元/实机/其它测试或最终回归；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。任务卡按要求删除，建议后续统一验证。
