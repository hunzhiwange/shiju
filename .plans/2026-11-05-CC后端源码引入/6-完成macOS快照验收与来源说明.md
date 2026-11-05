# 任务 6: 完成macOS快照验收与来源说明

批次：【批次 6】依赖：批次 5（含全部前置批次）。线性顺序 1 → 2 → 3 → 4 → 5 → 6；每批仅此任务，无并行任务。

状态：未开始

目的：以现有文件完成可独立复核的 macOS 交付证据。

来源任务：`.plans/2026-11-05-CC能力完整融合/1-完整引入独立后端包.md`；具体需求对应见下文。

预计会话范围：只复核前五项、完善来源说明和验收报告；不重复业务实现、不做跨平台测试。预计 20–35 分钟。单次聚焦会话完成；编译等待不扩大实现范围。

## 文件

- 新建：`crates/vw-agent-apps/来源说明.md`、`crates/vw-agent-apps/macOS快照验收.md`。
- 只读复核：`crates/vw-agent-apps/来源文件.json`、`crates/vw-agent-apps/包外引用.json`、`crates/vw-agent-apps/工作区隔离验收.md`、`crates/vw-agent-apps/upstream/`。

## 上下文

- 本项验收的是静态源码引入与主仓隔离，不是原 CC Switch 产品运行验收。
- 母任务 1 完整需求为本次唯一交付边界；API/SDK/Iced 和原生 crate 属母任务 2 以后。

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

文档与静态快照复核不适用失败/通过测试。Cargo metadata 及必要的 Clippy 需遵守槽位规则；没有新 Rust 行为，不添加单元测试。

## 步骤

1. 重新采集 macOS 系统版本 `sw_vers`、`uname -m`，只报告本机实际架构；不推断其他 macOS 架构通过，不要求 Windows/Linux 设备。
2. 运行以下源未变化、目标哈希、静态 include 校验；复核模式和集合相等。upstream 实际文件集合须与 manifest `文件` 的目标集合严格相等，额外文件/遗漏/哈希或可执行模式差异为零。
3. 逐项将母需求 1–5、独立执行边界及用户“只需要验收macOS”映射到当前交付路径/证据；包外引用的生成产物单独列出，不能假称原应用能直接编译。源前后状态变化时报告并停止验收，不擅自回滚源仓或混用不同快照。
4. 在槽位内重新执行 `cargo metadata --no-deps --format-version 1 --all-features` 与 `cargo locate-project --workspace --manifest-path crates/vw-agent-apps/upstream/src-tauri/Cargo.toml --message-format plain`。确认主运行图未加入来源包；审查 Git 差异未出现 API/桌面入口。
5. 查验批次 5 的 Clippy 证据；若其后主 workspace Rust/Cargo 输入未变且命令通过，不重复耗时验证，记录对应提交/输入指纹。发生变化或批次 5 待回归时执行 `cargo clippy --all-targets --all-features -- -D warnings`；仅本次相关问题可修复，无关错误按待回归交接。Cargo 产物进程结束后释放锁。
6. 来源说明.md 写用途、许可与提交、目录布局、准确清单、源基线字段、上述复核命令、生成项恢复入口、未构建/运行上游的边界及回退方式。回退只撤销本项隔离改动和本次新增快照，不给出递归删除未知目录的命令。
7. macOS快照验收.md 写需求映射、实际平台/版本/架构、每类文件数量、源/目标/引用/模式结果、无新入口、Clippy 证据、验证局限和命令退出码。证据是正式交付，不随编号任务文件删除。
8. 子任务全部交付后报告母任务 1 的可核验完成条件；本子任务不修改/删除母任务，尤其在任何项仍“待回归”时不得宣称母任务完成。

## 验证

```bash
python3 - <<'PYCODE'
import hashlib, json, pathlib, subprocess
m=json.loads(pathlib.Path("crates/vw-agent-apps/来源文件.json").read_text())
s=pathlib.Path(m["源目录"])
def git(*args):
    return subprocess.check_output(["git", "-C", str(s), *args]).decode()
assert git("rev-parse", "HEAD").strip() == m["源提交"]
assert git("status", "--porcelain=v1", "--untracked-files=all") == m["工作区状态"]
paths=set(git("ls-files", "-z").split("\0")) - {""}
assert paths == {x["路径"] for x in m["源文件基线"]}
for x in m["源文件基线"]:
    p=s/x["路径"]
    assert p.is_file() and not p.is_symlink(), str(p)
    assert hashlib.sha256(p.read_bytes()).hexdigest() == x["SHA256"], x["路径"]
print("来源未改变", len(paths))
PYCODE
```

```bash
python3 - <<'PYCODE'
import hashlib, json, pathlib
m=json.loads(pathlib.Path("crates/vw-agent-apps/来源文件.json").read_text())
items=m["文件"]
assert len({x["来源路径"] for x in items}) == len(items)
assert len({x["目标路径"] for x in items}) == len(items)
for x in items:
    p=pathlib.Path(x["目标路径"])
    assert p.is_file() and not p.is_symlink(), str(p)
    assert hashlib.sha256(p.read_bytes()).hexdigest() == x["SHA256"], str(p)
print("目标哈希通过", len(items))
PYCODE
```

```bash
python3 - <<'PYCODE'
import pathlib, re
root=pathlib.Path("crates/vw-agent-apps/upstream").resolve()
external=set()
count=0
for p in (root/"src-tauri").rglob("*.rs"):
    for m in re.finditer(r'include_(?:str|bytes)!\s*\(\s*"([^"]+)"', p.read_text()):
        q=(p.parent/m.group(1)).resolve()
        assert q.is_relative_to(root) and q.is_file(), str(q)
        count+=1
        if not q.is_relative_to(root/"src-tauri"):
            external.add(q.relative_to(root).as_posix())
assert len(external) == 9, sorted(external)
print("静态 include 引用通过", count, "包外唯一文件", len(external))
PYCODE
```

另执行上述 metadata/locate-project；Clippy 依据步骤 5 条件复用或重跑。对来源说明中的命令做可复制性检查，文档不得将 Windows/Linux 标成通过；全部平台原始文件仍须保留。

## 完成

证据包含母需求逐项覆盖、macOS 实测环境、零缺失/额外/字节与模式差异、必要跟踪引用已满足、生成项有恢复说明、源未改变、主运行隔离及 Clippy 真实结果。无关错误阻断时保留本任务供回归，不把创建本计划算作实施完成。


## 执行边界

- 只处理本任务文件列出的快照/证据范围。缺依赖时先检查前置交付；不要为突破阻塞修改无关模块、修改来源仓库或扩大为业务移植。
- `plan.md` 与母计划只读，进度/阻塞只写本任务；`prompt.md` 不记录进度。
- 完成后有实际代码、测试、配置或文档变更才使用 `$git-commit`，排除其他任务及工作区变更；只有任务状态更新/删除时不单独提交。
- 有当前完整证据才能标为 `已完成`；无关既有错误使回归不能运行时可为 `已完成，待回归` 并保留任务；无法交付时为 `已阻塞`，记录命令、原因和已检查内容。不把零测试或未验证记为通过。
