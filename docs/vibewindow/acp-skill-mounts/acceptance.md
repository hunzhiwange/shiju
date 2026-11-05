# ACP 技能目录挂载交付验收

2026-11-06，Darwin-arm64。依赖批次 32 已满足：任务 33 提交 `87d22d4c6` 的本轮证据，经当前源码 9 项散列及原始报告回读核实。此次仅发布文档，不重复运行 CLI/Cargo，不领取新槽位。

## 验收范围

16 项固定发行物通过 31 次真实 CLI/模型来源生命周期回归；Kimi 以明确负向证据跳过，不计成功或失败；Auggie、Cursor、Droid、Qoder、Trae 由用户明确决定免登录、免验证并接受交付。五项没有实测证据，保留原 candidate/unverified，不承诺其技能挂载支持。

| agent | 固定版本 | 结论 |
| --- | --- | --- |
| auggie | 0.36.0 | 用户豁免（未实测） |
| claude | 0.26.0 | verified |
| codex | 2.5.0 | verified |
| codebuddy | 2.179.3 | verified |
| goose | v1.57.0 | verified |
| vibe | 3.0.4 | verified |
| hermes | 0.19.0 | verified |
| snow | 0.9.14 | verified |
| copilot | 1.0.105 | verified |
| cursor | 2026.10.22-27dcf5f | 用户豁免（未实测） |
| droid | 1.3.0 | 用户豁免（未实测） |
| gemini | 0.66.0 | verified |
| iflow | 0.5.19 | verified |
| kilocode | 7.8.18 | verified |
| kimi | 2.5.0 | unsupported |
| kiro | 3.0.0 | verified |
| opencode | 1.19.5 | verified |
| openclaw | 2026.10.10 | verified |
| pi | 0.0.22 | verified |
| qoder | 1.1.82 | 用户豁免（未实测） |
| qwen | 0.26.7 | verified |
| trae | 0.121.3 | 用户豁免（未实测） |

OpenClaw 仅 session-model 通过，cli_local 未验收。各固定补丁发行物、engine 版本、散列、来源与工具计数见 [逐项清单](agents.json) 和 [本轮生命周期证据](evidence/lifecycle.json)。不推断 Linux、Windows、其他版本或未审核原始发行物。

## 本轮证据

- 31 次真实出站执行均退出 0，每次有效测试 1；首次模型请求无技能正文，正文、附件、脚本、隔离、添加/删除/V2、取消、空闲更新、权限拒绝与重启均通过。
- 自身服务端两个真实生产进程、19/19 检查通过，stdio/HTTP/SSE MCP 实际工具调用 3/1/1；包括失败保旧、revision 冲突、忙碌拒绝、清空、跨进程恢复与凭据不落盘。
- 生命周期修复红灯 1 项失败（退出 101），修复后 vw-acp 1 项、vw-agent 3 项及 resolve_session 9 项通过；最终全仓 Clippy 退出 0。完整命令、有效数量、退出码与原始脱敏报告位置保留在 lifecycle.json。
- 实际使用共享槽位 2、3、4，绝对路径分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-4`；均已释放锁并保留缓存，最终 Clippy 使用槽位 3。

另运行 `python3 -m unittest discover -s scripts/tests -p test_verify_acp_skill_mounts.py`：46 项通过，退出 0，不计真实 CLI 数量。

[regression.json](evidence/regression.json) 保存本次原始报告回读散列与审查计数。仅外部模型 HTTP 使用 fixture；生产 ACP、CLI、发现、文件、脚本和 MCP 均真实执行。这是同轮证据审查，未将旧轮报告、mock agent、握手或零测试计为通过。

## 客户端与资源边界

最小协商、新建外挂、完整替换/冲突/清空、加载恢复与 MCP 重传示例见 [ACP 对外接入](../ACP-对外接入.md)。路径属于执行端，来源重新审核；出站通过目录视图和原生工具按需读取，展开正文输入明确拒绝，不作文本 fallback。普通助手规则与原生显式技能调用保持既有语义。

更新失败保留旧资源；取消后等待请求结束再更新。关闭会话/进程释放连接和目录视图，持久化仅保留恢复所需描述，不保存 MCP 凭据。客户端关闭 stdio 子进程及其管道，远端 MCP 服务由其所有者管理。原始报告位于构建缓存；分享时只使用仓库脱敏证据，不分享账号配置或完整 MCP 载荷。
