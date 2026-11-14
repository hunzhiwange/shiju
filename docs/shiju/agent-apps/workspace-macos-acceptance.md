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
