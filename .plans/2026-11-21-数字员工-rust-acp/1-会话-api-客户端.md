# 任务 1 · Agent Cloud 会话 API 客户端

先阅读并遵守 [公共执行规则](plan.md)。

批次【批次 1】 · 无依赖  
状态为 `未开始`。

目的为给 Rust ACP 适配层提供带当前用户身份的员工会话产品 API。

来源为服务端会话 OpenAPI 与既有 Go 桥接契约。会话范围限于 `shiju-client` 的 HTTP 类型和行为。

## 文件

- 修改 `crates/libs/shiju-client/src/lib.rs`。
- 新建 `crates/libs/shiju-client/src/employee_conversations.rs` 和 `employee_conversations_tests.rs`。
- 参考 `crates/vw-desktop/src/app/digital_employees/api.rs` 当前网关和授权方式，以及 `crates/libs/shiju-client/src/apps_tests.rs` 的服务替身和脱敏断言。

## 步骤

1. 声明员工类别及目标、会话/消息/轮次 DTO 和固定业务错误。封装两类员工各自的集合列表和首条消息提交、通用历史读取、后续提交、轮次取消。依 OpenAPI 使用路径段编码、状态码和幂等键。保持 `assistant_id` 为两类会话共用的产品标识。
2. 默认使用现有员工页面的 HTTPS 网关根地址和当前账号 `X-Api-Key`。通过已脱敏的凭据类型构造敏感头。限制重定向、超时、响应体大小和错误正文暴露。生产入口固定来源，受控局部测试传入回环替身。
3. 把 401、403、不可见、忙、幂等冲突和暂时不可达映射为可安全展示的错误。向任务 2 交付可直接使用的会话客户端和响应字段。

## 测试计划与验收

- 新增 `employee_conversations_routes_and_scopes_requests` 和 `employee_conversations_redacts_errors_and_rejects_redirects`。第一组使用真实 reqwest 调用回环 HTTP fixture，核对路径、正文、幂等头、取消以及只发送用户凭据。第二组核对 401、403、404、409、错误体脱敏及 30x 目的地未收到凭据。
- 失败和通过命令均为 `python3 scripts/cargo_slot.py cargo test -j 1 -p shiju-client --lib employee_conversations_`。先以可编译的最小接线运行用例，预期现有 GET/POST 模式缺少员工类别路由、取消及错误约束。修复后记录两组有效测试均通过。

