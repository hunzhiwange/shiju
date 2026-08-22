# 任务 14: 覆盖 APIQL 查询接口

批次：【批次 14】 依赖批次 13

状态：未开始

目的：为 APIQL 实体列表、仅行、计数和分页提供统一查询 CLI。

来源任务：无

预计会话范围：四个逻辑操作覆盖带/不带尾斜杠的八条路由别名，只做读取，不扩展 APIQL 语法。

## 文件

- 修改：`crates/vw-gateway-client/src/client/mod.rs`
- 新建：`crates/vw-gateway-client/src/client/apiql_api.rs`
- 测试：`crates/vw-gateway-client/src/client/apiql_api_tests.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/apiql.rs`
- 测试：`crates/vw-cli/src/handlers/api/apiql_tests.rs`

## 上下文

- 所有业务请求经 `GatewayClient`，CLI 只负责参数、输入、确认和输出。
- 该任务完成时须在中央 `ApiCommands` 注册一次；因共享注册文件，依赖前一批次完成。
- 当前工作树的无关改动必须原样保留。
- `vw-api-types/src/apiql.rs` 已定义查询相关类型，应复用而非接受任意未验证键值。
- CLI 选择无尾斜杠规范路径；尾斜杠版本只记为协议别名。

## 测试计划

- 行为：APIQL 四个命令正确编码 entity_class、保留合法查询参数并输出结构化结果。
- 失败验证测试：`crates/vw-cli/src/handlers/api/apiql_tests.rs` 中的 `api_apiql_commands_use_canonical_routes_for_all_query_modes`
- 失败验证命令：`cargo test -p vw-cli api_apiql_commands_use_canonical_routes_for_all_query_modes --lib`
- 预期失败原因：GatewayClient 和 CLI 均没有 APIQL 入口。
- 通过验证命令：`cargo test -p vw-cli api_apiql_commands_use_canonical_routes_for_all_query_modes --lib`
- 模拟策略：本地服务器验证 query string 和四个规范 path，不模拟数据库或 SQL。

## 步骤

1. 先写 list/list-only/list-count/list-page 的失败路由测试。
2. 新增窄的 APIQL client 方法并复用类型化查询。
3. 实现 `api apiql`，拒绝空 entity_class 和非法分页值。
4. 输出由共享 JSON renderer 处理，尾斜杠别名不生成重复命令。
5. 运行聚焦测试与 clippy。

## 验证

- 运行：`cargo test -p vw-cli api_apiql_commands_use_canonical_routes_for_all_query_modes --lib`
- 运行：`cargo test -p vw-gateway-client apiql_client_uses_canonical_query_routes --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：四个规范路由与查询断言通过，别名只分类不重复，clippy 通过。
- 所需证据：失败时 client/command 缺失；通过后四模式测试 0 失败；clippy 0 警告。

## 完成

`api apiql` 以四个稳定操作覆盖八条协议路由。

