# 任务9：SDK驱动测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：迁移模型解析、传输请求体与构造错误测试；loopback HTTP 保持真实 SDK parser；宿主消息构造契约单独保留。。

来源任务：原代理测试大任务，已有三族只补验证，不重复迁移。

预计会话范围：一个明确行为族及其共享fixture、父接线；测试数量不作为任务边界。

## 文件与边界

- `crates/libs/llm-runtime/src/aisdk/host_tests/driver_tests.rs`。
- `crates/vw-agent/src/session/llm/aisdk/driver.rs`。
- `crates/libs/llm-runtime/src/aisdk/driver.rs`。

行为选择：迁移模型解析、传输请求体与构造错误测试；loopback HTTP 保持真实 SDK parser；宿主消息构造契约单独保留。。同源文件前序任务拥有的函数不重复接管；下列名称仅帮助定位，实际完整清单必须在编辑前按本行为记录。

## 已核查测试锚点

- `codex_skills_transport_chat_http_merges_instructions_and_preserves_user_body`，位于 `crates/libs/llm-runtime/src/aisdk/host_tests/driver_tests.rs`。
- `codex_skills_transport_responses_http_preserves_developer_directory_and_user_body`，位于 `crates/libs/llm-runtime/src/aisdk/host_tests/driver_tests.rs`。
- `codex_skills_transport_anthropic_http_keeps_directory_in_instruction_channel`，位于 `crates/libs/llm-runtime/src/aisdk/host_tests/driver_tests.rs`。

## 实施步骤

1. 对照本任务文件与实际父模块追踪生产所有者，记录本行为全部旧全名、cfg、fixture、资源、环境锁。以行为选择界定函数，不因文件相同吞并其它任务。
2. 纯库内局部行为迁入真实所有者独立测试文件；真实宿主权限、tenant、路由和运行时契约保留并给具体依赖理由。按行比较原断言，不扩大生产可见性或新增反向依赖。
3. 共享fixture和目标测试接线由本源文件前序任务最小建立，后序复用；修改前重读最新父文件，迁移源函数后删除宿主引入，检查没有双份执行。若已有库覆盖先做身份映射，不重复搬迁。
4. 在本卡写确切目标路径、包、过滤条件及预期数量后，再运行每个迁移族；保留宿主受影响行为也定向验证。不能只跑一个锚点就结束。

## 验证计划

纯迁移不制造人为红灯；证明原断言与新归属一致。已核查可定位命令：`python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib codex_skills_transport_chat_http_merges_instructions_and_preserves_user_body`。

库目录中的测试可能由vw-agent path/include编译，先确认当前实际归属；迁入后在所属库运行同一行为过滤，确有宿主依赖者用vw-agent命令。函数名过滤重名时收窄模块前缀。一个阶段的顺序命令合并同一cargo_slot.py入口以&&衔接。

## 完成证据

本行为完整旧→新身份和断言/资源映射，宿主保留逐项理由，移除接线证据，所属库有效运行数量/失败数/退出码，CI可发现target。不能用零测试、只编译或静态函数数代替通过。不修改未归本族函数，统计源码差额注明cfg口径。必要验证受阻保留卡，不把实现交给最终审查。
