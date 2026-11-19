# Cursor 官方工具与 MCP 共存复验

2026-11-19，Darwin arm64 上的受管 Cursor v5 通过直接 ACP 的无挂载、挂载、撤除三轮。使用 Chat Completions 回环夹具、原生 Cursor 执行器和真实的只读 stdio 服务。三轮使用同一个私有工作区，分别写入独立的随机读回标记。原生工具读工作区内文件，MCP 读工作区外的独立附件。提示和模型预置响应均不含标记。

## 固定入口

- 注册 key 为 `cursor`，命令为 `cursor-agent acp`。受管 `session_model` 验证入口和参数，追加 `--model <alias>`。本轮使用 `cursor-agent acp --model selected`，`--version` 返回 `2026.11.11-b912975`。
- 将已有受管 v5 制品复制到私有目录，按当前合约核对全部 351 个文件，均匹配。入口 `dist-package/cursor-agent` 的 SHA-256 为 `2ccc9a8e167797641448b5e5c936f006ba137a2555f117f38c5eb76a5238a233`。受管安装目录名为 `cursor-2026.11.11-b912975-model-source-v5`。固定官方归档的预期 SHA-256 为 `2c60b58fb7d9a54c7cb9460f0cbb10d451d4af025d4451f252e5f1d956f8c66f`。
- 私有冷安装在接收约 10.9 MiB 归档时主动中止，退出 130。首个未验证步骤是完整归档下载和摘要校验。三轮使用独立私有副本中已校验的受管制品。
- 按 `59d6447242` 的合约复核 v4 的 351 个文件并运行同一探针。两版的入口、原生执行模块及文件清单一致，唯一变化的运行文件为 `dist-package/session-model.cjs`，对应 `cf2512de4a` 将工具上限由 256 提高到 2000。实测两版模型目录一致，普通轮公布 `shell`，挂载轮增加 `coexist_probe-read_probe`。v5 保留 v4 的前台 shell、会话工作目录、原生 MCP 审批和有界续轮，v5 步数边界的实测沿用历史记录。
- ACP 协议为 v1，默认 agent 模式，凭据使用内存存储，模型配置为 0600 的私有文件，端点为 `127.0.0.1` 随机端口。宿主按次返回 `allow_once`，每次正向调用均出现原生审批请求。没有提供 force/yolo 或扩大授权。当前补丁将已发现的工具和 `shell` 合并，并在目录歧义时拒绝。服务内允许和拒绝名单缺少通用投影，本轮只验证按次原生审批。

## v5 三轮

| 轮次 | ACP 会话 ID | 官方 `shell`，Cursor 原生执行器 | `coexist_probe-read_probe`，本地 stdio 服务 | 续轮与终态 |
| --- | --- | --- | --- | --- |
| 无挂载 | `d0894414-2e1f-4264-8dc9-32e94065bd3e` | `call_before_shell`，完成并读回本轮文件 | 目录无该工具，服务零请求 | 2 次模型请求，`end_turn` |
| 挂载 | `bccd2b6c-71d3-4582-b83f-63d0cdcee79a` | `call_mounted_shell`，完成并读回本轮文件 | `call_mounted_mcp`，完成并读回独立附件，服务 RPC ID 2 | 3 次模型请求，`end_turn` |
| 撤除后新会话 | `666a68fa-b8a5-4a2f-9745-2a7531f925c8` | `call_removed_shell`，完成并读回本轮文件 | 目录无该工具，服务审计零新增 | 2 次模型请求，`end_turn` |

两类调用的 ACP 状态均由 `pending` 经 `in_progress` 到 `completed`。官方工具实际执行 `/bin/cat native-probe.txt`。目标服务在挂载轮收到 `initialize`（RPC ID 0）、`tools/list`（1）和 `tools/call`（2）。挂载轮第二次模型请求收到 shell 的原 ID 及读回标记，第三次请求收到两项同 ID 结果并返回确认。两个对照轮的第二次请求各自收到本轮 shell 结果。

## 拒绝、取消和加载

- 显式拒绝会话 `9e99841d-d121-4c22-98e6-6dc6ce919318` 中，宿主对 `call_denied_mcp` 返回 `reject_once`。ACP 标记 `failed`，下一次模型请求按原 ID 收到拒绝原因，随后 `end_turn`。服务仅初始化并列目录，零执行。
- 取消会话 `e0311c10-01d1-4c42-aa64-cbedd75c9459` 中，`call_cancelled_mcp` 等待审批时发送 `session/cancel`，提示以 `cancelled` 结束。最后工具状态为 `in_progress`，无续轮，服务零执行。
- 断开后在两个独立进程以原挂载会话 ID 调用 `session/load`。重传服务时，`call_loaded_shell` 和 `call_loaded_mcp` 各读回本次新标记并按原 ID 进入续轮，服务调用为 RPC ID 2。传空列表时，仅公布 shell，`call_withdrawn_shell` 读回新文件标记，目标服务零请求。两轮恢复均以 `end_turn` 结束。加载通知给旧工具分配了 `replay-*` 显示 ID，模型历史仍保留 `call_mounted_shell` 和 `call_mounted_mcp` 及原输出。入口声明 `loadSession: true`，未声明 resume，实测 `session/resume` 返回 `-32601`。
- v4 和 v5 的行为探针均退出 0。两版中，承载三轮的进程和重传挂载的进程关闭 stdin 后均超过探针的 5 秒退出期限，由探针终止，子进程退出码 `-9`。传空挂载列表的加载进程均退出 0，v5 的独立 resume 探测进程也退出 0。带挂载连接的自然退出仍缺通过证据。

## 验证范围

`node --test patches/acp-model-source/cursor/session-model.test.cjs` 退出 0，6 项通过。本轮未确认适配器裁剪缺陷，未修改适配代码。当前证据限受管 Darwin arm64 私有 Chat 路径，未运行 Cargo、当前网关或真实供应商模型。未补证固定官方原包的供应商选模及 MCP 完整闭环。macOS Intel 和 Linux 有官方 Unix 安装入口，Windows 有官方 PowerShell 安装入口，均未在本轮运行。

本机私有目录 `/tmp/shiju-cursor-coexist.NPZDuQ` 保留探针、ACP 轨迹、模型续轮请求及 stdio 审计。可从仓库根目录执行 `python3 <私有目录>/probe.py <受审入口> <新建私有证据目录> "$PWD"`。探针复用 `scripts/acp_mount_integration.py` 的只读服务，并额外记录 MCP RPC ID。

[现行验收](../../ACP验收.md) · [固定版历史及网关三轮](../../mcp-acceptance/cursor.md#新标准三轮通过) · [分组审计](../6.md#cursor)
