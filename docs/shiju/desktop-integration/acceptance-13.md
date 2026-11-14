# 设置页配置备份读取（任务 13）

2026-11-14：实现交付，未验证。遵照本轮用户约束，没有运行 Cargo、单元测试、实机测试或最终回归，未领取构建槽位，未设置 CARGO_TARGET_DIR。

依赖核对：读取 acceptance-1.md、acceptance-2.md；既有备份领域提供 backup_list::list_local_backups、backup_api::summary 与 lock_directory。复用其目录安全检查、作用域过滤、格式校验和元数据解码，不新增归档格式或配置 CRUD。来源快照沿用 ca58c0313360eae7c851fbd7553c1614c7af03e7；共享 checkout 存在并行未提交实现，本次仅交付自己的接线。

新增 GET /v1/agent-apps/data-backup-inventory：项目与租户来自请求及网关上下文，身份从认证策略解析，执行主机由独立 data_backups 项目授权及管理员 data_backup_root 约束。请求不接收主机路径或 principal_id。响应仅返回 DataBackupSummary，不包含备份内容；设置 Cache-Control: no-store。拒绝与读取错误保留非成功 HTTP 状态，不以空列表替代失败；非 macOS 返回 501。

GatewayClient::data_backup_inventory 校验端点租户、项目、目标主机，保留 HTTP 状态与结构化错误；桌面通过设置→应用→读取配置备份展示名称、字节数和创建时间。状态绑定网关、租户、项目、主机及目录/请求代数，忽略过期结果。错误通过既有 toast 展示，可重试。复用设置按钮样式和外层 4px 滚动条、7px 留白；未增加表单或等宽中文文本。

14 应用继续消费目录声明，不新增整批支持/不支持判断；本入口属于当前项目和主机的配置备份元数据读取。设备同步及备份恢复仍归各领域后续接口；本任务仅补齐一个真实读取入口，不宣称所有设置能力已完成。未来统一验证应覆盖真实 HTTP、权限拒绝、空列表、损坏/不安全存储、跨主机与过期响应，以及 macOS 明暗主题渲染。本次未新增或执行测试，不声称验收通过。
