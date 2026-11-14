# Pi AGENTS.md 文件真值契约

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 services/prompt.rs（pi_active_prompt_id、upsert_pi_prompt、delete_pi_prompt）及 services/pi_prompt_files.rs。入口为 vw_agent_apps::prompts::pi_agents；直接消费 targets 的 Pi 路径、持久化稳定列表顺序及 AppWriteGuard/ProviderTransactions。无需修改 lib.rs，已有 prompts 导出。

active_id/project 按正文完全匹配及列表顺序选择唯一条目，缺失与空文件区分。enabled 仅为输出投影，不作为持久化真值。apply 的 Edit 保留当前位置，停用副本编辑不发布正文；Deactivate 仅删除该条目仍拥有的文件，Delete 拒绝当前启用条目。所有目录保存前清除 enabled。

调用方必须持有已授权执行主机的 Pi guard，在锁内读取目录并核对目录及条目 revision；传入与客户端预期文件 revision 对应的执行端快照，不得用新快照代替过期预期。WholeFile 比较原文拒绝外部漂移。原生写失败不调用存储提交，存储失败则以已发布正文为条件回滚文件，避免覆盖后续外部编辑；双重失败显式返回 Rollback。进程崩溃恢复沿用底层文件事务，不宣称跨文件/目录数据库崩溃原子性。

存储提交回调须原子保存整个目录（含调用方准备的备份），失败不改变存储；preflight 负责授权、目录版本及激活前外部正文备份要求。条目 revision 和时间戳由存储适配器生成。API/SDK/UI 由后续任务接线，本项仅交付业务入口，与现有 MCode 回调式事务边界一致。

按本轮最新指令仅写代码及必要契约资料，未新增或执行测试，未运行 Cargo、macOS 实测或最终回归，未领取构建槽位。后续统一验证应覆盖重复择一、空/缺失文件、外部漂移、停用副本隔离、归属删除、文件及存储失败回滚。
