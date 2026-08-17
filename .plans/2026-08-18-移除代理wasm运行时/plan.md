# 移除代理 WASM 运行时计划

目标：移除代理内 WASM 运行时、模块工具及全部 WASM 运行时配置入口。

范围：删除实现、工具注册、配置类型、桌面运行时设置与相关 API 类型、测试、依赖和说明文档。

范围外：保留 `wasm32-unknown-unknown` 的桌面 Web 编译支持及其条件编译代码。

假设：用户要求删除的是代理的 `runtime.kind = "wasm"`，不是 Rust 目标平台的 WASM 支持。

## 设计决策

- 运行时仅保留 `native` 与 `docker`；旧的 `kind = "wasm"` 配置将被明确拒绝。
- 不保留兼容字段或迁移逻辑，避免继续暴露无效能力。

## 架构说明

- `vw-config-types` 是运行时配置的唯一类型来源。
- `vw-agent` 工厂和工具集决定可用运行时与工具；桌面设置仅展示其有效配置。

## 开发策略

- 先删除对应测试，验证它们不再编译；再移除生产实现与配置连线。
- 最终使用 `cargo clippy --all-targets --all-features -- -D warnings` 验证工作区。
