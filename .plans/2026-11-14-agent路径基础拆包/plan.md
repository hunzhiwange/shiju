# agent 路径基础拆包计划

目标：迁出只读 Git 发现及项目存储目录派生，给记忆后端提供不依赖 agent 的路径基础。

范围：`crates/libs/git-discovery`、`crates/libs/project-storage-paths` 及 agent 原入口；不迁移项目上下文或完整 worktree 功能。

范围外：不改 Git 写操作、用户目录规则、缓存清理、全局路径初始化或数据库。

假设：前置四包基础计划已有当前完成证据；原目录派生可原样保持。

## 设计决策

保留 Git 摘要与路径派生两个包：前者可单独使用，后者允许依赖已有 vw-config-types 常量；不将两者塞进 shared。

## 架构说明

agent/记忆后端 → project-storage-paths → git-discovery。agent 的原 git_repository 与 memory::paths 为薄导出。

## 开发策略

两个迁移串行，测试集及目录语义不变。对业务行为无改动，不制造失败红灯；实际边界缺陷按总计划先红后绿。

## 公共执行约定

必须继续阅读并遵守 [总计划公共执行、验证、Rust 资源与收尾规则](../2026-11-14-vw-agent大规模拆包/plan.md)。本文件与 prompt.md 只读，具体状态和证据记入编号任务。直接调用原生 Cargo、共享 10 槽位完整生命周期、Rust 重编串行、逐任务定向验证及有实际变更才提交的规则均从总计划继承。
