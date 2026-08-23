#![allow(clippy::expect_used)]

//! 将分散的集成测试模块聚合到一个测试二进制中，统一复用测试后端。
//! 具体用例仍按行为域保存在 `tests/suite/`，本文件不承载测试逻辑。

// Single integration test binary that aggregates all test modules.
// The submodules live in `tests/suite/`.
// 单一集成测试二进制聚合全部测试模块；子模块位于 `tests/suite/`。
mod test_backend;

mod suite;
