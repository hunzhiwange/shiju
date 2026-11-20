# Kilocode 技能共存验收

2026-11-20，Darwin arm64。当前受管 Kilocode 7.8.18 的入口为
`reviewed/kilocode acp --pure`，SHA-256 为
`19cc66a17aead6565b60dda07f7f7a47a1b73f72e0d91a9456fd94a483cfa7ff`。
探针将入口复制到私有临时配置目录，并校验固定契约中的摘要和
`--version`。探针使用 `session_model` 选择回环模型
`coexistence-fixture/deepseek-v4.1-flash-1017`，经工作区 MCP 配置选择只读
stdio 服务，并保持 `full_access=false`。

| 轮次 | `chat.done` | `chat.error` | 模型请求 | MCP 执行 |
| --- | ---: | ---: | ---: | ---: |
| 基线 | 0 | 1 | 0 | 0 |
| 挂载 | 未到达 | | | |
| 撤除 | 未到达 | | | |
| 拒绝对照 | 未到达 | | | |

首次走查退出 1，结构化报告为
`target/acp-skill-coexistence/kilocode-before.json`。走查复用了已有的
`target/rust-slot-6/debug/shiju`，网关 SHA-256 为
`a33ebf2cf7a35e986065bae9203472f539d2b0a96c32aa52e54d5874427f0f98`。
探针未持有构建槽位，也未重新编译。实际首个断点是
`requires_full_access`。桌面请求在启动供应商进程前即被拒绝，模型和权限
回读、原生调用 ID、会话视图及 MCP 对照均无运行证据。

宿主的 `ensure_desktop_native_access` 要求 Kilocode 拥有当前完全访问授权。
[历史 stdio MCP 验收](../mcp-acceptance/kilocode.md)使用该模式，记录为
`client_only`，没有观察到逐次 MCP 审批。
[固定补丁](../../../../patches/acp-model-source/kilocode/README.md)中的扫描器
仍以会话根替换原生发现，空选择仍禁用技能目录与内置技能。该差异来自制品
源码检查，本轮无法在模型请求中到达。保留原有受管制品和宿主门禁。
恢复任务需要受审的原生工具约束和 MCP 拒绝路径，之后再使用同一探针完成
三轮及拒绝验收。

## 受限启动对照

复核当前共享工作区后，`llm-runtime` 的桌面请求和 `acp-client`
的进程入口仍使用同一完全访问门禁。没有再用相同网关重复构建或探测。
MiMo 和 OMP 的新接线通过私有 HTTP 转接宿主选中的 stdio 服务，其生产
入口和调用授权与 Kilocode 已审的原生 stdio 路径不同。

直连当前重签入口，在私有 HOME、`acp --pure` 和空会话根下复现原生目录
缺失。模型收到一条请求，零次下游 MCP 调用。对照使用固定 npm 原版
SHA-256 `73e6626784cb314308e035f11b8cb8ab251624a593c2a110abeb94869514a237`，
经其原生 `skills.paths` 加载视图。私有重签原型
SHA-256 `08f3e90921807dc58471ba02986130851c4fadae1f92c64627dcf3950828dc6f`
则在同一个原生扫描器后追加视图。三份入口均通过严格码签校验。
原版和原型在各自四个独立会话中运行实际原生 `skill`、`read` 和 stdio
MCP，只用回环替换模型。挂载请求通过真实 `session/request_permission`
提交 `{"mcpInput":{}}`，只批准指定的只读工具一次。拒绝时供应商发送
同调用 ID 的失败更新并结束提示。探针据此校验原生审批结构与结果。
[固定版原生扫描器](https://raw.githubusercontent.com/Kilo-Org/kilocode/v7.8.18/packages/opencode/src/skill/index.ts)

| 轮次 | ACP 终态 | 模型请求 | 下游 MCP |
| --- | --- | ---: | ---: |
| 基线 | `end_turn` | 3 | 0 |
| 挂载 | `end_turn` | 6 | 1 |
| 撤除 | `end_turn` | 3 | 0 |
| 拒绝 | `end_turn` | 5 | 0 |

直连回执位于 `target/acp-skill-coexistence/kilocode-{production-v1,upstream,prototype}.json`。
这些对照均使用私有目录和模型夹具。正式网关的选模和权限回读尚未取得。

复用现有隔离方式，在同一原型上逐项验证 Seatbelt 的约束。禁止本机子进程时，
固定原生回环端口后 `session/new` 仍在目录服务返回 `-32603`，日志证实
`posix_spawn git` 返回 `EPERM`。启用原生索引禁用选项仍复现。
私有诊断中允许 fork 和固定系统 Git 后，`skill` 尝试下载 ripgrep 15.1.0，
并被出站限制阻止。再加入本机已存在的 ripgrep 私有副本，基线完成原生正文和
附件读取，挂载轮仍在模型首个目录缺少 stdio 工具，零次下游调用。证据位于
`target/acp-skill-coexistence/kilocode-seatbelt{,-git,-rg}.json`。
私有诊断未发布重签原型，也未改变生产安装和权限。

生产验收需要一个受审的隔离制品或启动合同，固定 Git 和 ripgrep 助手并提供
可受宿主逐次授权的 stdio 服务启动方式。若改用既有 HTTP 转接，需要扩展本卡
已审的供应商传输范围，并单独核对转接的调用身份及拒绝路径。Python 探针
语法检查通过。本轮未修改 Rust 或重新运行 Cargo。
