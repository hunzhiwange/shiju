# Junie 原生技能与会话挂载验收

在 Darwin arm64 使用签名 Junie 26.11.2（4100.5）、网关 `session_model` 及回环 Chat Completions 模型 `coexistence-fixture/deepseek-v4.1-flash-1017`。模型响应由夹具提供，技能内容和 MCP 结果由真实进程读取。

## 修复前

受管 v2 的隔离网关在会话 `ses_e06f44735ffeJ5klImxTsCuZhF` 到达回环模型三次，产生一个 `chat.done`、零个 `chat.error` 和零次 MCP 调用。首个功能断点发生在项目技能发现。启动器关闭默认位置扫描，模型请求公布了 `agent_skill_read_doc`，目录中缺少私有项目的 `.junie/skills` 测试项。结构化记录为 `target/acp-skill-coexistence/junie-before.json`。

## 受管 v3

- 固定原版归档 SHA-256 为 `ad103a996f0a4f5cee051f858bdf0fc33d4ee83b7c9219971d485046f084cd3c`，签名原生入口为 `4be1f51971aa638ebd9c3caa93e63d2a235e3a4bd133011476f6cbbbcfcc565e`。v3 启动器为 `e26a1a389b7ec546d88e7399ebe07624a9d0c91b7bf3c14ff4f228ff979606ae`。六项合约摘要、整包签名及版本检查均通过。
- 使用 Junie 的显式 `--skill-location` 注册项目技能目录，再追加经宿主验证的会话视图。仍关闭默认目录、配置和扩展发现。Seatbelt 仅为选中技能的目录增加只读范围，并保留原有进程和网络限制。固定版对项目技能和会话选集使用相同的原生 `agent_skill_read_doc`。补充同名对照中，项目技能的正文和附件优先返回。 [JetBrains 技能目录](https://junie.jetbrains.com/docs/agent-skills.html#skill-locations-and-configuration)
- 技能回合复用 Safe 模式，MCP 继续从原生审批进入受限的同端口 HTTP 转接，再调用 stdio 服务。夹具使用带参数的只读工具，使固定版在审批事件中携带桥接所需的参数。沙箱外的监听方按实际 FIFO 读取独立计数。既有桥接定向测试运行 4 项，退出 0。

## 签名进程对照

四轮独立创建会话。正文、附件和 MCP 值均随机生成，并通过模型续轮中的调用 ID 匹配。从第一次包含工具的请求确认目录可见性，未提前返回正文、附件或 MCP 值。四轮均读回 `brave_mode=off` 与 `v1:6:custom:custom:shiju`，返回 `end_turn`，受管进程退出 0。

| 轮次 | 原生会话 | 模型读回的调用数 | MCP 实际调用 | 原生 MCP 审批与结果 |
| --- | --- | --- | --- | --- |
| 无挂载 | `session-261120-211615-9ra8` | 2 | 0 | 无 |
| 挂载 | `session-261120-211622-17a2` | 5 | 1 | `c31ed2c8-632f-4c35-a7e6-7d6ef4fbf3c6` 单次批准并完成 |
| 撤除 | `session-261120-211630-4i66` | 2 | 0 | 无 |
| 拒绝 | `session-261120-211638-1129` | 5 | 0 | `65b1f9fa-c44b-4632-b190-fb62f53254ba` 单次拒绝，续轮读回拒绝结果 |

具体调用 ID 位于 `target/acp-skill-coexistence/junie-native.json`。

另外在 `/private/tmp` 下复验四轮。选中技能的源目录独立于工作区，得到相同的 0/1/0/0 计数。签名、Safe 读回和调用 ID 均记录于 `target/acp-skill-coexistence/junie-native-private-tmp.json`。

## 网关验收

槽位 9 的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库下的 `target/rust-slot-9`。定向构建 `cargo build --locked -j 1 -p vw-cli --bin shiju` 编译了 Junie 接线所在的 `llm-runtime`，随后因 `task-scheduler/src/store/postgres/task_plan.rs:247,249` 访问私有的 `PlanMember.id` 报 `E0616`，退出 101。错误行来自工作计划发布提交 `4e51bd8f9f`。共享工作区补上了调度器字段访问修复。随后在槽位 10 独占复用包含 Junie v3 合约和宿主接线的新版网关，探针退出 0。

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent junie --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/junie.json'
```


网关 SHA-256 为 `bb626e51a5235a6db6037d93b0242e945e573eea8aeb0471fc17da5c49d4fdc2`。两条缓存路径均为仓库下的 `target/rust-slot-10`。四轮都读回 `acp-coexistence / junie`、`session_model`、指定模型及 `full_access=false`。网关权限摘要为 `requested=false`、`support=unavailable`，原生 Safe 读回见进程对照。模型续轮按各自调用 ID 匹配正文、附件、MCP 值和拒绝结果。

| 轮次 | 网关会话 | `chat.done` / `chat.error` | 模型请求数 | MCP 次数及审批 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_e06be8f3cffetkYI7MTWtF40f9` | 1 / 0 | 7 | 0，无审批 |
| 挂载 | `ses_e06be62e2ffe7532eSySZhJw16` | 1 / 0 | 16 | 1，单次批准 |
| 撤除 | `ses_e06be2a9cffeBv75VSXYyUgaK4` | 1 / 0 | 7 | 0，无审批 |
| 拒绝 | `ses_e06bddbd7ffeOGIVms5i6bYnR6` | 1 / 0 | 16 | 0，单次拒绝 |

结构化证据和各轮调用 ID 位于 `target/acp-skill-coexistence/junie.json`。
