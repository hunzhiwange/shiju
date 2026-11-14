# agent 记忆后端拆包计划

目标：迁出记忆契约和七个存储实现，使后端测试不再依赖完整 agent 运行时。

范围：memory-types（traits、backend、none、EmbeddingProvider/NoopEmbedding）、Markdown、SQLite、PostgreSQL、MariaDB、Lucid、响应缓存、快照八包及生产工厂接线。

范围外：不迁 HTTP 嵌入实现、Qdrant、hygiene、CLI、迁移工厂，不改变数据目录、SQL、序列化、向量编码及现有 TLS 策略。

假设：基础四包、路径基础和安全计划有完成证据；使用前置 memory-algorithms 和 project-storage-paths。

## 设计决策

先契约后实现；HTTP provider/工厂继续在 agent，通过原 trait 注入后端。不新增配置/trait 方法；保留后端原构造器，避免大改调用方。

## 架构说明

agent 工厂 → 各后端 → memory-types/路径/算法；Lucid → memory-sqlite。后端不依赖 agent/shared，只有路径包使用配置常量。

## 开发策略

八个迁移串行。通过新包内部 `memory` 命名空间保留内部相对模块关系；业务入口薄导出。只调整跨包 use，不创建 app::agent 镜像或额外兼容协议。

## 公共执行约定

必须继续阅读并遵守 [总计划公共执行、验证、Rust 资源与收尾规则](../2026-11-14-vw-agent大规模拆包/plan.md)。本文件与 prompt.md 只读，具体状态和证据记入编号任务。直接调用原生 Cargo、共享 10 槽位完整生命周期、Rust 重编串行、逐任务定向验证及有实际变更才提交的规则均从总计划继承。
