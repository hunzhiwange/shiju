# 提示词写入 API 契约

任务22来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 commands/prompt.rs（upsert_prompt、delete_prompt、import_prompt_from_file）及 PromptPanel.tsx::handleCopyToApps。

POST /agent-apps/prompts/{save,delete,copy,import}。请求为 project_id、target_host_id、app_id、operation；operation.kind 必须匹配端点。租户取认证上下文，禁止客户端路径、租户、enabled 等额外字段。

save 的 operation 包含 revision、可选 id、name、content、可选 description；无 id 新建，有 id 更新已有条目。delete 包含 revision、id。已激活条目更新/删除明确冲突，由独立激活停用接口负责。成功返回持久化 catalog（含 revision）。

copy 包含 id、targets；复制源需要 prompts_read，逐目标需要 prompts_write，返回 results 中各 app_id 的 entry 或 error（code、status），保留部分成功。重复目标、同源目标明确拒绝。所有副本生成新 ID，保持停用。

import 包含 revision、source。source.kind=uploaded_content 时提供 name、content、可选 description；kind=current_file 时只读授权主机的固定应用文件，复用主机环境及 Pi 目录合同。两者均生成停用新条目，不写原生文件。

权限 policy grant 新增 prompts_write，与 prompts_read 一样是 project_id/app_id 对，继承既有租户、Bearer、主机授权；provider/aggregation 权限不能替代提示词权限。数据库必须由 GlobalDatabase Extension 注入，不回退用户全局目录。响应 no-store，错误不回显正文或路径。API 不适用于非 macOS。

按最新要求本轮只实施，未运行 Cargo、测试或实机验收，未领取槽位。SDK/UI 由后续任务消费。后续统一验证需覆盖成功、无权限、跨项目、错误主机、版本冲突、上传/当前文件区分及逐目标复制结果。
