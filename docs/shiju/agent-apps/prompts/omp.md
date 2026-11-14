# OMP 提示词目录合同（任务19）

来源基线：ca58c0313360eae7c851fbd7553c1614c7af03e7；omp_config.rs 的 get_omp_agent_dir/native_agent_dir 与 prompt_files.rs。依赖任务3已交付 prompts::targets::resolve 和 providers::paths::read。

统一目标入口将 OMP 分派至 prompts::omp::resolve，仅返回配置目录中的 AGENTS.md，归属 PromptLibrary。复用底层目录解析，避免提示词与模型配置选择不同 profile；XDG data_directory 不作为提示词目标。

- 默认：HOME/.omp/agent/AGENTS.md；PI_CONFIG_DIR 按底层合同替换 .omp。
- 命名 profile：HOME/.omp/profiles/<profile>/agent/AGENTS.md，优先 OMP_PROFILE，其次 PI_PROFILE；非法 profile 明确报错，不回退默认。
- 命名 profile 优先于继承 PI_CODING_AGENT_DIR；显式 default 忽略由 PI_PROFILE 派生的旧 profile agent 目录，其他合法继承目录按底层合同处理。
- 显式目录覆盖仍走底层授权边界；相对、越界及不可访问覆盖路径拒绝。返回路径不是 I/O 授权，实际读写继续由主机安全事务处理。
- pi_native_resources 固定 false：OMP 的 SYSTEM.md、APPEND_SYSTEM.md 和 prompts/ 模板能力不支持，不得向其开放 Pi 原生端点。来源未证明这些能力，不因 OMP 派生自 Pi 而推断支持。

本任务仅业务目标解析，无新增 API、SDK 或 UI。按用户本轮要求未新增或运行测试，未调用 Cargo、未领取槽位，未启动最终回归；以上为实现合同，非验证通过结论。后续统一验证默认/命名/无效 profile、显式 default 与继承目录、目录覆盖、XDG 配置/数据隔离和 Pi 能力拒绝。
