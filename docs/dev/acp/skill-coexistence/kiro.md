# Kiro 技能共存验收

2026-11-20，Darwin arm64。固定 Kiro 3.0.0 V3 的私有启动入口为
`kiro-cli acp`。生产驱动追加 `--agent-engine v3 --auth-method cli`，
通过 `session_model` 绑定 `coexistence-fixture/deepseek-v4.1-flash-1017`。
技能来自 `defaults.skills`，只读 stdio MCP 来自宿主配置及 `defaults.mcp`。
探针请求 `full_access=false`，使用私有工作区、凭据及回环模型响应。
[Kiro 工作区技能路径](https://kiro.dev/docs/skills/)
[历史 MCP 验收](../mcp-acceptance/kiro.md)

## 修复前

槽位 3 的现有生产网关构建 SHA-256 为
`403bb2827df94239bc831633f3cd3a1d10dd832594fe9d8471cc1c8bde2ebacf`。
运行 v1 探针退出 1，首条真实 Responses 请求缺少工作区的原生技能。
基线会话收到 0 个 `chat.done`、1 个 `chat.error`，MCP 执行为 0。
结构化报告位于 `target/acp-skill-coexistence/kiro-before.json`。
v1 引擎 SHA-256 为 `26907b1f2895cf28a850d7720b340cb206965d8ffbc17cb62dd78276f2dbd02d`。

## 受管 v2

v2 保留原生目录扫描、内置帮助、配置驱动的过滤与 profile 注册，使用原生
解析器把经宿主校验的会话视图加入会话注册表。目录有同名条目时保留原生条目
的优先级。空选集保留原生目录。终端转发和审批继续沿用固定版原有路径。

| 制品 | SHA-256 |
| --- | --- |
| 官方 ZIP | `0a8f067c4fe6a23a231aa3a18e3e4332120ef9ed3d2d7b6eea907beb8b4cdd8b` |
| 私有启动器 | `184699ab81f2df5649a89f8036678ff682fbf881689e6273c09afecdfabd29e8` |
| 原生 CLI | `4e4b2ee6b5021624d23c6ef78b18624c438ee898596a683f5a55889c512965bc` |
| 随包 Node | `46d1713acc486d844a8b944fb0f606366aebc70d6601646b9807abc7cb1a2201` |
| 补丁后 V3 入口 | `25726b5cf5b69c1cca0aa7685627db5dca37afde52e148d5af8f1a8b0c3083f9` |

官方 ZIP 分段下载后按固定摘要校验。仓库安装器读取私有归档副本，提取并
核对新旧引擎摘要。补丁通过 `node --check`。旧技能夹具的 3 项 Kiro
定向单测通过。旧夹具的未选中诱饵位于 `.shiju/skills`，用于检查宿主选择隔离。

独立 ACP 对照中，基线经 `disclose_context` 和 `read_file` 读取原生正文及附件。
挂载和拒绝对照同时读取原生与选中技能。同名的会话条目使用独立正文和附件，
激活仍读到工作区原生路径。模型从 `tool_load` 的目录取得
`coexistence::read_probe`，再调用 `tool_call`。放行后实际执行一次，
拒绝后为零次，并向模型返回对应调用 ID 的拒绝结果。

## 生产网关回归

网关构建及探针退出 0。两个 Cargo 缓存变量都指向仓库绝对路径下的
`target/rust-slot-3`。网关 SHA-256 为
`808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。
报告保存于 `target/acp-skill-coexistence/kiro.json`。表中的 ID 来自模型续轮。

| 轮次 | 会话 | done / error | 模型请求 | MCP 执行 / 审批 |
| --- | --- | --- | ---: | --- |
| 基线 | `ses_e0777b3d2ffe8cPVbc3pKgwJ8P` | 1 / 0 | 3 | 0 / 无 |
| 挂载 | `ses_e07771d34ffeh7fLOqyx6uj35N` | 1 / 0 | 7 | 1 / once |
| 撤除 | `ses_e07769732ffeYUplJrUfOspb2q` | 1 / 0 | 3 | 0 / 无 |
| 拒绝对照 | `ses_e07761e79ffetxCaJuBuGVn9U5` | 1 / 0 | 7 | 0 / reject |

各轮回读 `session_model`、`full_access=false`、预设及 `kiro` 后端。

- 基线关联调用为 `native` `call_6264df0a5256172987898a7d`、`native_read` `call_3c0b07adc80230d5e4ad0abe`。
- 挂载关联调用为 `native` `call_e10af3d16c94af6feef40c98`、`native_read` `call_50d673c83dcff0c46069e78c`、`selected` `call_bacd4d91882fefdab7a895d6`、`selected_read` `call_2001e57bed190cd55230ea83`、`mcp` `call_b654abf0c130fad95d8a3819`。
- 撤除关联调用为 `native` `call_23c0bb4f8857a578a21ad4d7`、`native_read` `call_05c9e8e7a2f8bbe5d56f250f`。
- 拒绝对照关联调用为 `native` `call_ab0dd6710bb9d9d1718458ec`、`native_read` `call_3f318a7f25530c6378e4a9af`、`selected` `call_59810d898474dbf8520b2eb6`、`selected_read` `call_3b9ca7e1afa0b38c37971ee9`、`mcp` `call_265e20b407a5f78d826a5cae`。

网关权限回读的 `support` 字段为 `unavailable`，上述逐次决定取自
`desktop-acp` 待审批和回复，执行数由只读 stdio 服务记录。
本轮未运行 Rust 单测或桌面构建。
