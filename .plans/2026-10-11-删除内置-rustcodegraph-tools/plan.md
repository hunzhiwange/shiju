# 删除内置 RustCodeGraph Tools 计划

目标：移除 VibeWindow 内置 RustCodeGraph Tool 及系统级耦合，同时保留可独立使用的 RustCodeGraph CLI 技能。

范围：删除 `vw-agent` 中原生 Tool 实现、注册、依赖和专用提示词；清理配置类型与桌面工具目录的强制注入和展示元数据；将内置技能说明改为只使用外部 CLI，并同步相关测试与评测文档。

范围外：不删除 RustCodeGraph 技能、不删除用户本地索引、不改动 RustCodeGraph 外部项目、不迁移旧工具配置。

假设：仓库中的 `skills/rustcodegraph` 和打包资源仍作为代码导航技能保留；删除后模型只能经 shell 调用已安装 CLI，未安装或未索引时按技能规则回退普通搜索。

## 设计决策

- 选择彻底删除内置 Tool，而不是增加开关或保留空壳；这样能消除 `vw-agent` 对 RustCodeGraph 库、生命周期线程、工具目录和专用系统提示的耦合。
- 保留 bundled skill 与仓库 skill，并统一为 CLI-first 契约；不再承诺原生工具存在。
- 现有配置中的 `rustcodegraph` 不做兼容迁移；通用工具过滤逻辑应自然忽略不可用工具。

## 架构说明

- Tool 生命周期集中在 `vw-agent/src/tools` 与 toolset 注册；桌面端另有工具目录规范化和元数据展示。
- `rustcodegraph` 同时存在于依赖、提示词、配置必选清单、桌面目录与技能资源，删除时必须逐层收口并保持通用 MCP 动态工具逻辑不变。

## 开发策略

- 删除工作以静态缺失检查、目标 crate 编译和仓库规定的全量 clippy 为主要证据。
- 先移除后端 Tool 与依赖，再清理桌面/配置耦合，最后更新技能契约与文档。
- 每项任务只删除 RustCodeGraph 专用分支，不顺带重构通用工具、技能或 MCP 机制。
- Rust 变更先运行 `cargo fmt --all`，再按任务使用共享 Cargo 槽位执行聚焦或全量 clippy。
- 不运行仓库禁止的全量 `cargo test`、`cargo fmt --all -- --check`、WASM 检查或 Makefile 命令。
- 每项任务取得证据后独立使用 `$git-commit` 提交，避免混入其它任务或工作区变更。
