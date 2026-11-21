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

主探针退出 1，v3 证据移存至 `target/acp-skill-coexistence/stakpak-v3.json`，独立诊断在同目录的 `stakpak-*.json`。Python 语法检查通过。复用了已有网关，未重新运行 Cargo 构建或 Rust 单测。当前未取得原生激活、选中技能读取、挂载后撤除、同轮 MCP 批准或拒绝证据。继续接入需要受审 Stakpak ACP 制品提供原生扫描与注册入口，并验证激活和附件访问只覆盖会话允许的目录。

## 2026-11-21 补充验证

- 私有 TOML 单独允许原生 `stakpak__view` 和 `stakpak__load_skill`，使用同摘要启动器及原生程序，绕开网关以隔离供应商行为。首次带工具的模型请求声明了 `load_skill`，但项目和私有 HOME 的 `.stakpak/skills` 均无目录元数据，正文和附件也未提前出现。随后以已知 URI 调用该工具，项目和 HOME 正文分别由 `call_2d466ee27e79ef5f9416cd8a`、`call_ae783c3bd3bbd606c02c36fe` 回传，目录外 URI 由 `call_dde9e1177a19be47bc0cd852` 返回 `LOAD_SKILL_ERROR`。模型回读 `deepseek-v4.1-flash-1017`，四个带工具模型请求后以 `end_turn` 结束。可从仓库根以 `PYTHONPATH=scripts python3 -c 'import json; from acp_skill_coexistence.stakpak import native_control; print(json.dumps(native_control()))'` 复现。也可设置 `ACP_STAKPAK_REVIEWED_DIR` 指向另一份摘要匹配的受管 v3。脱敏证据为 `target/acp-skill-coexistence/stakpak-native-toml.json`。
- 探针现复用 Cortex 的同沙箱 stdio 夹具。夹具在私有工作区通过只读 FIFO 供值，父进程记审计，受管桥仍执行原有工具名单及逐次审批。复用槽位 11 网关，在不选技能的两个独立会话单独复验 MCP。批准和拒绝各有一个 `chat.done`、零 `chat.error`，均回读 `session_model`、指定模型和 `full_access=false`。批准由 `call_f3fd2a18bdfc2b3b88b1c627` 回传随机值，下游执行一次。拒绝由 `call_82df7c10fd0939396273b58a` 回传 `TOOL_CALL_REJECTED`，下游执行零次。记录于 `target/acp-skill-coexistence/stakpak-mcp-only.json` 和 `stakpak-mcp-reject-only.json`。诊断回合缺少技能共存证据，主验收仍未通过。
- 当前共享变更未给 Stakpak 增加原生注册入口。固定源码的 `session/load` 只回填会话 ID 和模型状态。供应商系统提示文件只接受调用方准备的文本，无法调用技能扫描器。受管 Python 启动器可调整启动 TOML 和 MCP 目录，安装合约仍从固定归档校验并解出原生二进制。技能扫描接入 ACP 会话需要供应商新版或可审计的补丁制品，提供会话根注册以及发现和读取的路径约束。保留现行宿主门禁，无需为相同边界重建网关。此次通过 Python 语法检查，未运行 Cargo。

## 0.3.88 v4 受管源码接线

候选制品在原版 0.3.88 源码上应用固定补丁，并用上游锁文件和 Rust 1.94.1 构建。安装校验源归档、补丁和启动脚本摘要，记录产出的原生二进制摘要。v4 安装在独立版本目录。历史 v3 私有 TOML 对照可在提交 `71a2bd709` 的探针版本复现。

- Stakpak 原生扫描器先扫描项目的 `.stakpak/skills`，再扫描私有 HOME 中宿主发布的只读会话快照。项目同名条目优先。原生扫描排除禁用技能及指向目录外的符号链接。发现元数据只进入推理请求，恢复时按当前启动目录和会话快照重新生成。
- 原生 `load_skill` 使用当前目录中的精确 URI，经 ACP 单次批准后读取，并保留供应商的规范路径检查。官方 `view` 通过宿主已有文件回调读取附件。私有快照只加入读策略，沙箱仍禁止写入其路径。MCP 保留逐次批准，拒绝占用原生加载工具的名称。

Darwin arm64 槽位 10 使用 `cargo +1.94.1 build --locked -j 1 -p stakpak --bin stakpak` 编译通过。两个缓存路径均为仓库的 `target/rust-slot-10`。源归档 SHA-256 为 `7ffae3233a6d97d4bf142392ce747a720b6c94717259ee6527deed91664eeb00`，补丁为 `4873acd8bbdbcb0224db458256821e2cde986cee2298b7bc7433ec5bb616d7c4`，原生程序为 `901b4822f4c028fb32c3acdb3892d649696bed981abf131475bd1b55f6349e5c`。

真实进程使用同一私有 HOME 先建 ACP 会话，再重启并用 `session/load` 恢复。恢复后首轮目录包含项目及 HOME 技能，排除禁用技能和根外链接，且未提前注入正文。项目和 HOME 的单次批准读取分别以 `call_a56e1c6c9cfca03609f585ad`、`call_27fbdf7c3185c5816610e232` 回传。根外符号链接以 `call_c5b26ff538e3eb9ee85141af` 返回 `LOAD_SKILL_ERROR`，未触发审批。四个带工具模型请求后得到 `end_turn`。另以实际 Seatbelt 配置验证选中快照可读、写入被拒绝，私有运行目录仍可写。三项 MCP 桥定向 Python 测试通过。结构化证据位于 `target/acp-skill-coexistence/stakpak-v4-native.json`。

### v4 网关验收

在同一槽位构建 `vw-cli --bin shiju`，网关 SHA-256 为 `ff5da09342e645242942bc3208c4e579e417114c66e63b893e7fd256d00b79ca`。探针退出 0，四轮使用独立会话并回读 `stakpak`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017`、`full_access=false` 和权限请求状态 `requested=false`。模型响应由回环夹具提供，技能发现、激活和附件读取由真实 ACP 执行。撤除轮在同一私有工作区清空选集后创建新会话，模型只看到原生技能。

| 轮次 | `chat.done` / `chat.error` | 模型请求 | 下游 MCP | MCP 审批 |
| --- | --- | ---: | ---: | --- |
| 无挂载 | 1 / 0 | 3 | 0 | — |
| 挂载 | 1 / 0 | 6 | 1 | 单次批准 |
| 撤除 | 1 / 0 | 3 | 0 | — |
| 拒绝 | 1 / 0 | 6 | 0 | 拒绝 |

挂载轮原生技能和附件调用分别是 `call_463607536084bf8b63f9440c`、`call_411d8a3ee98ac578321798d2`，选中技能和附件是 `call_8626b177c22bbe2e70ddf6b1`、`call_1800d9fbecf4945fa9485f40`，MCP 是 `call_400c75d794eae39d278a8332`。拒绝轮以 `call_e15a460b21d70c0331f5a198` 回传 `TOOL_CALL_REJECTED`。结构化报告 `target/acp-skill-coexistence/stakpak.json` 保留全部会话和调用 ID。

本次在私有目录按受管文件布局暂存从固定源码构建的程序及安装回执，实际网关验证了制品。未在用户配置目录运行安装或发布制品。

同槽位运行 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_stakpak -- --test-threads=1`，退出 0。空选集与仅发布受信选中快照两个用例通过，548 项被过滤。
