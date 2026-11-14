# 首次导入（任务9）

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 `src-tauri/src/services/prompt.rs::PromptService::import_from_file_on_first_launch`。消费任务8的 `import::from_snapshot`、任务7的 `live_file::read` 和现有 SQLite/PostgreSQL 设置 CAS。

macOS 授权主机调度的首次目录入口为 `PromptsStore::list_on_first_access`。调用者必须提供覆盖目录初始化写入和原生文件读取的授权回调，并从已认证执行主机取得 HOME、覆盖目录及环境，不接受请求提供的主机路径。普通 `list/search` 保留纯存储读取语义；API/SDK/UI 的首次页面访问由后续入口任务接入此方法，不能直接调用普通 list 代替初始化。

目录以租户、项目、主机、应用隔离。`bootstrapComplete`（缺省 false）与条目、首次选择、revision 在同一个 CAS 内持久化。已有条目不读取文件，保留选择并封存初始化；空目录只导入非空白快照。缺失和空白文件完成零导入，后续出现内容也不自动导入。删除最后一个条目不会重新导入。CAS 竞争成功者独占创建，失败者只接受已提交的初始化结果，其余目录竞争明确冲突并要求重试。

导入正文复用显式导入的 UUID、UTF-8、应用边界和 MCode 字节上限；普通应用记录首次 active_id，Pi 保存为停用副本，实际状态仍由原生文件派生。本操作不写任何原生文件，不调用激活事务。读取、授权、解析或存储失败返回带 `prompt initialization failed` 上下文的错误，不写成功标志；调用者应显示初始化失败并允许重试，不能转换成成功空目录。

本轮遵从最新要求，未运行 Cargo、单元测试、实机或其它测试，未启动回归，未领取槽位。后续统一验证需覆盖并发首次访问仅一个条目、普通选择/Pi停用、已有条目保留、缺失/空白零导入及删除后不重导入、读取失败后重试、主机与授权拒绝、MCode上限、跨租户/项目/主机隔离、存储失败与CAS竞争；SQLite/PostgreSQL均需实际验证。不声明验证通过，未新增测试文件。
