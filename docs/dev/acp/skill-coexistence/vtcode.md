# VT Code ACP 技能共存验收

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

复核使用以下命令，退出码 0 表示对照执行完整。结构化报告为 `target/acp-skill-coexistence/vtcode-native-control.json`。共存验收仍沿用此前退出码 1 的证据，未重复构建网关或运行同一验收。

```sh
PYTHONPATH=scripts ACP_VTCODE_REVIEWED_BIN=/path/to/vtcode-acp \
  python3 -m acp_skill_coexistence.vtcode --original /path/to/vtcode \
  > target/acp-skill-coexistence/vtcode-native-control.json
```

继续接入所需制品是 Darwin arm64 的新受审 0.183.0 补丁二进制及其 SHA-256。其 ACP 入口须连接供应商技能扫描器，追加请求绑定的受信目录，并在既有权限回合内按名称加载说明及读取已激活技能的资源。通用 CLI 激活会注册技能衍生工具，需在 ACP 明确约束此分支，维持现有普通工具范围、沙箱及会话 MCP 单次授权。当前原包及 v2 均不满足该制品条件。
