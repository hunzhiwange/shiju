# 提示词排序与检索契约

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 database/dao/prompts.rs；检索参考 PromptLibrary.tsx，按本任务边界仅检索名称、描述、正文，不检索 ID。

PromptsStore::list/search 先按既有租户、项目、主机、应用读取目录，再调用 prompts::listing::list_entries。按 created_at ASC、id ASC 排序；缺失时间排在已知时间前。查询去除首尾空白，Unicode 小写匹配，空查询返回完整目录。返回 revision 始终覆盖完整目录，搜索不写入存储。

存储 PromptEntry 复用业务包类型，通过原路径重新导出；createdAt 为可空 Unix 毫秒。创建由存储签发时间，更新保留原时间。既有缺失字段按 Option 读取为 None，不新增迁移或顺序字段。模板目录排序不受影响。

API、SDK、UI 由后续任务接入 search；调用者须先完成授权，不能把作用域身份当授权凭据。本轮按用户要求未运行 Cargo、单元测试、实机或其他测试，未领取槽位；排序并列、空时间、中文查询、作用域隔离与版本冲突留待统一验证，不声明验证通过。
