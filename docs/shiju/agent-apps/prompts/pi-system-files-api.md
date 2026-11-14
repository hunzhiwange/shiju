# Pi 系统文件网关接口（任务 24）

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 services/pi_prompt_files.rs，映射 get_pi_prompt_file、replace_pi_prompt_file、delete_pi_prompt_file；前置领域实现见 pi-system-files.md。

三个独立 POST 接口：

- `/agent-apps/pi/prompt-files/read`：project_id、target_host_id、kind，返回 exists/content/revision。
- `/agent-apps/pi/prompt-files/replace`：以上字段加 expected_revision、content，返回更新快照。
- `/agent-apps/pi/prompt-files/delete`：以上字段加 expected_revision，返回是否曾存在的布尔值。

kind 仅接受 system_override、system_append。请求拒绝未知字段，租户来自网关认证上下文。沿用执行主机策略中对应项目 Pi 的 AggregationRead/AggregationWrite 授权（与 Pi 目录接口一致），并检查事务中的项目归属；无归属返回冲突，跨项目返回禁止。主机目录来自授权策略和执行端 Pi 环境解析，不接受客户端路径。仅支持 macOS；SDK/UI 由后续任务接入。

在阻塞线程取得现有 Pi 原生目录的共享锁，不创建目录；未初始化目录返回 invalid_configuration。事务存储必须已存在。拒绝 pending 恢复事务，提交前重新核对授权上下文和状态；领域服务继承当前项目绑定提交，防止被事务层误判为跨项目操作。文件替换沿用原始字节 revision，缺失使用 missing。陈旧或空 revision 为 409；空白内容 400；超过 1 MiB 为 413；非法 UTF-8 422。响应及错误均 no-store，无原始内容或主机路径错误日志。HTTP JSON 外层限额预留转义开销，业务层仍按解码后的文件字节限制。

本轮按用户要求仅实现及静态阅读差异；未运行 Cargo、单元测试、实机测试或最终回归，未领取槽位。后续统一验证应从真实认证 router 覆盖成功读改删、未知 kind、额外字段、旧 revision、超限、越租户/项目、错误主机、授权撤回、pending 恢复及符号链接拒绝；不宣称这些场景已通过。
