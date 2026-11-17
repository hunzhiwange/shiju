# 任务 4: 第三方Skills独立入口

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 2】 依赖：任务1

状态：未开始

目的：在第三方区域管理技能目录与应用绑定，保持原生技能配置独立。

来源任务：无；来自本次用户要求与截图。

预计会话范围：围绕本任务的单一页面或导航行为，复用现有领域，不重写服务。

## 文件

- `crates/libs/desktop-settings-skills/src/third_party.rs（新建）`
- `crates/libs/desktop-settings-skills/src/third_party_tests.rs（新建）`
- `crates/libs/desktop-settings-skills/src/lib.rs`

## 上下文与写入边界

- 主链为 system_settings → Apps → system_settings_apps；vw-agent-apps 是兼容导出层，真实实现位于 crates/libs，不向兼容层堆积UI。
- 推荐顺序与批次：批次2，依赖任务1；与同批其它任务无文件写入冲突。未列出的必要公共接线由任务7承担，领域内部公开导出由本任务承担。
- 新测试统一使用前缀 `third_party_task_04_`；必须在独立测试文件注册并确认过滤命中，不能使用零测试作为证据。

## 测试计划

- 行为：第三方绑定/安装消息保留 AppId 与上下文，取消操作不改变原生绑定；不支持目标不会发起写入。
- 失败验证测试：上述测试文件中新增 `third_party_task_04_scope_and_navigation` 等行为测试，优先复用现有fixture。
- 失败验证命令：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-skills --lib third_party_task_04_ -- --test-threads=1`
- 预期失败原因：现有原生技能入口不能表达独立第三方页面的目标和取消行为。
- 通过验证命令：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-skills --lib third_party_task_04_ -- --test-threads=1`
- 开发验证和任务内验收使用同一原生定向命令，要求新结果有效测试数大于0、失败0、退出码0。
- 统一回归由任务8覆盖跨任务路径；本任务必要行为测试在本任务完成。
- 模拟策略：使用真实类型、控件和消息；仅隔离远端网关、外部进程、凭据与磁盘写入，断言请求目标和返回数据形状。

## 步骤

读取 page、bindings_view、types 和 update/bindings_view.rs，复用导入、安装、绑定、作业与备份能力。提供显式应用目标的第三方视图入口，不将全部原生技能页挪走。复用状态并隔离展示选择，不改变市场、仓库或权限语义；任务7负责宿主装配。

## 验证

- 运行：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-skills --lib third_party_task_04_ -- --test-threads=1`
- 所需证据：记录预期失败和本次通过输出、命令退出码、有效测试数、所选AppId/主机作用域和已审查差异；UI改动补真实渲染路径证据。不得使用真实账号执行破坏性恢复或同步。
- 不因公共检查尚未执行而将本任务标为待回归。
