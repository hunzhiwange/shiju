# VT Code ACP 技能共存验收

## 受管网关联验

Darwin arm64 上的受管 VT Code 0.183.0 v3 已通过四轮真实 ACP 验收。私有网关 SHA-256 为 `54089fcae85172aeca2459440eae5fbb8c041c774d64ff9b045b669219044e19`。库和 CLI 均在槽位 3 使用 `--locked -j 1`，Cargo 目标和构建路径相同。验收退出 0。

| 轮次 | 独立会话 ID | chat.done / chat.error | 模型请求 | 审批 | MCP 执行 |
| --- | --- | --- | --- | --- | --- |
| 无挂载 | `ses_e039bbc9effeiU0x8H1Fj8kJI1` | 1 / 0 | 3 | 无 | 0 |
| 挂载 | `ses_e039b9784ffeKmb2qZ19yOcLCs` | 1 / 0 | 6 | once | 1 |
| 撤除 | `ses_e039b6c18ffe08uy76Fndjge75` | 1 / 0 | 3 | 无 | 0 |
| MCP 拒绝 | `ses_e039b4642ffeuaD7cJ03eugPsP` | 1 / 0 | 6 | reject | 0 |

四轮均回读 `acp-coexistence / vtcode`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false`。首次模型请求只包含技能目录元数据。原生技能在每轮均完成正文和资源回传，挂载及拒绝轮还完成所选技能的正文和资源回传。模型按原调用 ID 收到真实工具结果。拒绝轮返回关联的 MCP 权限拒绝结果，下游执行为零。逐次调用 ID、阶段许可回读和固定制品摘要见 `target/acp-skill-coexistence/vtcode.json`。

首次 v3 网关联验的无挂载轮通过，挂载轮在通用 stdio 夹具初始化时返回 `connection closed: initialize response`。VT Code 探针复用已有适配器的沙箱兼容 stdio 夹具，将脚本放入私有工作区，以只读 FIFO 提供随机附件并在沙箱外计数。同一网关、制品与审批策略复跑通过。

## 修复前与原生对照

2026-11-20 在 Darwin arm64 验证受管 VT Code 0.183.0 v2。补丁入口 SHA-256 为 `8b8f6c5204e1125b20d63c7c9637b027821b22d206517ebd00db5f513890cdfb`。私有网关复用槽位 11 的已构建 ShiJu，SHA-256 为 `69663985a6f90d50109d81e9b211ccb8ab4dfe8e73b67d726cca72ec14ef419a`。宿主通过 `defaults.skills` 和 `defaults.mcp` 解析选择，MCP 服务配置走已有 stdio 至私有 HTTP 投影。仅模型响应由回环夹具提供。

| 独立会话 | chat.done / chat.error | 模型请求 | 下游 MCP 调用 | 结果 |
| --- | --- | --- | --- | --- |
| 无挂载 | 1 / 0 | 2 | 0 | 原生 `list_files` 在续轮按调用 ID 返回项目 `.agents/skills` 下的夹具目录。 |
| 挂载 | 0 / 1 | 0 | 0 | 宿主返回 `vtcode: no verified skill directory mount branch`。 |
| 撤除 | 1 / 0 | 2 | 0 | 原生 `list_files` 再次在独立会话按调用 ID 返回夹具目录。 |
| 拒绝对照 | 0 / 1 | 0 | 0 | 同一宿主门禁阻止建链，未进入 MCP 审批。 |

两次无挂载请求走非流式 Chat Completions。模型可见工具为 `list_files`、`code_search` 和 `apply_patch`，探针只调用 `list_files`。首次请求中没有三个随机技能名、正文、附件或 MCP 返回值。网关回读为 `acp-coexistence / vtcode`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017`，`full_access=false`，权限状态 `requested=false`、`support=unavailable`。

当前阻塞有两层。受管模型回合使用 `vtcode-acp` 的独立目录和会话处理器。固定版 `zed/agent/handlers.rs` 取该目录并追加会话 MCP。原生 CLI 的 `SkillsManager` 和技能工具未接入该处理器。`vtcode-core` 的 `load_skill` 会注册技能衍生工具，源码将其标记为需要审批的变更操作。接入该工具需要验证注册后的工具仍受 ACP 权限、沙箱和中继工具目录约束，也需要受审的会话目录注册及附件读取路径。当前补丁和原包均无已验证的入口，因此保留宿主门禁和现有 MCP 一次性批准路径。 [现有 MCP 补丁](../../../../patches/acp-model-source/vtcode/README.md) · [原生技能工具实现](https://github.com/vinhnx/VTCode/blob/4fd8e47a885c/crates/codegen/vtcode-core/src/tools/skills/mod.rs)

标准验收在无挂载轮以退出码 1 停止。另用同一运行器逐阶段启动私有网关以确认表中其余断点。结构化证据见 `target/acp-skill-coexistence/vtcode.json` 和同目录 `vtcode-*-diagnostic.json`。探针接受 `ACP_VTCODE_REVIEWED_BIN` 指向现有受审制品，复制前后的使用范围限私有验证环境，执行前核对副本摘要与版本。新建构建在共享 Cargo 锁排队期间取消，复用槽位 11，两个 Cargo 缓存环境变量均指向该槽位。产品 Rust 代码和制品未变，未运行 Rust 单测。

后续需要在固定版本建立受审 ACP 技能目录、激活和资源读取路径，并在同一受管沙箱验证权限与批准后再放开挂载。正文与附件调用 ID、MCP 批准后一次执行及拒绝后零次执行仍待验收。

## 2026-11-21 复核原生入口

当前共享工作区的 VT Code 启动驱动、两个制品摘要、VT Code 中继过滤和供应商补丁与首次诊断一致。单独启动摘要匹配的 v1 原包（`88cbc99b8aac0c3a9b8596976c610026aebef14d89f9943735c38016a44919a4`）及 v2 补丁包，使用同一私有工作区、私有 HOME、禁写工作区的 Seatbelt 策略和回环模型。两个制品的 `skills list` 都发现了项目及 HOME 下的随机技能，说明目录位置和沙箱读取有效。

两个制品分别使用 `acp zed` 和 `acp standard`。共 4 个原生 ACP 会话、12 个完成回合。普通请求、`/skills list` 和显式 `$技能名` 均到达模型，但系统上下文没有夹具技能目录，请求没有技能正文或附件，也没有技能加载工具。ACP 宣告的静态 `skills` 命令没有动态夹具项。原始工具目录只有 `list_files`、`apply_patch`、`exec_command` 和 `write_stdin`，对照未调用这些工具，也未修改受管中继过滤。这将首次断点定位在供应商 ACP 会话工具和指令处理器。两种 ACP 目标共享该处理器，独立的 CLI 技能入口无法提供同一 ACP 会话的激活和资源回传。项目配置、私有 HOME 投影及改变注册名均无法替代加载入口。

v1/v2 复核使用提交 `f3ceadd078` 的探针。以下命令退出码 0 表示对照执行完整。结构化报告为 `target/acp-skill-coexistence/vtcode-native-control.json`。共存验收仍沿用此前退出码 1 的证据，未重复构建网关或运行同一验收。

```sh
PYTHONPATH=scripts ACP_VTCODE_REVIEWED_BIN=/path/to/vtcode-acp \
  python3 -m acp_skill_coexistence.vtcode --original /path/to/vtcode \
  > target/acp-skill-coexistence/vtcode-native-control.json
```

继续接入所需制品是 Darwin arm64 的新受审 0.183.0 补丁二进制及其 SHA-256。其 ACP 入口须连接供应商技能扫描器，追加请求绑定的受信目录，并在既有权限回合内按名称加载说明及读取已激活技能的资源。通用 CLI 激活会注册技能衍生工具，需在 ACP 明确约束此分支，维持现有普通工具范围、沙箱及会话 MCP 单次授权。当前原包及 v2 均不满足该制品条件。

## 固定版 ACP 只读加载补丁

沿 0.183.0 的原生元数据扫描和清单解析接通 ACP。项目及私有 HOME 下的 `.agents/skills` 按原生优先顺序发现，再追加本次启动独占的受信快照。工具目录仅公布名称和说明。激活与资源读取沿 ACP read 审批，调用时验证供应商主代理的读路径策略。激活状态仅保存在当前会话，资源路径必须属于已激活技能目录。补丁不运行技能脚本，也不注册由技能衍生的工具。 [补丁与复现方式](../../../../patches/acp-model-source/vtcode/README.md)

源码补丁应用及反向检查通过。Darwin arm64 槽位 3 的首版候选构建退出 0，独立原生 ACP 会话完成 6 个关联工具结果和 7 次模型请求。原生与选中技能的四个正文及附件结果各自回到原调用 ID，未激活资源读取和越界资源读取均被拒绝。另用 Seatbelt 私有夹具确认选中路径可读、不可写，未授权邻接文件保持不可读。网关联验使用相同的快照隔离和既有 MCP 审批。

最终 v3 制品以 Rust 1.100.0 在 Darwin arm64 槽位 3 构建，退出 0。硬编码合约的 SHA-256 为 `1b27c8fd3ee3b58f461db5d61dfccbf4a89c4a03a1efd98c75a9dd828c9d1670`，本地产物位于 `target/acp-skill-coexistence/vtcode-skill-candidate`。相同制品通过全部六次原生工具调用与两个读取拒绝对照，脱敏报告位于 `target/acp-skill-coexistence/vtcode-candidate-smoke.json`。v3 安装位于独立目录，v1 普通路径与 v2 已审 MCP 路径继续可用。
