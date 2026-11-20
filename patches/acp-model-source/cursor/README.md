# Cursor 2026.11.11-b912975 会话模型绑定

固定为 Darwin arm64 官方归档，在独立受管安装内应用补丁，不修改用户全局 CLI。安装器校验归档 SHA-256、原始补丁锚点文件以及全部 351 个最终运行文件；Rust 启动前再次核对产物。官方 ACP 启动形态参见 [Cursor ACP 文档](https://prod.cursor.com/docs/cli/acp)。

原版 ACP 依赖 Cursor 服务端推理，打包产物的隐藏本地模型选项未接入 ACP。补丁在原生 `AgentClient.run` 边界增加 Chat 推理与工具续接，保留 Cursor ACP 会话、模型选择、原生 shell 解析器、权限/钩子与执行器以及 protobuf/blob 会话存储。模型目录仅包含当前绑定的模型，不回退 Cursor 官方模型，也不启动其他代理。

本模式公布前台 `shell`、发现技能时的原生只读工具，以及 Cursor 从当前 ACP 会话发现的 MCP 工具。工具调用使用会话工具目录中的标识，经原生执行器和审批处理。ACP 工具完成事件携带原生 MCP 结果，并报告执行错误及拒绝状态。图片、总结历史、非 agent 模式、未知历史节点、并行工具及非正常流终态仍明确拒绝。原生 CLI-local 路径沿用原实现。加载会话时从原生历史恢复工具 ID、结果与推理，当前工具目录由本次 ACP 会话的挂载决定。

当前适配安装在独立的 `model-source-v6` 目录，可通过 ACP 系统安装入口取得。每轮最多执行 2000 次工具。达到上限时，适配器使用已有工具结果发起一次禁止再调用工具的收尾推理。若模型仍请求执行工具，该轮报错。

前台 shell 使用 ACP 新建或加载会话时选定的工作目录。工作目录缺失时拒绝该调用。

受管 ACP 在会话工作目录启动，使 Cursor 扫描本项目的原生技能。受信的 ShiJu 选集经 `SHIJU_ACP_CURSOR_SKILL_ROOT` 追加到 Cursor Agent Store 扫描根。无选集时使用原生目录。模型目录只公布可自动调用的技能元数据，正文和附件通过 Cursor 原生读文件执行器读取。同名的原生与挂载目录会拒绝歧义。私有模型路径维持原有的项目规则、shell 和 MCP 权限。

`SHIJU_CURSOR_MODEL_CONFIG` 只携带权限为 0600 的私有配置路径。配置包含选定模型、本机会话中继地址、可撤销短期令牌与输出上限；原始上游密钥不会交给 Cursor。Cursor 配置与历史使用独立目录，内存凭据存储避免读取用户登录令牌。受管子进程不继承云认证、Node 注入与代理环境；工具审批继续通过当前 ACP 客户端，未添加 force/yolo 或跳过审批。

验证命令：

```sh
node --test patches/acp-model-source/cursor/session-model.test.cjs
python3 scripts/acp_model_source_cursor_tests.py /absolute/path/to/reviewed/cursor-agent
python3 scripts/acp_model_source_cursor_tests.py /absolute/path/to/reviewed/cursor-agent --deny
python3 scripts/acp_model_source_cursor_tests.py /absolute/path/to/reviewed/cursor-agent --past-old-limit
python3 scripts/acp_model_source_cursor_tests.py /absolute/path/to/reviewed/cursor-agent --step-limit
python3 scripts/acp_model_source_cursor_tests.py /absolute/path/to/reviewed/cursor-agent --refuse-final-tool
```

原生回合验证覆盖选定模型、中文 SSE、推理与工具结果续接、会话加载后的历史恢复，以及允许/拒绝审批后的实际文件状态。真实指定模型的编辑器创建证据记于 `docs/dev/acp/ACP验收.md`，不以模拟模型测试代替真实验收。
