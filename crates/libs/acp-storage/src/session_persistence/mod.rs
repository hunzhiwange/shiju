//! 会话持久化子系统的模块聚合与公共导出。
//!
//! 本模块负责把会话记录的解析、序列化、索引和仓库操作整合为统一接口，
//! 供 CLI、运行时和恢复逻辑共享使用。
//!
//! # 主要子模块
//!
//! - repository：负责查找、读取、写入和关闭会话记录
//! - parse：负责把磁盘 JSON 解析为内存中的会话结构
//! - serialize：负责把会话结构稳定地写回磁盘
//! - index：负责维护会话索引以提升查询和列表性能

mod index;
mod parse;
mod repository;
mod serialize;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use index::{
    SESSION_INDEX_SCHEMA, SessionIndex, SessionIndexEntry, load_or_rebuild_session_index,
    read_session_index, rebuild_session_index, session_index_path, to_session_index_entry,
    write_session_index,
};
pub use parse::parse_session_record;
pub use repository::{
    DEFAULT_HISTORY_LIMIT, FindSessionByDirectoryWalkOptions, FindSessionOptions,
    SessionRepositoryError, SessionRepositoryResult, absolute_path, close_session,
    find_git_repository_root, find_session, find_session_by_directory_walk, iso_now, list_sessions,
    list_sessions_for_agent, normalize_name, resolve_session_record, write_session_record,
};
pub use serialize::serialize_session_record_for_disk;
