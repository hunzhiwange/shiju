# 应用提示词目标契约（任务3）

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 prompt_files.rs、agy_config.rs::customizations_dir、omp_config.rs；完整14应用矩阵见 coverage.md。

入口为 vw_agent_apps::prompts::targets::resolve。复用 providers::paths::read 的授权执行端 HOME、目录覆盖、环境及 OMP 活动 profile 规则；不读取或写入正文。共享解析器会观察元数据。agy 独立使用 HOME/.gemini/config/AGENTS.md，Hermes 使用 SOUL.md，Claude Desktop 返回不支持错误。其余固定文件名遵循来源矩阵。

仅 Pi 声明原生系统文件和模板能力，OMP 不继承。OpenClaw 返回 WorkspaceProjection 归属，永久编辑继续归工作区域。此模块不提供文件操作、HTTP、SDK 或 UI；后续消费者必须通过认证主机调用，并在实际 I/O 时重新进行无符号链接及事务校验，返回路径不是写授权。

本轮按用户要求未运行 Cargo、单元测试、实机测试或其他测试，未领取编译槽位，未启动最终回归。后续统一验证14应用默认/覆盖目录、OMP profile 与非法路径、Claude Desktop 拒绝、agy/Gemini隔离及符号链接安全边界。
