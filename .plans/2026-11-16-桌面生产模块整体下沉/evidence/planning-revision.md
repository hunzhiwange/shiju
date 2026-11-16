# 规划修订依据

用户澄清：宿主尽可能少，2万不是绝对数字。取消原10万与按比例保留的配额，按必要装配职责验收。

原31项只分配139031行，遗漏41249行；本次补齐为43项实施与1项审计，全部1416个src Rust文件均有唯一责任任务；另外589行构建和原生验收源码同样明确归属。未实施生产代码，未运行Cargo或任何测试。

代码导航已读取仓库RustCodeGraph技能；索引status可用，但大范围explore未给出可靠本轮文件定位，规划使用当前源码与静态清单补齐。已有实际入口包括desktop-config的PreferencesMessage、desktop-voice-ui::message::voice::State、desktop-settings-security::computer_control_update，新增收口优先复用这些已有边界。

验收必须逐项解释宿主保留代码的系统资源、生命周期或跨域效果职责，不能因达到某个行数终止，也不能借统计目录、测试改名或删除覆盖达标。
