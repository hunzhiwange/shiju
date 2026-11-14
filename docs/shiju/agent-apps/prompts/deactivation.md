# 停用与外部修改保护契约

来源：固定来源 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 cc-switch `src-tauri/src/services/prompt.rs`，对照 get_prompts、upsert_prompt、delete_prompt。消费任务10的普通激活目录、物理目标锁、ProjectScope 归属及文件事务合同。

入口：PromptsStore::reconcile_ordinary，动作 OrdinaryPromptReconciliation::Refresh / Deactivate。执行端提供已授权 host、home、路径覆盖和环境；authorize 回调必须复核动作权限与物理目标归属。Pi/MCode 使用已有专项文件事务，本入口明确拒绝，不能以普通 AGENTS.md 操作替代。API/SDK/UI 接线留后续任务。

停用当前选择要求调用者提供精确 live 前镜像（缺失与空区分），同时校验目录 revision 与事务 ProjectScope。先回填当前选择的非空 UTF-8 内容，再清空现存文件并原子 CAS 取消选择。缺失文件保持缺失。目录提交失败比较空文件前镜像后恢复原文；外部修改导致恢复冲突时明确要求重载，不覆盖。跨文件和目录数据库的进程崩溃原子性沿用激活入口的限制。

停用非当前条目不写文件、不取消其它选择。新建、导入、保存停用项继续使用现有目录入口，不触碰 live；启用项删除继续由现有 delete 拒绝。外部刷新仅更新当前选择，不激活条目、不更新停用副本；空、缺失、坏编码返回 RefreshWarning 并保留保存内容。没有启用项时不回填。此入口不处理项目规则与 Skill。

按本轮最新要求，未执行任何 Cargo、单元测试、实机或其它测试，未领取构建槽位，未启动最终回归。后续统一验证覆盖快照冲突、归属拒绝、停用当前/非当前项、缺失/空/坏编码、仅当前项回填、删除启用项拒绝、目录 CAS 失败恢复及恢复期间外部编辑冲突。此文档仅描述实现合同，不声明验证通过。
