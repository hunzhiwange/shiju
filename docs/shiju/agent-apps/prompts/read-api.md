# 提示词只读网关合同（任务21）

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 commands/prompt.rs 中 get_prompts、get_current_prompt_file_content、get_prompt_file_location。消费 listing、live-file、bootstrap、persistence 合同，分别对应任务6、7、9。

GET `/v1/agent-apps/prompts`、`/v1/agent-apps/prompts/current`、`/v1/agent-apps/prompts/location` 均要求 camelCase 查询 projectId、targetHostId、appId；可选 query 仅过滤目录。未知字段、未知 AppId、非法身份直接拒绝。租户来自 TenantScope 中间件，主机及主体来自现有 Bearer 策略。仅 macOS 本地主机支持，远程、离线及未授权主机不回退桌面主机。Claude Desktop、OpenClaw 不属于此应用编辑领域。

管理员 grant 新增 `prompts_read: [{project_id, app_id}]`，默认空，与现有 platform_id、company_id、host_id 联合约束。此能力包括读取指定原生提示词以及首次目录访问的一次性停用数据导入和初始化封存，不包含原生文件写入或一般条目修改。既有 provider/aggregation 权限不自动扩大为提示词权限。只有目录首次访问调用 PromptsStore::list_on_first_access；过滤在初始化后执行，避免过滤结果触发重导入。

目录返回 PromptCatalog（revision、bootstrapComplete、activeId、entries），entries 沿用 listing 的停用条目结构；当前文件返回 PromptTargetPreview，缺失与空文件保留不同 revision；位置仅返回 resource、displayPath、writable=false，不读取正文、不授予写权限。展示路径为 ~/ 相对路径。所有结果及业务错误带 no-store，错误不输出路径、正文或内部存储错误。初始化错误返回 503 prompt_initialization_failed，版本竞争返回 409 revision_conflict，允许重新请求。

共享授权导出：`gateway::api::handlers::agent_apps::{authorize_prompts_read, authorize_prompts_write}`，macOS 下签名相同：`(Option<&Path>, &HeaderMap, &ProjectScope, &str, AppId) -> Result<(PathBuf, Option<PathBuf>, String), StatusCode>`。返回授权主机 home、管理员 Pi directory、principal；调用者必须在 I/O/提交边界重新授权并比较上下文，不可将请求路径或主体作为可信输入。`prompts_write` 是独立同形 grant，仅供同批条目写入入口使用，不隐含原生激活/文件写权限；激活仍需原有事务和独立 native 写授权。新 grant 均默认拒绝。

Gateway runtime 在启动时注入 GlobalDatabase，handler 不自行构造用户全局路径；嵌入 router 的调用者需注入相同 Extension，缺失目录数据库明确 503。主机 home 和 Pi directory 来自授权策略，环境来自执行端；SDK/UI 接线由后续任务交付。

按最新要求，本轮仅实现代码和契约资料，未运行 Cargo、单元测试、实机或其他测试，未启动回归、未领取槽位，不声明验证通过。后续统一验证需覆盖真实 router 的授权成功、缺权限、跨租户/项目/应用、错误及远程主机、初始化 CAS 冲突/失败重试、已有目录不重导入、原生文件缺失/空文件/不安全路径与无数据库注入。
