//! 提供周期任务判定、团队有向无环图调度与工作节点选择算法。
//! 这些算法只消费显式快照，不在计算过程中访问或修改持久化状态。

pub mod recurring;

#[cfg(test)]
mod recurring_tests;
