# 任务1：全量归属基线

状态：已完成（静态基线完成，编译/行为未验证）。

交付 baseline.md、external-mounts.json、registrations.json、dependencies.json、host-ports-and-globals.json、snapshot.json 和可复查采集脚本。scope.json 只追加外部挂载归属，baseline.json、plan.md、prompt.md 未修改。

静态采集断言成功：1381个原文件、135228行、无集合/行数差异、唯一原文件所有者、所有原文件和任务路径存在。247个外部路径另列，82个孤立候选不冒充编译负担。两个既有缺失默认模块路径交任务17/18按其范围修正，具体位置见 baseline.md；不阻碍本任务归属基线交付。

RustCodeGraph status 可用；explore 通用 Host/fixture 查询结果过宽，模块属性/宏与清单基线使用源码静态扫描核对，不以调用图代替编译注册。

未运行任何 Cargo、单元/实机测试；真实运行注册数未取得。未来验证入口见 baseline.md。下一步按任务2至58承接外部挂载，任务59审查全部 residue 和孤立候选。
