# aionrs 技能共存验收

## v3 只读接入

受管安装修订升级为 `aionrs-0.2.11-model-source-v3`。安装器核对 0.2.11 固定源码及补丁摘要，再构建引擎。补丁沿用引擎的技能扫描和同名优先级，仅为受管启动接入附加目录，并以只读原生 `Skill` 返回正文和所在目录。隐藏技能、声明 hooks、模型或工具覆盖及 fork 上下文的技能在受管目录中不可调用。只读激活不运行内嵌 shell 或应用上下文修改。 [引擎补丁](../../../../patches/acp-model-source/aionrs/read-only-skills.patch)

只将已验证选集中的工作区技能链接加入本次启动目录的 `.aionrs/skills`。外部技能路径和非受管模型来源的挂载会被拒绝。桥接在 `session/new` 核对选集名称及规范化目标与宿主投影一致。中继公布 `Skill` 后，桥接按该次调用请求 ACP 单次批准。附件通过原生 `Read` 从投影目录解析至工作区内文件。写入、网络、派生进程和 MCP 单次批准均沿用现有边界。 [选集投影](../../../../crates/libs/llm-runtime/src/acp/skill_mounts/aionrs.rs) · [桥接审批](../../../../crates/libs/acp-server/src/aionrs/backend.rs)

### 原生引擎对照

编译后的 v3 引擎在生产沙箱策略下直接运行 JSON Stream 并退出 0。模型首次请求只看到原生与选中技能的目录条目。四次单次批准后，五个 Chat Completions 请求按调用 ID 确认原生正文、原生附件、选中正文和选中附件。对照使用 `Skill`、`Read`、`Skill`、`Read`，未启动网关和 MCP。调用 ID 见 `target/acp-skill-coexistence/aionrs-v3-native.json`。

MCP 夹具在原有 stdio 子进程沙箱中读取私有工作区文件，仅在 broker 的临时目录记录调用，宿主核对记录后才返回结果。独立桥接对照中，批准调用 `allow_1` 实际执行一次，拒绝调用 `deny_0` 执行零次，两次结果均进入对应模型续轮，桥接退出 0。该对照复用了已审 v2 broker，正式探针使用 v3 桥接。

槽位 9 的 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_aionrs -- --test-threads=1` 退出 0，实际运行 1 项。测试覆盖空选集、仅投影选中目录和工作区外来源拒绝。引擎构建和 Rust 定向测试的两个缓存路径均为仓库内 `target/rust-slot-9`。

### 真实网关四轮

2026-11-21，Darwin arm64。主入口退出 0，四轮均用独立网关会话。以下 ID 分别对应模型续轮中的原生激活、原生附件、选中激活、选中附件及 MCP。首次模型请求无随机正文或附件，未选技能始终未进入目录。

| 轮次 | 会话 | chat.done / chat.error | 模型请求 | MCP 决策 / 执行数 | 续轮调用 ID |
| --- | --- | ---: | ---: | --- | --- |
| 空选集 | `ses_e060ef24bffengy9lWfGCcjln7` | 1 / 0 | 3 | — / 0 | `call_73e4e29662b564bb127c7c45`, `call_7d01bdfa221fb3ac088b0eac` |
| 挂载 | `ses_e060ee3e8ffecUWrlnT3a7gPMM` | 1 / 0 | 6 | 单次批准 / 1 | `call_674c1eb73909b90e3225d774`, `call_62ffd3502c667fddac39fe5b`, `call_ffb7b6f8f9b710a560904b16`, `call_fc754aa59c10e80c093bf263`, `call_88310b780bfcb6c2ef4fe5f8` |
| 撤除 | `ses_e060ed1c4ffeZ0i4VKtavS5QSY` | 1 / 0 | 3 | — / 0 | `call_a283e9afc1a5c3fdcd96a0e9`, `call_34207df7c1f46ab56ce8cd93` |
| 拒绝对照 | `ses_e060ec41cffejalUG3w7RLkz5c` | 1 / 0 | 6 | 拒绝 / 0 | `call_b429d54cc2f05ba5c945e3bf`, `call_54ee234b0e05f8c918f63923`, `call_3313edfacd0c50289bb312b3`, `call_39f15ed3bb47fa9a94231ab9`, `call_8ef82b9dad3a115328fdbd48` |

四轮均回读 `acp-coexistence / aionrs`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false`，权限状态 `requested=false`。拒绝发生在原有桥接单次审批，下游调用计数为零。桥接对照还验证了错误目录、遗漏选集和错误名称均在引擎启动前返回 ACP `-32602`。

| 制品 | SHA-256 |
| --- | --- |
| 槽位 3 的 shiju | `b2df327cf4e8cd820216995fce313fb2c00c51fc0615ccb68a2ee1f7d8396d0f` |
| aionrs-acp | `ba9dd1c97821e9c7c80be2b019b5178ea084008d9f6f0bc0851669eb3384276c` |
| aionrs | `950ce9ddbbecb4bd7146590d0ff5fb816e7bcf20542630c7ec7e64fd116de3d5` |
| 只读补丁 | `4c56c494fdddbb82eebe47481c79048b5abafa7135db388532bd117af150a0f1` |

复用含本任务合约与接线的共享网关。桥接执行 `cargo build --locked -j 1 -p vw-acp --bin aionrs-acp` 并退出 0。桥接构建和正式回环持有槽位 3，两个缓存路径均为仓库内 `target/rust-slot-3`。旧默认夹具路径的 Unix socket 路径达 117 字节，挂载轮在建会话时连接失败。探针选用 `/tmp` 下的私有临时目录后通过。正式报告为 `target/acp-skill-coexistence/aionrs.json`。

设置 `ACP_AIONRS_REVIEWED_DIR` 指向 v3 已构建的私有制品目录，其中包含 `aionrs`、`aionrs-acp` 与安装回执，然后在拥有已构建网关的槽位中运行：

```sh
python3 scripts/verify_acp_skill_coexistence.py --agent aionrs \
  --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" \
  --output target/acp-skill-coexistence/aionrs.json
```

## v2 修复前基线

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

## v2 历史复验

以下命令对应提交 `5ef5d99d69` 的 v2 探针。设置 `ACP_AIONRS_REVIEWED_DIR` 指向包含上述两个受审制品的目录。默认读取本机 v2 安装目录，探针仅复制制品到私有网关配置目录。

```sh
python3 -c 'import sys; sys.path.insert(0, "scripts"); from cargo_slot import run; raise SystemExit(run(sys.argv[1:], number=6))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent aionrs --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/aionrs.json'
```

复用已有槽位 6 的网关，两个 Cargo 缓存路径均为仓库内 `target/rust-slot-6`。严格验收与 Read 对照均退出 1。脱敏报告为 `target/acp-skill-coexistence/aionrs-v2.json` 和 `target/acp-skill-coexistence/aionrs-read-control.json`。本次未改 Rust，未重编网关、未执行 Rust 定向单测。
