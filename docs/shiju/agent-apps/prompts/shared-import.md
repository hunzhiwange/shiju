# 分享提示词导入契约

来源：coverage.md 固定提交 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 deeplink/prompt.rs、deeplink/utils.rs 和 PromptConfirmation.tsx；读取仓库 upstream 快照。入口为 vw_agent_apps::prompts::shared_import。

parse 接受已 URL 解码的 resource/app/name/content/description/enabled 字段，拒绝额外字段、非 prompt 资源、不支持应用、缺失或空白名称/正文、非法 Base64 与非 UTF-8；兼容标准与 URL-safe Base64、缺失 padding 和 query 中的空格替换。enabled 只映射 activation_suggested，不提供执行授权。不注册品牌协议或 OS 分发。

import 消费共享 ImportPromptShareRequest；confirmed 必须为 true，目标沿用共享资源权限边界。MCode 内容限制按 UTF-8 字节计数。服务端生成 UUID 和 Unix 毫秒时间；DisabledSharedPrompt 没有 enabled 参数。save_disabled 必须绑定认证主机下的 create-only 停用保存，签发 PromptEntry revision 并保存传入的 ID、时间。KeepDisabled 不调用 activate。显式 Activate 携带条目 revision 和用户确认的目标 revision；activate 必须绑定既有授权及事务激活所有者。成功返回 Activated；失败返回 ImportedActivationFailed，包含停用副本与原始错误；保存失败直接返回 Save，不调用激活。不得记录正文或错误中的敏感载荷。

当前工作区只有提示词 DTO、目标解析及模板 slug，尚无提示词条目存储或激活实现。因此本项交付解析和显式回调编排，没有伪造持久化、主机写入或默认成功实现。任务4/10的所有者需提供真实 save_disabled/activate；任务37负责网关与SDK接入，任务38负责确认UI。异步存储接入时需在其执行上下文保持同样的顺序和错误语义。调用者不得将链接 activation_suggested 直接当作用户确认。

按用户最新要求，本轮仅写代码及必需契约资料；未运行 Cargo、任何测试、实机验收或最终回归，未领取槽位。后续统一验证需覆盖确认拒绝、字段与编码拒绝、停用保存不激活、显式激活成功、保存失败不激活、激活失败副本保留、UUID/毫秒时间及真实存储/授权事务。以上均未验证，不表示场景通过。任务36编号卡按最新要求在实现后删除。
