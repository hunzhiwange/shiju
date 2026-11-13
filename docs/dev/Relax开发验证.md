# ShiJu 使用 Relax 的开发验证

## 默认开发命令重新验收与 macOS 桌面包装

重新直接运行 `cargo dev-gateway` 与 `cargo dev-desktop`，没有指定测试端口或覆盖日常桌面配置。gateway 在默认 `127.0.0.1:42617` 返回 HTTP 200、`status: ok`，响应 PID 与本次运行器一致。原先未包装的桌面已进入 macOS 事件循环，`relax inspect ... panics` 没有挂起 panic；这不能单凭进程存在就认定窗口可用。

本次为 macOS 桌面入口增加自动生成的 `.cache/relax-dev/ShiJu Relax Dev.app`，复制同一运行器并设置每个 checkout 独立的 bundle identity，直接执行包内程序，保留当前终端的环境变量、参数及进程生命周期。更新运行器后自动替换包内副本；Linux 不经过此包装。无需手工创建或打开 `.app`，仍执行原来的 `cargo dev-desktop`。

修复后 CUA 能按该专用 bundle 定位 `ShiJu Relax Dev` 的 `识矩 - 项目` 窗口、读取菜单并观察屏幕渲染；两端保留运行供人工检查。脚本聚焦测试增至 6 项，覆盖包身份、执行权限及运行器更新。证据见[默认命令重新验收](evidence/relax-startup-2026-11-13.json)。

**仍有限制：**终端仍会出现依赖后台兼容诊断，包括 iced_aw 宏导入和部分未实现标准库方法。此次没有将这些报错全部修复，也没有通过删除诊断或关闭检查伪装成功；健康响应和独立窗口只证明本次启动路径，不等于所有业务功能均通过。

## 团队一键入口（当前仓库）

团队成员拉取本次变更后，在仓库根目录执行；无需复制本机 `/tmp`、设置环境变量或安装全局 Cargo 插件：

```bash
# 可选：提前安装；下面两条开发命令首次也会自动执行安装。
cargo dev-setup

# 两个终端分别运行，保持常驻。
cargo dev-gateway
cargo dev-desktop
```

希望继续输入原来的两条命令时，在各终端执行一次：

```bash
source scripts/relax-env.sh
cargo run -p vw-cli --bin shiju -- gateway
# 另一个已 source 的终端：
cargo run -p vw-desktop --bin shi-ju
```

`source` 只给当前 bash/zsh 终端增加 `cargo` 函数，精确转发上述两条命令到 Relax；其他 Cargo 命令仍调用系统 Cargo。关闭终端或 `unset -f cargo` 恢复。没有 source 时，原生 `cargo run` 仍走 rustc。添加其他参数时建议使用 `cargo dev-gateway --port 54771` 等专用入口，不会自动创建或迁移桌面网关身份；端口与桌面现有连接配置应一致。

Cargo 别名先编译仓库内无第三方依赖的小型启动器 `tools/relax-dev`，再调用 Python 脚本；这是 Cargo 支持的 `run` 别名，不依赖全局插件。Cargo 配置机制见[官方文档](https://doc.rust-lang.org/cargo/reference/config.html#alias)。

安装由 `scripts/relax_dev.py` 完成：校验仓库两份补丁，下载固定提交，应用兼容修复及 vendored 版本筛选修复并 release 构建；按工具链、提交与补丁哈希缓存。安装锁防止两个终端重复安装；不完整安装会重试，损坏的已安装二进制会明确拒绝。首次需联网，耗时与占用明显高于后续启动。

为兼容本机 rsproxy 等祖先目录 registry 配置，脚本通过 `cargo metadata --locked --all-features` 获取 Cargo 已校验下载的依赖，按 `Cargo.lock` 复制 registry 包并生成 directory-source 校验清单；Git 依赖仍使用 Cargo 缓存。收集全部 features 的依赖只是为委托 Cargo 准备完整源，不改变两端运行的默认 features。这样避开整个 workspace 的 `cargo vendor` 对 registry/Git 同名同版本 `crossterm` 的冲突。按锁文件更新依赖副本，在忽略目录 `.cache/relax-dev/run/.cargo/config.toml` 生成局部 directory-source 配置。Relax 从此目录启动，解析的仍是当前完整工作区，**直接编辑当前 Git 工作区即可热更新**，无需同步冻结快照。运行时工作目录为该子目录，应用若使用相对路径，需留意这一点。脚本不覆盖已有 Cargo 配置，也不改依赖 features、用户身份或认证设置。

缓存和原生输出均在 `.cache/relax-dev`，已被仓库忽略。更新锁文件后入口会重新准备依赖；工具版本/补丁变更后自动使用新工具目录。退出所有相关进程后可删除该目录完整重装。不要删除后仍期待当前进程继续热更新。

要求开发机已有 Python 3、Git、Rust/Cargo 和项目原有的 C/C++ 及平台 GUI 构建依赖；本入口自动安装 Relax，不接管系统包管理器。支持入口平台为 macOS/Linux，定量实测仅 macOS arm64；Windows 明确拒绝并保留原生 Cargo 路径，不能把尚未验证的平台说成已支持。当前后续工作区业务修改仍需实际启动验证；完整生态兼容性限制见下文。

本轮一键入口已在 macOS arm64 实际完成自动安装、gateway HTTP 200 / `status: ok`、桌面进入 main 后持续运行 30 秒和源码监听；桌面实际渲染未在本轮重新做图像验收。5 项脚本测试、两版本真实编译探针与缺失版本负向检查、仓库及启动器 Clippy 均通过。依赖后台兼容诊断仍存在，不能视为所有功能通过。测试进程已关闭；[本轮证据](evidence/relax-team-2026-11-13.json)。可运行 `python3 scripts/verify_relax_vendor.py` 复验新增解析补丁。

## 2026-11-13 编辑循环重新实测

按要求作废机器高负载时的上一轮记录，重新测量完整 gateway 与桌面。**对本次保持签名不变的函数体字符串改动，Relax 常驻热更新明显提速：网关约 58 倍，桌面标题约 55 倍。** 每项均为 5 次有效采样的中位数，不把预热或首次启动混入编辑耗时。

| 场景 | Cargo | Relax 常驻 | 中位数比值 |
| --- | ---: | ---: | ---: |
| gateway：保存到新 HTTP 标记响应 | 67.784 秒 | 1.160 秒 | 58.45 倍 |
| 桌面：构建及启动 / 保存到 OS 新标题 | 36.249 秒 | 0.656 秒 | 55.29 倍 |

逐次秒数：

- gateway Cargo：66.803 / 68.101 / 68.951 / 67.784 / 67.679；Relax：1.382 / 1.160 / 1.154 / 1.210 / 1.100。
- 桌面 Cargo：37.499 / 36.844 / 36.249 / 32.977 / 36.048；Relax：0.768 / 0.608 / 0.765 / 0.656 / 0.626。

### 测了什么

沿用上一轮验证成功的完整源码冻结快照与同一修复版运行器（SHA `58a16d94f7f1efda643180bb4c4c7fb5743e8514030f222acd755f190c0ab646`），不裁剪 workspace 或默认 features。仅临时修改快照里的两个函数体：`vw-agent` 的 `gateway/health.rs::handle_health` 在健康 JSON 中加入唯一 `edit_probe`；`vw-desktop` 的 `app/impls/app_basic.rs::App::title` 改变 Home/Project 窗口标题字符串。数字标记等长，函数签名和返回类型保持不变。

网关计时从保存前开始，到 `/v1/health` 返回 HTTP 200、`status: ok` 与本轮唯一标记结束。Cargo 包括停止旧实例、`cargo run` 构建及新实例启动；五个新 PID 均核实属于本次启动。Relax 五轮始终为 PID 29514，返回不同的新标记；日志每轮都显示安装 1 个新函数，排除了旧响应或重启替代热更新。

桌面 Cargo 使用 `cargo build` 后把同一原生产物经临时 `.app` 启动，以便 CUA 精确识别。其列值为构建/复制耗时加实际启动至 OS 标题可读的耗时；工具调用间的人工协调空等被扣除，原始 wall time 与扣除量逐项保留。五次实际启动耗时为 0.867 / 0.918 / 0.923 / 0.965 / 0.988 秒，构建分别为 36.614 / 35.919 / 35.310 / 31.995 / 35.053 秒。

Relax 桌面先开启 CUA 标题观察，再按定时任务保存源码，保证观察器开始早于每次保存，避免协调空等掺入热更新时间。五轮均在同一 PID 40306、同一窗口内确认 `d21` 至 `d25` 新标题；最后实际截图确认主界面仍正常渲染。日志每轮安装 1 个函数，安装阶段约 39–71 毫秒；表格使用的是更完整的保存到 OS 标题生效时间。

### 环境、排除项和边界

- Mac16,7，14 个逻辑 CPU、48 GiB 内存；Cargo 固定 4 jobs，原有 dev `opt-level = 1` / `line-tables-only` 保留。Relax 使用默认 balanced JIT，并传 `--cfg debug_assertions`；两个编译器的优化策略、线程策略和 profile 实现仍有差异。
- 每轮采样前检查两次即时 CPU 空闲率，均达到 60% 才保存；没达到时先等待，等待不计入耗时。定时桌面首条保存有 40 秒观察器准备时间，所有检查与保存时间戳均保留；没有隔离 CPU 或锁定频率。
- 上一轮高负载预热中的 `rustc` SIGKILL 已单独记录为废弃；本轮两个原生预热均成功。两边保留缓存，不执行 `cargo clean`。预热、首次启动、桌面初始窗口和一次缺少启动时间戳的协议试跑均不进入上述 5 次统计。
- 网关本轮首次可用时间另记为 Cargo 18.947 秒、Relax 41.781 秒；它们有各自缓存与启动准备条件，不能据此推导干净全量构建的提速。
- 本轮顺序为先 Cargo、后 Relax，没有随机交错；样本数量有限。结果适用于这两个函数体常量改动，不外推到类型、trait、依赖、布局变化或所有 GUI 控件。
- 测量后已退出自有网关和桌面，释放 slot 9；两个快照文件 SHA 均恢复原值。实际工作区业务源码与全局 Cargo 配置未改。本轮仅归档文档/数据，使用真实构建、HTTP 与窗口观测验收，没有运行全量单元测试。

明细、负载、原始时间与排除记录见 [重新实测数据](evidence/relax-edit-2026-11-13.json)，测量脚本审查归档见 [脚本文本](evidence/relax-edit-2026-11-13.py.txt)。原始日志和脚本运行环境保留在 `/tmp/shiju-relax-edit-bench-20261113-restart`；脚本文本用于审查，重新执行需先准备快照、原件备份和独立构建槽。

建议先将 Relax 常驻用于这些已验证的函数体热改场景，再单独验证真实复杂视图与类型修改。兼容性和完整运行边界见下文原有验证记录。


## 如何在本机获得这个效果

### 结论与待办

这是**开发时保持程序运行、保存源码后替换函数**的收益，不是把原来的 `cargo run` 加一个配置就自动快 55–58 倍。需要换用本次修复版 Relax 启动程序，然后保留进程；下一次调用改过的函数时才会执行新代码。

| 要做的事 | 状态 / 操作 |
| --- | --- |
| 修复 reqwest、Rhai、Tower 等兼容问题 | 已完成，补丁在 `patches/relax/compatibility.patch`；普通未打补丁的基线版本不能替代 |
| 准备运行器、Relax 标准库、完整 ShiJu 源码和依赖缓存 | 本机临时环境已有；先按下面快速体验，长期使用再迁出 `/tmp` |
| 用 Relax 启动 gateway 和桌面并保持运行 | 需要执行下面两条启动命令；不要加 `--unattended` |
| 修改正确的源码目录并触发相关行为 | 快速体验编辑的是 `/tmp/.../source`，不是当前 Git 工作区 |
| 将开发对象切到最新工作区 | 尚未验证：本轮使用冻结快照，当前工作区后来新增的代码不包含在结论中 |
| 继续做 Cargo 检查和发布构建 | 保留；Relax 不替代 `cargo clippy --all-targets --all-features -- -D warnings` 和 `cargo build --release` |

**本报告提供配置操作说明，本次没有安装到全局、修改 shell 配置或启动新的常驻服务。**

### A. 立即体验：复用本机已验证环境

前提：以下 `/tmp` 目录尚未被系统清理，且本地 `54771` 端口空闲。可先用 `lsof -nP -iTCP:54771 -sTCP:LISTEN` 检查；若已占用，先辨认进程，不能直接结束其他服务。以下配置只影响当前终端，不修改 `~/.cargo/config.toml`。

在**两个终端分别**执行同一段环境配置：

```bash
cd /tmp/shiju-relax-full-eval-20261112/source
export RELAX=/tmp/shiju-relax-fix-evidence/relax-validated
export CARGO_HOME=/tmp/shiju-relax-full-eval-20261112/cargo-home
export CARGO_NET_OFFLINE=true
export CARGO_BUILD_JOBS=4
export MAKEPAD_RUN_OFFLINE=1
export MAKEPAD_RUN_STD=/tmp/shiju-relax-eval-tool/libs/relax/compiler/std
# 使用新的专属缓存和 Cargo 输出目录，不占用已释放的测试 slot 9。
export MAKEPAD_RUN_CACHE_DIR=/tmp/shiju-relax-personal-dev/cache
export CARGO_TARGET_DIR=/tmp/shiju-relax-personal-dev/cargo-target
export CARGO_BUILD_BUILD_DIR=/tmp/shiju-relax-personal-dev/cargo-build
export VIBEWINDOW_CONFIG_DIR=/tmp/shiju-relax-edit-bench-20261113-restart/desktop-config
mkdir -p "$MAKEPAD_RUN_CACHE_DIR" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR"
test -x "$RELAX"
test -d "$MAKEPAD_RUN_STD"
shasum -a 256 "$RELAX"
```

运行器哈希应为 `58a16d94f7f1efda643180bb4c4c7fb5743e8514030f222acd755f190c0ab646`。新建空缓存会重新预热，第一次启动可能较慢；上面的路径可持续复用，不要每次换缓存或清理缓存。

终端一启动 gateway，看到日志后在第三个终端检查健康接口：

```bash
"$RELAX" run --verbose --allow-c --cfg debug_assertions \
  vw-cli --bin shiju -- gateway --host 127.0.0.1 --port 54771
```

```bash
curl --noproxy '*' --fail --silent --show-error \
  http://127.0.0.1:54771/v1/health
```

确认返回 `status: ok` 后，终端二启动桌面：

```bash
"$RELAX" run --verbose --allow-c --cfg debug_assertions \
  vw-desktop --bin shi-ju
```

这里复用了测试专用桌面配置，其 gateway 地址为 `127.0.0.1:54771`，不会覆盖日常桌面配置。若这些配置文件已不存在，需在桌面网关设置中配置同一地址和对应身份；不要把网关身份或认证信息随意替换为空。若改端口，还要同时修改该专用目录中 `gateway-client-bootstrap.json` 顶层和 `servers` 所选项的 `host` / `port`。无需测试时的 HTTP 转发代理，也无需为手工使用创建 CUA 专用 `.app`。

### B. 保存后怎么确认真的生效

1. 保持上述两个进程运行，用编辑器打开 `/tmp/shiju-relax-full-eval-20261112/source`。
2. 先只改 `crates/vw-desktop/src/app/impls/app_basic.rs` 中 `App::title()` 的 Home/Project 标题字符串，例如把 `识矩 - 项目` 改为 `识矩 - 项目（热更新验证）`。保持函数签名、返回类型不变。
3. 保存后观察终端出现类似 `app_basic.rs: patched (... 1 fn(s) installed)`，再让窗口发生一次交互/重绘，检查标题。进程 PID 应保持不变。
4. gateway 可在 `crates/vw-agent/src/gateway/health.rs::handle_health` 的健康 JSON 中临时增加 `"edit_probe": "local_01"`，保存后重新请求 `/v1/health`，确认新字段。恢复该改动后再请求，确认字段消失。
5. 恢复标题改动；结束开发时桌面正常退出，gateway 在所属终端按 Ctrl+C。

日志显示 `patched` 只说明代码已替换；已存在的窗口、已计算的结果、已加载的配置不会自动重新初始化。应触发一次真正调用改后函数的操作。正在执行中的旧调用也不能用来确认新函数效果。

| 配置项 | 为什么需要 |
| --- | --- |
| 不传 `--unattended` | 本次固定版的该选项明确禁用 live session；旧历史记录里用它做失败退出测试，不能照抄到热更新开发 |
| `--cfg debug_assertions` | 保留 ShiJu 开发模式的条件编译行为；不代表完全复刻 Cargo dev profile |
| `--allow-c` | 当前依赖中的原生 C/C++ 组件仍需 Cargo、C 编译器及相关构建环境 |
| `MAKEPAD_RUN_STD` | 指向与修复版匹配的 Relax 标准库源码；只复制可执行文件不够 |
| 固定缓存目录 | 复用编译结果；空缓存启动不属于这里的亚秒/秒级编辑耗时 |
| `CARGO_HOME` 和两个 offline 变量 | 复用本次已备齐依赖的隔离环境，避开本机 registry 配置冲突；缺依赖时离线模式不会自动下载 |
| 两个 Cargo 输出目录变量 | 同时覆盖仓库 `target-dir` 与 `build-dir`，避免与常规 Cargo 开发共享输出路径 |
| `VIBEWINDOW_CONFIG_DIR` | 将本次桌面连接设置与日常配置隔离；gateway 仍是完整运行时，不是模拟服务 |

### C. 长期使用：迁出临时目录

以下是**待执行的安装方案**，没有把新安装路径误计为已验收环境。工具重建后需重新跑上面的健康检查与标题热改。要求 Git、现有 Rust/Cargo 和本机 C/C++ 工具链可用。新环境联网拉取工具；大体积依赖缓存复制需要预留磁盘空间。

为避开本机用户目录祖先中的 Cargo registry 配置，本方案沿用“工作区置于用户目录之外”的做法，使用仅当前用户可访问的 `/Users/Shared` 子目录。选择一个未存在的目标目录；若已存在，先检查内容，不要覆盖已有环境。

```bash
export SHIJU_RELAX_ROOT="/Users/Shared/shiju-relax-$(id -un)"
# 仅首次执行；若目录已存在，mkdir 会失败，应先检查而不是继续覆盖。
(umask 077; mkdir "$SHIJU_RELAX_ROOT")
```

确认目录创建成功后，复制已验证的完整快照和依赖缓存。`-L` 会把缓存符号链接指向的内容复制为实际文件，避免迁移后继续依赖 `/tmp`；可能占用数 GiB 以上空间。不要在源目录正被修改或编译时复制。

```bash
rsync -a --exclude='.git' --exclude='/target/' --exclude='/target-dir/' \
  /tmp/shiju-relax-full-eval-20261112/source/ "$SHIJU_RELAX_ROOT/source/"
rsync -aL /tmp/shiju-relax-full-eval-20261112/cargo-home/ \
  "$SHIJU_RELAX_ROOT/cargo-home/"
mkdir -p "$SHIJU_RELAX_ROOT/desktop-config"
cp /tmp/shiju-relax-edit-bench-20261113-restart/desktop-config/gateway-client-bootstrap.json \
  "$SHIJU_RELAX_ROOT/desktop-config/"
chmod 600 "$SHIJU_RELAX_ROOT/desktop-config/gateway-client-bootstrap.json"
```

随后下载固定工具版本、应用仓库内补丁并重建；这些操作只修改独立工具目录：

```bash
git clone https://github.com/makepad/relax.git "$SHIJU_RELAX_ROOT/tool"
git -C "$SHIJU_RELAX_ROOT/tool" checkout 0e2a536309c5ee7c7a47d8806d603a9c12b9a586
cd "$SHIJU_RELAX_ROOT/tool"
git apply --check /Users/xiangmin/worker/code/shiju/patches/relax/compatibility.patch
git apply /Users/xiangmin/worker/code/shiju/patches/relax/compatibility.patch
CARGO_HOME="$SHIJU_RELAX_ROOT/cargo-home" \
CARGO_TARGET_DIR="$SHIJU_RELAX_ROOT/tool-target" \
CARGO_BUILD_BUILD_DIR="$SHIJU_RELAX_ROOT/tool-build" \
  cargo build --release -p relax-runner --bin relax -j 4
```

若依赖未备齐，先在这个隔离目录解决 Cargo 下载/registry 配置，不要关闭下载校验或修改 ShiJu features 来绕过。构建输出的二进制哈希可能受路径/工具链影响，不要求与原二进制逐字节相同；应用补丁的基线和内容应固定。工具 checkout 与标准库目录需保留。[补丁说明](../../patches/relax/README.md)记录了补丁校验值与回滚方法。

长期使用时，两个终端改用以下环境，再执行 A 中相同的 gateway / 桌面启动命令：

```bash
export SHIJU_RELAX_ROOT="/Users/Shared/shiju-relax-$(id -un)"
cd "$SHIJU_RELAX_ROOT/source"
export RELAX="$SHIJU_RELAX_ROOT/tool-target/release/relax"
export CARGO_HOME="$SHIJU_RELAX_ROOT/cargo-home"
export CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true MAKEPAD_RUN_OFFLINE=1
export MAKEPAD_RUN_STD="$SHIJU_RELAX_ROOT/tool/libs/relax/compiler/std"
export MAKEPAD_RUN_CACHE_DIR="$SHIJU_RELAX_ROOT/cache"
export CARGO_TARGET_DIR="$SHIJU_RELAX_ROOT/cargo-target"
export CARGO_BUILD_BUILD_DIR="$SHIJU_RELAX_ROOT/cargo-build"
export VIBEWINDOW_CONFIG_DIR="$SHIJU_RELAX_ROOT/desktop-config"
mkdir -p "$MAKEPAD_RUN_CACHE_DIR" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR"
```

该 `source` 仍是开发快照，不带 Git 历史。实际日常开发若要改最新 Git 工作区，需要将启动目录和编辑器统一指向那份完整 checkout，保留其锁文件及依赖版本，并重新验证；不要改了原工作区却等待快照里的程序更新。本报告没有宣称后续工作区改动也已经兼容。

### 什么时候重启或退回 Cargo

- 函数体内的小范围逻辑/字符串变化可先尝试热改；本次只定量验证了两个字符串场景。
- 修改类型布局、函数签名、trait/impl 头、导入、宏或依赖后，不应期待相同效果；停止并重新启动 Relax，再做行为确认。固定版官方也明确列出了这些热更新边界，见 [固定版本说明](https://github.com/makepad/relax/blob/0e2a536309c5ee7c7a47d8806d603a9c12b9a586/README.md#the-tradeoffs)。
- 遇到编译诊断、panic、没有安装新函数或行为无法核实，保留日志与最小改动，使用 Cargo 对照。不要忽略错误后把旧行为当成热更新成功。
- 本次全依赖后台检查仍有 520 项兼容诊断。复杂视图、全部工具调用、所有平台没有完整验收；Relax 适合作为可选开发路径，Cargo 继续承担合并检查与发布。


## 完整联动结论

2026-11-13：**Tower 实例求解阻塞已修复。最终同一 Relax 二进制已通过 gateway 健康检查，并完成桌面打开项目、实际 API 请求及模型对话回传。** 仍有未执行依赖路径和跨 crate 泛型共享的兼容缺口，不能宣称全生态兼容。

## Tower 修复与最终联动

失败诊断确认：并非某个类型不满足 `Send`，而是一条有限的自动 trait 证明链在 128 层耗尽预算。`vw-agent` 已声明 `recursion_limit = "256"`，其应用检查通过；Relax 却把库内代码生成也统一放入 CLI 入口的 128 层环境。纯标准库双 crate 最小复现在原生 Rust 成功、旧 Relax 生成虚表时失败。

修复为实例保留代码生成 crate，区分非泛型定义库与需要调用方上下文的实例。`SlotKey`、虚表、后台 lowering、延迟请求、CTFE 证书和持久化缓存携带这一身份；相应 Unpin/析构结果缓存也隔离。磁盘 codec 升级为 35，包含稳定 crate 身份与依赖；没有取全局最大值、提高全局限制、跳过路由或放宽 trait 条件。临时深栈追踪已移除，保留失败阶段与具体义务诊断。

最终二进制 SHA-256：`58a16d94f7f1efda643180bb4c4c7fb5743e8514030f222acd755f190c0ab646`。

| 验收 | 实际证据 |
| --- | --- |
| gateway | 最终进程 PID 47505 实际监听 `127.0.0.1:42617`；`GET /v1/health` 返回 HTTP 200、`status: ok`，响应 PID 与监听者一致、build_id 与二进制 SHA 一致 |
| 桌面 | 同 SHA 二进制经临时 `.app` 启动，CUA 确认真实主窗口、项目打开、终端和设置渲染 |
| 实际联动 | 独立桌面配置连接本地验证代理 `56374 → 42617`；真实健康、配置、工具、会话、项目、文件、worktree、Git 接口成功响应。代理仅转发并记录方法/路径/状态码，不模拟后端、不记录认证头或载荷 |
| 对话 | 从桌面发送 `Reply exactly RELAX_LINK_OK. Do not use tools or modify files.`；`POST /v1/chat/stream` HTTP 200，界面最终显示准确模型回复 `RELAX_LINK_OK` 且运行完成 |
| 回归 | 最终二进制原 16 个 fixture 共 38 个 JIT 测试全部通过；另有 async Send 10、solver 4、TypeCycle 1、缓存 codec 1 个聚焦测试通过 |
| Clippy | ShiJu 当前工作区 `cargo clippy --all-targets --all-features -- -D warnings` 通过；Relax 同命令被既有 `makepad-gpu-trace` lint 阻断，定向检查又被既有 build-script/base lint 阻断 |

双 crate 原失败用例在最终版冷启动与缓存重启中通过；低预算 inline、`Rc` 非 Send、无限义务仍按原生结果拒绝。两个依赖上游已有泛型单态实例共享的用例仍然失败，旧版也失败，未把这些结果计为通过。

首次并发创建项目触发过数据库唯一键竞争。源码存在独立 `get` 后 `create` 的窗口；现有原生二进制的 5 组并发对照均复现 1 成功、4 唯一键失败，随后单次请求成功。原生二进制不是本次快照的精确构建，因此该对照只作支持性证据。本次保留该业务问题，没有通过修改业务源码掩盖它；最终版重新打开已存在的临时项目时 6 个 resolve 请求均成功。

成功边界是本机这条实际运行链路。全依赖后台检查仍有 520 项兼容诊断；跨 crate 上游泛型共享、未执行路径、Linux/Windows、通用工具执行尚未完整验证。界面显示对话处理 46 秒包含网络、模型和首次 JIT，不能据此计算编译提速。本次测试进程和占用锁已清理，临时测试项目/会话证据保留；没有修改全局 Cargo 配置或对外推送。

最终补丁、哈希、完整命令和检查数据见 [补丁说明](../../patches/relax/README.md) 与 [Tower 验证数据](evidence/relax-tower-2026-11-13.json)。构建验证后期同时设置 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 到领取的隔离槽；前期仅设置 target-dir 仍沿用了仓库 build-dir，证据中已区分。后续编辑循环计时已完成，结果与适用范围见上方重新实测章节。

## 2026-11-12 历史记录


2026-11-12 在 macOS arm64 上验证并修复：**reqwest 和 Rhai 的原始兼容问题已解决，Relax 已能显示 ShiJu 桌面主界面；gateway 仍因 Tower trait 实例解析错误而无法健康运行。** 目前不能把整个 ShiJu 开发流程替换为 Relax。

## 修复后的验证

可应用、可回滚的源码补丁保存在 [patches/relax](../../patches/relax/README.md)，基线仍为 Relax `0e2a536309c5ee7c7a47d8806d603a9c12b9a586`。先在 Makepad `work` 源码中修复或核对已合入的修复，再同步至本地 Relax；没有修改 ShiJu 业务源码、依赖 features 或全局 Cargo 配置，也没有对外提交代码。

原始阻塞的修复方式：

- **reqwest**：同步上游已有修复，在 `vec!` 展开时按列表元素规则移除为假的 `#[cfg]` 元素。原始 Relax 用现有 `vec_cfg_elements.rs` 用例复现失败，rustc 和修复版均通过。
- **Rhai**：保留宏捕获的 `stmt` 类型与完整 token 边界，避免无分号语句和后续宏调用被连起来解析。没有放宽普通表达式的分号规则；片段标记扩为 `u64`，保留完整 `u32` 偏移空间。

完整启动又暴露出必要的底层缺口，因此补丁还包括：`Weak::into_raw/from_raw`、切片 `split_off/split_off_first`、`Chain: BufRead`、`PhantomPinned: Debug`、函数泛型参数约束内的匿名 `impl Trait` 收集、上游已有的静态项异步闭包 well-formedness 修复，以及 macOS 原生隐藏符号访问器。最后补齐 AArch64 的行首 `#` 汇编注释支持；输入寄存器、`readonly` 等 asm 选项及屏障调用保留，没有绕过 zeroize 或 TLS 证书验证。

| 目标 | 修复后实际结果 | 边界 |
| --- | --- | --- |
| CLI gateway | 自有应用检查 11/11 crates、0 errors；进入 `main`，越过 `libssh2_init`，打印监听地址后，在 Tower 路由实例化时退出 | 没有通过健康检查，不能提供后端服务 |
| 桌面端 shi-ju | 自有应用检查 10/10 crates、0 errors；进入 `main`；最终补丁源码构建经 CUA 确认显示菜单、应用图标、项目卡片和文件夹按钮，已越过启动过渡页 | 仅验证启动与实际渲染；未验证项目打开、对话、设置控件或与 Relax gateway 的联动 |

桌面观察使用临时 `.app` 启动器包裹**同一个修复版 Relax 二进制**，以便 CUA 精确识别测试进程，避免与已有同名 ShiJu 应用混淆。没有替换产品打包逻辑。被观察的最终二进制 SHA-256 与交付的本地运行器一致；验证后测试实例已关闭。图像实际显示来自原有配置/运行环境，不能据此声称后端请求由本次 Relax gateway 成功处理。

最终运行时传入 `--cfg debug_assertions`，对齐用户指定 `cargo run` 的开发条件分支。Relax 默认采用 release 依赖图；不传该标记时，ShiJu 会走不同的本机服务引导逻辑，窗口可能停在“正在启动工作环境”。这只对齐条件编译行为，不是两个编译器所有 profile 参数或优化策略完全一致的证明。本轮没有据此计算完整程序提速倍数。

桌面日志仍有第三方依赖宏与后台检查诊断，例如 `iced_aw` 字体生成宏的相对文件路径，以及部分未调用依赖方法。这些不阻止本次主界面显示，但其他控件或业务路径可能仍触发兼容问题，不能称为全功能通过。

### 验证证据

- 最终运行器执行 16 个现有聚焦 fixture，共 **38 个 JIT 测试通过、0 失败**，覆盖宏边界、嵌套 APIT、调用 binder、Rc、切片、IO 和 asm 注释。
- Rust 原生前端聚焦检查 `cargo test --release -p relax-front parser:: -- --test-threads=1`：**12 个通过**。没有运行全量单元测试。
- 宏表达式必须保持不透明的负向用例在 rustc 和 Relax 中均被拒绝；静态项异步闭包用例旧 Relax 失败，修复版和 rustc 通过。
- macOS 自有小型 C 验证确认隐藏函数、隐藏数据和 weak hidden 函数能通过访问器取得正确值，原符号仍不导出，TLS 不生成访问器。实际新链接库也导出了 `libssh2_init` 对应的地址访问器。
- 修复版 release 构建成功。Relax workspace 的 `cargo clippy --all-targets --all-features -- -D warnings` 被未改动的 `objc-sys` exception 模块和既有 lint 错误阻塞；ShiJu 同命令被其他并行开发内容中的未使用导入/字段/方法阻塞，没有修改无关问题。
- 补丁在基线 index 上 `git apply --cached --check` 通过，在已修复源码上反向应用检查通过。

最终源码构建、逐项 fixture 结果、入口运行结果和清理状态见 [修复验证数据](evidence/relax-fix-2026-11-12.json)。原始日志和本地运行器保留在 `/tmp/shiju-relax-fix-evidence`，源码在 `/tmp/shiju-relax-eval-tool` 与 `/tmp/shiju-relax-fix-makepad`；系统清理临时目录后，可用仓库补丁重建。

### 当时的 gateway 阻塞

```text
tower::util::boxed_clone_sync::BoxCloneSyncService::new
failed to resolve instance for CloneService::clone_box:
  MapFuture<MapIntoResponse<HandlerService<handle_nextcloud_talk_webhook, ...>>, closure#5>
  Request<Body>
Unimplemented (libs/relax/types/src/instance.rs:238)
```

错误发生在路由服务的 trait 实例解析阶段，尚不能从日志判断是哪一个约束失败。现有 `types/src/next_solver/codegen.rs` 把参数规范化、候选选择、嵌套义务求解、impl item 匹配四个阶段都折叠为 `Unimplemented`；上游对应求解器代码与本地基线一致，没有找到可直接同步的修复。

**当时的后续步骤（2026-11-13 已执行）：**先在上述四个错误映射点保留阶段和失败义务；若是嵌套义务失败，保留 `NextSolverError` 的具体 predicate，而不是只返回 `NoSolution`。再针对实际失败的约束制作最小复现并修复，重跑完整入口和健康检查。当前没有猜测是 Send、Sync、Clone 或哪个 Future 约束，也没有跳过路由或放宽 trait 接受条件。

## 完整程序启动验证 修复前

实际执行了用户指定的两条命令，保留默认 features，没有关闭模块或修改业务源码：

```bash
cargo run -p vw-cli --bin shiju -- gateway
cargo run -p vw-desktop --bin shi-ju
```

| 目标 | Cargo 结果 | Relax 结果 |
| --- | --- | --- |
| CLI gateway | 构建约 3 分 38 秒后进入运行；监听 `127.0.0.1:42617`，`GET /v1/health` 返回 HTTP 200、`status: ok` | 约 92.55 秒后退出，exit 1；reqwest 0.12.28 与 Rhai 1.25.1 加载失败，未进入 `main` |
| 桌面端 shi-ju | 构建完成并进入运行，进程持续存活、未见 panic；Cargo 报告 4 分 24 秒，包含等待 CLI 构建锁的时间 | 约 83.60 秒后退出，exit 1；reqwest 0.13.4 加载失败，未进入 `main` |

桌面端的 Cargo 结果确认到进程启动；CUA 无法定位命令行启动的未打包可执行文件，已有多个同名 ShiJu 应用，因此**没有把旧窗口当作本次窗口，也没有确认新窗口渲染**。构建同时出现 3 项既有 worktree settings 死代码警告，本次未修改这些并行开发内容。

Relax 的实际阻塞：

```text
CLI:
rhai-1.25.1/src/eval/chaining.rs:71:9:
expected `;` or `}` after expression (found Ident `defer`)

reqwest-0.12.28/src/async_impl/client.rs:838:33:
removing an expression is not supported in this position

Desktop:
reqwest-0.13.4/src/async_impl/client.rs:837:33:
removing an expression is not supported in this position
```

reqwest 对应 `vec!` 中带 `#[cfg(feature = "http2")]` 的元素；Rhai 对应 `defer! { ... }`。这是本次遇到的 Relax 源码加载/展开兼容性阻塞，Cargo 能编译这些依赖。CLI 的日志显示加载了 666 个 crates、约 444 万行；桌面端 537 个 crates、约 378 万行。两次都明确输出 `run stops before main`，所以这些失败耗时**不是启动提速数据**，也无法测完整程序热更新。

为越过环境问题继续验证，采取了以下步骤：

1. 直接在仓库运行 Relax 时，先被用户目录 `.cargo` 的 rsproxy sparse registry 替换配置阻塞；单独设置 `CARGO_HOME` 仍会读到仓库祖先目录中的配置。
2. 将完整工作区的 tracked 与 untracked 文件复制到 `/tmp/shiju-relax-full-eval-20261112/source`，共 15,878 个文件、约 428 MiB，包含当前未提交修改；没有缩减 workspace 成员、features 或依赖。
3. 使用临时 `CARGO_HOME` 复用原 registry/git 缓存，并仅复制 source registry 配置供委托 Cargo 构建使用，未改全局配置。全程离线。
4. 按 Relax 的明确提示启用 `--allow-c`，由 Cargo 和 C 编译器处理 QuickJS、OpenSSL、libgit2 等依赖，之后才遇到上面的实际源码兼容性错误。

最终运行命令如下；`RELAX` 指向前文固定提交构建的可执行文件：

```bash
cd /tmp/shiju-relax-full-eval-20261112/source
export CARGO_HOME=/tmp/shiju-relax-full-eval-20261112/cargo-home
export CARGO_NET_OFFLINE=true MAKEPAD_RUN_OFFLINE=1
RELAX=/tmp/shiju-relax-eval-tool/target/release/relax
"$RELAX" run --unattended --verbose --allow-c vw-cli --bin shiju -- gateway
"$RELAX" run --unattended --verbose --allow-c vw-desktop --bin shi-ju
```

`--unattended` 使编译错误直接退出，避免测试进程等待编辑；没有启用跳过加载失败的运行方式。失败原因与原始日志摘要保存在 [完整验证数据](evidence/relax-full-2026-11-12.json)，原始日志在 `/tmp/shiju-relax-full-eval-20261112`。本次启动的 gateway 和桌面测试进程在验证后关闭，原有进程不动。

## 样本与环境

- ShiJu 源码快照：`1e687363bf5a7ec4c594156e17e8181059052ee3`；所选 crate 与工作区源码逐文件 SHA-256 比对一致。
- Relax：[`makepad/relax`](https://github.com/makepad/relax/tree/0e2a536309c5ee7c7a47d8806d603a9c12b9a586)，提交 `0e2a536309c5ee7c7a47d8806d603a9c12b9a586`，CLI 显示 `0.1.0 (beta)`。
- 主机：Mac16,7，14 个逻辑 CPU，48 GiB 内存；Rust/Cargo 1.99.1，LLVM 23.1.1。
- 样本：`vw-config-types`，64 个 Rust 文件、10,491 行，含测试；没有其他 ShiJu crate 依赖，包含 serde、schemars 派生宏、chrono-tz、directories、shellexpand 和 tracing。
- 临时工作区只保留该成员，复制原 `Cargo.lock`、`.cargo` 和 profile；裁剪锁文件后没有引入新的依赖版本。原配置的 `opt-level = 1`、`debug = "line-tables-only"` 保留。
- 设置 `CARGO_BUILD_JOBS=4`，离线使用已有依赖源码。所有试验修改仅发生在临时副本，结束后源码哈希全部恢复一致；仓库只新增本报告和数据。

## 兼容性结果

Cargo 与 Relax 执行了同名的 200 个库测试，**逐项结果完全相同：199 个通过，1 个失败**。失败项为 `config::config_tests::config_debug_impl_shows_providers_count`，在 `src/config_tests.rs:374` 断言 Debug 输出包含 `providers_count: 1`。该失败在 Cargo 下稳定复现，本次未修复，也未算作 Relax 新增问题。

后续性能对比显式跳过这一个测试，两边均为 199 个通过。额外验证：

- `relax check vw-config-types --lib` 成功，报告 0 errors。
- 在隔离样本运行 `cargo clippy --all-targets --all-features --offline --locked -- -D warnings` 成功；检查范围为所选 crate。
- 常驻测试中，将 `default_provider_backoff_ms()` 的返回值从 500 改为 501，得到 1 个预期失败；恢复源码后 2 个测试通过。
- 热更新日志确认每次修改安装了 1 个函数，随后 `testverdict` 的 group 递增，排除了仅重放旧结果的情况。

## 实测耗时

单位为秒；除首次运行外均列中位数。每次启动计时包含启动进程、编译和测试；常驻模式从写入源码前计时，直到收到对应 `testverdict`，包含文件检测、热更新和测试。

| 场景 | 次数 | Cargo | Relax | 观察 |
| --- | ---: | ---: | ---: | --- |
| 首次执行全部 200 个测试 | 各 1 | 49.466 | 39.049 | 两边均含已知失败，不能作为全绿基准 |
| 无修改，重新启动并执行 199 个测试 | 各 5 | 0.080 | 0.498 | Cargo 更快 |
| 改函数体，重新启动并执行 199 个测试 | 各 3 | 1.879 | 2.370 | Relax 未提速 |
| 改函数体，执行相同的 2 个可靠性测试 | 各 5 | 2.095 | **0.278** | Relax 保持常驻，约快 **7.5 倍** |

常驻启动到首次测试结果另需 4.149 秒，未混入热更新耗时。5 次热更新分别为 `1.2308 / 0.3666 / 0.2775 / 0.2274 / 0.1526` 秒；Cargo 对应 5 次为 `11.0400 / 1.8224 / 2.0063 / 2.0951 / 2.1969` 秒。

修改的是 `src/reliability.rs` 中 `default_provider_backoff_ms()` 的函数体，使用返回值仍为 500 的不同加法表达式；两边每次都执行相同的两个测试，没有通过减少测试数量获得提速。另做的 501 负向验证确认了行为变更能够进入运行中的函数。

**测量限制：**机器同时存在其他编译任务；样本数量有限，没有严格控制调频与后台负载。首次运行使用已有依赖源码缓存，不含下载与 Relax 工具构建；工具构建另耗时约 166 秒。保留了 Cargo 的增量缓存，未执行 `cargo clean`。7.5 倍只描述本机该函数与两个测试的编辑循环，不能外推到其他 crate、完整 GUI 或运行时吞吐。

## 复现入口

本次工具与样本保留在 `/tmp/shiju-relax-eval-tool` 和 `/tmp/shiju-relax-eval-20261112`，系统清理临时目录后需要重建。原始日志、`measure.py`、`compare.py`、`live_compare.py`、`live_negative.py` 都在后一个目录；可审查计时过程。可持久保存的结果见 [结构化数据](evidence/relax-2026-11-12.json)。

从仓库根目录创建样本：

```bash
python3 - <<'PY'
from pathlib import Path
import shutil, tempfile
repo = Path.cwd()
sample = Path(tempfile.mkdtemp(prefix="shiju-relax-sample-"))
shutil.copytree(repo / "crates/vw-config-types", sample / "crates/vw-config-types")
shutil.copy(repo / "Cargo.lock", sample / "Cargo.lock")
shutil.copytree(repo / ".cargo", sample / ".cargo")
manifest = (repo / "Cargo.toml").read_text()
profiles = manifest[manifest.index("[profile.dev]"):manifest.index("[patch.crates-io]")]
(sample / "Cargo.toml").write_text(
    '[workspace]\nmembers = ["crates/vw-config-types"]\nresolver = "3"\n\n' + profiles
)
print(sample)
PY
```

新建样本需先在样本根目录运行 `cargo metadata --offline --format-version 1`，让 Cargo 裁剪锁文件；随后使用 `--locked`。构建工具时显式指定输出目录，避免当前仓库 `.cargo` 将产物放入共享目录：

```bash
git clone https://github.com/makepad/relax.git /tmp/shiju-relax-tool-repro
git -C /tmp/shiju-relax-tool-repro checkout 0e2a536309c5ee7c7a47d8806d603a9c12b9a586
cargo build --release --manifest-path /tmp/shiju-relax-tool-repro/Cargo.toml \
  --target-dir /tmp/shiju-relax-tool-repro/target -p relax-runner --bin relax -j 4
```

保持工具 checkout 路径不变；从样本的 `crates/vw-config-types` 目录执行：

```bash
export CARGO_NET_OFFLINE=true MAKEPAD_RUN_OFFLINE=1 CARGO_BUILD_JOBS=4
# Cargo 基线；不传过滤器可复现完整 200 项结果。
cargo test -p vw-config-types --lib --offline --locked -- reliability::reliability_tests::
# 另一个终端：启动后仅修改临时副本的函数体，观察 patched 和 testverdict。
/tmp/shiju-relax-tool-repro/target/release/relax test --live vw-config-types \
  --lib -- reliability::reliability_tests::
```

该版本 `relax test` 不接受 `--locked`，本次通过复制锁文件与离线环境保持版本固定。重新启动执行的命令为 `relax test vw-config-types --lib`；要执行其余 199 项，Cargo 与 Relax 都追加 `-- --skip config::config_tests::config_debug_impl_shows_providers_count`。

## 接入建议

先把 `vw-config-types` 的 `relax test --live` 作为可选的本地开发工具，适合反复修改函数体并立即跑测试。保留 Cargo 作为合并检查、Clippy 与发布链路；本次没有改动项目构建配置或添加依赖。

以上是早期样本的接入起点。2026-11-13 已修复 Tower，并验证 gateway 健康、桌面联动与函数体热更新；当前启用步骤以上方“如何在本机获得这个效果”为准。完整业务和所有依赖方法尚未验证，合并检查与发布继续使用 Cargo。

Relax 官方仍明确列出部分 crate 不支持、类型布局等编辑需重启，以及发布继续使用 Cargo 的限制，参见 [固定版本说明](https://github.com/makepad/relax/blob/0e2a536309c5ee7c7a47d8806d603a9c12b9a586/README.md#the-tradeoffs) 与 [已知问题](https://github.com/makepad/relax/blob/0e2a536309c5ee7c7a47d8806d603a9c12b9a586/libs/relax/KNOWN_GAPS.md)。
