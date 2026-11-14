# 跨应用停用副本契约

来源：固定上游 PromptPanel.tsx handleCopyToApps、PromptCopyDialog.tsx。
业务入口：PromptsStore::copy_to_apps；纯业务目标校验与服务端 UUID 生成：vw_agent_apps::prompts::copy::new_copy_id。

先授权并读取源快照，逐目标授权相同租户、项目、执行主机下的目标应用。调用者必须传入真实资源授权检查。拒绝相同应用、重复目标、Claude Desktop 和 OpenClaw；Hermes 可显式选择。同名条目不覆盖：采用新 UUID 与已有 create-only/CAS 保存，每个目标独立返回结果；并发冲突明确失败，不隐式重试。正文、名称、说明保留，副本不携带激活能力，不写 live 文件。MCode 复制同样限制 32768 UTF-8 字节。API、SDK、UI 由后续任务接入。

本轮按最新要求不运行 Cargo、测试或实机验证，无通过声明、无槽位领取；后续统一验证应覆盖权限拒绝、部分成功、同名保留、源不变、新 ID、重复目标、MCode 边界及并发冲突。
