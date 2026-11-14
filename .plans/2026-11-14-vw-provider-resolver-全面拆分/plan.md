# 提供商解析全面拆分计划

目标：按源码职责拆出独立库，原包保留发布门面；本计划生成不代表代码已拆分。

范围：vw-provider-resolver 全部源码、测试、资源、平台分支及直接消费者。

范围外：功能修复、依赖升级、构建测试及实机验证。

假设：保持 0.6.0、edition 2024、MIT；前序领域计划尚未实施，不把计划库当作现有接口。

## 设计决策

- 采用领域库；整体搬包保留耦合，仅拆文件不能隔离依赖，逐函数建库增加无用边界。
- 纯类型已经位于 vw-shared，优先复用其模型/协议/认证契约，不另造 resolver-types。
- 全部批次串行，共享 Cargo.toml 与消费者有写冲突。

## 架构说明

依赖方向、逐文件归属和直接消费者见 [审计](audit.md) 与 [引用清单](consumer-refs.md)。基础库不得依赖原门面或解析组装库；共享静态状态各保留一个所有者。

## 开发策略

保持行为的结构迁移，保留并迁移既有断言，不做行为修复。本次用户覆盖要求禁止测试驱动运行；发现问题记录源码证据，不宣称动态通过。

## 公共执行约定

- 每卡先读本文件与对应卡，确认前置任务实际落地；本文件生成后只读，不记录进度或删除。prompt.md 是固定入口，已删除卡为历史索引。
- 使用 /Users/xiangmin/.codex/skills/do-task-plans/SKILL.md；导航遵循 skills/rustcodegraph/SKILL.md，索引未覆盖或过时用 rg，不启动 Cargo 更新索引。
- 常规可逆事项自主推进；先查调用者、工厂和测试，不覆盖已有工作区变更。局部困难先查根因，仅关键歧义或新增授权集中询问，不将沉默视为批准。
- 每个新库创建 crates/libs/<任务指定名称>/Cargo.toml 与 src/lib.rs；包名与目录同名、0.6.0、edition 2024、MIT，按根工作区既有模式登记 members，并修改直接消费者清单、调用路径。依赖版本/features/target cfg 按原配置保留，路径按新目录层级重算；只公开实际跨库调用。
- 前序计划依赖须先核实产物：shared 任务 11—17 的 model-protocols、model-reasoning、model-catalog-types、model-types、model-state、auth-types、auth-file-store；config 任务 1/29 的 config-runtime-paths/config-aggregate。未落地时继续使用当前 vw-shared/vw-config-types 的真实导出，记录临时边，不创建同义库。前序计划实施时再接其实际导出。

## 公共验证与资源规则

- 用户明确覆盖仓库和技能默认要求：只写代码，不运行任何 Cargo 命令（含 metadata/fmt/check/test/clippy），也不间接执行；不运行单元、脚本、GUI 或实机测试，不领取构建槽位。
- 只做静态读取、rg、git diff --check、git diff --stat 及模块/清单/资源路径人工核对。Cargo.lock 不修改，留待以后授权的统一验证生成。
- 后续验证未验证：每张卡所列行为、迁移测试及相关 crate/原生与 wasm 目标编译，未来另行授权。届时 Rust 重型编译串行，遵循 scripts/cargo_slot.py 仓库入口；本计划不安排自动验证任务。

## 公共收尾规则

- 状态仅用未开始、已完成、已阻塞、待回归。已完成表示迁移与静态审查完成，不表示编译/测试/性能通过；不因用户延后动态验证留卡。
- 每卡有实际代码、配置或文档变更，使用 /Users/xiangmin/.codex/skills/git-commit/SKILL.md 先提交，中文说明明确“未运行 Cargo、测试及实机验证”，只暂存本任务内容。没有实际变更不建空提交。
- 提交成功后删除对应编号任务卡，plan.md、prompt.md 保留；提交失败保留卡并记录证据。关键阻塞调查无解时保留卡，记录尝试、所需输入和未验证项，已有变更仍须提交。
