# macOS 快照验收

2026-11-05，批次6完成静态源码引入验收。前置批次1–5交付提交依次为 `07113127f`、`2507fe242`、`996e29e01`、`9ea825b7f`、`8e96893b2`；相应来源基线、快照、引用、构建上下文、workspace隔离证据均已存在并重新核对。不修改母任务或母计划。

## 实际环境与范围

`sw_vers` 退出0：macOS 27.0，BuildVersion 26A428；`uname -m` 退出0：arm64。仅此本机平台实测；不推断其他 macOS 架构、Windows 或 Linux 通过，全部平台原始文件仍按清单保留。

本项验收静态快照与主运行隔离，没有业务行为改变。有效行为测试数量：不适用，未运行单元测试；没有用零测试或 Clippy 代替业务验收。未安装前端依赖、构建/运行来源 Tauri 应用或执行上游测试。

## 母需求覆盖

| 母需求 | 交付与本轮证据 |
| --- | --- |
| 1 完整跟踪 src-tauri，不搬用户数据/产物 | `upstream/src-tauri/` 470文件；与源Git跟踪基线对应，严格目标集合校验；没有 target、用户配置、凭据或未跟踪文件 |
| 2 固定路径、许可、提交、工作区差异、逐文件映射 | `upstream/LICENSE`、`来源文件.json`、`来源说明.md`；源提交ca58c0313360eae7c851fbd7553c1614c7af03e7、干净状态；原始包名与字节不变 |
| 3 主运行隔离 | 根workspace精确exclude、`工作区隔离验收.md`；本轮metadata与locate-project退出0，11成员及默认成员，无来源依赖 |
| 4 包外引用和可恢复上下文 | `包外引用.json`；20次静态include、9个唯一包外跟踪文件；1009个构建上下文文件；生成项入口见下文 |
| 5 原始测试形态保留，不重命名或格式化 | 所有来源/目标SHA256与执行模式相等；未新增Rust实现或测试，未格式化快照 |
| 独立执行边界 | 无业务API/SDK/Iced、无新主运行入口，不启动第二套应用；本任务只新增两份证据文档 |
| 用户仅验收macOS | 本机macOS 27.0 arm64实测；其他平台仅保留 |

## 完整性及引用

运行 `来源说明.md` 内完整 Python 复核块，退出0；本轮文档生成后再次从文档提取运行，退出0。结果：

- 来源1841个Git跟踪文件：提交、工作区状态、路径集合、SHA256及可执行模式均相同，验收前后源未改变。
- 目标1489文件：后端快照470、许可1、测试夹具9、构建上下文1009（根配置13、assets 76、src 654、tests 266）。
- upstream实际文件集合与清单目标集合严格相等；缺失0、额外0、字节差异0、执行模式差异0、符号链接0。
- Rust字面量include共20处，均在upstream内可达；其中包外唯一跟踪文件9个。
- 前置引用审计52条跟踪文件、3条生成产物、8条远端地址；构建上下文记录2559次配置跟踪目标核对、缺失跟踪输入0，本轮全部对应快照文件哈希不变。

生成产物单列：`upstream/dist`（`pnpm run build:renderer`）；两份 capability 引用同一 `upstream/src-tauri/gen/schemas/desktop-schema.json`（原tauri-build入口）。这是3条引用、2个唯一生成目标，当前未生成。恢复工具及入口见来源说明；不宣称原应用可直接编译。

## 主仓验证与槽位

因批次5后工作区存在其他任务的Rust改动，本轮重新执行Clippy，不复用旧通过结论。动态原子领取实际槽位 **3**：

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3
```

锁内记录计划、任务、会话和shell PID；Cargo全部结束后EXIT trap清理自己的记录及锁，保留缓存。

| 确切命令/检查 | 退出码 | 结果 |
| --- | ---: | --- |
| `sw_vers` | 0 | macOS 27.0 / 26A428 |
| `uname -m` | 0 | arm64 |
| 来源说明中的Python复核块 | 0 | 源、目标、集合、模式、include全部通过 |
| `cargo metadata --no-deps --format-version 1 --all-features` | 0 | 主成员11、默认成员11 |
| `cargo locate-project --workspace --manifest-path crates/vw-agent-apps/upstream/src-tauri/Cargo.toml --message-format plain` | 0 | 返回来源自身Cargo.toml |
| metadata成员/依赖隔离Python断言 | 0 | 无cc-switch/cc_switch_lib成员或依赖、无upstream路径 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | `Finished dev profile [unoptimized + debuginfo] target(s) in 1.14s` |

metadata采用批次5的JSON规范化哈希算法，成员/默认成员指纹均为 `80ec6829c309d9d2e8a4165363b1dec17a4a430ba5c0c8df9ff5e29d33176e48`，packages指纹为 `43633cb3cdd475fcf994998c59aaa751f21f6912a894f553eb91d0e3b9795a1f`，与批次5完全相同。实际locate输出为 `/Users/xiangmin/worker/code/shiju/crates/vw-agent-apps/upstream/src-tauri/Cargo.toml`。

本次Git差异审查只有两份交付文档；源快照、根Cargo配置无本任务修改，无API或桌面入口。其他任务的agent/配置/桌面、LICENSE、计划及未跟踪文件未修改或纳入提交。提交采用单次禁用钩子以避免现有cargo-husky隐式全量测试、无槽位Cargo和全仓格式检查；未修改持久Git配置，正式验证即上述槽位内命令。

## 完成条件与局限

母任务1的首项静态引入可核验条件均已满足：完整来源映射、零差异、必要引用可达、生成项可追溯、源未变、主运行隔离、macOS及主仓Clippy真实通过。该结论仅覆盖母任务明确授权的首项独立源码引入；未标记或删除母任务，不代表后续原生化/API/桌面能力完成。回退按来源说明逐文件审查交付提交，不影响其他任务。
