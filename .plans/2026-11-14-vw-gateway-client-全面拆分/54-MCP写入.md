# 任务 54：MCP写入

先阅读 [公共规则](plan.md)、[全文件审计](source-map.md) 和 [依赖审计](audit.md)。

状态：已完成

批次：54，线性顺序 54；依赖：前一批次 53。符号依赖卡：无额外前置。不并行。

目标：crates/libs/gateway-mcp-write。

## 来源与迁移边界

- `crates/vw-gateway-client/src/client/mcp_management_write.rs`
- `crates/vw-gateway-client/src/client/mcp_management_write_tests.rs`

真实函数入口（包括需要一起迁移的私有 helper）：client/mcp_management_write.rs: mcp_management_create, mcp_management_update, mcp_management_delete, mcp_management_write。

跨域调用：未发现跨域 self 方法调用。



## 实施步骤

1. 阅读上述完整文件及关联测试，重新搜索 consumer-refs.md 所列四包的直接调用者；核实前置源码落地，禁止假设计划包已经存在。逐文件归属表覆盖测试与私有符号，不能只迁移 pub 方法。
2. 将本卡实现迁入目标库的独立模块；领域客户端为拟新增具体结构体，显式持有已落地 GatewayTransport。替换 self.client/self.endpoint/native_transport 为传输访问器，保持返回类型、错误文案、URL/query/body、认证与租户校验。跨域调用按上表显式组合所有者；不能依赖聚合 GatewayClient 或复制另一领域实现。
3. 更新 workspace members、原包/直接消费者 Cargo.toml 及 lib.rs 导出和显式转发；只引入本卡真实使用依赖，保留原 cfg、path 属性、原生/wasm 分支与错误拒绝路径。既有类型优先复用 vw-api-types、vw-shared 实际导出，不引入臆测类型库。
4. 测试源码随行为迁移至独立测试文件；共享 test_support 的客户端工厂改为本领域类型，保留请求捕获和所有安全断言。门面兼容测试留原包。include 路径按新相对位置核对，当前包未发现 include_str!/include_bytes! 资源，不新增资源或 build.rs。
5. 静态审查新库不得引用 vw_gateway_client；核对源模块已经移除或只剩转发，消费者直达领域库。完成后按公共契约提交并删除本卡，不执行动态验证。

## 静态证据与后续验收

允许命令（仓库根）：`git diff --check`、`git diff --stat`、`rg -n 'vw_gateway_client|pub use|mod |path =' crates/libs/gateway-mcp-write crates/vw-gateway-client`（门面任务改查原包）；阅读实际调用者并核对清单路径。rg 无匹配按搜索目的判断。

后续另行授权统一验收：本卡来源中既有测试的全部行为、线协议方法/路径/查询/正文、错误类型与文案、空值/404、认证与租户隔离、原生/wasm cfg 及门面兼容。涉及流式接口另验 SSE 帧边界、断流/取消/恢复与 WebSocket 关闭；涉及宿主请求另验禁重定向/重试及授权拒绝。当前编译、测试、性能与实机均未验证，不宣称通过。

## 实现归属与静态证据

- `gateway-mcp-write/src/mcp_management_write.rs` 拥有 create/update/delete 与私有 `mcp_management_write`；具体客户端持有 GatewayTransport，复用 gateway-app-settings 的错误类型。
- 原写入测试前四个场景及 create_request/record helper 迁至独立领域测试；test_support 工厂改为领域客户端。检查与绑定调用兼容场景留在原包。
- 原包三个公开方法只做显式转发；workspace、门面和桌面清单接线，桌面 form/import 直达领域客户端。四个直接消费者重新搜索，仅桌面存在本卡生产调用。
- 前置任务53的提交 73c9022bd 与 gateway-mcp-read 实现已确认。
- 静态核对新库无 vw_gateway_client 反向引用，协议、凭据、租户上下文、错误、cfg 与原生传输策略保持。git diff --check 通过。
- 按用户覆盖要求未运行 Cargo、测试及实机验证，未做最终回归；编译和行为留待以后统一授权验证。
