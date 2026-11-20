# Stakpak 原生技能与会话技能共存验收

2026-11-20，Stakpak 0.3.88 v3 的技能共存验收阻塞。真实网关无挂载时完成普通文件读取，首个模型请求缺少原生技能目录和 `load_skill`。选中技能时，宿主在推理前以 `stakpak: no verified skill directory mount branch` 拒绝启动。[宿主门禁](../../../../crates/libs/llm-runtime/src/acp/skill_mounts/mod.rs)

## 固定入口

- Darwin arm64，受管版本回读为 0.3.88。发行归档 SHA-256 为 `9c18808f7a3352a5ca2cbb9f123f2b2fafda03a5abef446c20065bb6ec629d1f`。私有验收目录中的五个制品文件均与合约匹配。启动器 SHA-256 为 `e23af645b513eae95f62d964ea067aeca76d88bc421a33656eefae7a7f2609c6`，原生程序为 `70c559eff250a5be0dcdb19f31f13dca86847ee57648dba78e531506bb482ebc`。[安装合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 该版本的交互 CLI 扫描项目和用户的 `.stakpak/skills` 并构造技能目录。ACP 的构造与新建会话路径沿用可选系统提示，未调用技能扫描器。原生 `load_skill` 根据配置根的规范路径校验 URI。现行私有 TOML 的受审工具列表为 `view`、`create` 和 `str_replace`。追加私有 HOME 下的目录无法让这条 ACP 路径发布原生目录。[交互 CLI](https://github.com/stakpak/agent/blob/v0.3.88/cli/src/main.rs) · [ACP 源码](https://github.com/stakpak/agent/blob/v0.3.88/cli/src/commands/acp/server.rs) · [原生扫描器](https://github.com/stakpak/agent/blob/v0.3.88/libs/api/src/local/skills/mod.rs) · [私有 TOML 驱动](../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/stakpak.rs)
- 复用槽位 11 的实际网关，两个 Cargo 缓存变量均指向仓库的 `target/rust-slot-11`，网关 SHA-256 为 `69663985a6f90d50109d81e9b211ccb8ab4dfe8e73b67d726cca72ec14ef419a`。探针使用本地 OpenAI Chat Completions 夹具和 `deepseek-v4.1-flash-1017` 模型 ID。夹具输出上限为 16000，以接受 Stakpak 的上下文流程向中继发出的请求。能力门禁、沙箱和审批沿用现行实现。

## 真实进程结果

探针复用 `defaults.skills` 和 `defaults.mcp` 的业务选集。原生、选中和未选技能使用不同随机标记。无挂载时首次模型请求没有技能名、正文或附件标记。探针用已知路径调用官方 `view`，随机附件值按原调用 ID 返回，证实文件读取和模型续轮正常。

| 独立诊断轮次 | `chat.done` / `chat.error` | 模型请求 | 普通读取调用 ID | 下游 MCP |
| --- | --- | --- | --- | --- |
| 无挂载 | 1 / 0 | 2 | `call_b199b71a34a672d199956daa` | 0 |
| 挂载 | 0 / 1 | 0 | — | 0 |
| 撤除对照 | 1 / 0 | 2 | `call_ea831ee351e5360dc77c7bf8` | 0 |
| 拒绝对照 | 0 / 1 | 0 | — | 0 |

无挂载及撤除对照回读 `stakpak`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017`、`full_access=false` 和权限 `requested=false`。撤除对照在独立私有环境验证空选集，未验证先成功挂载再撤除的生命周期。

额外执行了两轮仅选 MCP 的诊断。受管启动器在 `session/new` 返回 `Stakpak could not mount the selected MCP tools`，两轮均无模型请求、审批或下游调用。用同制品的 stdio 桥直接运行共享夹具复现了 `MCP server disconnected`。该桥限制脚本读取至私有工作区及临时目录，并限制写入至子进程临时目录。共享探针从工作区外启动仓库脚本并在子进程临时目录外写审计文件。后续共存回归需要适配这条沙箱边界的夹具。[受管 MCP 历史回执](../mcp-acceptance/stakpak.md#新标准网关验收通过)

主探针退出 1，证据位于 `target/acp-skill-coexistence/stakpak.json`，独立诊断在同目录的 `stakpak-*.json`。Python 语法检查通过。复用了已有网关，未重新运行 Cargo 构建或 Rust 单测。当前未取得原生激活、选中技能读取、挂载后撤除、同轮 MCP 批准或拒绝证据。继续接入需要受审 Stakpak ACP 制品提供原生扫描与注册入口，并验证激活和附件访问只覆盖会话允许的目录。
