# agent 安全组件拆包计划

目标：迁出密钥存储、网关配对和进程隔离后端，保持认证、加密及拒绝行为。

范围：`crates/libs/secret-store`、`gateway-pairing`、`process-sandbox`；agent 保留策略与沙箱选择工厂。

范围外：不搬 security::policy、OTP/estop 编排、semantic_guard、gateway_skey_store；不改认证协议、密钥文件格式或隔离降级。

假设：路径基础计划完成（串行资源前置），已有安全模块测试可迁移；系统二进制可用性按真实宿主记录。

## 设计决策

组件分别建包，不把配置、网关或 tools 引入底层。沙箱后端共享已有 Sandbox trait；工厂留在 agent，保留原 feature 的转发。

## 架构说明

agent → 三个独立安全包；gateway-pairing 只允许现有 vw-config-types 作为配置类型叶子依赖。安全包不依赖 agent/shared。

## 开发策略

三个任务串行。加密、认证及沙箱函数体机械迁移，必要定向测试在任务内完成；宿主回退不代表 Linux 隔离验收。

## 公共执行约定

必须继续阅读并遵守 [总计划公共执行、验证、Rust 资源与收尾规则](../2026-11-14-vw-agent大规模拆包/plan.md)。本文件与 prompt.md 只读，具体状态和证据记入编号任务。直接调用原生 Cargo、共享 10 槽位完整生命周期、Rust 重编串行、逐任务定向验证及有实际变更才提交的规则均从总计划继承。
