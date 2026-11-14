# Pi 模板改名合同

来源为 cc-switch 固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 PiPromptTemplateService::upsert original_slug 分支；消费任务 17 的 templates 模块与共享 Pi 文件事务。

macOS 入口 `PiPromptTemplateService::upsert_with_original` 接收 original_slug。省略或与新名完全相同时使用普通 upsert；改名必须存在旧文件且 revision 一致，同时目标缺失。两个 slug 均校验，不允许 missing revision 将不存在的旧文件当作改名成功。授权主机仍须锚定 prompts 目录并持有 Pi guard，传入租户及授权 preflight，不接收客户端任意路径。

使用实际目标目录查找判断冲突。大小写不敏感卷上仅大小写改名（及卷识别的 Unicode 别名）返回 Conflict，旧文件不变；大小写敏感卷上两者为不同名称，可按正常改名处理。同名保存沿用现有 revision 保护。

共享事务同时暂存目标创建与旧文件删除，WholeFile 分别重核目标 missing 和旧内容快照；先发布并同步目标，再删除旧文件。目标写入失败时旧文件保留，准备失败的临时文件由底座清理。部分发布后可能暂时存在两份文件，保留 durable pending，由既有恢复流程前滚完成，不另做反向写入覆盖外部编辑。删除已成功但状态落盘失败属于已发布事务，不能将错误响应理解为未发生改名；调用者须重新读取并走共享恢复。恢复前的发布不是多文件原子操作。无关模板不参与变更。

API、SDK、UI 由后续任务接入，本项只提供业务入口。按本轮最新要求未写或执行测试，未调用 Cargo，未领取槽位，未做 macOS 实测或最终回归；以上为实现合同，未经运行验证。后续统一验证需覆盖成功、同名更新、目标冲突、旧 revision 冲突、旧文件缺失、目标写入失败、部分发布恢复、无关文件保留，以及 macOS 实际卷的大小写行为。
