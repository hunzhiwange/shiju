//! 测试 PostgreSQL 工作代理列清单与行映射位置索引保持一致。
//! 该契约防止查询列顺序调整后字段被静默读取到错误位置。

use super::WORKER_COLUMNS;
