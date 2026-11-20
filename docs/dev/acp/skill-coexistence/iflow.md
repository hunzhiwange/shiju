# iFlow 原生技能与 ShiJu 会话技能

- 平台为 Darwin arm64。固定入口为 `@iflow-ai/iflow-cli@0.5.19` 的 `bundle/entry.js`，以 `--experimental-acp` 启动。宿主为 `session_model` 追加模型锁，使用私有 OpenAI Chat Completions 回环和 `deepseek-v4.1-flash-1017`。
- v1 回归基线使用槽位 10 中已有的网关 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。同一探针退出 1，首个断点为基线模型请求缺失项目原生技能目录。网关产生 1 个 `chat.error`，未触发技能或 MCP。报告位于 `target/acp-skill-coexistence/iflow-baseline-cached.json`。
- v2 在原生全局、项目目录之后追加会话视图。同名时保留原生优先级。有选集时继续由宿主验证会话视图和受管制品。空选集保留原生 `Skill` 注册，受管流程仍禁用进程全局技能缓存回退。
- v2 的 `bundle/entry.js` SHA-256 为 `55710b7cfea2e5aefebfe0acec4eaf3e647d34538ee5835a3d724a49853d1fa6`，`bundle/iflow.js` 为 `e8590e0e5923809928c2c831272fe4d05f917a13e95dfd862fb1c52c4634cd57`。版本与 npm lock 保持 0.5.19。
- 补充直连 ACP 取证的四轮均使用 `default` 模式，返回 `end_turn`。模型分别收到了 2、5、2、5 个按调用 ID 关联的技能激活、附件读取或 MCP 结果。目录在首次请求只包含元数据。挂载轮以单次批准运行只读 stdio 工具，拒绝轮在模型续轮收到取消结果。MCP 下游计数为 0、1、0、0。
- 受管安装器使用固定 lock 和私有 npm 缓存重新安装，退出 0，入口报告 `0.5.19`。直连的额外同名对照读到项目原生正文和原生附件。两个 iFlow 挂载夹具定向用例通过。

## 真实网关复验

共享构建槽位 3 执行 `cargo build --locked -j 1 -p vw-cli --bin shiju`，退出 0。复用已构建的 v2 网关快照，SHA-256 为 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。验收持有槽位 9，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为仓库绝对路径下的 `target/rust-slot-9`。执行 `verify_acp_skill_coexistence.py --agent iflow` 退出 0，完整调用 ID 与会话记录见 `target/acp-skill-coexistence/iflow.json`。

| 独立会话 | 模型请求 | `chat.done` / `chat.error` | 读取和审批 | MCP 下游 |
| --- | ---: | ---: | --- | ---: |
| 基线 | 3 | 1 / 0 | 原生技能及附件 | 0 |
| 挂载 | 6 | 1 / 0 | 原生与选中技能及附件，MCP 单次批准 | 1 |
| 撤除 | 3 | 1 / 0 | 原生技能及附件 | 0 |
| 拒绝 | 6 | 1 / 0 | 原生与选中技能及附件，MCP 显式拒绝 | 0 |

四轮均回读 `iflow`、`session_model` 和 `coexistence-fixture/deepseek-v4.1-flash-1017`。宿主回读 `requested=false` 和 `support=unavailable`，按工作区边界审批。三类随机内容分别经原生 `Skill`、`read_file` 和 stdio MCP 回传。各轮初始模型请求只见技能元数据，目录均未出现未选技能。获准 MCP 调用 `call_2c3a6f0deae141e60c977295` 返回试验值，拒绝调用 `call_8865c6a0dcd7629594f09def` 返回取消结果。
