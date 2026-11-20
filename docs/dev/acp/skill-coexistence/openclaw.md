# OpenClaw 2026.10.10 技能共存验收

2026-11-20 在 Darwin arm64 验证受管修订 `openclaw-2026.10.10-model-source-v2`，入口版本为 `OpenClaw 2026.10.10 (7dc3350)`。本机只有 v1，故在 0700 私有目录复制该制品，校验并还原扫描器的上游字节，再应用仓库现行 v2 补丁。探针校验包名、版本、npm 入口及合约的三个 SHA-256。

| 文件 | SHA-256 |
| --- | --- |
| `openclaw.mjs` | `469e60c9f60b615ff71c985a54238ee532a8f949866d76aebe7d6e4bc7219902` |
| `dist/workspace-skill-sources-DqAcMM_z.mjs` | `db073f36c9ac2d94a9a51b5076bbe6e3ae780c67ec820e81ef000bef1bb83459` |
| `dist/mcp-stdio-transport-Dlawzoy1.mjs` | `5bc81023917314bfba0ab85f6987909845a44ae43cfbde2e60038a71bf100b35` |

## 私有 ShiJu 网关

从槽位 3 复制已构建的 `shiju`，SHA-256 为 `403bb2827df94239bc831633f3cd3a1d10dd832594fe9d8471cc1c8bde2ebacf`。探针持有槽位 9，构建与目标路径均为仓库内的 `target/rust-slot-9`。请求指定 `session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false`。挂载和拒绝对照通过 `defaults.skills` 及工作区 MCP 配置选取夹具，使用受审的只读 stdio 传输。

| 轮次 | 会话 | `chat.done` | `chat.error` | 模型请求 | 下游 MCP |
| --- | --- | ---: | ---: | ---: | ---: |
| 基线 | `ses_e079b1435ffep4x6rc1bQ7kCQ3` | 0 | 1 | 0 | 0 |
| 挂载 | `ses_e079a6730ffeSujIOnqttA18pK` | 0 | 1 | 0 | 0 |
| 撤除 | `ses_e079a0c25ffer55BKdpquhotWY` | 0 | 1 | 0 | 0 |
| 拒绝对照 | `ses_e0799c4b0ffeDWmeh25yQA7Pjn` | 0 | 1 | 0 | 0 |

标准验收在基线退出 1。其余三种配置各使用一个新网关和会话单跑，均退出 1。首个断点均为 `requires_full_access`，由 `ensure_desktop_native_access` 在供应商启动前返回。拒绝对照也止于启动门禁，未到达逐次审批。模型绑定、权限状态回读和调用 ID 均未取得。结果保存在 `target/acp-skill-coexistence/openclaw.json` 及同目录的 `openclaw-{mounted,removed,rejected}.json`。

## 供应商加载器对照

在同一私有 v2 副本中，用原生异步运行时加载器单独扫描临时工作区。未设会话根时发现工作区原生技能，空根时目录为空，投影会话根时只发现选中技能。非选中技能始终不可见。直接调用同步目录读取接口时，投影软链接未被接收。异步路径会传递补丁设置的目标白名单，加载结果见 `target/acp-skill-coexistence/openclaw-scanner.json`。这组诊断验证了目录差异，网关中的正文和附件回读仍无实测结果。

恢复本项需要兼容现有审批约束的原生工具控制及 MCP 拒绝路径，以及可与会话视图合并的受信原生根。当前私有 Gateway 的 MCP 投影依赖完全访问，ACP 桥没有逐次 MCP 审批。现有生产配置、制品合约和 `cli_local` 门禁保留。 [OpenClaw 权限与传输审计](../mcp-policy-audit/3.md#openclaw)

Python 共享验收判定的 5 项定向测试通过。复用已有网关，未重新构建或运行 Rust 单测。
