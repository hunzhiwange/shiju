# 分享载荷网关与 SDK

来源沿用 shared-import.md 固定快照的 deeplink/prompt.rs 与确认流程。依赖任务36的 shared_import::import，以及任务27沿用的 PromptsStore、authorize_prompts_write/activation 和事务激活入口。

POST /v1/agent-apps/prompts/import-share 接收共享 ImportPromptShareApiRequest：project_id、target_host_id、revision（目录 CAS）与 import（领域 ImportPromptShareRequest）。租户来自网关 TenantScope；x-agent-apps-token 经既有项目、应用和执行主机授权。只支持 macOS；不涉及 OS 协议分发，不接受主机路径。confirmed 必须为 true；activation_suggested 不提供授权，也不替代 activation 的显式选择。

停用保存通过 create_shared 保留服务端 UUID 与毫秒时间并执行目录 CAS。KeepDisabled 不访问原生文件；Activate 在保存前检查激活授权，保存后读取目标快照并比较 expectedTargetRevision，再交给既有激活入口在文件锁内比较精确内容、检查目录 revision 并提交事务。保存失败直接返回 HTTP 错误；保存成功后激活失败返回 HTTP 200 的 imported_activation_failed，包含停用 entry 和静态 error.code/status，副本 ID 可供后续重试。不会回滚已成功导入的副本。

共享 ImportPromptShareResult 三种 kind 为 imported_disabled、activated、imported_activation_failed。entry 复用领域 PromptEntry，revision 是不透明的目录版本字符串。SDK agent_app_prompts_import_share 校验 scope.app_id 与 target_app 一致，复用已有带租户上下文与敏感令牌标记的传输入口，直接返回共享联合类型，不丢弃局部成功或将其转换成整体失败。UI 由任务38消费，本任务不修改 UI。

按本轮最新要求未运行 Cargo、单元测试、HTTP 往返、实机测试或最终回归，未领取 Cargo 槽位。后续统一验证需覆盖实际 router 的成功、无令牌、越项目 scope、错误 host、未确认、目录冲突、目标 revision 冲突、停用状态，以及 SDK 对三种结果和错误状态的保真传输。以上为待验证场景，不表示通过。
