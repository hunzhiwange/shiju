# Cursor 2026.11.11-b912975 会话模型绑定

固定为 Darwin arm64 官方归档，在独立受管安装内应用补丁，不修改用户全局 CLI。安装器校验归档 SHA-256、原始补丁锚点文件以及全部 351 个最终运行文件；Rust 启动前再次核对产物。官方 ACP 启动形态参见 [Cursor ACP 文档](https://prod.cursor.com/docs/cli/acp)。

原版 ACP 依赖 Cursor 服务端推理，打包产物的隐藏本地模型选项未接入 ACP。补丁在原生 `AgentClient.run` 边界增加 Chat 推理与工具续接，保留 Cursor ACP 会话、模型选择、原生 shell 解析器、权限/钩子与执行器以及 protobuf/blob 会话存储。模型目录仅包含当前绑定的模型，不回退 Cursor 官方模型，也不启动其他代理。

本模式只公布前台 `shell` 工具，供模型检查、创建和编辑文件；不是原生远程工具集的完整替代。未接入的远程服务明确失败，MCP、图片、总结历史、非 agent 模式、未知历史节点、并行工具及非正常流终态明确拒绝。原生 CLI-local 路径保持原实现。推理正文与 `reasoning_content` 保存在原生历史中；加载会话后恢复工具 ID、结果与推理。

`SHIJU_CURSOR_MODEL_CONFIG` 只携带权限为 0600 的私有配置路径。配置包含选定模型、本机会话中继地址、可撤销短期令牌与输出上限；原始上游密钥不会交给 Cursor。Cursor 配置与历史使用独立目录，内存凭据存储避免读取用户登录令牌。受管子进程不继承云认证、Node 注入与代理环境；工具审批继续通过当前 ACP 客户端，未添加 force/yolo 或跳过审批。

验证命令：

```sh
node --test patches/acp-model-source/cursor/session-model.test.cjs
python3 scripts/acp_model_source_cursor_tests.py /absolute/path/to/reviewed/cursor-agent
python3 scripts/acp_model_source_cursor_tests.py /absolute/path/to/reviewed/cursor-agent --deny
```

原生回合验证覆盖选定模型、中文 SSE、推理与工具结果续接、会话加载后的历史恢复，以及允许/拒绝审批后的实际文件状态。真实指定模型的编辑器创建证据记于 `docs/dev/acp/ACP验收.md`，不以模拟模型测试代替真实验收。
