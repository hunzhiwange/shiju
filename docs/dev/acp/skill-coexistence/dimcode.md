# DimCode ACP 技能共存验收

任务 28 已阻塞。固定 DimCode 0.5.28 的原生扫描器在只读会话中发现项目技能，但供应商权限层拒绝 `skill` 激活。在同一权限下，原生 Streamable HTTP 完成 MCP 初始化和目录读取，工具调用同样在 ShiJu 审批前被拒绝。

## 制品与直接 ACP 对照

Darwin arm64 上复用受管 `dimcode-0.5.28-model-source-v2/dim acp`。归档 SHA-256 为 `71634686a50bf9510862ec18a8381c49f5caf8df510c67085f2efac1773f04de`，受管入口 SHA-256 为 `ca470dad0e7663b669769ff8d22bc260339eb46fbff08b9dd7fc553ea225f746`。副本通过严格签名检查并返回 `dimcode 0.5.28`。归档冷下载超时，沿用摘要匹配的缓存制品。[已受审的 HTTP/SSE 路径](../mcp-policy-audit/coexistence/dimcode.md)

在隔离 HOME 和项目中，以 `session-selected/selected-model` 绑定私有 Chat Completions 回环。用既有仓库直接 ACP 夹具做只读对照，保持 `read-only`，增加项目 `.agents/skills` 中带随机正文的技能及原生 `skill` 调用。两次真实 ACP prompt 均返回 `end_turn`，进程退出 0。模型请求都列出了技能，原生 `read` 按同一调用 ID 读回项目普通文件。

| 对照 | 关联工具调用 | 原生结果 | 宿主审批 / 下游 MCP |
| --- | --- | --- | --- |
| 无挂载 | `call_before_skill` | 原生技能激活失败 | 0 / 0 |
| 只挂 HTTP MCP | `call_mounted_skill`、`call_mounted_coexistence__read_probe` | 原生技能激活失败，MCP 返回 `Tool call denied: source: default` | 0 / 0，服务实收 `initialize` 和 `tools/list` |

受管入口的 `read-only` 预设明确许可普通 `read`，对其它未许可工具默认拒绝且禁用交互审批。ShiJu 当前只在已有完整访问批准时同步 `workspace-write`。历史正对照在该模式下取得 MCP 允许一次及取消的结果。任务 28 沿用 `full_access=false`，现有技能挂载能力门禁和 stdio 拒绝继续有效。

## 网关探针

新增 `--agent dimcode` 验收入口，从 `defaults.skills` 和宿主 MCP 配置选择夹具。探针核对实际 Chat Completions 声明，要求原生和选中技能的激活、附件读取以及 MCP 均有同 ID 续轮。目标模型为 `coexistence-fixture/selected-model`，来源为 `session_model`。

本轮复用槽位 10 的已验网关，网关 SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。Cargo 两条缓存路径均为仓库的 `target/rust-slot-10`。设置 `ACP_DIMCODE_REVIEWED_BIN` 可指定摘要和签名相符的受管入口副本。

```sh
PYTHONPATH=scripts python3 -c 'import sys; from cargo_slot import run; sys.exit(run(sys.argv[1:], number=10))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent dimcode --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/dimcode.json'
```

- 正式无挂载轮退出 1，零 `chat.done`、一次 `chat.error`，未到达带工具的模型请求。只出现无工具的辅助模型请求。网关报告供应商会话在 `session/resume` 与 `session/load` 中均找不到，切换到私有项目作为网关工作目录仍复现。
- 独立挂载轮退出 1，零有效模型请求、审批及 MCP 执行，首个断点为 `dimcode: no verified skill directory mount branch`。脱敏报告为 `target/acp-skill-coexistence/dimcode-mount-gate.json`。
- Python 语法检查通过。无 Rust 改动，未启动 Cargo 构建或 Rust 单测。撤除、宿主审批拒绝对照和成功模型及权限回读均未取得。

继续接入需要在原只读边界内可调用的原生技能激活及可提交宿主逐次审批的远程 MCP 执行入口。先验证这两条原生路径，再评估追加会话发现根。
