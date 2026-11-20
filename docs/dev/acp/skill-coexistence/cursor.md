# Cursor ACP 技能共存

2026-11-20 在 Darwin arm64 核对 Cursor `2026.11.11-b912975`。受审归档 SHA-256 为 `2c60b58fb7d9a54c7cb9460f0cbb10d451d4af025d4451f252e5f1d956f8c66f`，私有测试副本逐项核对了 351 个受管文件。入口 `dist-package/cursor-agent` 摘要为 `2ccc9a8e167797641448b5e5c936f006ba137a2555f117f38c5eb76a5238a233`。v6 的 `4119.index.js` 摘要为 `44d84cfe8240ecd11acd93d8ba5e2e4522feacc1142294bcd9305fc871386a77`，`session-model.cjs` 摘要为 `136c5f321b0228c0091cc4e1cc81eb56a310711510773dcb04a9875d5d93a82b`。

v5 使用原生 ACP v1 完成了无挂载的纯文本回合。首个 Chat Completions 请求公布项目技能的路径与描述，正文留待读取。严格探针随后确认 v5 只公布 `shell`，在发起技能读取前以 ACP `-32603` 结束。通过槽位 10 的旧网关启动 v5 时，在受管制品校验处停止，模型请求为零。检查该旧网关内嵌数据确认两项 v5 补丁摘要缺失，因此这次网关结果只作为制品错配的证据。

匹配 v5 合约的槽位 3 网关使用相同严格探针，退出 1。无挂载会话有一次模型请求和一个 `chat.error`，项目技能目录为空。通过将原生进程目录与 ACP 会话目录分离复现，进一步确认固定 ACP 的共享技能扫描取进程目录。该红测记录在 `target/acp-skill-coexistence/cursor-compatible-v5-red.json`，网关摘要为 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。

v6 将 ShiJu 会话视图追加到 Cursor Agent Store 的原生扫描路径，并等待读取完成。受管进程以 ACP 会话工作目录启动。私有模型桥仅公开技能元数据，通过同一原生读执行器读取正文和附件，并将结果按调用 ID 写入 Cursor 会话历史和模型续轮。发现同名的原生与挂载目录时明确拒绝歧义。宿主将挂载限制在受管 `session_model` 和启用工具的会话。拒绝所有工具或显式关闭工具时，私有模型桥省略技能目录和原生读取工具。

## 原生进程验证

四个私有 ACP 进程使用 `deepseek-v4.1-flash-1017` 的回环 Chat Completions 响应。所有回合以 `end_turn` 结束。原生只读执行器回传随机正文和附件，MCP 走真实 stdio 服务与 Cursor 原生审批，拒绝结果也按调用 ID 进入模型续轮。

| 轮次 | 原生正文和附件 | 选中正文和附件 | MCP 审批 | 实际 MCP 调用 |
| --- | --- | --- | --- | --- |
| 无挂载 | 通过 | 无 | 无 | 0 |
| 挂载 | 通过 | 通过 | 允许一次 | 1 |
| 撤除 | 通过 | 无 | 无 | 0 |
| 拒绝对照 | 通过 | 通过 | 拒绝一次 | 0 |

挂载轮退出后删除会话视图的选中链接，并使用新的 Cursor 进程与原数据目录调用 `session/load`。模型续轮保留此前读文件及 MCP 的五个调用 ID 和对应内容。当前技能目录只有原生技能，工具目录只有 `shell` 和 `read`。会话再次以 `end_turn` 结束。另用原生技能的 `disable-model-invocation: true` 验证该条目未出现。禁用读取的受管回合只公布既有 shell，技能目录为空。

`node --test patches/acp-model-source/cursor/session-model.test.cjs` 退出 0，7 项通过，包含原生技能路径和符号链接逃逸检查。

网关结构化证据保存至 `target/acp-skill-coexistence/cursor.json`。

## 当前网关验证状态

槽位 3 中的 v6 网关构建退出 101。共享工作区的 Qoder 请求接线调用了私有函数，并传入与其签名不符的泛型技能视图，触发 E0603 和 E0308。构建在启动探针前结束。首次构建日志保留在 `target/acp-skill-coexistence/cursor-build-qoder-failure.log`。Qoder 接线修复后，在槽位 3 运行 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_cursor -- --test-threads=1`，退出 0，2 项通过。并行的同目标构建随后在 `task-scheduler` 报 E0616。PostgreSQL 工作计划发布代码访问了私有的 `PlanMember.id`，当前源码仍有此访问。Cursor 槽位内的第二次构建尚在等待锁，已取消该等待并核实清理。保留 `target/acp-skill-coexistence/cursor-shared-scheduler-blocker.log`。网关的三轮及拒绝对照等待共享目标恢复后复验。
