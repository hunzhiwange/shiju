# 我的工作台并行运行两个 Codex ACP

同一个 `acp-codex` 配置可以启动多个独立会话。主会话用 `shiju-cli-workbench` 技能发任务，每个子会话各自回传结果。

## 首次配置

1. 在识矩右上角打开 **设置 → 智能体 → ACP**，搜索 `Codex`。找到 **Codex CLI**，缺少适配器时点 **一键安装**，然后点 **测试连接**。若显示「需要认证」，在运行识矩网关的机器上按 Codex CLI 的提示完成认证。使用 ChatGPT 登录时可在该机器的终端运行 `codex login`，再用 `codex login status` 查看。最后以工作台 ACP 会话实际能回复为准。
2. 在 **设置 → 智能体 → 官方智能体** 搜索 `Codex`。确认 **Codex CLI** 卡片标记为「已启用」，并记下卡片上的配置标识 `acp-codex`。适配器安装和智能体启用是两个状态。使用自定义配置时，记下自己的标识，并确认网关支持该配置使用 CLI 本地模型来源。
3. 回到 **识矩首页 → 我的工作台 → 打开**，在左侧选要工作的项目。点会话栏右侧的 **＋**。可选择 **Codex · 终端** 当主会话，在终端里直接粘贴提示词。也可以使用同一工作台中已能调用命令的 ACP 对话当主会话。选择 ACP 对话后，用右侧的 **↑ 发送消息** 输入提示词。
4. 让主会话读取 `shiju-cli-workbench` 技能。在 ShiJu 源码仓库里可直接让它读取 `skills/shiju-cli-workbench/SKILL.md`。其它项目可将整个技能目录复制到项目的 `.agents/skills/shiju-cli-workbench/`，或放到主会话所用 Agent 的技能目录，然后新开主会话。工作台主会话使用继承的连接，先运行 `shiju workbench list` 验证。

普通终端也能运行 `shiju workbench list`。默认用网关用户 `0`，由网关按当前终端目录匹配项目。普通文件夹尚未登记时，首次调用会在网关登记该目录，随后即可列出会话。从其它目录执行时，可以传 `--directory "/项目的绝对路径"` 或 `--project <项目 ID>`。要查看配对用户的会话，按通用连接参数提供匹配的 `--user` 和网关凭据。

## 复制这段话做首次验收

> 使用 `shiju-cli-workbench` 技能，先运行 `shiju workbench list` 检查连接。我授权你在当前项目新开两个 `acp-codex` 工作台 ACP 会话。给 A 发送「只回复 `WORKBENCH_CODEX_A`，不调用工具」，给 B 发送「只回复 `WORKBENCH_CODEX_B`，不调用工具」。用两个不同的会话和请求 ID，发完两条再分别等待和读取。列出两路状态和原文回复。不要修改文件或设置。遇到错误或审批时告诉我具体是哪一路。

验收通过时，两条投递都显示 `completed`，回复分别为 `WORKBENCH_CODEX_A` 和 `WORKBENCH_CODEX_B`。确认同时执行时，在两轮都结束前观察两路运行状态是否有重叠。正式使用时，替换 A、B 的任务描述即可。

## 继续测试子会话互发消息

> 使用 `shiju-cli-workbench` 技能，继续用刚才的 A、B 会话。我授权 A 和 B 在当前项目互发本次测试消息。先追问 A，让它用 `shiju workbench send` 直接问空闲的 B「2 加 3 等于多少？」。让 A 等待、读取并回报 B 的原文和请求 ID。等 A 恢复空闲后，再追问 B，让它用同样的方式问 A「7 减 4 等于多少？」。主会话用两个接收方的会话 ID 和请求 ID 分别核对结果。

预期是 A 从 B 收到 `5`，B 从 A 收到 `3`，主会话能分别查到两条直接投递。

## 常见卡点

| 看到什么 | 怎么处理 |
| --- | --- |
| ＋ 菜单没有 Codex 的 ACP 对话 | 核对当前项目、`acp-codex` 的启用状态，以及当前网关是否提供 Codex 的 CLI 本地模型来源。 |
| `--user is required` | 此提示来自旧版 CLI。使用包含默认用户和目录定位支持的 CLI，或在工作台主会话内运行。 |
| `workbench directory must be an existing absolute directory on the gateway` | 检查该目录是否存在于网关机器。连接远程网关时，可以用 `--directory` 指定网关机器上的绝对路径。 |
| `disconnected` 或 `gateway connection failed` | 新开工作台主会话，确认它能使用与网关配套的 `shiju` CLI。如果仍然失败，检查主会话的命令环境能否访问同机回环连接。 |
| 「启动成功 · 握手成功 · 需要认证」 | 按网关主机上的 Codex CLI 提示处理认证，再试发一条短消息。握手结果只覆盖适配器连接。 |
| `waiting_for_approval` 或 `timed_out` | 在工作台处理审批，或用原会话 ID 和请求 ID 查询状态后继续等待。 |

命令详见 [shiju-cli-workbench 技能](../../skills/shiju-cli-workbench/SKILL.md)。

## 2026-11-26 实测记录

- 在 `shiju` 项目的「我的工作台」，原生 Codex 读取了技能。`shiju workbench list` 返回 `disconnected`。主会话拿到了继承的连接，但其命令环境无法连到对应的本机回环端口。改用与当前网关配套的 CLI 后仍是 `gateway connection failed`。
- Codex CLI 的 ACP 适配器已安装，官方智能体 `acp-codex` 已启用。设置中的连接测试显示「启动成功 · 握手成功 · 需要认证」。该项目的工作台 ＋ 菜单当时未显示 Codex ACP 对话。
- 换用工作台里的 Claude Code ACP 做主会话，发送后显示「ACP 请求失败」。两路 Codex ACP 都未启动，派发、执行重叠和结果回收尚未验证。
