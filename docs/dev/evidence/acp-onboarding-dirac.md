# Dirac 接入证据

日期：2026-11-13；实机 macOS arm64；npm 当前稳定发行物 `dirac-cli@0.5.24`，Node 24.14.0。不是任务历史记录中的 0.5.11。

## 来源与安装

- 官方 [npm registry](https://registry.npmjs.org/dirac-cli)、[CLI 文档](https://github.com/dirac-run/dirac/blob/master/cli/README.md)、[源码](https://github.com/dirac-run/dirac)。查询 `npm view dirac-cli version repository homepage bin --json` 返回 version=0.5.24、bin=dist/cli.mjs、repository=dirac-run/dirac。
- 固定发行物 dist/cli.mjs SHA256：`c88e043f590a520da3345050e32bbe6930a68cefad60945ef4a7d14648217429`。
- 官方要求 Node >=22.13.0 <25。宿主 Node 26 不满足要求，本次隔离准备 Node 24.14.0；执行发行物 `--version` 返回 0.5.24。未更新用户全局安装。
- 参考固定 tar.gz 的 025 迁移已只读核验：binary_name=dirac、npx argv 为 -y dirac-cli --acp、技能目录 .dirac/skills；没有复制历史 yolo 授权。
- 官方源码调查 HEAD=a7cf472759250159246af4f0729a4aa8f2ff3dc3；源码用于调查，能力结论仍以固定发行物实测为准。
- 临时目录 `/tmp/shiju-dirac-probe`；隔离安装 `npm install --prefix /tmp/shiju-dirac-probe --ignore-scripts --registry=https://registry.npmjs.org dirac-cli@0.5.24`。不提交二进制、原始配置、凭据或用户消息。

## 真实 CLI 协议

使用临时 HOME、--config 和工作目录，以及本地 HTTP 模型响应服务；只使用虚拟凭据。agentInfo 为 dirac/0.5.24，protocolVersion=1；loadSession=true，prompt 支持 image/embeddedContext、不支持 audio。模型请求实际到达 `/v1/chat/completions`，model=fixture-model。

- two-turns.log：同会话两轮 end_turn，同进程 session/load 成功。
- denial.log：write_to_file 触发真实 session/request_permission，宿主 cancelled 后未生成目标文件。
- write-final.log：允许 allow_once 后，通过真实 fs/read_text_file、fs/write_text_file 写入临时文件，内容为 fixture，两轮 end_turn。早期夹具未回应 read 或误把对端请求当响应，修正后重新运行，不使用早期结果作为通过证据。
- exec-denial-verified.log：真实 execute_command 权限请求返回 cancelled，命令目标文件未生成。早期命令夹具字段不符合上游 commands 数组契约，不作为拒绝证据。
- cancel.log：活动轮 cancelled，下一轮 end_turn。
- auth-failure.log/model-failure.log：HTTP401/404 的受控上游拒绝进入供应商 API 错误更新；上游最终仍返回 end_turn，不能仅凭 stopReason 断言模型请求成功。未切换模型。
- mcp.log：向 new/load 传入带启动标记的 selected stdio 服务，服务未启动。官方 DiracAgent.newSession/loadSession 未消费 mcpServers；握手未声明 MCP 支持。宿主必须显式拒绝所选 MCP，不能把接受参数当作支持。

## 双重交付

- `dirac` 注册：本地发现 `dirac --acp`，备用 `npx -y dirac-cli@0.5.24 --acp`；安装白名单固定相同版本。
- `acp-dirac` 由生产 `merged_agent_catalog` 派生；builtin=true、runtime.backend=dirac、默认停用、不允许作为子代理；名称、说明、推荐提示词及模型/权限/技能默认继承走既有链路。目录、保存重读、启用/停用与流入口身份通过生产入口测试，外部进程边界使用受控 ACP 对端。
- 新增会话模型分支使用 OpenAI Chat 中继，`DIRAC_PROVIDER/MODEL/BASE_URL/API_KEY` 绑定到会话选中模型与请求能力；配置目录按租户、项目、会话、provider/model 和 generation 哈希隔离，验证目录0700、所属用户与非符号链接。拒绝额外启动参数、用户冲突环境和非回环 endpoint，令牌不进入 argv 或配置文件。
- 真实 CLI 的相同显式环境契约已用回环服务证明 fixture-model 到达实际 Chat 请求；原生测试验证 profile 的路由变量、私有目录、冲突拒绝和释放。实际网关到外部模型的全链路与其他平台仍由独立验收任务跟进，不冒称已执行额外桌面/公网模型验收。

## 能力边界

- Native 完全访问不开放，维持 ClientOnly 的宿主权限门控；yolo/auto_approve 不等于宿主授权。
- 技能源码 skillsStorage.getSkillsDirectoriesForScan 合并项目 .dirac/.claude/.ai/.agents 与全局目录；当前没有经验证的只挂载所选技能的隔离入口。保持现有明确拒绝分支，不声称上游不支持技能。
- MCP 原生 ACP 参数未被消费；与 CLI 自己的全局 MCP 配置不是同一能力，不能以全局配置替代会话选择。
- Unix/Windows 仅安装契约数据测试；其他 OS 未实机运行。未声称跨进程恢复已通过。

## 原生验证

所有 Cargo 经 `python3 scripts/cargo_slot.py`。红灯阶段使用槽位 8，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-8`：

- `cargo test -p vw-acp --test onboarding_dirac -- --test-threads=1`：退出101，2通过/1失败，注册缺失。
- `cargo test -p vw-config-types --lib acp_onboarding_dirac -- --test-threads=1`：退出101，0通过/1失败，安装映射缺失。
- `cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_dirac -- --test-threads=1`：退出101，0通过/3失败，builtin metadata、binding 和生产配置/路由缺失。仅沿用前批次命令行编译优化调整，没有更改 features 或仓库 profile。

第二阶段槽位 4，两个缓存目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。MCP 红灯退出101（4通过/1失败），证明需要宿主提前拒绝。

当前已取得：ACP 5/5、安装 1/1、注册保护 24/24、目录 12/12 均退出0；整仓 `cargo clippy --all-targets --all-features -- -D warnings` 退出0。

初次 vw-agent 通过阶段发现本任务测试夹具的 moved backend 编译错误，修正后又取得测试失败：2通过/2失败（固定 MCP 默认项仍引用 selected，环境冲突夹具误携带保留 launch reference）。已修正夹具，最终聚焦4/4通过、退出0；最终 Clippy 退出0。

最终回归阶段槽位2，两个缓存目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。日志 red.log、stage.log、final.log 位于临时证据目录；槽位入口结束后自动释放自有锁，保留缓存。

因工作区存在并发任务修改，格式化限于本任务 Rust 文件，使用 `rustfmt --edition 2024 --config skip_children=true`，避免全仓格式化覆盖无关变更。

最终槽位2命令：

- `cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_dirac -- --test-threads=1`：4通过，退出0；包含内置标识、目录绑定、生产配置保存重读/启用停用/正确流路由和模型隔离冲突测试。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出0，34.95s。

槽位4其余确切命令：`cargo test -p vw-acp --test onboarding_dirac -- --test-threads=1`（5/5）；`cargo test -p vw-config-types --lib acp_onboarding_dirac -- --test-threads=1`（1/1）；`VIBEWINDOW_CONFIG_DIR=/tmp/shiju-dirac-probe/registry-config cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1`（24/24）；`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib agent::catalog_tests -- --test-threads=1`（12/12）。均退出0。

原生 vw-agent 测试链接仍输出仓库已有的 macOS __eh_frame 过大提示，首次修改前红灯也存在；最终 Clippy 无警告失败。未运行全量单测、fmt check、wasm check 或 Makefile。未修改只读 plan.md，未纳入其他任务变更。批次3已有 DimCode 提交724514d1c，依赖已满足；其额外独立验收不影响本接入的代码依赖。后续用户模型全链路验收沿用25号任务。
