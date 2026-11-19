# Kilo Code 原生工具与 MCP 共存复验

2026-11-19，Darwin arm64 上固定原版和受管版 7.8.18 均通过独立进程三轮对照。同进程空挂载新会话仍可调用旧 MCP 服务，宿主逐服务名单也缺投影。保留任务卡，待宿主路径复验。 [共存验收](../../ACP验收.md) · [分组审计](../5.md#kilocode) · [历史网关验收](../../mcp-acceptance/kilocode.md)

## 固定制品和权限

| 入口 | 实际启动 | SHA-256 |
| --- | --- | --- |
| 原版 7.8.18 | `@kilocode/cli-darwin-arm64/bin/kilo acp` | `73e6626784cb314308e035f11b8cb8ab251624a593c2a110abeb94869514a237` |
| 受管 7.8.18 | `reviewed/kilocode acp --pure` | `19cc66a17aead6565b60dda07f7f7a47a1b73f72e0d91a9456fd94a483cfa7ff` |

- 两个缓存制品均与合约一致，`initialize` 回读 Kilo 7.8.18、ACP v1、load/resume 及 HTTP/SSE 能力。注册 key `kilocode`，通用原生入口为 `kilocode acp`，未找到本地 CLI 时回退 `npx -y @kilocode/cli acp`。探针直启固定原二进制；浮动 npm 入口未执行。受审安装当前限 Darwin arm64，Linux 未安装受审制品，Windows 私有状态 ACL 门禁显式拒绝，未实机运行这两端。 [注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L556) · [制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L261) · [安装](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/install.py#L683) · [Windows 门禁](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/kilocode.rs#L154)
- 各轮使用私有 HOME/XDG 目录、合成模型标识 `fixture/session` 与轮换的只读文件内容。配置通过 `KILO_CONFIG` 和 `KILO_CONFIG_CONTENT` 注入，关闭自动更新和模型列表抓取，临时令牌通过环境变量引用。回环 Chat Completions 夹具只发确定调用，官方 `read` 读工作区，仓库只读 stdio 服务实际读取另一份项目外附件。ACP 配置回读模式为 `code`，正向三轮沿用原生默认权限，两个工具未发审批请求。宿主桌面路径仍要求现行完全访问授权，历史网关采用 `client_only`。本次为直接 ACP，无桌面授权变更。 [按会话模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/kilocode.rs) · [桌面授权门禁](../../../../../crates/libs/acp-client/src/client/process.rs#L32)

## 独立进程三轮

| 入口和轮次 | ACP 会话 ID 前缀 | 工具、执行方、调用 ID 和状态 | 服务调用 / 提示终态 / 退出码 |
| --- | --- | --- | --- |
| 原版，无挂载 | `ses_e0d49b0` | Kilo `read`，`call_upstream_before_native`，`completed` | 0 / `end_turn` / 0 |
| 原版，挂载 | `ses_e0d499f` | Kilo `read`，`call_upstream_mounted_native`，`completed`；local stdio `read_probe`，`call_upstream_mounted_mcp`，`completed` | 1 / `end_turn` / 0 |
| 原版，撤除 | `ses_e0d4994` | Kilo `read`，`call_upstream_removed_native`，`completed` | 0 / `end_turn` / 0 |
| 受管，无挂载 | `ses_e0d4919` | Kilo `read`，`call_reviewed_before_native`，`completed` | 0 / `end_turn` / 0 |
| 受管，挂载 | `ses_e0d490a` | Kilo `read`，`call_reviewed_mounted_native`，`completed`；local stdio `read_probe`，`call_reviewed_mounted_mcp`，`completed` | 1 / `end_turn` / 0 |
| 受管，撤除 | `ses_e0d48fa` | Kilo `read`，`call_reviewed_removed_native`，`completed` | 0 / `end_turn` / 0 |

每个挂载轮的服务审计均为 MCP RPC ID `2`、`read_probe`、`isError=false`。模型目录声明 `local_read_probe`，最终模型续轮按两个 ACP 调用 ID 收到本轮新生成的原生文件和附件内容。无挂载和撤除进程均未声明目标工具，且正常回读原生文件。

## 选模、Skill、审批和恢复

- 双模型配置下，两版均通过 `session/set_config_option` 切至 `fixture/alternate`，主对话回环收到 `fixture-alternate`，两类工具继续执行。另有辅助请求沿用 `small_model` 的 `fixture-model`。两版均保留所选 Skill；受管版设置绝对 `SHIJU_KILO_SKILL_ROOT` 后，原版可见的项目内对照 Skill 从模型输入移除，原生 `skill` 和 `read` 工具声明保留。此次未调用 Skill 正文。固定补丁职责仍为私有 Skill 根触发扫描限制并重签制品。 [补丁职责](../../../../../patches/acp-model-source/kilocode/README.md)
- 两版各用一个新会话配置目标 MCP 工具为 `ask`。单次允许时 `call_upstream_allow_mcp`、`call_reviewed_allow_mcp` 各成功一次，服务 RPC ID `2`，结果进入续轮。选 `reject_once` 时两版 `call_*_reject_mcp` 均为 `failed`，服务调用为零，提示 `end_turn`。审批等待中发送 `session/cancel` 并返回取消，`call_*_cancel_mcp` 均未完成，服务调用为零，提示 `cancelled`。显式原生 `deny` 时模型目录移除目标工具，无权限请求，服务调用为零。退出码均为 0。
- 两版分别在成功挂载后退出，以相同私有状态重新启动，向 `session/load` 或 `session/resume` 发空挂载列表恢复原会话。下一次提示均为 `end_turn`，模型读取历史结果，目录无目标工具，无新增服务调用，退出码均为 0。load 可重放此前完成事件。同进程 load/resume 未复验。

## 待回归边界

- **同进程撤除断点。** 保持挂载进程、工作目录和模型配置，另发 `session/new` 并传 `mcpServers: []`。两版在新会话首个模型请求仍声明 `local_read_probe`。重复探针在两会话之间更换附件，再让模型调用残留工具。原版 `call_upstream_revoke-live_mcp` 和受管版 `call_reviewed_revoke-live_mcp` 均在新会话中 `completed`，服务 RPC ID `3` 实际读到更换后的内容，结果按同 ID 进入模型续轮，`end_turn` / 0。第一个失败点是同进程空列表新建会话后的工具目录。两版均出现，未发现受管补丁裁剪官方工具。宿主缓存键包含挂载配置，会区分空表与非空表。当前直接探针没有验证宿主切换和旧进程退役，需在任务 43 用实际宿主入口复验。 [客户端缓存键](../../../../../crates/libs/llm-runtime/src/acp/session.rs#L115)
- **逐服务名单断点。** 共享默认项投影按选中的服务生成 MCP 定义，Kilo Code 缺逐服务 `allowedTools` / `deniedTools` 的传递及显式拒绝。按当前投影的无名单 ACP 消息直连重放，两版目标仍声明且实际执行，`call_upstream_host-filter_mcp`、`call_reviewed_host-filter_mcp` 各一次，服务 RPC ID `2`，`end_turn` / 0。此项是静态宿主追踪加直接协议重放，未通过网关提交带名单请求。任务 43 应复验带名单请求的明确拒绝或下游强制执行。历史三工具私有转发器仅限制该服务。 [宿主投影](../../../../../crates/libs/session-engine/src/acp_defaults.rs#L110) · [任务 43](../../../../../.plans/2026-11-19-acp-工具共存/43-集成与证据收口.md)
- 探针和合成轨迹位于本机私有目录 `/tmp/shiju-kilo22.zEyQdL`。重放时将探针复制到新的 0700 目录，在仓库根目录依次运行 `probe.py`、`extra.py`、`declare.py`、`revoke.py`，分别用 `python3 <私有目录>/<脚本> "$PWD"` 启动。最后运行同目录的 `check.py`。本次结果核对覆盖 26 个场景，退出 0。此次仅新增回执，未改运行时代码，未运行 Cargo、网关、桌面构建、真实模型或 HTTP/SSE MCP。
