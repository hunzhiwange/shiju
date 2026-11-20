# OMP 原生技能与 ShiJu 会话技能共存

2026-11-20，Darwin arm64，固定独立发行物 OMP 18.22.0，安装修订 `omp-18.22.0-model-source-v1`。入口 SHA-256 为 `d185113d63eba23925e5d70ab661a424b827c06c98f8e766c3bf4ad62ddaff9a`。探针将摘要匹配的入口复制到隔离网关的私有安装目录，业务选择通过 `defaults.skills` 和 `defaults.mcp` 注入。

私有 `--config` 追加经过校验的会话视图，未选时固定为空列表。该覆盖层仅设置 `skills.customDirectories`，原生目录、项目技能开关及过滤条件由 OMP 处理。探针在项目层配置额外目录，核对该目录无法将未选 ShiJu 技能带入目录。同名时按固定版 OMP 优先级处理，自定义目录技能保留原名，内容不同的原生技能可用命名空间访问。技能正文与附件均由原生 `read` 通过 `skill://` 按需读取。 [OMP 18.22.0 技能文档](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.0/docs/skills.md)

Seatbelt 对视图及其选中来源只追加读取项。Seatbelt 约束进程、网络及写入，启动参数为 `--no-extensions`、`--no-lsp` 和 `--offline`。MCP 使用回环 HTTP 到只读 stdio 转接及宿主逐次审批。 [MCP 受管入口](../mcp-acceptance/omp.md#受管入口接线)

## 修复前

槽位 3 的旧网关摘要为 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。无挂载控制轮退出 0，一个 `chat.done`，零 `chat.error` 和零 MCP 调用，模型通过 Chat Completions 收到 `read`、`write`、`edit` 并返回终态。回读为 `session_model`、`coexistence-fixture/selected-model` 和 `full_access=false`。控制轮报告为 `target/acp-skill-coexistence/omp-red-control.json`。严格探针退出 1，首个失败是旧启动参数 `--no-skills` 导致原生技能目录缺失。独立挂载轮退出 1，宿主返回 `omp: no verified skill directory mount branch`，一个 `chat.error`，模型、审批和下游 MCP 调用均为零。报告为 `target/acp-skill-coexistence/omp-red.json` 和 `target/acp-skill-coexistence/omp-red-mount.json`。

## 固定制品直连对照

使用与网关相同的受管 Seatbelt 规则、固定入口、HTTP 转接、stdio 夹具和模型探针。基线、挂载、撤除与拒绝均以 `end_turn` 结束，请求数依次为 3、6、3、6。三轮都回传了随机原生技能正文及附件，挂载和拒绝轮均另行读取选中技能。模型按调用 ID 收到许可后的 MCP 随机值和拒绝后的失败结果，下游调用数为 0、1、0、0。另用项目 `skills.enabled: false` 确认该项目禁用了原生技能目录。直连报告为 `target/acp-skill-coexistence/omp-native.json`。现有 OMP MCP 转接 Python 用例 4 项通过。

## 正式网关验收

槽位 11 的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均指向 `target/rust-slot-11`。执行 `cargo build --locked -j 1 -p vw-cli --bin shiju`，退出 0。槽内执行 `python3 scripts/verify_acp_skill_coexistence.py --agent omp --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/omp-before.json`，退出 0，报告归档至 `target/acp-skill-coexistence/omp.json`。网关 SHA-256 为 `69663985a6f90d50109d81e9b211ccb8ab4dfe8e73b67d726cca72ec14ef419a`。

| 阶段 | 新会话 | `chat.done` / `chat.error` | 模型请求数 | MCP 宿主决策 / 下游调用 |
| --- | --- | --- | --- | --- |
| 基线 | `ses_e06a3e77dffeTl2I16P3vXM9z2` | 1 / 0 | 3 | 无 / 0 |
| 挂载 | `ses_e06a3ab90ffenFtZK6ol7LjSaE` | 1 / 0 | 6 | 允许一次 / 1 |
| 撤除 | `ses_e06a3650effeX71M3HocXpmSyU` | 1 / 0 | 3 | 无 / 0 |
| 拒绝 | `ses_e06a32ea0ffeUqYRwIT38Xqr2l` | 1 / 0 | 6 | 拒绝 / 0 |

四轮回读后端 `omp`、版本 `18.22.0`、来源 `session_model`、模型 `coexistence-fixture/selected-model` 及 `full_access=false`。挂载轮原生正文和附件调用 ID 为 `call_dfa0a1e7472e3e778c912e81`、`call_0e4832c7f04258b60a58d466`，选中技能正文和附件调用 ID 为 `call_4bbc8861e964e239e8d14698`、`call_4e35096efd93024fcb816639`。获准 MCP 调用 `call_9c30f1e9ed0c18c8b858adac` 将随机值回传模型，拒绝轮调用 `call_039a0796501bf49f50508dff` 将失败结果回传模型。

Rust 定向单测复用槽位 9 的 `llm-runtime` 测试缓存，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均指向 `target/rust-slot-9`。执行 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_omp -- --test-threads=1`，退出 0，实际运行 1 项，1 项通过。构建和单测分别出现 `gateway-agent-apps` 未使用导入以及 `agent-database`、AISDK 测试辅助函数的未使用告警。
