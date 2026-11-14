# MCode 提示词事务契约

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7，services/prompt.rs 的 upsert_mcode_prompt、enable_mcode_prompt 及 mcode_config.rs 的 write_and_commit。

入口：prompts::mcode::validate_content 与 write_and_commit。保存（包括停用副本）和激活均按 UTF-8 字节限制 32768 字节，不截断；分享导入复用同一校验。目标仅为授权 Mcode 目录内固定 AGENTS.md。

执行端在读取条目、备份和写入整个阶段持有既有 AppWriteGuard；同应用及同物理目录串行化。调用者提供文件原始快照，在 preflight 校验授权、归属及条目版本；库复用 ProviderTransactions 和 WholeFile 完成原子替换与文件恢复。数据库提交回调必须原子保存条目、启用状态及备份，失败不得留下部分库变更。失败后恢复原文件，包括不存在与空文件的区别；外部写入冲突会明确返回 Rollback 错误，不覆盖外部内容。进程崩溃后的跨文件/提示词库原子性不在同步回滚保证内。

停用副本仅校验并写库；活动编辑、激活及需要清空文件的停用调用事务入口。启用前的原文备份、条目选择及 revision 归提示词业务调用者，本模块不重造存储或授权。API、SDK、UI 由后续任务接入。

遵照本轮指令，未运行 Cargo 或任何测试，未领取槽位，未启动回归。统一验证需覆盖 32768/32769 ASCII 字节、中文跨边界、原文件缺失/空/非空时的写库失败恢复、外部编辑冲突、预检拒绝及同目标串行化。
