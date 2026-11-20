# aionrs 技能共存验收

2026-11-20，Darwin arm64。空选集能运行并发现原生技能。受管中继省略原生 `Skill`，选中 ShiJu 技能后的新会话又在宿主能力门禁处被拒绝。保留 v2 的工具权限、沙箱及安装合约。

## 受审入口

原生 `aionrs --version` 返回 0.2.11，合约固定源码提交 `8e61a90329fa9f67c4fdf7e97fe02c24dba33f75`。下载的源码归档与合约 SHA-256 一致。实际运行采用 v2 私有制品副本和原有沙箱，按受审路径从隔离 stdio 到模型中继同端口投影 MCP。 [受管合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [工具权限审计](../mcp-policy-audit/coexistence/aionrs.md)

| 入口 | SHA-256 |
| --- | --- |
| 槽位 6 的 shiju | `8ee6357b62005376c74ef75d26aa03e9379af73f81e9e8b3a63f5552ec768c52` |
| aionrs-acp | `b2ab720e864645b1b0dc681fcf852873c501d3a8f19de7b9a1ffc2408c9a466a` |
| aionrs | `aae767bf2d5132d730a77526132fe3c689ff2cf60c81284d0b7a86f97ffa1662` |

## 实测

| 场景 | chat.done / chat.error | 模型请求 | MCP 执行数 | 结果 |
| --- | ---: | ---: | ---: | --- |
| 空选集，严格验收 | 0 / 1 | 1 | 0 | 原生目录条目已出现，未出现未选项、正文或附件。工具表有 Read、Write、Edit、Glob，缺少 Skill |
| 空选集，Read 对照 | 1 / 0 | 3 | 0 | 对照在实际目录发现后使用原有 Read，经两次单次批准分别取得正文及附件 |
| 选中技能和 MCP，Read 对照继续 | 0 / 1 | 0 | 0 | 宿主返回 `aionrs: no verified skill directory mount branch` |
| 撤除、MCP 拒绝 | 未运行 | — | — | 探针在上述首个失败处终止 |

Read 对照中，正文调用 `call_14dd49389f504f4ee2770d58` 和附件调用 `call_76abe20329fddc5cd94055ee` 均匹配模型续轮的返回 ID。回读为 `acp-coexistence / aionrs`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017`，`full_access=false`，权限状态 `requested=false`。直接连接同一桥接与受管沙箱作对照时，模型收到引擎公布的 `Skill`。调用 `unmounted_0` 得到 `Only workspace file tools are supported`，未产生 ACP 审批请求，工具终态 `failed`，会话 `end_turn`，桥接退出 0。

## 首个断点与续验条件

原生扫描器从当前工作区的 `.aionrs/skills` 发现技能。引擎库有 `AgentBootstrap::extra_skill_dirs`，固定 CLI 的 `build_engine` 没有接入。将选集写入私有 HOME 的原生目录也无法通过现有激活路径，且会把选集作为优先级更高的用户级目录。网关中继只转发四个工作区文件工具和桥接已发现的 MCP 项，直接 ACP 对照又确认桥接在审批之前独立拒绝 `Skill`。 [原生扫描器](https://github.com/iOfficeAI/aionrs/blob/8e61a90329fa9f67c4fdf7e97fe02c24dba33f75/crates/aion-skills/src/loader.rs) · [引擎启动](https://github.com/iOfficeAI/aionrs/blob/8e61a90329fa9f67c4fdf7e97fe02c24dba33f75/crates/aion-cli/src/bootstrap.rs) · [模型中继](../../../../crates/libs/model-relay-http/src/aionrs_mcp.rs) · [桥接审批](../../../../crates/libs/acp-server/src/aionrs/backend.rs)

继续需要审定原生只读激活方式、目录优先级和会话视图附件的只读路径。固定版 `Skill` 会处理技能内嵌命令、hooks 和上下文修改，直接加入两个工具名单会放行未经审定的工具行为。当前 stdio 夹具的审计文件也位于 MCP 沙箱可写目录之外，后续必须用沙箱允许的审计路径核对拒绝后零调用。 [原生激活](https://github.com/iOfficeAI/aionrs/blob/8e61a90329fa9f67c4fdf7e97fe02c24dba33f75/crates/aion-agent/src/skill_tool.rs) · [MCP 子进程沙箱](../../../../crates/libs/acp-server/src/aionrs/mcp.rs)

## 复现

设置 `ACP_AIONRS_REVIEWED_DIR` 指向包含上述两个受审制品的目录。默认读取本机 v2 安装目录，探针仅复制制品到私有网关配置目录。

```sh
python3 -c 'import sys; sys.path.insert(0, "scripts"); from cargo_slot import run; raise SystemExit(run(sys.argv[1:], number=6))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent aionrs --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/aionrs.json'
```

复用已有槽位 6 的网关，两个 Cargo 缓存路径均为仓库内 `target/rust-slot-6`。严格验收与 Read 对照均退出 1。脱敏报告为 `target/acp-skill-coexistence/aionrs.json` 和 `target/acp-skill-coexistence/aionrs-read-control.json`。本次未改 Rust，未重编网关、未执行 Rust 定向单测。
