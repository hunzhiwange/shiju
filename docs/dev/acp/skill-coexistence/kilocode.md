# Kilocode 技能共存验收

2026-11-20，Darwin arm64。当前受管 Kilocode 7.8.18 的入口为
`reviewed/kilocode acp --pure`，SHA-256 为
`19cc66a17aead6565b60dda07f7f7a47a1b73f72e0d91a9456fd94a483cfa7ff`。
探针将入口复制到私有临时配置目录，并校验固定契约中的摘要和
`--version`。探针使用 `session_model` 选择回环模型
`coexistence-fixture/deepseek-v4.1-flash-1017`，经工作区 MCP 配置选择只读
stdio 服务，并保持 `full_access=false`。

| 轮次 | `chat.done` | `chat.error` | 模型请求 | MCP 执行 |
| --- | ---: | ---: | ---: | ---: |
| 基线 | 0 | 1 | 0 | 0 |
| 挂载 | 未到达 | | | |
| 撤除 | 未到达 | | | |
| 拒绝对照 | 未到达 | | | |

首次走查退出 1，结构化报告为
`target/acp-skill-coexistence/kilocode-before.json`。走查复用了已有的
`target/rust-slot-6/debug/shiju`，网关 SHA-256 为
`a33ebf2cf7a35e986065bae9203472f539d2b0a96c32aa52e54d5874427f0f98`。
探针未持有构建槽位，也未重新编译。实际首个断点是
`requires_full_access`。桌面请求在启动供应商进程前即被拒绝，模型和权限
回读、原生调用 ID、会话视图及 MCP 对照均无运行证据。

宿主的 `ensure_desktop_native_access` 要求 Kilocode 拥有当前完全访问授权。
[历史 stdio MCP 验收](../mcp-acceptance/kilocode.md)使用该模式，记录为
`client_only`，没有观察到逐次 MCP 审批。
[固定补丁](../../../../patches/acp-model-source/kilocode/README.md)中的扫描器
仍以会话根替换原生发现，空选择仍禁用技能目录与内置技能。该差异来自制品
源码检查，本轮无法在模型请求中到达。保留原有受管制品和宿主门禁。
恢复任务需要受审的原生工具约束和 MCP 拒绝路径，之后再使用同一探针完成
三轮及拒绝验收。
