# Poolside 原生技能与会话技能共存验收

2026-11-20，Poolside 1.0.16 v2 的受管技能路径验收阻塞。无挂载普通读取完成。挂载时，宿主在模型推理前返回 `poolside: no verified skill directory mount branch`。受管启动器的私有配置还显式禁用了 `skill`，无挂载请求中没有原生技能目录或激活工具。[宿主门禁](../../../../crates/libs/llm-runtime/src/acp/skill_mounts/mod.rs) · [受管启动器](../../../../patches/acp-model-source/poolside/launcher.py)

## 制品与入口

- Darwin arm64，Docker 29.9.2。复用缓存的受管目录，四项文件均按安装合约核对，`pool --version` 回读 1.0.16。原生程序 SHA-256 为 `5dcb0162fd21a535e4f67117125e7d205e27a6881862aa223d0424082848f874`，启动器为 `9f4a7ac15e4d7d5804ad90ed326183e100e071b8ba977f37603893c4bb29145c`。另核对固定发行归档 SHA-256 `0932af3eb2b57a863acacb664ec8b2b1d3a76c2570a788b086001608cc585f74`。[安装合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 独立诊断用同摘要原版 `pool-native acp --sandbox required` 和私有设置。首个 Chat Completions 请求声明 `skill(name)`，列出了 `.agents/skills` 和 `.poolside/skills` 下的随机技能名，未包含正文或附件。调用 `diagnostic-skill` 后，模型续轮按原调用 ID 收到 `.agents/skills` 的正文，原生提示以 `end_turn` 结束。另将 `.poolside/skills` 放入受管工作区，启动器在 ACP 握手前以项目配置覆盖错误退出 1。[Poolside 1.0.16 的技能说明](https://github.com/poolsideai/pool/blob/v1.0.16/README.md#spec-support)

## 网关结果

探针启动实际网关、受管 ACP 和 Docker，供应商模型响应由回环夹具提供。通过 `defaults.skills` 和 `defaults.mcp` 选集，配置现有 stdio 服务，由受管启动器负责原有 HTTP 转接。普通读取使用私有技能夹具的附件路径，按调用 ID 校验模型续轮中的随机值。四轮都使用独立会话。无挂载与撤除回读 `poolside`、`coexistence-fixture/selected-model`、`session_model`、`full_access=false` 和 `access_status.requested=false`，访问状态的 `support` 为 `unavailable`。

| 轮次 | `chat.done` / `chat.error` | 模型请求 | 普通读取调用 ID | 下游 MCP |
| --- | --- | --- | --- | --- |
| 无挂载 | 1 / 0 | 2 | `call_b293506f5c594cde196ce85b` | 0 |
| 挂载 | 0 / 1 | 0 | 无 | 0 |
| 撤除 | 1 / 0 | 2 | `call_a8d3f8273ea63b1a01184830` | 0 |
| 拒绝对照 | 0 / 1 | 0 | 无 | 0 |

探针主入口在无挂载轮完成普通读取后，因受管原生技能目录和激活工具缺失退出 1。其余轮次逐次调用同一运行器独立诊断，均退出 1。拒绝对照停在技能门禁，未到 MCP 工具审批。未取得选中技能的发现或读取、已批准 MCP 调用和撤权验收证据。历史受管 MCP 验收覆盖 HTTP 桥及单次审批。[历史审批与撤权回执](../mcp-policy-audit/coexistence/poolside.md#审批恢复与入口边界)

约定的新构建进入槽位 3 后等待共享 Cargo 锁，确认证据指向受管原生入口后取消排队，入口退出 130，未开始本次编译。随后在重新领取的槽位 3 复用现有网关，两个 Cargo 缓存变量均指向 `target/rust-slot-3`，网关 SHA-256 为 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。本次没有 Rust 实现改动，未运行 Rust 单测或桌面构建。结构化证据位于 `target/acp-skill-coexistence/poolside.json` 和同目录的按阶段报告。探针退出时按私有工作区精确匹配清理沙箱容器。

继续接入需要经审查的原生技能注册入口，明确只读会话视图的扫描范围并验证激活和撤销，随后复跑三轮及 MCP 拒绝对照。
