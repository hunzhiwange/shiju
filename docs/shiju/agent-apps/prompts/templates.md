# Pi 模板目录合同

来源：cc-switch 固定来源 ca58c0313360eae7c851fbd7553c1614c7af03e7，src-tauri/src/services/pi_prompt_files.rs 的 PiPromptTemplateService。消费任务 15 的 slug 校验、16 的 metadata 摘要及 14 的 Pi 原生文件共享事务约定。

macOS 业务入口 prompts::templates::PiPromptTemplateService 提供 list/read/upsert/delete。执行主机先授权租户、操作及原生 Pi agent/prompts 目录，再取得 AppId::Pi 的 AppWriteGuard 并持有至事务完成。guard 必须锚定 prompts 目录本身；不接受请求路径。缺失目录由主机确认后以 list(None) 返回空列表；创建目录属于授权主机的目录准备步骤，权限错误不能当缺失。所有 Pi 文件与模板复用同一 Pi 应用锁。

只列出有效 slug 的小写 .md 文件，以 slug 排序；忽略其他文件、非 UTF-8 文件名及无效 slug，不修改它们。通过保留的目录描述符枚举，匹配文件使用底座 O_NOFOLLOW 读取，符号链接及非普通文件拒绝。内容上限 1 MiB，非法 UTF-8 拒绝，允许空模板。摘要复用任务 16，不展开原生变量。

存在文件 revision 为原始内容 SHA-256 小写十六进制，缺失为 missing。增改删必须提交当前 revision，陈旧或空 revision 冲突；missing 可创建，删除缺失文件返回 false。WholeFile 与 commit_checked 再次核对快照并调用授权 preflight，通过现有 pending/recovery 事务发布，文件权限 0600。改名入口见 template-rename.md。API/SDK/UI 由后续任务接入。

按本轮要求未新增或运行测试，未调用 Cargo，未执行实机验收或最终回归，未领取槽位；实现未经运行验证。统一验证应覆盖稳定排序、重复列出、缺失目录、空内容、revision 冲突、删除幂等、非模板保留、1 MiB 边界、UTF-8、符号链接、授权失败与事务恢复。
