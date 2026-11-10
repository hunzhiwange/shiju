# 任务 55: 验收macOS余额账号

批次：【批次 55】 依赖：批次54。线性执行顺序1→2→…→59；本批只有本任务，不安排并行。

状态：已阻塞

目的：按已交付查询适配器逐一记录macOS真实账号结果和上游版本，缺账号明确未验证，不将fixture当实测。

来源任务：母任务7；`母任务7官方额度与钱包要求`。上游路径均相对 `/Users/xiangmin/worker/code/cc-switch/`；来源清单路径相对母计划目录。

预计会话范围：只处理本标题对应的一项适配/规则/验收；不顺手实现其他供应商、登录、路由或账单。代码目标约30–60分钟（不含共享构建等待）；若出现多个独立行为或前置底座缺失，记录阻塞，不能扩成整块重建。

## 文件

- 新建/修改（本计划拟定）：`docs/shiju/apps/usage/macos_accounts.md`。

## 上下文

- 母任务1来源快照、2原生底座、3应用注册、4供应商配置、5目录元数据、6账号授权相关合同和实现均已验收。
- 当前新业务包尚未注册，代码任务开始必须检查实际Cargo.toml、模块、调用者及独立测试。本文拟新增路径若与已交付同职责模块重叠，应扩展唯一实现并在本任务记录准确路径，不保留双份体系。
- 先读 `.plans/2026-11-05-余额套餐额度与查询脚本/来源覆盖索引.md`、`docs/shiju/apps/usage/contract.md`（任务7创建；来源盘点和合同任务不依赖其存在）。
- 本次只验收macOS；不增加Windows/Linux验收。发现范围变化或新需求先询问用户是否记录回来源，不能擅改母计划。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-10`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–10。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 10 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 验证计划

- 本任务验收已有行为，不新增生产逻辑；测试驱动开发不适用，发现缺陷回写本任务后做最小修复并先补失败测试。
- macOS命令：`sw_vers`、`uname -m`；`cargo run -p vw-cli --bin shiju -- --help`。该命令仅核对当前CLI实际启动入口；必须先读底座交付的macOS启动说明，用其真实命令启动隔离网关并在证据记录确切命令。启动帮助成功不是查询验收。
- 步骤：仅按本任务的余额类别逐适配器取合法测试账号做只读查询；使用已交付SDK和运行中的真实网关，不复制用户凭据到记录。核对数值、窗口/重置、缺凭据、过期、限流及部分失败；每个适配器记录真实返回的脱敏摘要。真实失败无法安全制造时以已有聚焦测试证据补充并标明边界。没有账号的行保持未验证，任务不能伪报全通过。
- 所需证据：系统/架构、二进制及上游版本、实际启动命令、操作顺序、脱敏结果、截图/日志位置、退出码、Cargo槽位和未验证项。零账号或无实际窗口不能记作通过。

## 完成与阻塞

- 获得当前证据才能标为 `已完成`；有实际代码/测试/配置/文档变更时用 `$git-commit` 仅提交本任务变更；仅编号任务状态更新或删除不单独提交。
- 完成后最终回复报告证据并删除本任务文件，长期证据留正式文档。无关既有错误导致验证不能完成可记 `已完成，待回归` 并保留文件；缺真实前置能力/账号或资源则在本文件记录 `已阻塞`、具体原因和已运行检查，不假报通过。
- 不扩展到整块底座或无关母任务；若依赖不足，只修复本任务必需且可保持小范围的依赖，较大设计变化先说明并确认来源记录。
- plan.md只读，prompt.md不是进度来源，阻塞只写本文件；不覆盖、不提交工作区其他变更。

## 2026-11-10 验收记录

已确认批次 54 的提交 `ad9b49174` 及[无窗口交付记录](../../docs/shiju/apps/usage/coverage.md#任务-54-macos-无窗口链路)。该批交付的是使用 SDK、真实网关 Router 和本机上游夹具的验收。macOS 27.0（26A428），arm64。来源为 CC Switch 4.0.25，核对本机上游及内置快照的固定提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。

### 真实账号

检查本机进程环境的供应商凭据变量是否存在，并以只读方式统计 CC Switch 本地供应商配置。没有发现目标供应商的验收账号。仅记录配置匹配数，未提取或输出凭据值，未发起真实上游查询。

| 适配器 | 本机余额夹具通过数 | 真实账号验收 |
| --- | ---: | --- |
| DeepSeek 钱包与赠金 | 4 | 未验证，正式刷新已接通，缺测试账号 |
| StepFun | 4 | 未验证，缺测试账号和网关原生余额刷新接线 |
| SiliconFlow 国内及国际 | 4 | 未验证，两站均缺测试账号和网关原生余额刷新接线 |
| OpenRouter Credits | 3 | 未验证，缺测试管理密钥和网关原生余额刷新接线 |
| Novita | 3 | 未验证，缺测试账号和网关原生余额刷新接线 |
| 智谱个人版无套餐时回落余额 | 4 | 未验证，缺可触发回落的测试账号和网关原生刷新接线 |

当前 `NativeQuery::select` 的余额分支仅支持 DeepSeek，其他余额适配器无法通过正式 SDK 刷新。补齐涉及五类供应商的正式分派，包括 SiliconFlow 地区和智谱无套餐判定，超出本项验收的局部修复范围。复验需要补齐这些入口并提供已授权的测试账号。真实余额、币种、钱包分项、适用的窗口或重置信息，以及上游过期、限流和部分失败均待验收。无截图。

### 已执行的检查

命令 `sw_vers`、`uname -m`、`git -C ../cc-switch rev-parse HEAD` 均退出 0。Cargo 各阶段动态领取槽位 1，绝对路径 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 使用同一路径。

| 源码 | 确切命令 | 结果与日志 |
| --- | --- | --- |
| 共享工作区 | `cargo run -p vw-cli --bin shiju -- --help` | 退出 101，未启动。既有 `crates/vw-agent/src/gateway/api/handlers/session/ui_ops.rs:110` 出现 E0560，`GatewaySessionUiSummary` 缺少 `full_access`。日志 `/tmp/shiju-task55.q8eq8K/help.log`。 |
| 共享工作区 | `cargo test -p vw-agent-apps usage_balances_ -- --nocapture` | 退出 0，22 项通过，0 项失败，905 项被过滤。测试为本机 HTTP 夹具，覆盖金额和币种、缺密钥、上游拒绝、部分适配器限流与智谱无套餐判定。日志 `/tmp/shiju-task55.q8eq8K/balances.log`。 |
| 共享工作区 | `cargo test -p vw-agent usage_macos_api_ -- --nocapture` | 退出 101，执行 0 项，遇到同一 E0560。日志 `/tmp/shiju-task55.q8eq8K/gateway-sdk-fixtures.log`。 |
| 隔离提交快照 | `cargo run -p vw-cli --bin shiju -- --help` | 退出 0，CLI 帮助包含 `gateway`。日志 `/tmp/shiju-task55.q8eq8K/snapshot/help.log`。 |
| 隔离提交快照 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1/debug/shiju --version` | 退出 0，输出 `shiju 0.6.0`。日志 `/tmp/shiju-task55.q8eq8K/snapshot/version.log`。 |
| 隔离提交快照 | `cargo test -p vw-agent usage_macos_api_ -- --nocapture` | 退出 0，2 项通过，0 项失败。SDK 通过真实 Router 和本机 HTTP/TLS 夹具核对 DeepSeek 余额、429 后缓存过期、拒绝及取消。日志 `/tmp/shiju-task55.q8eq8K/snapshot/gateway-sdk-fixtures.log`。 |
| 隔离提交快照 | `cargo test -p vw-agent usage_usage_refresh_ -- --nocapture` | 退出 0，4 项通过，0 项失败。包括批量刷新、权限重检及凭据轮换的通用夹具证据。日志 `/tmp/shiju-task55.q8eq8K/snapshot/refresh-fixtures.log`。 |

隔离快照取自批次 54 提交 `ad9b49174`，带入共享工作区已有的 `postgres_tests.rs` 单行测试参数修正。参照[正式网关实机启动记录](../../docs/shiju/agent-apps/macos/claude.md#最终正式-api-实机场景)，使用 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1/debug/shiju gateway --host 127.0.0.1 --port 56407` 启动。子进程使用独立 HOME、配置目录及临时目录，未配置账号或供应商授权。确切环境和参数见 `/tmp/shiju-task55.q8eq8K/snapshot/gateway-command.json`，运行日志见同目录 `gateway.log`。启动后探测 `GET http://127.0.0.1:56407/v1/health` 返回 HTTP 200，随后本任务发送 SIGTERM，进程退出状态为 -15（Shell 等价 143）。此操作核对 CLI 和正式网关启动。

两轮共 28 项有效夹具测试通过。macOS 调试链接出现既有的 `__eh_frame` 大小提示。未改 Rust 代码，未运行格式化或 Clippy。网关及 Cargo 子进程均已退出，两轮自有槽位锁已释放，编译缓存保留。仅更新本任务状态和证据，无单独提交。

### 续跑复核

- 重新以只读方式检查 ShiJu 正式和调试 SQLite 的 `gateway_settings`，两库 `app_providers.records` 均为 0 条。CC Switch SQLite 对七个区域或供应商入口的匹配也均为 0。供应商凭据常用变量及其测试变体在当前进程和 macOS `launchctl` 环境中均无值。只输出数量和存在性，未复制任何凭据。统计见 `/tmp/shiju-task55-resume.F6gvru/account-inventory.log`。
- 直接在共享工作区重跑 `cargo run -p vw-cli --bin shiju -- --help` 和 `cargo test -p vw-agent --lib usage_macos_api_ -- --nocapture`，均退出 101，执行 0 项测试，仍报 `ui_ops.rs:110` 的 `GatewaySessionUiSummary.full_access` E0560。日志见 `/tmp/shiju-task55-resume.F6gvru/help.log` 和同目录 `gateway-sdk-fixtures.log`。两次验证环境均为槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 与其相同。
- 同一共享工作区以 `RUSTC_WORKSPACE_WRAPPER=/tmp/shiju-task55-resume.F6gvru/fresh/workspace-rustc` 生成独立的工作区编译指纹。包装脚本仅原样执行编译器，槽位仍为 1，既有缓存保留。重新运行 `cargo run -p vw-cli --bin shiju -- --help` 和 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1/debug/shiju --version` 均退出 0，版本 `0.6.0`。日志见 `/tmp/shiju-task55-resume.F6gvru/fresh/help.log` 及同目录 `version.log`。
- 启动 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1/debug/shiju gateway --host 127.0.0.1 --port 0`，网关以独立 HOME、配置及临时目录绑定 `127.0.0.1:57107`。先探测 `GET /v1/health` 得到 200，再用隔离租户选择器且不带令牌请求 `GET /v1/apps/usage/snapshot` 得到 401。未产生上游查询。发送 SIGTERM 后退出 -15（Shell 等价 143）。准确参数和环境见 `/tmp/shiju-task55-resume.F6gvru/fresh/gateway-command.json`，观察结果见同目录 `results.log` 和 `gateway.log`。
- 同一编译指纹运行 `cargo test -p vw-agent --lib usage_macos_api_ -- --nocapture` 退出 0，2 项通过、0 项失败、11873 项被过滤。链接器仍提示 `__eh_frame` 大小。日志 `/tmp/shiju-task55-resume.F6gvru/fresh/gateway-sdk-fixtures.log`。复验结束后端口无监听、两阶段自有槽位锁均已释放，缓存保留。
- 未发现第二条正式原生余额路径。库直调会跳过网关与 SDK，General 脚本产生 `CustomScript` 结果，本机夹具复用模拟上游。这三种路径无法验收真实余额。OpenRouter 原生适配器要求已授权的管理密钥，智谱需要能实际触发无个人套餐回落的账号。外部验收资源仍缺失，状态保持「已阻塞」。
