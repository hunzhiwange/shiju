# 任务2：提示词组成与指令分离

实现完成；按用户覆盖要求不运行 Cargo、单元测试、实机测试或工具链验证，全部验证延期。没有退出码、通过数、失败数或 ignored 数可报告；以下数量只来自源码。

前置证据：evidence/baseline.md（任务1实际输出名）。当前目录当前分支工作，未创建 checkout/worktree。plan.md/prompt.md 未修改。

## 断言映射

| 原具名用例 | 领域同名用例 | 保留宿主边界 |
| --- | --- | --- |
| system_prompt_runtime_context_renders_model_directory_and_available_capabilities_once | 保留单一统一标记、首模型、工作目录、Skills/Task Tracking/Subagents、忽略第二模型、无模板残留全部断言 | 整条移除；真实环境由同来源未修改的 environment_wraps_missing_model_with_explicit_root 与 system_includes_environment_for_explicit_root 覆盖 |
| agents_context_base_system_excludes_instruction_content | content/base_content 不含指令正文；额外确认端口确实提供非空指令上下文 | 保留真实 AGENTS.md 读取，断言指令上下文长度和正文；移出组成断言 |
| system_prompt_keeps_project_agents_independent_from_empty_identity_whitelist | 空白名单下正文与基础内容分离、可见路径和上下文均保留 | 保留真实 AGENTS.md 读取、路径归一化及空身份白名单下上下文正文；移出正文分离断言 |

## 接线与范围

- 新文件 src/host_split_2_tests.rs 通过 src/prompt.rs 的 cfg(test) 子模块注册，可检查父模块私有 base_content，不暴露生产接口。
- 夹具仅实现现有 PromptHost：固定环境、按真实临时目录构造已解析指令、拒绝误走 system_resolved。不复制被测组成算法，不声明真实文件读取验收。
- 生产宿主 crates/vw-agent/src/session/prompt.rs 已将 system_with_metadata/system_with_agent_metadata 委托领域实现，并通过真实 Host 提供环境与工作区读取，本任务无须改动该接线。
- 原宿主文件继续由宿主 path 模块挂载；领域不反向依赖宿主。未迁 environment_wraps_missing_model_with_explicit_root 或其它测试族，未改依赖。
- 源码候选：新增领域3条，原指定宿主3条变2条（移除1条，保留薄用例2条）。实际编译注册与通过情况未知。
- 与既有 prompt_tests.rs 对照：其 runtime_parts 测试覆盖消息顺序，不替代本组组装断言；无删除该相邻测试。

后续统一验证：运行任务原定的领域 host_split_2_tests 过滤、两条保留宿主过滤、宿主注册清单；确认各目标编译和有效用例数。此次未执行这些命令，不以源码核对代替通过。
