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
