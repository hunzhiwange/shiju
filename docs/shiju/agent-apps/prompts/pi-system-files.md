# Pi 原生系统文件合同

来源：cc-switch `ca58c0313360eae7c851fbd7553c1614c7af03e7`，`src-tauri/src/services/pi_prompt_files.rs` 的 PiPromptFileService / PiPromptFileKind。

业务入口：macOS `vw_agent_apps::prompts::pi_system_files::PiPromptFileService`。只接受 SystemOverride（SYSTEM.md）与 SystemAppend（APPEND_SYSTEM.md），没有任意路径参数，不连接识矩系统提示词。

调用者必须先完成租户、操作与执行主机授权，以执行主机路径解析结果取得 Pi 原生 agent 目录的共享 `AppWriteGuard`，并持锁完成读取和提交。此入口拒绝非 Pi guard；目录安全解析与授权属于执行主机，不接受客户端提供的目录。所有 Pi 原生文件及模板写入必须复用同一 AppId::Pi 锁。底层保留目录描述符及 O_NOFOLLOW 防止文件符号链接越界。

读取返回 exists/content/revision，缺失 revision 为 `missing`，存在文件为原始 UTF-8 字节的 SHA-256 小写十六进制摘要。现有空文件可读，与缺失不同。读写上限 1 MiB；替换拒绝全空白，内容不裁剪。非法 UTF-8 拒绝。replace/delete 必须传读取的 revision，空 revision 或陈旧 revision 返回 Conflict；删除已缺失文件且 revision=missing 返回 false。事务再次比较原始快照，提交前执行调用者提供的授权 preflight；事务恢复要求调用者重新读取。

本项交付领域服务；API/SDK/UI 接线由对应任务负责。按本轮用户要求未执行 Cargo、单元测试、实机验收及最终回归；未领取构建槽位。后续统一验证需覆盖成功读改删、缺失/空文件、陈旧 revision、空白/超限/非法 UTF-8、符号链接拒绝和 Pi 共享锁串行行为。
