# 任务 3 · Windows私有状态

先阅读并遵守 [公共执行规则](plan.md)。

批次：1，依赖无。

状态：未开始

目的：使 Windows GLM 模型驱动能在受验证的私有状态目录中启动。

## 文件与步骤

- 负责 `crates/libs/llm-runtime/src/acp/model_source/drivers/glm_acp_agent.rs` 及同级独立 `glm_acp_agent_tests.rs`。如需依赖声明只调整 `llm-runtime/Cargo.toml` 的 Windows 项。测试模块接线写进 GLM 驱动文件。
- 利用项目现有 Windows 安全处理及 ACL API。以实际进程身份核对目录所有权与 ACL，对当前绑定建立私有目录并拒绝不安全继承、重解析点和权限验证失败。覆盖已存在目录及并发复用。核对代理私有配置与会话目录实际落在该目录。继续验证回环地址及模型能力，保持 Unix 所有权和 `0700` 校验。
- Windows 单测以当前用户临时目录验证可用和不安全路径，使用占位令牌验证拒绝后没有启动副作用。新增过滤前缀 `glm_state_directory_`。

## 验证

在 Windows 原生隔离环境用公共槽位约定执行 `cargo test -j 1 -p llm-runtime --lib glm_state_directory_`。红测应看到安全私有目录仍因 Windows 分支拒绝。绿测须确认合法目录成功，不安全 ACL 或重解析点拒绝。复用同一过滤条件验证 Unix 路径，同时确认实际测试数。
