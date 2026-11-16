# 任务25 实施记录（尚未完成）

遵守禁测：未运行 Cargo、测试、绘制或回归。仅独立 rustfmt 与差异静态检查。

本次已接线：apps_catalog::ObservationLoaded → settings-apps::catalog_update::finish_observation，宿主持有缓存时间并映射错误 Toast；LocalReady 使用领域的上下文/代际/忙碌校验。未引入反向依赖。

测试迁移：原 lifecycle_tests、uninstall_tests、launch_tests 全部行为用例迁入 host_module_25_tests 同名子模块，保留测试名与断言。生命周期夹具只持有局部 LifecycleState、rows、context、Notice，直接调用真实 lifecycle_update::update；网络事务测试仍调用真实 execute。原 Toast 文案断言改为领域 Notice::Error 同文案；原宿主 HostInput 启动失效操作改为领域 LaunchState::invalidate 后检查迟到结果。此处仍需补宿主跨域契约覆盖。

来源基线 4670 行，当前 3893 行，宿主净减少 777 行；生产候选基线 683 行，当前 679 行。本次未新建宿主文件。数字包含此前迁移及共享调用者字段调整，不能全部归因本次。

未完成：目录请求状态、凭证修改后的局部转换仍在宿主；catalog/details/drag/preferences 页面与 UI refresh、window 测试尚未整组迁移；现有 host_split 等价断言尚未逐条核对，残留逐函数审计尚未完成。因此保留任务卡，不能标记完成或待回归。

以后获授权才可运行：python3 scripts/cargo_slot.py cargo test -p desktop-settings-apps --lib host_module_25_tests -- --test-threads=1；宿主 check 仍遵循统一验证。本次无编译或行为通过结论。
