//! 记忆契约、后端分类与空实现。

pub mod backend;
pub mod embeddings;
pub mod none;
pub mod traits;

pub use backend::{
    MemoryBackendKind, MemoryBackendProfile, classify_memory_backend, default_memory_backend_key,
    memory_backend_profile, selectable_memory_backends,
};
pub use none::NoneMemory;
pub use traits::{Memory, MemoryCategory, MemoryEntry};
