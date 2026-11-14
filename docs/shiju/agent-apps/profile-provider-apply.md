# 项目组合供应商槽位接入合同

2026-11-14，任务 11：实现槽位编排，未经编译或行为验证。

`project_profiles::apply_provider::apply_provider` 仅消费 `SnapshotScope.group` 的供应商引用，输出可空的供应商逐项结果。null 不调用任何读取/写入；非法 ID 拒绝。执行端读取回调必须先核对原领域写权限、目标引用存在和实际模式，即使目标等于当前也不能省略引用核对。Missing、Denied、Unsupported 等使用类型化失败和固定原因码，不转发原生错误或秘密。目标等于实际 `in_use_provider` 时不调用切换回调，因而不要求重投影或重写文件。

切换回调接收同一个项目、主机、应用组和完整模式观察前像，必须在原有事务锁内复核前像、供应商记录及物理项目槽归属。直连路径消费原供应商/登录事务；代理路径消费原路由发布，不调用直连切换、退出或停用代理。返回成功前必须完成全部所需发布并重新观察状态；已提交但发布未完成用 `PublicationPending`，不可当全部成功。编排核对返回值：实际目标正确、模式和 attached 不变、stack 不变、非活动指针不变。后置核对失败只返回 Conflict，不尝试跨领域回滚；已经提交的副作用仍由原事务所有者负责恢复。

已静态核对的真实入口为 `providers/claude.rs::ClaudeFile::apply_with_managed_login`、`providers/codex_catalog.rs::CodexFiles::apply`、`providers/claude_desktop.rs::ClaudeDesktopFiles::apply`，以及 `providers/mode_apply.rs::ClaudeMode::apply` 和 agent 的 `app_routing/provider_publication.rs::apply`。不能将 direct adapter 用于代理路径：Claude direct 会写 Direct mode，Codex/Desktop direct 在代理态明确拒绝。账号切换、原生前像和目录权限继续沿用各领域所有者，组合不复制文件投影或存储 current 指针。

当前交付的是槽位编排入口，不是已接通的生产执行端。后续任务 16/17 和 API 执行端须在完整 prepare/apply/select 串行化范围内组装上述真实回调；不支持的应用/模式、远端不可达、权限失败和发布待恢复须显式失败，禁止空实现或用 DTO 回显伪造成功。组合层不新增文件写入器、路由生命周期、账号所有者或权限来源。

遵循用户本轮覆盖要求：未编写/执行测试，未运行 Cargo、构建、检查、实机或最终回归；未领取 Cargo 槽位，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 未设置。后续需补并执行 `profile_t11_`：直连只切直连指针，代理只切实际路由并保留 attached/direct/stack，相同供应商零写入，缺失/非法 ID/拒绝/不支持/发布待恢复逐项失败，null 与其它组隔离，以及前像并发变化与错误后置状态拒绝。原定向测试要求至少 3 个有效测试，当前没有通过数或退出码。
