# Pi 0.0.22 技能共存验收

2026-11-20 在 Darwin arm64 复核 `pi-0.0.22-model-source-v2` 和 Pi 0.73.1。受管安装器在 0700 私有目录完成安装，核对 npm 制品完整性、固定源码提交和锁文件。探针在新私有目录再次核对包名、版本和五项 SHA-256。[受管合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L162)

| 文件 | SHA-256 |
| --- | --- |
| `pi-acp/dist/index.js` | `579df420c0a8d19f72bf34343e592174ff50d40776fda6311c04f488a44dad60` |
| `pi-acp/dist/shiju-mcp-extension.mjs` | `ec5c5f34a37d3d3d0767e3341420c95c57878f1abf308f531545dde5cfab8c8e` |
| `pi-acp/dist/shiju-mcp.py` | `ede7bf31f0eafe51f6dd0572d0cea482fe3be3eb715534b2f566d87e5c8aa915` |
| `pi-acp/dist/shiju-stdio.py` | `e6870d58c6c59cb1f1d224a66cdaa8455b34057249912b7bce5b9e21f95a074a` |
| `@mariozechner/pi-coding-agent/dist/cli.js` | `e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa` |

## 私有网关

复用已构建的 `shiju`，SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。两条 Cargo 缓存路径均为仓库下的 `target/rust-slot-10`。请求指定 `full_access=false`、`session_model` 和 `coexistence-fixture/deepseek-v4.1-flash-1017`。挂载与拒绝对照通过 `defaults.skills` 及宿主 MCP 配置选择夹具。

| 轮次 | 新会话 | `chat.done` | `chat.error` | 模型请求 | 下游 stdio 调用 |
| --- | --- | ---: | ---: | ---: | ---: |
| 基线 | `ses_e078d83f5ffeDyNO6FUU6xPUVq` | 0 | 1 | 0 | 0 |
| 挂载 | `ses_e078d6a69ffeh79OY9YJTkvW9Z` | 0 | 1 | 0 | 0 |
| 撤除 | `ses_e078d5143ffegv4pM7PqxsQs3a` | 0 | 1 | 0 | 0 |
| 拒绝对照 | `ses_e078d359cffeVUsQiZ4RL7Ojyi` | 0 | 1 | 0 | 0 |

标准验收止于基线，退出 1。其余三种请求各在新网关独立运行，也分别退出 1。四次首个断点均为 `requires_full_access`。宿主在准备技能视图及启动 Pi 前拒绝请求，因此未取得模型和权限回读、网关工具调用 ID 或 MCP 审批。原生 Bash、编辑和写入可以绕过宿主审批，当前受限入口依赖启动门禁。[宿主检查](../../../../crates/libs/llm-runtime/src/acp/request.rs#L187) · [Pi 权限审计](../mcp-policy-audit/5.md#pi)

标准报告位于 `target/acp-skill-coexistence/pi.json`，独立轮次位于同目录的 `pi-{mounted,removed,rejected}.json`。共享验收判定的 5 项 Python 单测通过。此次复用已有网关构建，未重新编译或运行 Rust 单测。

## 固定制品直连对照

在私有工作区对固定制品直接发送 ACP 请求，模型沿真实 OpenAI Chat Completions 协议回环。ACP 目录取自 `session/new` 的启动信息，RPC 目录取自 Pi 发出的首条带工具模型请求。五次提示均返回 `end_turn`。MCP 沿原受审 macOS stdio 沙箱运行，探针在沙箱可写的临时目录观察实际 `tools/call`，临时目录在调用完成后回收。

| 状态 | ACP 和 RPC 共同目录 | 下游 stdio 调用 |
| --- | --- | ---: |
| 原生目录，未设会话根 | 原生技能 | 0 |
| 空会话根 | 空 | 0 |
| 挂载并批准 | 仅选中的 ShiJu 技能 | 1 |
| 撤除 | 空 | 0 |
| 挂载并拒绝 | 仅选中的 ShiJu 技能 | 0 |

- 原生技能正文和附件分别在 `probe5552f69707ab0fc3e55a0b55`、`probe662ff050261cb616fb65050d` 的模型续轮出现。挂载轮的对应 ID 是 `probe966bc49ba135da010fdbe0f5`、`probedef1f3810aabf6be047a32b7`。
- 批准轮以 `probe51f34ce7bc200706954a8e35` 关联 ACP 审批及模型结果。拒绝轮以 `probe5f32c5bc6607a44444935c6f` 关联 ACP 审批及拒绝结果，下游无调用。非选中技能始终未出现在目录中。
- 现行补丁以会话根替换 ACP 目录，在 RPC 侧以 `--no-skills --no-extensions` 配合单个显式 `--skill` 根。直接对照定位了两个目录共同缺少原生技能的断点。结果保存在 `target/acp-skill-coexistence/pi-direct.json`。[固定补丁](../../../../patches/acp-model-source/pi/pi-acp-0.0.22-state-dir.patch) · [Pi 0.73.1 技能加载源码](https://github.com/earendil-works/pi/blob/781152fc24841dc54b22284514604048ebe5e2c9/packages/coding-agent/src/core/skills.ts)

## 受管 v3 聚焦验证

固定 Pi 0.73.1 的 `DefaultPackageManager` 解析当前项目技能设置。适配器只将词法路径和实际目标均位于项目 `.pi/skills` 的已启用文件作为显式 `--skill` 参数，随后追加宿主验证的会话根。Pi 原生规则决定同名优先级，ACP 启动及命令目录读取实际 RPC 结果。受管启动保留 `--no-skills --no-extensions`，按需加载既有会话 MCP 扩展。普通 CLI 本地模型延续会话视图行为。

从同一固定源码提交和锁文件构建受管 v3。TypeScript 检查和构建退出 0，重新解包源码、应用仓库补丁后得到相同的 `dist/index.js` 字节，SHA-256 为 `625fb0af3d7170a000c9ce763355091c64ce67ad6be00abd548e49f1dcf05001`。引擎和三个 MCP 组件沿用上表的摘要。直连探针先校验制品版本和全部摘要，再通过私有回环请求验证：

| 轮次 | ACP 会话 | 终态 | 同 ID 续轮结果 | 下游调用 |
| --- | --- | --- | --- | ---: |
| 基线 | `01a1f990-333d-710a-b9fc-e46f36f1344b` | `end_turn` | 原生正文与附件 | 0 |
| 挂载 | `01a1f990-382c-702e-8c99-17ae03f14b38` | `end_turn` | 两类正文与附件、MCP | 1 |
| 撤除 | `01a1f990-3d50-726e-9be4-930dd8f5b22e` | `end_turn` | 原生正文与附件 | 0 |
| 拒绝 | `01a1f990-423d-715c-b29f-d4b0e00abf66` | `end_turn` | 两类正文与附件、MCP 拒绝 | 0 |
| 撤除后重载 | `01a1f990-382c-702e-8c99-17ae03f14b38` | `end_turn` | 原生正文与附件 | 0 |

挂载与拒绝轮分别以 `probea09648165e2bad2c83759e8f`、`probea06f1719139d6dc0518542ae` 关联 ACP 审批和模型续轮。全部新会话首条模型请求均只有目录元数据。原生显式禁用技能、视图外来源和越界符号链接均未入目录，冲突时读取原生定义，工作区自动扩展未执行。重载保留既有历史，新目录只显示原生技能。

```sh
VIBEWINDOW_CONFIG_DIR=<私有制品目录> PYTHONPATH=scripts python3 -m acp_skill_coexistence.pi_direct
```

结构化结果见 `target/acp-skill-coexistence/pi-v3-direct.json`。当前共享代码的请求和启动检查仍对 Pi 要求完全访问。仅用 `--tools read` 约束模型工具表仍可由原生读工具读取工作区外文件，现有 stdio 沙箱只约束 MCP 子进程。直连探针覆盖供应商路径，桌面网关仍需受审的受限 Pi 启动契约。该契约应约束原生文件读取和启动扩展，保留活动会话的逐次 MCP 审批，并在拒绝时禁止下游执行。当前审批档下的网关断点未变化，本轮复用这些失败证据，未重复构建网关或运行 Rust 单测。[启动检查](../../../../crates/libs/acp-client/src/client/process.rs#L33)

## 受管 v4 受限启动

Darwin arm64 的桌面读取审批档可进入固定 `session_model` 路径。宿主校验适配器摘要及同安装目录的 Pi 引擎。Seatbelt 仅启动原生 RPC 子进程，读取范围为项目和经宿主验证的技能目标，写入限当前 Pi 私有状态，网络限模型回环端口。原生工具表只保留 `read`，选中的 MCP 继续由 ACP 适配器通过逐次审批及原 stdio 沙箱执行。全拒绝、禁用工具、非受审平台及 CLI 本地模型均不使用这个入口。

受管入口 SHA-256 为 `720185f6ac86542d8ceddbb8e9cbbff6320c39334168fe1d2f3525780878f847`，固定源码 TypeScript 检查及构建退出 0。共享探针 Python 单测 5 项通过。

| 场景 | ACP 会话 | 终态 | 下游调用 |
| --- | --- | --- | ---: |
| 基线 | `01a1fbb8-59b0-7701-888d-e68c7a50ff02` | `end_turn` | 0 |
| 挂载 | `01a1fbb8-5f0a-7070-88e0-cb808f87463b` | `end_turn` | 1 |
| 撤除 | `01a1fbb8-6682-73a0-8302-78633488619c` | `end_turn` | 0 |
| 拒绝 | `01a1fbb8-6cc6-734e-b451-a1111b46d8ed` | `end_turn` | 0 |
| 撤除后重载 | `01a1fbb8-5f0a-7070-88e0-cb808f87463b` | `end_turn` | 0 |
| 审批中取消 | `01a1fbb8-78e5-77a0-82d1-a134fac49e2e` | `cancelled` | 0 |
| 越界绝对路径 | `01a1fbb8-7dba-7583-9c0c-3bcdfe89c8e0` | `end_turn` | 0 |
| 越界符号链接 | `01a1fbb8-82dc-7597-8117-2386aec4319c` | `end_turn` | 0 |

直连挂载轮从工作区外的选中技能真实读取正文和附件。批准调用 `probe86224172ad13d2031ddc56b5` 返回随机 MCP 值，拒绝调用 `probe5173b62f4bfd39d0ba006e76` 向模型返回拒绝原因。取消发生在审批等待期间，下游调用为零。真实 Pi `read` 拒绝越界绝对路径及工作区内指向越界文件的符号链接。原生禁用项和未选中项不入目录，自动扩展未运行。系统探针对同一策略验证工作区文件可读、跨界文件及工作区写入被拒、子进程启动和未授权回环端口连接被拒。结构化报告为 `target/acp-skill-coexistence/pi-v4-direct.json` 及 `target/acp-skill-coexistence/pi-v4-os.json`。

复跑系统对照时，在固定源码应用补丁、安装构建依赖后，从仓库根运行：

```sh
node --import "$PI_SOURCE/node_modules/tsx/dist/loader.mjs" \
  patches/acp-model-source/pi/verify-sandbox.mts \
  "$PI_SOURCE/src/pi-rpc/restricted.ts" "$PI_ENGINE"
```

受限网关回归与 Rust 聚焦用例另行记录。
