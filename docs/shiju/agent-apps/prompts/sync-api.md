# 保存态同步网关合同

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 sync_all_to_live，任务20的 PromptsStore::sync_to_live 和 prompts::sync::sync_all。

macOS：POST /agent-apps/prompts/sync，JSON 使用 projectId、targetHostId、apps（非空且不重复）。不接收文件路径。所有目标必须有当前项目的 prompts.write 授权；由执行主机策略提供 home 和事务根，普通投影拒绝 OpenClaw。数据库由网关注入。

返回202，响应包含 operation（既有 AppOperationStatus）、projectId、targetHostId 和 targets。通过既有 GET/DELETE /agent-apps/operations/{id}?targetHostId=... 查询/取消，/events 长轮询复用同一所有者。每次查询、长轮询返回和取消复验租户、提交项目、执行主机、所有目标授权及提交者身份；其他有权限的主体也不能取消。进程内最多保留1024个操作，容量满时回收终态操作；重启不承诺恢复操作句柄。

targets 每项含 appId、status、reason、duplicateSelection。status 为 succeeded、failed、skipped、not_executed；跳过原因 native_pi_selection、unsupported_claude_desktop、no_selection；取消的未执行目标原因为 cancelled。处理中 targets 仅包含已处理目标。单项失败继续，取消仅在事务边界生效，保留已完成结果。存在失败结束 Failed，取消剩余目标结束 Cancelled，否则 Completed。底层错误、正文和真实路径不出现在响应中。

SDK/UI由后续任务消费；本任务不承接其实现。2026-11-14按本轮要求仅实现代码与合同，未执行Cargo、单元/实机或其它测试，未领取槽位、未启动最终回归。后续统一验证应覆盖真实router鉴权、提交/进度/取消、不同主体、租户、项目、错误host、部分失败及事务冲突。
