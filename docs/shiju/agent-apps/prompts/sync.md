# 保存态同步合同

来源：cc-switch 固定来源快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 `services/prompt.rs`：`sync_to_live`、`sync_all_to_live`、`project_prompt_set_to_path`。

业务入口为 `prompts::sync::sync_all`；实际存储/文件适配为 `PromptsStore::sync_to_live`、`sync_all_to_live`（macOS）。调用者供应已授权执行主机、目录覆盖、环境、事务与每目标授权回调，不能将客户端路径当作授权。普通目标使用项目文件事务；MCode 使用已有专属事务及 UTF-8 字节限长。OpenClaw 仍由 workspace 所有者管理，普通投影拒绝。

同步只读取保存选择，不调用激活、首次导入或 live 回填。无选择时保留文件；读取 live 仅用于事务的预期前像。持有目标锁期间读取目录和 catalog，发布前复验权限及完整 catalog 快照。恢复后的库不会被陈旧 live 内容覆盖。当前持久化合同仅允许唯一 active_id；通用 selection 对多选快照按现有列表稳定顺序选择首项，Published.duplicate_selection 表达已写入但需警告，缺失 ID 拒绝。

Pi 返回 NativePiSelection，Claude Desktop 返回 UnsupportedClaudeDesktop；不执行普通投影。目标失败仍继续处理后续目标。每个目标产生 TargetResult，错误结果不包含原文和底层敏感错误。调用者通过 emit 将结果接入既有事件/持久化所有者。取消复用 lifecycle::operations::Operation；只在目标开始前检查，正在执行的事务完成后才进入下一边界。已完成目标不回滚，剩余目标逐项报告 Cancelled。

宿主应在已有 Operation worker 内调用；根据 SyncReport.cancelled 结束为 Cancelled，否则存在 Failed 时结束为 Failed，其余为 Completed，并保留逐目标报告。此任务交付业务/存储接口；API、SDK、UI 由后续任务接入，不能宣称已经接线。

2026-11-14 本轮按用户要求仅写代码及必要合同资料，未运行 Cargo、单元测试、实机测试或其它测试，也未启动最终回归；没有领取构建槽位。后续统一验证需覆盖部分失败继续、目标边界取消、无选择保留、多选警告、恢复后无回填、MCode 限长与权限/快照冲突。
