# 现有文件显式导入（任务8）

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 `src-tauri/src/services/prompt.rs::PromptService::import_from_file`。直接消费任务4的 `PromptsStore::create` 与任务7的 `live_file::read`。

业务入口为 `vw_agent_apps::prompts::import::{from_snapshot,from_upload}`。当前文件通过 `PromptsStore::import_current_file` 在 macOS 上读取授权主机的固定应用目标快照，然后保存精确正文；缺失明确失败，空文件有效。读取沿用任务7的路径、安全身份、UTF-8 与主机一致性检查。快照表示读取时状态，导入期间外部编辑不构成原文件写入授权。

上传通过 `PromptsStore::import_uploaded_content` 只接收客户端选择的字节、名称和说明，没有路径字段，也不会解析客户端路径。UTF-8 严格解码，保留中文、空白与换行。Claude Desktop、OpenClaw 和非应用资源拒绝；MCode 使用现有 32768 UTF-8 字节上限。认证调度必须先完成租户、项目、主机与导入动作授权，HOME、覆盖目录与环境不能由不可信请求提供。

每次导入由服务端生成 UUID v4，避免原实现按秒命名导致覆盖。存储仅包含停用条目字段，调用 create-only 并比较调用者提供的 catalog revision；重复 ID 或并发版本冲突明确失败，不覆盖、不激活、不回填其他条目，也不创建、清空或修改原文件。调用者遇到冲突应重新读取目录版本后显式重试。

API、SDK、Iced 由后续任务接线。本轮按最新要求只写代码与必要契约资料，未执行 Cargo、任何测试或实机验收，未启动最终回归，未领取构建槽位，不声明验证通过。统一验证需覆盖原文件字节与目录保持不变、连续导入唯一 ID、停用与既有条目保留、缺失/空文件、非法 UTF-8、应用拒绝、MCode 上限、路径/主机拒绝、存储失败及版本冲突。未新增测试文件。
