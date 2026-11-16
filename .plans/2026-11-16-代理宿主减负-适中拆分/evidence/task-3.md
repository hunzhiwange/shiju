# 任务3：配置目录选择与补丁形状

实现完成；用户明确要求所有验证延期。未运行任何 Cargo 命令、单元测试或实机测试；无退出码、通过数、失败数或 ignored 数证据，不声称编译或测试通过。

## 断言映射

| 原宿主用例 | 领域模块 host_split_3_tests 中同名用例 | 保留宿主部分 |
| --- | --- | --- |
| has_explicit_directory_detects_non_empty_query_or_header_values | 原五条断言：无目录、空白 query、有效 query、空白 header、有效 header | 无 |
| has_explicit_directory_ignores_non_utf8_header_value | 原非法 UTF8 header 不视为显式目录断言 | 无 |
| config_patch_rejects_non_object_patch | 真实 config_patch::<TestHost>，保留 BAD_REQUEST 和错误消息两条断言 | 无 |

源码定义迁出3条，新增领域定义3条，保留本组宿主定义0条；这些是静态数，实际 lib-test 清单待统一验证。来源直接注册链见 baseline.md；领域由 lib.rs 在 cfg(all(test, not(target_arch = "wasm32"))) 独立注册。

## 夹具与接线

TestHost 的 with_instance 只执行传入操作，目录由测试 query 提供；其余未使用端口明确 panic，避免伪支持。补丁测试使用真实 TempDir 和真实领域更新路径；schema/config_save.rs 在存储访问前拒绝非对象，因此无需迁移 EnvGuard、AppState 或数据库/提供者故事。已有 config_api_tests 为 HTTP 客户端夹具，不能替代此处理器错误映射断言，未复制该夹具。

宿主 router/stateful_router 和 config_patch 包装已调用 gateway_config 真实生产实现，保持接线；仅删除无调用者的 has_explicit_directory 测试导入。不修改生产算法，不增加公共接口或反向依赖。

修改范围：gateway-config 的 lib.rs、新测试模块、Cargo.toml（仅 tempfile 测试依赖）、Cargo.lock 对应依赖条目，以及宿主 config.rs/config_tests.rs。宿主 config_tests.rs 静态减少49行，config.rs 减少2行。编号任务文件按用户指令删除；plan.md/prompt.md 只读。

统一验证待补：原基线因用户覆盖未运行；领域 host_split_3_tests 应发现3条用例；宿主 lib-test 注册/编译和公共检查均未运行。
