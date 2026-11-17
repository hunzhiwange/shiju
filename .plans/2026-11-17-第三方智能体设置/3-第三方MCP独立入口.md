# 任务 3: 第三方MCP独立入口

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 2】 依赖：任务1

状态：未开始

目的：提供第三方应用 MCP 管理入口，明确目录与发布绑定的作用对象。

来源任务：无；来自本次用户要求与截图。

预计会话范围：围绕本任务的单一页面或导航行为，复用现有领域，不重写服务。

## 文件

- `crates/libs/desktop-settings-mcp/src/third_party.rs（新建）`
- `crates/libs/desktop-settings-mcp/src/third_party_tests.rs（新建）`
- `crates/libs/desktop-settings-mcp/src/lib.rs`

## 上下文与写入边界

- 主链为 system_settings → Apps → system_settings_apps；vw-agent-apps 是兼容导出层，真实实现位于 crates/libs，不向兼容层堆积UI。
- 推荐顺序与批次：批次2，依赖任务1；与同批其它任务无文件写入冲突。未列出的必要公共接线由任务7承担，领域内部公开导出由本任务承担。
- 新测试统一使用前缀 `third_party_task_03_`；必须在独立测试文件注册并确认过滤命中，不能使用零测试作为证据。

## 测试计划

- 行为：对所选第三方应用执行绑定或发布仅影响该目标，原生运行时配置和其它应用保持原值。
- 失败验证测试：上述测试文件中新增 `third_party_task_03_scope_and_navigation` 等行为测试，优先复用现有fixture。
- 失败验证命令：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-mcp --lib third_party_task_03_ -- --test-threads=1`
- 预期失败原因：现有入口依赖智能体设置页，缺少独立第三方目标和作用域呈现。
- 通过验证命令：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-mcp --lib third_party_task_03_ -- --test-threads=1`
- 开发验证和任务内验收使用同一原生定向命令，要求新结果有效测试数大于0、失败0、退出码0。
- 统一回归由任务8覆盖跨任务路径；本任务必要行为测试在本任务完成。
- 模拟策略：使用真实类型、控件和消息；仅隔离远端网关、外部进程、凭据与磁盘写入，断言请求目标和返回数据形状。

## 步骤

检查 catalog、management_context、bindings 和现有发布服务，复用真实目录/绑定能力，导出可由桌面装配的视图及消息适配入口。展示目标应用/主机；不修改原生 MCP 的运行时启用状态，不复制服务或存储。目录可以共享，绑定和发布动作必须明确目标；未支持操作显示原因。任务7负责宿主消息映射。

## 验证

- 运行：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-mcp --lib third_party_task_03_ -- --test-threads=1`
- 所需证据：记录预期失败和本次通过输出、命令退出码、有效测试数、所选AppId/主机作用域和已审查差异；UI改动补真实渲染路径证据。不得使用真实账号执行破坏性恢复或同步。
- 不因公共检查尚未执行而将本任务标为待回归。
