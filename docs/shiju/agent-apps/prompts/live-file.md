# 当前应用文件安全预览（任务7）

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 prompt_files.rs 和当前文件读取/位置命令。直接依赖为任务3的 targets::resolve，协议复用 PromptTargetPreview、PromptContentSnapshot、PromptRevision。

macOS 入口 `vw_agent_apps::prompts::live_file::read` 接收认证调度提供的主机、HOME、应用配置覆盖及执行环境。调用方先完成租户和动作授权。应用 ID 为封闭枚举，不接收文件名；Claude Desktop 不支持，OpenClaw 归工作区所有者并拒绝此入口。Pi 原生资源由独立任务处理。

固定目标以 providers::paths::open_beneath 的逐层 openat/O_NOFOLLOW 打开，拒绝链接、特殊文件及硬链接；读取后比较描述符元数据并重新解析当前目标确认身份。路径替换或读取期间变化返回 ChangedDuringRead，错误不带正文或原始 IO 载荷。UTF-8 解码失败明确返回 InvalidUtf8。读取不创建目录或文件。

缺失返回 content=None，空文件返回 Some("")。版本为带域标记和存在状态的 SHA-256，按精确 UTF-8 字节计算；缺失与空文件版本不同。显示路径缩短 HOME 为 ~，不能作为后续写入授权。writable=false 表示本只读入口不声明修改权限。后续写入必须重新解析并在事务锁内比较版本。快照仅描述读取时的状态，不保证调用返回后文件不再变化。

本项只交付业务入口，API、SDK、Iced 接线由对应任务处理。本轮遵照最新要求未执行任何 Cargo、单元测试、实机或其他测试，未启动最终回归，未领取槽位；不声明验证通过。后续统一验证需覆盖 Unicode、缺失/空文件版本、无效 UTF-8、主机不匹配、目录及文件符号链接、硬链接、文件替换、覆盖目录逃逸及非文件目标。
