# 任务4：配置写入前校验

实现完成。按用户本次覆盖指令，未运行任何 Cargo 命令、单元测试或实机测试，所有验证统一延期；无测试退出码、有效数、失败数及 ignored 数证据，不宣称编译或测试通过。

## 断言映射

下列原宿主用例均迁入 gateway-config/src/host_split_4_tests.rs，保留原名：

| 原用例 | 迁入领域的断言 | 本组保留宿主定义 |
| --- | --- | --- |
| handle_api_config_put_rejects_invalid_toml | BAD_REQUEST、Invalid TOML 错误消息 | 0 |
| handle_api_config_put_rejects_type_mismatch_after_toml_parsing | 真实默认配置 TOML 中将 default_temperature 改为字符串，断言 BAD_REQUEST、Invalid TOML | 0 |
| handle_api_config_put_rejects_invalid_config_without_mutating_runtime_state | 空白 gateway.host 返回 BAD_REQUEST、精确错误片段、原运行时 host 未变 | 0 |

静态迁出3条、领域新增3条、原文件减少70行。领域 lib.rs 以 cfg(all(test, not(target_arch = "wasm32"))) 注册；实际编译注册清单未采集。

## 夹具与真实调用链

参考任务3的局部 ConfigHost 模式，使用仅包含 Arc<Mutex<Config>> 的局部状态，不复制 AppState，不反向依赖宿主。任务3夹具用于无状态 patch，本组需要状态与认证成功端口，因此不为一次复用提取共享框架。认证端口确定性成功，水合对本组无掩码输入返回 incoming；真实鉴权和水合仍由宿主负责。未用端口明确 panic，防止无效配置意外触达 Hook 发布、pairing 更新及会话失效。

测试直接调用真实 gateway_config::handlers::handle_api_config_put。宿主 config.rs 已通过 ConfigHost 包装调用此入口，本次无需改生产代码。真实鉴权、密钥存储失败、Hook 原子发布、query engine 失效相关宿主测试均保留。未改变安全约束、生产算法或依赖清单。

修改文件：领域 lib.rs、新建 host_split_4_tests.rs、宿主 config_tests.rs、本证据及删除编号任务文件。plan.md/prompt.md 只读。

统一验证待补：迁移前基线未运行；领域 host_split_4_tests 预期发现3条，宿主 lib-test 编译与注册清单及公共检查均未运行。
