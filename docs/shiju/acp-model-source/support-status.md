# 模型来源切换支持状态

快照日期：2026-11-05。基于当前工作区生产能力表及已存证据，HEAD 为
`dc804fa14`，包含尚未提交的模型来源默认值修复。本次未重新探测上游发行版。
后续对比以此日期、版本和平台为基线，不将这里的结论外推到其他版本。

## “不支持切换”的含义

本文记录的是从“智能体原生模型”（`cli_local`）切到“客户端模型”
（`session_model`）的能力，不是智能体原生账号内的模型选择能力。

- 支持客户端模型的智能体：新会话草稿默认客户端模型，可在首次发送前切换来源。
- 会话开始后锁定智能体、模型来源及所选模型。更改任一项需要新建会话。
- 不支持客户端模型的智能体：新会话默认原生模型，客户端模型按钮禁用，
  后端拒绝相应切换；切换操作不会自动关闭浮窗。
- 原生模型来源仍依赖执行主机上的 CLI、配置和登录状态，能力表中的
  `cli_local` 支持不代表 CLI 已安装或账号可用。
- “未支持”是本仓库当前的接入结论，不代表厂商永远不支持第三方模型。

## 当前不能切到客户端模型：5 项

以下五项在生产 `verified_release` 中均无条目，driver 明确拒绝托管启动。
“调查版本”来自已有[集成清单](integration.md)，不是已开放能力的版本。

| 智能体 / key | 调查版本 | 当前阻塞原因 | 后续解除条件 |
| --- | --- | --- | --- |
| Auggie / `auggie` | `0.36.0`；另探测 `1.2.0-prerelease.202611031510` | 固定版拒绝扩展 Provider 注册；预览版虽加载扩展，`registerProvider` 仍明确拒绝 | 提供 ACP 可用的模型服务注入入口，证明主/辅助模型使用指定服务且无账号模型回退；见[调查证据](evidence/auggie.md) |
| Cursor / `cursor` | `2026.10.22-27dcf5f` | CLI 注入任意第三方 Provider endpoint/key 尚无经过验证的上游契约 | 原 CLI ACP 提供可验证的任意 Provider 接入口，并完成真实模型请求验收；SDK 的托管模型能力不能代替该入口 |
| Droid / `droid` | `1.3.0` | 固定版 ACP 双向传输与原生配置根目录隔离尚未验证 | 补齐双向 ACP、专属配置目录隔离及真实模型请求证据；不能据此断言 Droid 本身不支持第三方 Provider |
| Qoder / `qoder` | `1.1.82` | Custom Provider 需要获授权的个人账号登录及相应权益 | 在具备授权和权益的环境验证 Provider 注册、ACP 使用、配置隔离与无回退；账号模型选择不等于任意 Provider 注册 |
| TRAE / `trae` | `0.121.3` | 固定版需要获授权的平台登录，且缺少经过验证的专属运行目录隔离 | 补齐授权登录、固定版本运行目录隔离及真实 ACP 模型请求；不能用另一个 trae-agent 项目替代 traecli |

各项拒绝原因以生产 driver 为准：
[Auggie](../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/auggie.rs)、
[Cursor](../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/cursor.rs)、
[Droid](../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/droid.rs)、
[Qoder](../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/qoder.rs)、
[TRAE](../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/trae.rs)。

## 有客户端模型能力记录：17 项

以下版本均受 **macOS Apple Silicon（Darwin arm64）** 平台门控。
Windows、Linux、macOS Intel 当前不公布客户端模型支持，默认原生模型并禁用客户端来源切换。

| key | 证据版本 | 额外运行时条件 |
| --- | --- | --- |
| `claude` | ACP `0.26.0` / SDK `0.2.96` / Claude Code `2.1.96` | 原生发行版 |
| `codebuddy` | `2.179.3` | 原生发行版 |
| `snow` | `0.9.14` | 原生发行版 |
| `copilot` | `1.0.105` | 原生发行版 |
| `kilocode` | `7.8.18` | 原生发行版 |
| `kimi` | `2.5.0` | 原生发行版 |
| `kiro` | `3.0.0` | 原生发行版 |
| `opencode` | `1.19.5` | 原生发行版 |
| `openclaw` | `2026.10.10` | 原生发行版 |
| `codex` | ACP `2.5.0` / Codex `0.165.0` | 需安装并通过身份校验的审查补丁运行时 |
| `gemini` | `0.66.0` | 同上 |
| `iflow` | `0.5.19` | 同上 |
| `qwen` | `0.26.7` | 同上 |
| `pi` | ACP `0.0.22` / Pi `0.73.1` | 同上 |
| `goose` | `1.57.0` | 同上 |
| `vibe` | `3.0.4` | 同上 |
| `hermes` | `0.19.0` | 同上 |

八项补丁运行时缺失或校验不通过时，同样不公布客户端模型支持。
未知自定义 driver 也不默认获得客户端模型能力。
版本列是证据基线，不表示任意新版兼容；能力表也不能代替实际可执行文件身份校验。
运行时安装与逐项证据入口见[集成记录](integration.md)。

智能体具备接入口，不代表任意客户端模型都可用。所选模型仍须满足协议、工具、
采样能力、启用状态及凭据要求；不兼容会拒绝切换，不静默改用原生账号。
例如 Claude 使用 Chat 协议时关闭 Claude 专属 thinking，不转换签名；
原生 Messages 推理路径保留原有能力，Responses 等未支持的转换仍会拒绝。

Codex 客户端模型支持原生 Responses 和支持工具调用的 OpenAI 兼容 Chat
Completions。Chat 路径转换流式事件、命名空间函数和 `apply_patch` 等自由文本工具，
并保留工具调用与结果的关联。Chat 路径关闭 Responses 托管网页搜索，拒绝加密推理
和服务端历史引用。原生 Responses 路径使用供应商自身的能力。

## 客户端模型推理档位

适用平台为 Darwin arm64。表中四组均使用固定 CLI、托管客户端模型和本机回环上游。桌面在已启动会话的输入栏打开「ACP 选项」，按执行主机返回的档位和当前值显示。会话忙碌或选档请求未结束时禁用编辑。托管进程在轮次结束后释放时，已保存的选项仍可读取，修改值随下一轮恢复到原生会话。

| 智能体与固定版本 | 同一模型及协议 | 可用控制时机 | 两次出站字段 |
| --- | --- | --- | --- |
| Codex ACP `2.5.0` / Codex `0.165.0`，审查补丁版 | GPT-5.5（夹具标识 `acp-fixture-primary`），`/v1/responses` | 首轮成功后在同一会话设档，恢复时重放 | `reasoning.effort` 为 `low`、`high` |
| OpenCode `1.19.5`，审查运行时 | `gpt-5.1`，`/v1/chat/completions` | 首轮成功后在同一会话设档 | `reasoning_effort` 为 `low`、`high` |
| Claude Code ACP `0.31.0` / SDK `0.2.119` / Claude Code `2.1.119`，审查运行时 | `claude-opus-4-6`，`/v1/messages` | 首轮成功后在同一会话设档 | `output_config.effort` 为 `low`、`high`，两轮均为 adaptive thinking |
| GitHub Copilot `1.0.105`，审查运行时 | 模型声明支持 OpenAI Chat 推理档位（夹具标识 `acp-fixture-primary`），`/v1/chat/completions` | BYOK 首轮成功后在同一会话设档。默认显示值也可点击确认 | `reasoning_effort` 为 `low`、`high` |

Codex Chat、Claude Chat 和 Copilot Messages 尚无本表所示的客户端档位验收。Claude ACP `0.26.0` 未公布动态 effort。Copilot 的 BYOK 首轮启动参数未通过验证，桌面在会话启动后提供动态选档。OpenCode 与 Copilot 的默认对照请求省略档位字段。模型未公布的档位由后端拒绝。

固定版本、控制契约与离线运行命令见 [Codex](evidence/codex.md)、[OpenCode](evidence/opencode.md)、[Claude Code](evidence/claude.md)及 [GitHub Copilot](evidence/copilot.md)。

## 后续对比方法

1. 核对生产[能力表与证据版本](../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs)
   的 `capabilities`、`verified_release`，以及对应 driver 的拒绝条件。
2. 从执行主机查询 `GET /v1/acp-model-source/{agent}`，记录
   `model_source_capabilities`、`acp_model_source`、`verified_cli_release`、`verified_platform`。
   该读取不启动 CLI；界面以执行主机结果为准。
3. 对版本变化记录旧/新版本、平台、运行时身份、阻塞原因变化及证据路径。
   解除阻塞须补真实 CLI 的两种来源、实际模型请求、辅助请求、配置隔离、
   工具回合、取消、恢复与无回退证据，不能仅以安装成功或 ACP 握手成功判定支持。
4. 更新本页快照日期与对应行，保留历史证据文件；同时核对默认来源和按钮禁用状态。

验收流程见[真实 CLI 离线模型验收](verification.md)。
