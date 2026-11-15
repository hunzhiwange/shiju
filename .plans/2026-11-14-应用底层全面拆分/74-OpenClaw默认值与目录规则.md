# 任务 74：OpenClaw默认值与目录规则

先阅读并遵守 [公共执行规则](plan.md)。

批次：【补充批次 74】依赖：任务 73（及任务 12）；入口顺序 12 → 73 → 74 → 13，后续仍严格串行。

状态：未开始

目的：从任务 38 划出已审计的叶子边界，不重复其来源或目标职责。只写代码、静态审查、提交并删除完成卡；不运行任何 Cargo、测试或实机，未来统一验证另行授权。

## 文件与契约

- 来源：`crates/vw-agent-apps/src/` 下 openclaw_catalog.rs、openclaw_catalog_tests.rs、openclaw_defaults.rs 的纯逻辑与 openclaw_defaults_tests.rs。
- 目标：`crates/libs/app-openclaw-config-rules/Cargo.toml`、`src/lib.rs`、独立测试及内聚子模块。
- 迁移符号：OpenClawModelCatalog/Entry、get_model_catalog、set_model_catalog；OpenClawAgentsDefaults/Form、document、get_agents_defaults、prepare_agents_defaults 及 form/with_form。
- 实际依赖：app-config-patch::patch_json5、任务 12 的 app-config-write::WriteError、serde、serde_json（arbitrary_precision）。任务 12 的库尚未实施，必须先确认真实导出，不将上述候选路径当成现有接口。
- 消费者：原门面、任务 38；真实外部调用文件见架构清单及其本次补充审计。
- 修改：根 workspace、原包清单与模块导出、架构所有权表及真实直接消费者；不修改 Cargo.lock。

## 实施步骤

1. 读取源定义、全部调用方和相邻测试，核实前置契约、cfg 与最小公开面；新库不能反依赖门面。
2. 移动上述真实符号与独立测试，禁止复制实现或跨包 path 引用。macOS cfg 保持；沿用原数据形状和错误，不新增 trait 或服务注册。
3. 注册 workspace 与 path 依赖，原路径重导出同一类型；直接消费者改用所属库，检查私有访问。
4. 更新任务 38 的移交落点及架构清单，确认唯一所有者。
5. 静态检查 `git diff --check`，检索新增库无门面回依赖，核对清单、模块、资源、测试注册；中文提交注明未运行 Cargo、测试及实机，成功后删卡。

## 边界与后续验收

openclaw_defaults::set_agents_defaults 及其 OpenClawFiles/ProviderTransactions/TenantScope/AppState 发布依赖仍由任务 38 接收；原模块组合纯库重导出与该函数。catalog 整文件迁移；defaults 拆符号，全部现有纯测试迁入独立文件。任务 38 继续负责 model/env/tools/health 及发布，不能把原生文件依赖带入规则库。后续验收覆盖扩展字段、注释保留、旧 timeout 优先级、非有限值拒绝、缺失与空目录区分及授权锁一致性；本轮均未验证。
