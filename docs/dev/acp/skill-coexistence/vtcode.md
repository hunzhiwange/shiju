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
