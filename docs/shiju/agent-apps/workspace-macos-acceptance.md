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
